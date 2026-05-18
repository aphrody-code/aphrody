// SPDX-License-Identifier: Apache-2.0
//
// Integration tests — exercise the public lib API end-to-end against a
// temp directory laid out like `.claude/skills/`.

use std::fs;

use aphrody_skills_runtime::{
    discover::discover_skills_in_path, parse_frontmatter, split_frontmatter,
    sources::SourceSlug, validate_frontmatter,
};

const SKILL_BODY: &str = "---\nname: alpha\ndescription: An alpha skill that runs alpha things.\nlicense: Apache-2.0\n---\n# Alpha\nBody here.\n";

#[test]
fn end_to_end_discover_parse_validate() {
    let td = tempfile::tempdir().unwrap();
    let root = td.path();
    for name in ["alpha", "beta", "gamma"] {
        let dir = root.join(name);
        fs::create_dir(&dir).unwrap();
        let body = SKILL_BODY.replace("alpha", name);
        fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    // Discover.
    let hits = discover_skills_in_path(root, SourceSlug::ClaudeCode);
    assert_eq!(hits.len(), 3);

    // Parse + validate each one.
    for hit in &hits {
        let text = fs::read_to_string(&hit.skill_md_path).unwrap();
        let (fm_text, body_text) = split_frontmatter(&text);
        assert!(!fm_text.is_empty(), "frontmatter must be non-empty");
        assert!(body_text.contains("Body here."));

        let fm = parse_frontmatter(&text);
        assert_eq!(fm.name.as_deref(), Some(hit.name.as_str()));
        let errs = validate_frontmatter(&fm, Some(&hit.name));
        assert!(errs.is_empty(), "unexpected validation errors: {errs:?}");
    }
}
