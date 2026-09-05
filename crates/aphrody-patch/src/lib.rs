// SPDX-License-Identifier: Apache-2.0
//! `aphrody-patch` — the LLM-native file-edit patch format.
//!
//! This crate parses and applies the `apply_patch` edit format that models such
//! as the OpenAI Codex family emit:
//!
//! ```text
//! *** Begin Patch
//! *** Update File: src/lib.rs
//! @@ fn main()
//! -    println!("old");
//! +    println!("new");
//! *** End Patch
//! ```
//!
//! It offers three entry points:
//!
//! - [`parse_patch`] for a complete patch document.
//! - [`StreamingPatchParser`] for parsing a patch as it streams in.
//! - [`apply_patch`] to apply parsed hunks over a [`PatchFileSystem`]
//!   (with [`StdFileSystem`] backed by [`std::fs`]).
//!
//! Application locates context with a Unicode-tolerant fuzzy line search
//! ([`seek_sequence`]) so ASCII-authored diffs still apply to source containing
//! typographic dashes, fancy quotes, or non-breaking spaces.
//!
//! The crate is pure (no async runtime, no OS-specific code) and compiles for
//! `wasm32-unknown-unknown`. The format is inspired by OpenAI Codex apply-patch
//! (Apache-2.0), re-implemented here.

mod apply;
mod fs;
mod parser;
mod seek;
mod streaming;

pub use apply::AppliedSummary;
pub use apply::ApplyError;
pub use apply::apply_patch;
pub use fs::PatchFileSystem;
pub use fs::StdFileSystem;
pub use parser::ApplyPatchArgs;
pub use parser::Hunk;
pub use parser::ParseError;
pub use parser::UpdateChunk;
pub use parser::parse_patch;
pub use seek::seek_sequence;
pub use seek::seek_sequence_eof;
pub use streaming::StreamingPatchParser;

use similar::ChangeTag;
use similar::TextDiff;

/// Render a unified-diff-style preview between `old` and `new` line-by-line.
///
/// This is a convenience for surfacing what a patch changed; it is not part of
/// the parse/apply pipeline. Lines are tagged with `+`, `-`, or a leading space
/// for context, mirroring the apply-patch body format.
#[must_use]
pub fn unified_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        out.push(sign);
        out.push_str(change.value());
        if !change.value().ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::io;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// In-memory [`PatchFileSystem`] for hermetic tests.
    #[derive(Default)]
    struct MemFs {
        files: Mutex<HashMap<PathBuf, String>>,
    }

    impl MemFs {
        fn with(files: &[(&str, &str)]) -> Self {
            let map = files
                .iter()
                .map(|(p, c)| (PathBuf::from(p), (*c).to_string()))
                .collect();
            Self {
                files: Mutex::new(map),
            }
        }

        fn get(&self, path: &str) -> Option<String> {
            self.files.lock().unwrap().get(Path::new(path)).cloned()
        }
    }

    impl PatchFileSystem for MemFs {
        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such file"))
        }
        fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
            self.files
                .lock()
                .unwrap()
                .insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }
        fn remove(&self, path: &Path) -> io::Result<()> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }
        fn exists(&self, path: &Path) -> bool {
            self.files.lock().unwrap().contains_key(path)
        }
    }

    #[test]
    fn round_trip_add_file() {
        let fs = MemFs::default();
        let args = parse_patch("*** Begin Patch\n*** Add File: a/b.txt\n+one\n+two\n*** End Patch")
            .unwrap();
        let summary = apply_patch(&args, &fs).unwrap();
        assert_eq!(summary.added, vec![PathBuf::from("a/b.txt")]);
        assert_eq!(fs.get("a/b.txt").as_deref(), Some("one\ntwo\n"));
    }

    #[test]
    fn round_trip_delete_file() {
        let fs = MemFs::with(&[("gone.txt", "bye\n")]);
        let args = parse_patch("*** Begin Patch\n*** Delete File: gone.txt\n*** End Patch").unwrap();
        let summary = apply_patch(&args, &fs).unwrap();
        assert_eq!(summary.deleted, vec![PathBuf::from("gone.txt")]);
        assert!(!fs.exists(Path::new("gone.txt")));
    }

    #[test]
    fn round_trip_update_file() {
        let fs = MemFs::with(&[("src/lib.rs", "fn main() {\n    old();\n    keep();\n}\n")]);
        let args = parse_patch(concat!(
            "*** Begin Patch\n",
            "*** Update File: src/lib.rs\n",
            "@@ fn main() {\n",
            "-    old();\n",
            "+    new();\n",
            "     keep();\n",
            "*** End Patch",
        ))
        .unwrap();
        apply_patch(&args, &fs).unwrap();
        assert_eq!(
            fs.get("src/lib.rs").as_deref(),
            Some("fn main() {\n    new();\n    keep();\n}\n")
        );
    }

    #[test]
    fn round_trip_move_file() {
        let fs = MemFs::with(&[("old.rs", "a();\n")]);
        let args = parse_patch(
            "*** Begin Patch\n\
             *** Update File: old.rs\n\
             *** Move to: new.rs\n\
             @@\n\
             -a();\n\
             +b();\n\
             *** End Patch",
        )
        .unwrap();
        let summary = apply_patch(&args, &fs).unwrap();
        assert!(!fs.exists(Path::new("old.rs")));
        assert_eq!(fs.get("new.rs").as_deref(), Some("b();\n"));
        assert_eq!(summary.modified, vec![PathBuf::from("new.rs")]);
        assert_eq!(summary.deleted, vec![PathBuf::from("old.rs")]);
    }

    #[test]
    fn fuzzy_unicode_dash_applies() {
        // Source contains an EN DASH; the patch removes it using ASCII '-'.
        let fs = MemFs::with(&[("note.txt", "title \u{2013} subtitle\nrest\n")]);
        let args = parse_patch(concat!(
            "*** Begin Patch\n",
            "*** Update File: note.txt\n",
            "@@\n",
            "-title - subtitle\n",
            "+TITLE\n",
            " rest\n",
            "*** End Patch",
        ))
        .unwrap();
        apply_patch(&args, &fs).unwrap();
        assert_eq!(fs.get("note.txt").as_deref(), Some("TITLE\nrest\n"));
    }

    #[test]
    fn end_of_file_anchor() {
        let fs = MemFs::with(&[("f.txt", "a\nb\nc\n")]);
        let args = parse_patch(concat!(
            "*** Begin Patch\n",
            "*** Update File: f.txt\n",
            "@@\n",
            " c\n",
            "+d\n",
            "*** End of File\n",
            "*** End Patch",
        ))
        .unwrap();
        apply_patch(&args, &fs).unwrap();
        assert_eq!(fs.get("f.txt").as_deref(), Some("a\nb\nc\nd\n"));
    }

    #[test]
    fn multi_chunk_update() {
        let fs = MemFs::with(&[("m.txt", "1\n2\n3\n4\n5\n6\n")]);
        let args = parse_patch(
            "*** Begin Patch\n\
             *** Update File: m.txt\n\
             @@\n\
             -2\n\
             +TWO\n\
             @@\n\
             -5\n\
             +FIVE\n\
             *** End Patch",
        )
        .unwrap();
        apply_patch(&args, &fs).unwrap();
        assert_eq!(fs.get("m.txt").as_deref(), Some("1\nTWO\n3\n4\nFIVE\n6\n"));
    }

    #[test]
    fn add_file_that_exists_errors() {
        let fs = MemFs::with(&[("x.txt", "here\n")]);
        let args = parse_patch("*** Begin Patch\n*** Add File: x.txt\n+new\n*** End Patch").unwrap();
        let err = apply_patch(&args, &fs).unwrap_err();
        assert!(matches!(err, ApplyError::FileExists(_)));
    }

    #[test]
    fn delete_missing_file_errors() {
        let fs = MemFs::default();
        let args = parse_patch("*** Begin Patch\n*** Delete File: nope.txt\n*** End Patch").unwrap();
        let err = apply_patch(&args, &fs).unwrap_err();
        assert!(matches!(err, ApplyError::MissingFile(_)));
    }

    #[test]
    fn context_not_found_errors() {
        let fs = MemFs::with(&[("c.txt", "alpha\nbeta\n")]);
        let args = parse_patch(
            "*** Begin Patch\n\
             *** Update File: c.txt\n\
             @@\n\
             -gamma\n\
             +delta\n\
             *** End Patch",
        )
        .unwrap();
        let err = apply_patch(&args, &fs).unwrap_err();
        assert!(matches!(err, ApplyError::ContextNotFound { .. }));
    }

    #[test]
    fn missing_marker_parse_error() {
        assert!(matches!(parse_patch("nope"), Err(ParseError::InvalidPatch(_))));
    }

    #[test]
    fn streaming_equivalent_to_direct() {
        let patch = "*** Begin Patch\n\
             *** Add File: a.txt\n\
             +x\n\
             *** Update File: b.txt\n\
             @@\n\
             -p\n\
             +q\n\
             *** End Patch";
        let direct = parse_patch(patch).unwrap();
        let mut parser = StreamingPatchParser::new();
        // Push in two uneven slices.
        parser.push(&patch[..30]).unwrap();
        parser.push(&patch[30..]).unwrap();
        let streamed = parser.finish().unwrap();
        assert_eq!(direct, streamed);
    }

    #[test]
    fn unified_diff_renders_changes() {
        let diff = unified_diff("a\nb\nc\n", "a\nB\nc\n");
        assert!(diff.contains("-b\n"));
        assert!(diff.contains("+B\n"));
        assert!(diff.contains(" a\n"));
    }

    #[test]
    fn std_filesystem_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested/created.txt");
        let fs = StdFileSystem;
        let args = ApplyPatchArgs {
            hunks: vec![Hunk::AddFile {
                path: target.clone(),
                contents: "content\n".to_string(),
            }],
        };
        apply_patch(&args, &fs).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "content\n");
    }
}
