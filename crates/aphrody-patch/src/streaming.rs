// SPDX-License-Identifier: Apache-2.0
//! Incremental patch parser that accepts the patch in arbitrary chunks.
//!
//! [`StreamingPatchParser`] produces the same hunks as [`crate::parse_patch`]
//! for any input split into pieces, which lets a UI render hunks as a model
//! streams its tool call.

use std::path::PathBuf;

use crate::parser::ADD_FILE_MARKER;
use crate::parser::ApplyPatchArgs;
use crate::parser::BEGIN_PATCH_MARKER;
use crate::parser::CHANGE_CONTEXT_MARKER;
use crate::parser::DELETE_FILE_MARKER;
use crate::parser::EMPTY_CHANGE_CONTEXT_MARKER;
use crate::parser::END_PATCH_MARKER;
use crate::parser::EOF_MARKER;
use crate::parser::Hunk;
use crate::parser::MOVE_TO_MARKER;
use crate::parser::ParseError;
use crate::parser::UPDATE_FILE_MARKER;
use crate::parser::UpdateChunk;

use Hunk::AddFile;
use Hunk::DeleteFile;
use Hunk::UpdateFile;
use ParseError::InvalidHunk;
use ParseError::InvalidPatch;

const ENVIRONMENT_ID_MARKER: &str = "*** Environment ID: ";

/// Incremental parser for the `apply_patch` format.
///
/// Feed bytes via [`StreamingPatchParser::push`] (any chunking is fine) and
/// call [`StreamingPatchParser::finish`] once the stream ends to obtain the
/// fully validated [`ApplyPatchArgs`].
#[derive(Debug, Default, Clone)]
pub struct StreamingPatchParser {
    line_buffer: String,
    mode: Mode,
    hunks: Vec<Hunk>,
    line_number: usize,
}

#[derive(Debug, Default, Clone, Copy)]
enum Mode {
    #[default]
    NotStarted,
    StartedPatch,
    AddFile,
    DeleteFile,
    UpdateFile {
        hunk_line_number: usize,
    },
    EndedPatch,
}

impl StreamingPatchParser {
    /// Create an empty parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the hunks parsed so far.
    #[must_use]
    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }

    /// Feed the next slice of patch text. Complete lines are parsed eagerly; an
    /// incomplete trailing line is buffered until the next `push`/`finish`.
    ///
    /// # Errors
    /// Returns [`ParseError`] as soon as a malformed line is encountered.
    pub fn push(&mut self, chunk: &str) -> Result<(), ParseError> {
        for ch in chunk.chars() {
            if ch == '\n' {
                let mut line = std::mem::take(&mut self.line_buffer);
                line.truncate(line.strip_suffix('\r').map_or(line.len(), str::len));
                self.line_number += 1;
                self.process_line(&line)?;
            } else {
                self.line_buffer.push(ch);
            }
        }
        Ok(())
    }

    /// Finalise the stream and return the parsed patch.
    ///
    /// # Errors
    /// Returns [`ParseError`] when the stream did not terminate with
    /// `*** End Patch` or when a buffered final line is malformed.
    pub fn finish(mut self) -> Result<ApplyPatchArgs, ParseError> {
        if !self.line_buffer.is_empty() {
            let line = std::mem::take(&mut self.line_buffer);
            self.line_number += 1;
            if line.trim() == END_PATCH_MARKER {
                self.ensure_update_hunk_not_empty(line.trim())?;
                self.mode = Mode::EndedPatch;
            } else {
                self.process_line(&line)?;
            }
        }

        if !matches!(self.mode, Mode::EndedPatch) {
            return Err(InvalidPatch(
                "The last line of the patch must be '*** End Patch'".to_string(),
            ));
        }

        Ok(ApplyPatchArgs { hunks: self.hunks })
    }

    fn ensure_update_hunk_not_empty(&self, line: &str) -> Result<(), ParseError> {
        if let Some(UpdateFile { path, chunks, .. }) = self.hunks.last() {
            if chunks.is_empty() {
                if let Mode::UpdateFile { hunk_line_number } = self.mode {
                    return Err(InvalidHunk {
                        message: format!("Update file hunk for path '{}' is empty", path.display()),
                        line_number: hunk_line_number,
                    });
                }
            }
            if chunks
                .last()
                .is_some_and(|c| c.old_lines.is_empty() && c.new_lines.is_empty())
            {
                if line == END_PATCH_MARKER {
                    return Err(InvalidHunk {
                        message: "Update hunk does not contain any lines".to_string(),
                        line_number: self.line_number,
                    });
                }
                return Err(InvalidHunk {
                    message: format!(
                        "Unexpected line found in update hunk: '{line}'. Every line should start \
                         with ' ' (context line), '+' (added line), or '-' (removed line)"
                    ),
                    line_number: self.line_number,
                });
            }
        }
        Ok(())
    }

    /// Handle a hunk header / end marker. Returns `Ok(true)` when the line was
    /// consumed as such.
    fn handle_hunk_header(&mut self, trimmed: &str) -> Result<bool, ParseError> {
        if trimmed == END_PATCH_MARKER {
            self.ensure_update_hunk_not_empty(trimmed)?;
            self.mode = Mode::EndedPatch;
            return Ok(true);
        }
        if let Some(path) = trimmed.strip_prefix(ADD_FILE_MARKER) {
            self.ensure_update_hunk_not_empty(trimmed)?;
            self.hunks.push(AddFile {
                path: PathBuf::from(path),
                contents: String::new(),
            });
            self.mode = Mode::AddFile;
            return Ok(true);
        }
        if let Some(path) = trimmed.strip_prefix(DELETE_FILE_MARKER) {
            self.ensure_update_hunk_not_empty(trimmed)?;
            self.hunks.push(DeleteFile {
                path: PathBuf::from(path),
            });
            self.mode = Mode::DeleteFile;
            return Ok(true);
        }
        if let Some(path) = trimmed.strip_prefix(UPDATE_FILE_MARKER) {
            self.ensure_update_hunk_not_empty(trimmed)?;
            self.hunks.push(UpdateFile {
                path: PathBuf::from(path),
                move_to: None,
                chunks: Vec::new(),
            });
            self.mode = Mode::UpdateFile {
                hunk_line_number: self.line_number,
            };
            return Ok(true);
        }
        Ok(false)
    }

    fn process_line(&mut self, line: &str) -> Result<(), ParseError> {
        let trimmed = line.trim();
        match self.mode {
            Mode::NotStarted => {
                if trimmed == BEGIN_PATCH_MARKER {
                    self.mode = Mode::StartedPatch;
                    return Ok(());
                }
                Err(InvalidPatch(
                    "The first line of the patch must be '*** Begin Patch'".to_string(),
                ))
            }
            Mode::StartedPatch => {
                if line.starts_with(ENVIRONMENT_ID_MARKER) {
                    return Ok(());
                }
                if self.handle_hunk_header(trimmed)? {
                    return Ok(());
                }
                Err(self.invalid_header(trimmed))
            }
            Mode::AddFile => {
                if self.handle_hunk_header(trimmed)? {
                    return Ok(());
                }
                if let Some(line_to_add) = line.strip_prefix('+') {
                    if let Some(AddFile { contents, .. }) = self.hunks.last_mut() {
                        contents.push_str(line_to_add);
                        contents.push('\n');
                        return Ok(());
                    }
                }
                Err(self.invalid_header(trimmed))
            }
            Mode::DeleteFile => {
                if self.handle_hunk_header(trimmed)? {
                    return Ok(());
                }
                Err(self.invalid_header(trimmed))
            }
            Mode::UpdateFile { .. } => self.process_update_line(line),
            Mode::EndedPatch => Ok(()),
        }
    }

    fn process_update_line(&mut self, line: &str) -> Result<(), ParseError> {
        let update_line = line.trim_end();
        if self.handle_hunk_header(update_line)? {
            return Ok(());
        }

        let line_number = self.line_number;
        let Some(UpdateFile {
            move_to, chunks, ..
        }) = self.hunks.last_mut()
        else {
            return Err(self.unexpected_update_line(line));
        };

        if chunks.is_empty()
            && move_to.is_none()
            && let Some(dest) = update_line.strip_prefix(MOVE_TO_MARKER)
        {
            *move_to = Some(PathBuf::from(dest));
            return Ok(());
        }

        let is_context_marker =
            update_line == EMPTY_CHANGE_CONTEXT_MARKER || update_line.starts_with(CHANGE_CONTEXT_MARKER);
        if is_context_marker
            && chunks
                .last()
                .is_some_and(|c| c.old_lines.is_empty() && c.new_lines.is_empty())
        {
            return Err(InvalidHunk {
                message: format!(
                    "Unexpected line found in update hunk: '{line}'. Every line should start with \
                     ' ' (context line), '+' (added line), or '-' (removed line)"
                ),
                line_number,
            });
        }

        if update_line == EMPTY_CHANGE_CONTEXT_MARKER {
            chunks.push(new_empty_chunk(None));
            return Ok(());
        }
        if let Some(ctx) = update_line.strip_prefix(CHANGE_CONTEXT_MARKER) {
            chunks.push(new_empty_chunk(Some(ctx.to_string())));
            return Ok(());
        }

        if update_line == EOF_MARKER {
            if chunks
                .last()
                .is_some_and(|c| c.old_lines.is_empty() && c.new_lines.is_empty())
            {
                return Err(InvalidHunk {
                    message: "Update hunk does not contain any lines".to_string(),
                    line_number,
                });
            }
            if let Some(chunk) = chunks.last_mut() {
                chunk.is_eof = true;
            }
            return Ok(());
        }

        if line.is_empty() {
            ensure_chunk(chunks);
            if let Some(chunk) = chunks.last_mut() {
                chunk.old_lines.push(String::new());
                chunk.new_lines.push(String::new());
            }
            return Ok(());
        }
        if let Some(text) = line.strip_prefix(' ') {
            ensure_chunk(chunks);
            if let Some(chunk) = chunks.last_mut() {
                chunk.old_lines.push(text.to_string());
                chunk.new_lines.push(text.to_string());
            }
            return Ok(());
        }
        if let Some(text) = line.strip_prefix('+') {
            ensure_chunk(chunks);
            if let Some(chunk) = chunks.last_mut() {
                chunk.new_lines.push(text.to_string());
            }
            return Ok(());
        }
        if let Some(text) = line.strip_prefix('-') {
            ensure_chunk(chunks);
            if let Some(chunk) = chunks.last_mut() {
                chunk.old_lines.push(text.to_string());
            }
            return Ok(());
        }

        if chunks
            .last()
            .is_some_and(|c| !c.old_lines.is_empty() || !c.new_lines.is_empty())
        {
            return Err(InvalidHunk {
                message: format!(
                    "Expected update hunk to start with a @@ context marker, got: '{line}'"
                ),
                line_number,
            });
        }

        Err(InvalidHunk {
            message: format!(
                "Unexpected line found in update hunk: '{line}'. Every line should start with \
                 ' ' (context line), '+' (added line), or '-' (removed line)"
            ),
            line_number,
        })
    }

    fn invalid_header(&self, trimmed: &str) -> ParseError {
        InvalidHunk {
            message: format!(
                "'{trimmed}' is not a valid hunk header. Valid hunk headers: \
                 '*** Add File: {{path}}', '*** Delete File: {{path}}', '*** Update File: {{path}}'"
            ),
            line_number: self.line_number,
        }
    }

    fn unexpected_update_line(&self, line: &str) -> ParseError {
        InvalidHunk {
            message: format!(
                "Unexpected line found in update hunk: '{line}'. Every line should start with \
                 ' ' (context line), '+' (added line), or '-' (removed line)"
            ),
            line_number: self.line_number,
        }
    }
}

fn new_empty_chunk(context_marker: Option<String>) -> UpdateChunk {
    UpdateChunk {
        context_marker,
        old_lines: Vec::new(),
        new_lines: Vec::new(),
        is_eof: false,
    }
}

fn ensure_chunk(chunks: &mut Vec<UpdateChunk>) {
    if chunks.is_empty() {
        chunks.push(new_empty_chunk(None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_patch;
    use pretty_assertions::assert_eq;

    fn stream_in_chars(patch: &str) -> Result<ApplyPatchArgs, ParseError> {
        let mut parser = StreamingPatchParser::new();
        for ch in patch.chars() {
            parser.push(&ch.to_string())?;
        }
        parser.finish()
    }

    #[test]
    fn streaming_matches_direct_parse() {
        let patch = "\
*** Begin Patch
*** Add File: docs/notes.md
+# Notes
+
+body
*** Update File: src/config.rs
@@ impl Config
-    old: bool,
+    new: bool,
     keep: bool,
*** Delete File: src/legacy.rs
*** Update File: src/main.rs
*** Move to: src/bin/app.rs
@@ fn run()
-    a();
+    b();
*** End Patch";

        let direct = parse_patch(patch).unwrap();
        let streamed = stream_in_chars(patch).unwrap();
        assert_eq!(direct, streamed);
    }

    #[test]
    fn streaming_handles_crlf() {
        let mut parser = StreamingPatchParser::new();
        parser
            .push("*** Begin Patch\r\n*** Update File: file.txt\r\n@@\r\n-old\r\n+new\r\n*** End Patch\r\n")
            .unwrap();
        assert_eq!(
            parser.finish().unwrap().hunks,
            vec![UpdateFile {
                path: PathBuf::from("file.txt"),
                move_to: None,
                chunks: vec![UpdateChunk {
                    context_marker: None,
                    old_lines: vec!["old".to_string()],
                    new_lines: vec!["new".to_string()],
                    is_eof: false,
                }],
            }]
        );
    }

    #[test]
    fn finish_requires_end_patch() {
        let mut parser = StreamingPatchParser::new();
        parser
            .push("*** Begin Patch\n*** Add File: f.txt\n+hi\n")
            .unwrap();
        assert_eq!(
            parser.finish(),
            Err(InvalidPatch(
                "The last line of the patch must be '*** End Patch'".to_string()
            ))
        );
    }

    #[test]
    fn finish_processes_final_line_without_newline() {
        let mut parser = StreamingPatchParser::new();
        parser
            .push("*** Begin Patch\n*** Add File: file.txt\n+hello\n*** End Patch")
            .unwrap();
        assert_eq!(
            parser.finish().unwrap().hunks,
            vec![AddFile {
                path: PathBuf::from("file.txt"),
                contents: "hello\n".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_bad_first_line() {
        let mut parser = StreamingPatchParser::new();
        assert_eq!(
            parser.push("bad\n"),
            Err(InvalidPatch(
                "The first line of the patch must be '*** Begin Patch'".to_string()
            ))
        );
    }

    #[test]
    fn rejects_empty_update_hunk() {
        let mut parser = StreamingPatchParser::new();
        assert_eq!(
            parser.push("*** Begin Patch\n*** Update File: file.txt\n*** End Patch\n"),
            Err(InvalidHunk {
                message: "Update file hunk for path 'file.txt' is empty".to_string(),
                line_number: 2,
            })
        );
    }
}
