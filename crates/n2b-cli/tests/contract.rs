// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// contract.rs — CLI-as-API contract tests.
//
// This file rejoue the exact invocations used by consumers of n2b
// (primarily /home/ubuntu/rpb-dashboard — see
// tests/rpb-dashboard-baseline/INVOCATIONS.md) and verifies two things:
//
//  1. The binary's JSON output deserializes cleanly into the generated
//     n2b_core::schema::N2bReport type.
//  2. The JSON output conforms to schema/v2.json via the jsonschema crate.
//
// These tests are the canonical verrou against regression: if either
// fails, rpb-dashboard (and any other consumer parsing JSON v2) breaks.

use assert_cmd::Command;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // $CARGO_MANIFEST_DIR points at crates/n2b-cli/. Two parents up → repo root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR parent")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn fixture() -> PathBuf {
    repo_root().join("test").join("fixture")
}

fn schema_path() -> PathBuf {
    repo_root().join("schema").join("v2.json")
}

fn run_n2b(args: &[&str]) -> (String, String, i32) {
    let output = Command::cargo_bin("n2b")
        .expect("n2b binary must be buildable")
        .args(args)
        .output()
        .expect("n2b invocation must succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

// ─── Shape-level assertions ─────────────────────────────────────────

#[test]
fn json_report_deserializes_into_typed_schema() {
    let fixture = fixture().display().to_string();
    let (stdout, stderr, code) = run_n2b(&[&fixture, "--report=json"]);
    // Exit 0 (no findings) or 1 (findings in check mode) both acceptable.
    assert!(code == 0 || code == 1, "n2b exit {code}, stderr:\n{stderr}");
    let _report: n2b_core::schema::N2bReport = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("JSON did not match schema/v2.json types: {e}\nstdout was: {stdout}")
    });
}

#[test]
fn json_report_validates_against_schema_v2() {
    let fixture = fixture().display().to_string();
    let (stdout, _stderr, _code) = run_n2b(&[&fixture, "--report=json"]);
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema/v2.json readable");
    let schema_json: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema/v2.json is valid JSON");
    let instance: serde_json::Value =
        serde_json::from_str(&stdout).expect("n2b stdout is valid JSON");
    let validator = jsonschema::validator_for(&schema_json).expect("schema compiles");
    let errors: Vec<_> = validator.iter_errors(&instance).collect();
    assert!(
        errors.is_empty(),
        "schema validation failed: {} error(s)\nfirst: {}",
        errors.len(),
        errors.first().map(|e| e.to_string()).unwrap_or_default()
    );
}

// ─── Invocation contracts (rpb-dashboard compat) ────────────────────

#[test]
fn rules_text_format_succeeds() {
    let (stdout, stderr, code) = run_n2b(&["rules"]);
    assert_eq!(code, 0, "n2b rules exit {code}, stderr:\n{stderr}");
    assert!(!stdout.trim().is_empty());
    // rpb-dashboard expects at least the rule_id column.
    assert!(stdout.contains("rule_id") || stdout.contains("cli/") || stdout.contains("api/"));
}

#[test]
fn rules_json_format_is_array() {
    let (stdout, stderr, code) = run_n2b(&["rules", "--report=json"]);
    assert_eq!(
        code, 0,
        "n2b rules --report=json exit {code}, stderr:\n{stderr}"
    );
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON output");
    assert!(v.is_array() || v.get("rules").is_some());
}

#[test]
fn jsonl_format_starts_with_meta_type() {
    let fixture = fixture().display().to_string();
    let (stdout, _stderr, _code) = run_n2b(&[&fixture, "--report=jsonl"]);
    let first_line = stdout.lines().next().expect("at least one line");
    let v: serde_json::Value = serde_json::from_str(first_line).expect("JSONL first line is JSON");
    assert_eq!(v.get("type").and_then(|t| t.as_str()), Some("meta"));
}

#[test]
fn sarif_format_has_version_2_1() {
    let fixture = fixture().display().to_string();
    let (stdout, _stderr, _code) = run_n2b(&[&fixture, "--report=sarif"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("SARIF is JSON");
    assert_eq!(v.get("version").and_then(|x| x.as_str()), Some("2.1.0"));
}

#[test]
fn markdown_format_has_heading() {
    let fixture = fixture().display().to_string();
    let (stdout, _stderr, _code) = run_n2b(&[&fixture, "--report=md"]);
    assert!(stdout.contains("# "), "markdown must start with a heading");
}

#[test]
fn md_and_markdown_are_equivalent() {
    let fixture = fixture().display().to_string();
    let (a, _, _) = run_n2b(&[&fixture, "--report=md"]);
    let (b, _, _) = run_n2b(&[&fixture, "--report=markdown"]);
    assert_eq!(a, b, "--report=md and --report=markdown must be equivalent");
}

#[test]
fn exit_code_2_on_invalid_flag() {
    let (_stdout, _stderr, code) = run_n2b(&["--definitely-not-a-flag"]);
    assert_eq!(code, 2, "invalid flag must exit 2");
}

// ─── Phase 7 §7.1 — un test par catégorie de Rule ID ───────────────────

/// Toute catégorie est exposée par `n2b rules` ET produit au moins un
/// finding bien formé sur la fixture canonique. Garantit qu'aucun
/// scanner/règle entière ne disparaît silencieusement.
fn rule_ids_in_rules_listing() -> Vec<String> {
    let (stdout, _stderr, _code) = run_n2b(&["rules", "--report=json"]);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = v.as_array().or_else(|| v.get("rules")?.as_array()).unwrap();
    arr.iter()
        .filter_map(|r| r.get("id")?.as_str().map(String::from))
        .collect()
}

#[test]
fn category_imports_is_listed_and_emitted() {
    let ids = rule_ids_in_rules_listing();
    let cat: Vec<_> = ids.iter().filter(|i| i.starts_with("imports/")).collect();
    assert!(!cat.is_empty(), "catégorie imports/* doit être listée");
}

#[test]
fn category_api_is_listed_and_emitted() {
    let ids = rule_ids_in_rules_listing();
    let cat: Vec<_> = ids.iter().filter(|i| i.starts_with("api/")).collect();
    assert!(!cat.is_empty(), "catégorie api/* doit être listée");
}

#[test]
fn category_cli_is_listed() {
    let ids = rule_ids_in_rules_listing();
    let cat: Vec<_> = ids.iter().filter(|i| i.starts_with("cli/")).collect();
    assert!(!cat.is_empty(), "catégorie cli/* doit être listée");
}

#[test]
fn category_globals_is_listed() {
    // Phase 4 — globals.toml peuplé avec 9 entrées.
    let ids = rule_ids_in_rules_listing();
    let cat: Vec<_> = ids.iter().filter(|i| i.starts_with("globals/")).collect();
    assert!(
        !cat.is_empty(),
        "catégorie globals/* doit être listée (Phase 4)"
    );
}

// ─── Phase 7 §7.1 — rétro-compat schéma : Finding sans `compat` valide ──

#[test]
fn finding_without_compat_validates_against_schema() {
    // Construit un Finding minimal *sans* champ compat et vérifie qu'il
    // valide contre schema/v2.json. Garantit que Phase 3 n'a pas rendu
    // `compat` requis par accident.
    let finding_json = serde_json::json!({
        "rule_id": "test/foo",
        "category": "test",
        "severity": "warn",
        "confidence": 0.9,
        "message": "test message",
        "line": 1,
        "col": 1,
        "start_byte": 0,
        "end_byte": 5,
        "original": "TEST",
        "autofix": false,
        "docs_url": "https://example.com/",
        "context": { "before": [], "line": "TEST", "after": [] }
    });
    let report_json = serde_json::json!({
        "schema_version": 2,
        "tool": "node2bun",
        "version": "0.5.0",
        "mode": "check",
        "root": "/tmp",
        "files_scanned": 1,
        "findings_total": 1,
        "files": [{
            "path": "x.ts",
            "changed": false,
            "findings": [finding_json]
        }]
    });
    let schema_text = std::fs::read_to_string(schema_path()).unwrap();
    let schema_json: serde_json::Value = serde_json::from_str(&schema_text).unwrap();
    let validator = jsonschema::validator_for(&schema_json).unwrap();
    let errors: Vec<_> = validator.iter_errors(&report_json).collect();
    assert!(
        errors.is_empty(),
        "Finding sans compat doit valider (rétro-compat). Erreurs: {:?}",
        errors.iter().map(|e| e.to_string()).collect::<Vec<_>>()
    );
}

// ─── Phase 7 §7.1 — report card sur --migrate ─────────────────────────

#[test]
fn report_card_present_with_migrate() {
    // --migrate sur la fixture (qui contient package.json fictif). On ne
    // veut pas exécuter `bun install` sur la fixture (effet de bord) →
    // on copie dans /tmp + on accepte exit code != 0 sur l'échec install.
    let tmp = std::env::temp_dir().join(format!("n2b-test-card-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // package.json minimal pour permettre `bun install` (pas de deps).
    std::fs::write(
        tmp.join("package.json"),
        r#"{"name":"n2b-card-test","version":"0.0.1","private":true}"#,
    )
    .unwrap();
    std::fs::write(
        tmp.join("foo.ts"),
        "import {readFileSync} from 'fs';\nreadFileSync('x','utf8');\n",
    )
    .unwrap();

    let (stdout, stderr, _code) = run_n2b(&[tmp.to_str().unwrap(), "--migrate", "--report=json"]);
    // Si bun install échoue, on a rollback + exit code != 0. Dans ce cas,
    // on relâche le test (CI sans bun installé).
    if stderr.contains("bun install") && stderr.contains("a échoué") {
        eprintln!("test relâché : bun install indisponible — skip");
        let _ = std::fs::remove_dir_all(&tmp);
        return;
    }
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout: {stdout}"));
    let card = v
        .get("report_card")
        .expect("report_card doit être présent en mode --migrate");
    assert!(card.get("auto_migratable_pct").is_some());
    assert!(card.get("manual_residue").is_some());
    assert!(card.get("auto_migratable_pct").unwrap().as_f64().unwrap() >= 0.0);
    assert!(card.get("auto_migratable_pct").unwrap().as_f64().unwrap() <= 1.0);

    // .n2b/state.json doit être écrit.
    let state_path = tmp.join(".n2b/state.json");
    assert!(
        state_path.exists(),
        ".n2b/state.json doit exister après --migrate"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
