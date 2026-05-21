// SPDX-License-Identifier: Apache-2.0
//
// SKILL.md YAML front-matter parser.
//
// Port of packages/aphrody-skills/src/frontmatter.ts. Uses `serde_yaml` so
// the full YAML 1.2 subset is supported (the TS version reimplemented a
// custom parser to stay zero-dep; Rust pulls serde_yaml from the workspace).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Optional `od:` (open-design) metadata block recognised by several
/// upstream catalogs (open-design, openclaw, google-labs).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OdMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
}

/// Normalised view of a SKILL.md frontmatter. Matches the TS `FrontMatter`
/// interface field-for-field (snake_case kept for compat with serde JSON
/// output that downstream callers consume).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontMatter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub od: Option<OdMeta>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

/// Split a SKILL.md source into its frontmatter (between two `---` fences)
/// and body. If no opening fence is present, returns `("", src)`.
#[must_use]
pub fn split_frontmatter(src: &str) -> (String, String) {
    // Accept CRLF or LF, optional trailing whitespace on the fence line.
    let trimmed_start = src.trim_start_matches('\u{feff}');
    let after_open = match strip_open_fence(trimmed_start) {
        Some(rest) => rest,
        None => return (String::new(), src.to_string()),
    };
    match find_close_fence(after_open) {
        Some((fm_end, body_start)) => (
            after_open[..fm_end].to_string(),
            after_open[body_start..].to_string(),
        ),
        None => (String::new(), src.to_string()),
    }
}

fn strip_open_fence(s: &str) -> Option<&str> {
    let rest = s.strip_prefix("---")?;
    // Allow trailing spaces, then require a newline.
    let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '\t');
    if let Some(r) = rest.strip_prefix("\r\n") {
        return Some(r);
    }
    if let Some(r) = rest.strip_prefix('\n') {
        return Some(r);
    }
    None
}

/// Returns `(fm_end_offset, body_start_offset)` within `s`.
fn find_close_fence(s: &str) -> Option<(usize, usize)> {
    // Scan line-by-line for a line equal to `---` (with optional trailing ws).
    let mut offset: usize = 0;
    for line in s.split_inclusive('\n') {
        let line_no_eol = line.trim_end_matches('\n').trim_end_matches('\r');
        if line_no_eol.trim_end() == "---" {
            let fm_end = offset.saturating_sub(0);
            let body_start = offset + line.len();
            return Some((fm_end, body_start));
        }
        offset += line.len();
    }
    None
}

/// Parse the YAML frontmatter of a SKILL.md document. Tolerant: returns an
/// empty (default) [`FrontMatter`] if the document has no fence, or if the
/// YAML inside the fence is malformed.
#[must_use]
pub fn parse_frontmatter(src: &str) -> FrontMatter {
    let (fm, _body) = split_frontmatter(src);
    if fm.trim().is_empty() {
        return FrontMatter::default();
    }
    // Two-step parse: first into a free-form Value, then project into our
    // strongly-typed struct. This lets us normalise fields (e.g. collapse
    // whitespace in `description`) and tolerate unknown keys.
    let value: serde_yaml::Value = match serde_yaml::from_str(&fm) {
        Ok(v) => v,
        Err(_) => return FrontMatter::default(),
    };
    project(&value)
}

fn project(v: &serde_yaml::Value) -> FrontMatter {
    let mut out = FrontMatter::default();
    let map = match v {
        serde_yaml::Value::Mapping(m) => m,
        _ => return out,
    };
    for (k, val) in map {
        let key = match k.as_str() {
            Some(s) => s,
            None => continue,
        };
        match key {
            "name" => {
                out.name = val.as_str().map(str::to_string);
            }
            "description" => {
                if let Some(s) = val.as_str() {
                    // TS collapses runs of whitespace into single spaces and trims.
                    let collapsed = collapse_ws(s);
                    out.description = Some(collapsed);
                }
            }
            "license" => {
                out.license = val.as_str().map(str::to_string);
            }
            "when_to_use" => {
                out.when_to_use = val.as_str().map(str::to_string);
            }
            "triggers" => {
                if let serde_yaml::Value::Sequence(seq) = val {
                    out.triggers = seq
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect();
                }
            }
            "od" => {
                if let serde_yaml::Value::Mapping(m) = val {
                    let mut od = OdMeta::default();
                    for (kk, vv) in m {
                        if let (Some(ks), Some(vs)) = (kk.as_str(), vv.as_str()) {
                            match ks {
                                "mode" => od.mode = Some(vs.to_string()),
                                "category" => od.category = Some(vs.to_string()),
                                "upstream" => od.upstream = Some(vs.to_string()),
                                _ => {}
                            }
                        }
                    }
                    out.od = Some(od);
                }
            }
            "metadata" => {
                if let serde_yaml::Value::Mapping(m) = val {
                    for (kk, vv) in m {
                        if let (Some(ks), Some(vs)) = (kk.as_str(), vv.as_str()) {
                            out.metadata.insert(ks.to_string(), vs.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws && !out.is_empty() {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_no_fence_returns_empty_fm() {
        let (fm, body) = split_frontmatter("# title\nhello\n");
        assert!(fm.is_empty());
        assert_eq!(body, "# title\nhello\n");
    }

    #[test]
    fn split_extracts_fm_and_body() {
        let src = "---\nname: foo\n---\n# body\nhello\n";
        let (fm, body) = split_frontmatter(src);
        assert_eq!(fm, "name: foo\n");
        assert_eq!(body, "# body\nhello\n");
    }

    #[test]
    fn parse_basic_fields() {
        let src = "---\nname: my-skill\ndescription: A   skill\nlicense: Apache-2.0\n---\nbody\n";
        let fm = parse_frontmatter(src);
        assert_eq!(fm.name.as_deref(), Some("my-skill"));
        assert_eq!(fm.description.as_deref(), Some("A skill"));
        assert_eq!(fm.license.as_deref(), Some("Apache-2.0"));
    }

    #[test]
    fn parse_triggers_and_od() {
        let src = "---\nname: x\ntriggers:\n  - alpha\n  - beta\nod:\n  mode: design\n  upstream: github.com/foo\n---\nb\n";
        let fm = parse_frontmatter(src);
        assert_eq!(fm.triggers, vec!["alpha".to_string(), "beta".to_string()]);
        let od = fm.od.expect("od");
        assert_eq!(od.mode.as_deref(), Some("design"));
        assert_eq!(od.upstream.as_deref(), Some("github.com/foo"));
    }

    #[test]
    fn parse_tolerates_missing_fence() {
        let fm = parse_frontmatter("no fence here\n");
        assert!(fm.name.is_none());
        assert!(fm.description.is_none());
    }

    #[test]
    fn parse_handles_crlf() {
        let src = "---\r\nname: crlf\r\ndescription: line\r\n---\r\nbody\r\n";
        let fm = parse_frontmatter(src);
        assert_eq!(fm.name.as_deref(), Some("crlf"));
        assert_eq!(fm.description.as_deref(), Some("line"));
    }
}
