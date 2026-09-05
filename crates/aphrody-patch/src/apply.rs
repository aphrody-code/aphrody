// SPDX-License-Identifier: Apache-2.0
//! Applies parsed patches against a [`PatchFileSystem`].

use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

use crate::fs::PatchFileSystem;
use crate::parser::ApplyPatchArgs;
use crate::parser::Hunk;
use crate::parser::UpdateChunk;
use crate::seek::seek_sequence_eof;

/// Errors produced while applying a patch to a filesystem.
#[derive(Debug, Error)]
pub enum ApplyError {
    /// A filesystem operation failed.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    /// The patch targets a file that should exist but does not.
    #[error("file does not exist: {0}")]
    MissingFile(PathBuf),
    /// The patch targets a path that already exists for an add operation.
    #[error("cannot add file, path already exists: {0}")]
    FileExists(PathBuf),
    /// Context or removed lines from a chunk could not be located in the file.
    #[error("failed to find expected lines in {path}:\n{lines}")]
    ContextNotFound { path: PathBuf, lines: String },
    /// A `@@` context marker could not be located.
    #[error("failed to find context '{context}' in {path}")]
    ContextMarkerNotFound { context: String, path: PathBuf },
}

/// Summary of files touched by a successful [`apply_patch`] call.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct AppliedSummary {
    /// Files created (in patch order).
    pub added: Vec<PathBuf>,
    /// Files modified or renamed (in patch order). Renames record the
    /// destination path.
    pub modified: Vec<PathBuf>,
    /// Files removed (in patch order). Renames record the source path here.
    pub deleted: Vec<PathBuf>,
}

/// Apply every hunk in `args` to `fs`, returning a summary of touched files.
///
/// Update chunks are located with [`crate::seek::seek_sequence`] (exact then
/// fuzzy) and replacements are applied in descending index order so earlier
/// edits never shift later ones.
///
/// # Errors
/// Returns [`ApplyError`] on the first hunk that cannot be applied; hunks
/// processed before the failure may already have been written to `fs`.
pub fn apply_patch<F: PatchFileSystem>(
    args: &ApplyPatchArgs,
    fs: &F,
) -> Result<AppliedSummary, ApplyError> {
    let mut summary = AppliedSummary::default();

    for hunk in &args.hunks {
        match hunk {
            Hunk::AddFile { path, contents } => {
                if fs.exists(path) {
                    return Err(ApplyError::FileExists(path.clone()));
                }
                fs.write(path, contents).map_err(|source| ApplyError::Io {
                    context: format!("failed to add file {}", path.display()),
                    source,
                })?;
                summary.added.push(path.clone());
            }
            Hunk::DeleteFile { path } => {
                if !fs.exists(path) {
                    return Err(ApplyError::MissingFile(path.clone()));
                }
                fs.remove(path).map_err(|source| ApplyError::Io {
                    context: format!("failed to delete file {}", path.display()),
                    source,
                })?;
                summary.deleted.push(path.clone());
            }
            Hunk::UpdateFile {
                path,
                move_to,
                chunks,
            } => {
                let original = fs.read(path).map_err(|source| ApplyError::Io {
                    context: format!("failed to read file to update {}", path.display()),
                    source,
                })?;

                let new_contents = derive_new_contents(&original, path, chunks)?;
                let dest = move_to.as_ref().unwrap_or(path);

                fs.write(dest, &new_contents).map_err(|source| ApplyError::Io {
                    context: format!("failed to write updated file {}", dest.display()),
                    source,
                })?;

                if let Some(move_to) = move_to {
                    if move_to != path {
                        fs.remove(path).map_err(|source| ApplyError::Io {
                            context: format!("failed to remove moved-from file {}", path.display()),
                            source,
                        })?;
                        summary.deleted.push(path.clone());
                    }
                }
                summary.modified.push(dest.clone());
            }
        }
    }

    Ok(summary)
}

/// Compute the new file contents after applying `chunks` to `original`.
fn derive_new_contents(
    original: &str,
    path: &Path,
    chunks: &[UpdateChunk],
) -> Result<String, ApplyError> {
    let mut lines: Vec<String> = original.split('\n').map(String::from).collect();
    // Drop the trailing empty element from a final newline so line counts match
    // standard diff semantics.
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }

    let replacements = compute_replacements(&lines, path, chunks)?;
    let mut new_lines = apply_replacements(lines, &replacements);

    if !new_lines.last().is_some_and(String::is_empty) {
        new_lines.push(String::new());
    }
    Ok(new_lines.join("\n"))
}

/// Each replacement is `(start_index, old_len, new_lines)`.
type Replacement = (usize, usize, Vec<String>);

/// Resolve every chunk into a concrete replacement against `original_lines`.
fn compute_replacements(
    original_lines: &[String],
    path: &Path,
    chunks: &[UpdateChunk],
) -> Result<Vec<Replacement>, ApplyError> {
    let mut replacements: Vec<Replacement> = Vec::new();
    let mut line_index = 0usize;

    for chunk in chunks {
        if let Some(ctx) = &chunk.context_marker {
            match seek_sequence_eof(original_lines, std::slice::from_ref(ctx), line_index, false) {
                Some(idx) => line_index = idx + 1,
                None => {
                    return Err(ApplyError::ContextMarkerNotFound {
                        context: ctx.clone(),
                        path: path.to_path_buf(),
                    });
                }
            }
        }

        if chunk.old_lines.is_empty() {
            // Pure insertion: append before the trailing empty line if present.
            let insertion_idx = if original_lines.last().is_some_and(String::is_empty) {
                original_lines.len() - 1
            } else {
                original_lines.len()
            };
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        let mut pattern: &[String] = &chunk.old_lines;
        let mut new_slice: &[String] = &chunk.new_lines;
        let mut found = seek_sequence_eof(original_lines, pattern, line_index, chunk.is_eof);

        // The final empty string in `old_lines` represents the file's
        // terminating newline, which we stripped from `original_lines`. Retry
        // without it (and without the matching empty tail of `new_lines`).
        if found.is_none() && pattern.last().is_some_and(String::is_empty) {
            pattern = &pattern[..pattern.len() - 1];
            if new_slice.last().is_some_and(String::is_empty) {
                new_slice = &new_slice[..new_slice.len() - 1];
            }
            found = seek_sequence_eof(original_lines, pattern, line_index, chunk.is_eof);
        }

        match found {
            Some(start) => {
                replacements.push((start, pattern.len(), new_slice.to_vec()));
                line_index = start + pattern.len();
            }
            None => {
                return Err(ApplyError::ContextNotFound {
                    path: path.to_path_buf(),
                    lines: chunk.old_lines.join("\n"),
                });
            }
        }
    }

    replacements.sort_by_key(|(idx, _, _)| *idx);
    Ok(replacements)
}

/// Apply replacements to `lines`, in descending index order so earlier edits
/// never shift the positions of later ones.
fn apply_replacements(mut lines: Vec<String>, replacements: &[Replacement]) -> Vec<String> {
    for (start_idx, old_len, new_segment) in replacements.iter().rev() {
        let start_idx = *start_idx;
        for _ in 0..*old_len {
            if start_idx < lines.len() {
                lines.remove(start_idx);
            }
        }
        for (offset, new_line) in new_segment.iter().enumerate() {
            lines.insert(start_idx + offset, new_line.clone());
        }
    }
    lines
}
