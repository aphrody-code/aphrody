// SPDX-License-Identifier: Apache-2.0
//! Parser for the LLM-native `apply_patch` edit format.
//!
//! The grammar (a slightly lenient superset of the OpenAI Codex apply-patch
//! format) looks like:
//!
//! ```text
//! *** Begin Patch
//! *** Add File: path
//! +line
//! *** Delete File: path
//! *** Update File: path
//! *** Move to: path
//! @@ optional context marker
//!  context line
//! -removed line
//! +added line
//! *** End of File
//! *** End Patch
//! ```
//!
//! The parser tolerates leading/trailing whitespace around markers and a
//! lenient heredoc wrapper (`<<EOF ... EOF`) that some models emit when they
//! mistakenly pass the patch as a literal argument rather than via stdin.

use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

pub(crate) const BEGIN_PATCH_MARKER: &str = "*** Begin Patch";
pub(crate) const END_PATCH_MARKER: &str = "*** End Patch";
pub(crate) const ADD_FILE_MARKER: &str = "*** Add File: ";
pub(crate) const DELETE_FILE_MARKER: &str = "*** Delete File: ";
pub(crate) const UPDATE_FILE_MARKER: &str = "*** Update File: ";
pub(crate) const MOVE_TO_MARKER: &str = "*** Move to: ";
pub(crate) const EOF_MARKER: &str = "*** End of File";
pub(crate) const CHANGE_CONTEXT_MARKER: &str = "@@ ";
pub(crate) const EMPTY_CHANGE_CONTEXT_MARKER: &str = "@@";

/// Errors produced while parsing a patch into [`Hunk`]s.
#[derive(Debug, PartialEq, Eq, Error, Clone)]
pub enum ParseError {
    /// The overall patch envelope is malformed (missing `*** Begin Patch` /
    /// `*** End Patch`, empty environment id, ...).
    #[error("invalid patch: {0}")]
    InvalidPatch(String),
    /// A specific hunk is malformed; carries the 1-based line number.
    #[error("invalid hunk at line {line_number}: {message}")]
    InvalidHunk { message: String, line_number: usize },
}

use ParseError::InvalidHunk;
use ParseError::InvalidPatch;

/// A single file operation parsed from a patch.
#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Hunk {
    /// Create a new file with the given contents.
    AddFile { path: PathBuf, contents: String },
    /// Remove an existing file.
    DeleteFile { path: PathBuf },
    /// Edit (and optionally rename) an existing file.
    UpdateFile {
        path: PathBuf,
        /// Destination path when the update also renames the file.
        move_to: Option<PathBuf>,
        /// Ordered chunks; each chunk's context occurs after the previous one.
        chunks: Vec<UpdateChunk>,
    },
}

impl Hunk {
    /// Path affected by this hunk, preferring the move destination for renames.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Hunk::AddFile { path, .. } | Hunk::DeleteFile { path } => path,
            Hunk::UpdateFile {
                move_to: Some(path),
                ..
            }
            | Hunk::UpdateFile {
                path,
                move_to: None,
                ..
            } => path,
        }
    }
}

use Hunk::AddFile;
use Hunk::DeleteFile;
use Hunk::UpdateFile;

/// A contiguous block of changes within an [`Hunk::UpdateFile`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UpdateChunk {
    /// Optional single context line (e.g. a function signature) used to narrow
    /// down where the chunk applies. Comes from a `@@ context` marker.
    pub context_marker: Option<String>,
    /// Lines that must be present (context + removed) before the change.
    pub old_lines: Vec<String>,
    /// Lines that replace `old_lines` (context + added).
    pub new_lines: Vec<String>,
    /// When true, `old_lines` are anchored to the end of the file.
    pub is_eof: bool,
}

/// Both the parsed hunks and the canonical (re-joined) patch text.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ApplyPatchArgs {
    /// Ordered file operations.
    pub hunks: Vec<Hunk>,
}

/// Parse a full patch document into [`ApplyPatchArgs`].
///
/// Tolerates a leading/trailing heredoc wrapper (`<<EOF`, `<<'EOF'`,
/// `<<"EOF"`) when the inner body is a valid patch.
///
/// # Errors
/// Returns [`ParseError`] when the envelope or any hunk is malformed.
pub fn parse_patch(patch: &str) -> Result<ApplyPatchArgs, ParseError> {
    let lines: Vec<&str> = patch.trim().lines().collect();
    let hunk_lines = check_patch_boundaries_lenient(&lines)?;

    let mut remaining: &[&str] = strip_environment_id_preamble(hunk_lines)?;
    // Line 1 is the begin marker; hunk content starts on line 2.
    let mut line_number = if remaining.len() == hunk_lines.len() {
        2
    } else {
        3
    };

    let mut hunks: Vec<Hunk> = Vec::new();
    while !remaining.is_empty() {
        let (hunk, consumed) = parse_one_hunk(remaining, line_number)?;
        hunks.push(hunk);
        line_number += consumed;
        remaining = &remaining[consumed..];
    }

    Ok(ApplyPatchArgs { hunks })
}

/// Drops a leading `*** Environment ID: <id>` line if present, validating that
/// the id is non-empty.
fn strip_environment_id_preamble<'a>(
    hunk_lines: &'a [&'a str],
) -> Result<&'a [&'a str], ParseError> {
    const ENVIRONMENT_ID_MARKER: &str = "*** Environment ID: ";
    let Some(first_line) = hunk_lines.first() else {
        return Ok(hunk_lines);
    };
    let Some(environment_id) = first_line.trim_start().strip_prefix(ENVIRONMENT_ID_MARKER) else {
        return Ok(hunk_lines);
    };
    if environment_id.trim().is_empty() {
        return Err(InvalidPatch(
            "apply_patch environment_id cannot be empty".to_string(),
        ));
    }
    Ok(&hunk_lines[1..])
}

/// Validates the `*** Begin Patch` / `*** End Patch` envelope, peeling off a
/// heredoc wrapper first if one is present. Returns the inner hunk lines.
fn check_patch_boundaries_lenient<'a>(
    lines: &'a [&'a str],
) -> Result<&'a [&'a str], ParseError> {
    match check_patch_boundaries_strict(lines) {
        Ok(inner) => return Ok(inner),
        Err(strict_err) => {
            if let [first, .., last] = lines {
                let is_heredoc_open =
                    *first == "<<EOF" || *first == "<<'EOF'" || *first == "<<\"EOF\"";
                if is_heredoc_open && last.ends_with("EOF") && lines.len() >= 4 {
                    return check_patch_boundaries_strict(&lines[1..lines.len() - 1]);
                }
            }
            Err(strict_err)
        }
    }
}

/// Strict envelope check. Returns the slice between the markers.
fn check_patch_boundaries_strict<'a>(
    lines: &'a [&'a str],
) -> Result<&'a [&'a str], ParseError> {
    let (first, last) = match lines {
        [] => (None, None),
        [only] => (Some(*only), Some(*only)),
        [first, .., last] => (Some(*first), Some(*last)),
    };

    let first = first.map(str::trim);
    let last = last.map(str::trim);

    match (first, last) {
        (Some(f), Some(l)) if f == BEGIN_PATCH_MARKER && l == END_PATCH_MARKER => {
            Ok(&lines[1..lines.len() - 1])
        }
        (Some(f), _) if f != BEGIN_PATCH_MARKER => Err(InvalidPatch(
            "The first line of the patch must be '*** Begin Patch'".to_string(),
        )),
        _ => Err(InvalidPatch(
            "The last line of the patch must be '*** End Patch'".to_string(),
        )),
    }
}

/// Parse a single hunk from the start of `lines`. Returns the hunk and the
/// number of lines consumed.
fn parse_one_hunk(lines: &[&str], line_number: usize) -> Result<(Hunk, usize), ParseError> {
    let first_line = lines[0].trim();

    if let Some(path) = first_line.strip_prefix(ADD_FILE_MARKER) {
        let mut contents = String::new();
        let mut consumed = 1;
        for add_line in &lines[1..] {
            if let Some(line_to_add) = add_line.strip_prefix('+') {
                contents.push_str(line_to_add);
                contents.push('\n');
                consumed += 1;
            } else {
                break;
            }
        }
        return Ok((
            AddFile {
                path: PathBuf::from(path),
                contents,
            },
            consumed,
        ));
    }

    if let Some(path) = first_line.strip_prefix(DELETE_FILE_MARKER) {
        return Ok((
            DeleteFile {
                path: PathBuf::from(path),
            },
            1,
        ));
    }

    if let Some(path) = first_line.strip_prefix(UPDATE_FILE_MARKER) {
        let mut remaining = &lines[1..];
        let mut consumed = 1;

        let move_to = remaining
            .first()
            .and_then(|line| line.strip_prefix(MOVE_TO_MARKER));
        if move_to.is_some() {
            remaining = &remaining[1..];
            consumed += 1;
        }

        let mut chunks = Vec::new();
        while !remaining.is_empty() {
            if remaining[0].trim().is_empty() {
                consumed += 1;
                remaining = &remaining[1..];
                continue;
            }
            if remaining[0].starts_with('*') {
                break;
            }
            let (chunk, chunk_lines) = parse_update_chunk(
                remaining,
                line_number + consumed,
                chunks.is_empty(),
            )?;
            chunks.push(chunk);
            consumed += chunk_lines;
            remaining = &remaining[chunk_lines..];
        }

        if chunks.is_empty() {
            return Err(InvalidHunk {
                message: format!(
                    "Update file hunk for path '{}' is empty",
                    Path::new(path).display()
                ),
                line_number,
            });
        }

        return Ok((
            UpdateFile {
                path: PathBuf::from(path),
                move_to: move_to.map(PathBuf::from),
                chunks,
            },
            consumed,
        ));
    }

    Err(InvalidHunk {
        message: format!(
            "'{first_line}' is not a valid hunk header. Valid hunk headers: \
             '*** Add File: {{path}}', '*** Delete File: {{path}}', '*** Update File: {{path}}'"
        ),
        line_number,
    })
}

/// Parse one update chunk: an optional `@@` context marker followed by
/// context/added/removed lines.
fn parse_update_chunk(
    lines: &[&str],
    line_number: usize,
    allow_missing_context: bool,
) -> Result<(UpdateChunk, usize), ParseError> {
    if lines.is_empty() {
        return Err(InvalidHunk {
            message: "Update hunk does not contain any lines".to_string(),
            line_number,
        });
    }

    let (context_marker, start_index) = if lines[0] == EMPTY_CHANGE_CONTEXT_MARKER {
        (None, 1)
    } else if let Some(context) = lines[0].strip_prefix(CHANGE_CONTEXT_MARKER) {
        (Some(context.to_string()), 1)
    } else {
        if !allow_missing_context {
            return Err(InvalidHunk {
                message: format!(
                    "Expected update hunk to start with a @@ context marker, got: '{}'",
                    lines[0]
                ),
                line_number,
            });
        }
        (None, 0)
    };

    if start_index >= lines.len() {
        return Err(InvalidHunk {
            message: "Update hunk does not contain any lines".to_string(),
            line_number: line_number + 1,
        });
    }

    let mut chunk = UpdateChunk {
        context_marker,
        old_lines: Vec::new(),
        new_lines: Vec::new(),
        is_eof: false,
    };
    let mut parsed = 0usize;

    for line in &lines[start_index..] {
        if *line == EOF_MARKER {
            if parsed == 0 {
                return Err(InvalidHunk {
                    message: "Update hunk does not contain any lines".to_string(),
                    line_number: line_number + 1,
                });
            }
            chunk.is_eof = true;
            parsed += 1;
            break;
        }
        match line.chars().next() {
            None => {
                chunk.old_lines.push(String::new());
                chunk.new_lines.push(String::new());
            }
            Some(' ') => {
                chunk.old_lines.push(line[1..].to_string());
                chunk.new_lines.push(line[1..].to_string());
            }
            Some('+') => chunk.new_lines.push(line[1..].to_string()),
            Some('-') => chunk.old_lines.push(line[1..].to_string()),
            _ => {
                if parsed == 0 {
                    return Err(InvalidHunk {
                        message: format!(
                            "Unexpected line found in update hunk: '{line}'. Every line should \
                             start with ' ' (context line), '+' (added line), or '-' (removed line)"
                        ),
                        line_number: line_number + 1,
                    });
                }
                // Start of the next hunk / marker; stop here.
                break;
            }
        }
        parsed += 1;
    }

    Ok((chunk, parsed + start_index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn rejects_bad_hunk_header() {
        assert_eq!(
            parse_one_hunk(&["bad"], 234),
            Err(InvalidHunk {
                message: "'bad' is not a valid hunk header. Valid hunk headers: \
                          '*** Add File: {path}', '*** Delete File: {path}', \
                          '*** Update File: {path}'"
                    .to_string(),
                line_number: 234,
            })
        );
    }

    #[test]
    fn missing_context_marker_is_rejected_when_not_allowed() {
        assert_eq!(
            parse_update_chunk(&["bad"], 123, false),
            Err(InvalidHunk {
                message: "Expected update hunk to start with a @@ context marker, got: 'bad'"
                    .to_string(),
                line_number: 123,
            })
        );
    }

    #[test]
    fn parses_context_added_removed() {
        assert_eq!(
            parse_update_chunk(
                &[
                    "@@ change_context",
                    "",
                    " context",
                    "-remove",
                    "+add",
                    " context2",
                    "*** End Patch",
                ],
                123,
                false,
            ),
            Ok((
                UpdateChunk {
                    context_marker: Some("change_context".to_string()),
                    old_lines: vec![
                        String::new(),
                        "context".to_string(),
                        "remove".to_string(),
                        "context2".to_string(),
                    ],
                    new_lines: vec![
                        String::new(),
                        "context".to_string(),
                        "add".to_string(),
                        "context2".to_string(),
                    ],
                    is_eof: false,
                },
                6,
            ))
        );
    }

    #[test]
    fn parses_eof_anchor() {
        assert_eq!(
            parse_update_chunk(&["@@", "+line", "*** End of File"], 123, false),
            Ok((
                UpdateChunk {
                    context_marker: None,
                    old_lines: Vec::new(),
                    new_lines: vec!["line".to_string()],
                    is_eof: true,
                },
                3,
            ))
        );
    }

    #[test]
    fn full_patch_with_all_hunk_kinds() {
        let parsed = parse_patch(
            "*** Begin Patch\n\
             *** Add File: path/add.py\n\
             +abc\n\
             +def\n\
             *** Delete File: path/delete.py\n\
             *** Update File: path/update.py\n\
             *** Move to: path/update2.py\n\
             @@ def f():\n\
             -    pass\n\
             +    return 123\n\
             *** End Patch",
        )
        .unwrap();
        assert_eq!(
            parsed.hunks,
            vec![
                AddFile {
                    path: PathBuf::from("path/add.py"),
                    contents: "abc\ndef\n".to_string(),
                },
                DeleteFile {
                    path: PathBuf::from("path/delete.py"),
                },
                UpdateFile {
                    path: PathBuf::from("path/update.py"),
                    move_to: Some(PathBuf::from("path/update2.py")),
                    chunks: vec![UpdateChunk {
                        context_marker: Some("def f():".to_string()),
                        old_lines: vec!["    pass".to_string()],
                        new_lines: vec!["    return 123".to_string()],
                        is_eof: false,
                    }],
                },
            ]
        );
    }

    #[test]
    fn empty_update_hunk_errors() {
        assert_eq!(
            parse_patch(
                "*** Begin Patch\n\
                 *** Update File: test.py\n\
                 *** End Patch"
            ),
            Err(InvalidHunk {
                message: "Update file hunk for path 'test.py' is empty".to_string(),
                line_number: 2,
            })
        );
    }

    #[test]
    fn missing_begin_marker_errors() {
        assert_eq!(
            parse_patch("bad"),
            Err(InvalidPatch(
                "The first line of the patch must be '*** Begin Patch'".to_string()
            ))
        );
    }

    #[test]
    fn missing_end_marker_errors() {
        assert_eq!(
            parse_patch("*** Begin Patch\nbad"),
            Err(InvalidPatch(
                "The last line of the patch must be '*** End Patch'".to_string()
            ))
        );
    }

    #[test]
    fn update_without_explicit_context_marker() {
        let parsed = parse_patch(
            "*** Begin Patch\n*** Update File: file2.py\n import foo\n+bar\n*** End Patch",
        )
        .unwrap();
        assert_eq!(
            parsed.hunks,
            vec![UpdateFile {
                path: PathBuf::from("file2.py"),
                move_to: None,
                chunks: vec![UpdateChunk {
                    context_marker: None,
                    old_lines: vec!["import foo".to_string()],
                    new_lines: vec!["import foo".to_string(), "bar".to_string()],
                    is_eof: false,
                }],
            }]
        );
    }

    #[test]
    fn heredoc_wrapper_is_stripped() {
        let inner = "*** Begin Patch\n*** Add File: foo\n+hi\n*** End Patch";
        for wrapper in ["<<EOF", "<<'EOF'", "<<\"EOF\""] {
            let wrapped = format!("{wrapper}\n{inner}\nEOF\n");
            let parsed = parse_patch(&wrapped).unwrap();
            assert_eq!(
                parsed.hunks,
                vec![AddFile {
                    path: PathBuf::from("foo"),
                    contents: "hi\n".to_string(),
                }]
            );
        }
    }

    #[test]
    fn environment_id_preamble_is_skipped() {
        let parsed = parse_patch(
            "*** Begin Patch\n\
             *** Environment ID: remote\n\
             *** Add File: hello.txt\n\
             +hello\n\
             *** End Patch",
        )
        .unwrap();
        assert_eq!(
            parsed.hunks,
            vec![AddFile {
                path: PathBuf::from("hello.txt"),
                contents: "hello\n".to_string(),
            }]
        );
    }

    #[test]
    fn empty_environment_id_errors() {
        assert_eq!(
            parse_patch(
                "*** Begin Patch\n\
                 *** Environment ID:   \n\
                 *** Add File: hello.txt\n\
                 +hello\n\
                 *** End Patch"
            ),
            Err(InvalidPatch(
                "apply_patch environment_id cannot be empty".to_string()
            ))
        );
    }
}
