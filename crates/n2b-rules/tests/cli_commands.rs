// Copyright 2026 Yohan Pierre
// SPDX-License-Identifier: Apache-2.0

use n2b_rules::cli_commands::apply_cli_rules;

#[test]
fn skips_hash_commented_command() {
    let src = "# npm install\nnpm install\n";
    let (findings, out) = apply_cli_rules("script.sh", src);
    assert_eq!(findings.len(), 1, "only the non-commented line is reported");
    assert_eq!(
        out, "# npm install\nbun install\n",
        "commented line must remain untouched"
    );
}

#[test]
fn skips_slash_commented_command() {
    let src = "// npx tsc\nnpx tsc\n";
    let (findings, out) = apply_cli_rules("note.md", src);
    assert_eq!(findings.len(), 1);
    assert_eq!(out, "// npx tsc\nbunx tsc\n");
}

#[test]
fn rewrites_all_when_no_comment() {
    let src = "npm install\nnpm test\n";
    let (findings, out) = apply_cli_rules("a.sh", src);
    assert_eq!(findings.len(), 2);
    assert_eq!(out, "bun install\nbun test\n");
}

#[test]
fn indented_comment_still_skipped() {
    let src = "    # npm install\nnpm install\n";
    let (_, out) = apply_cli_rules("a.sh", src);
    assert_eq!(out, "    # npm install\nbun install\n");
}
