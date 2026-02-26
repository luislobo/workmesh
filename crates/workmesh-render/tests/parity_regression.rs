use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use workmesh_render::RenderError;

#[derive(Debug, Deserialize)]
struct FixtureCase {
    tool: String,
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct ErrorFixtureCase {
    tool: String,
    arguments: Value,
    error_kind: String,
    error_contains: String,
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn list_json_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .expect("read fixture directory")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    out.sort();
    out
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

#[test]
fn renderer_output_parity_regression() {
    let root = fixtures_root();
    let cases_dir = root.join("cases");
    let expected_dir = root.join("expected");
    let update = std::env::var("WORKMESH_UPDATE_RENDER_FIXTURES")
        .ok()
        .as_deref()
        == Some("1");

    let mut covered_tools: BTreeSet<String> = BTreeSet::new();

    for case_path in list_json_files(&cases_dir) {
        let case_raw = fs::read_to_string(&case_path).expect("read case file");
        let case: FixtureCase = serde_json::from_str(&case_raw).expect("parse case fixture");
        covered_tools.insert(case.tool.clone());

        let result = workmesh_render::dispatch_tool(&case.tool, &case.arguments)
            .unwrap_or_else(|err| panic!("dispatch failed for {}: {err}", case_path.display()));
        let rendered = result
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("text output missing for {}", case_path.display()));

        let stem = case_path
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("fixture stem");
        let expected_path = expected_dir.join(format!("{stem}.txt"));

        if update {
            fs::write(&expected_path, rendered).expect("write expected fixture");
            continue;
        }

        let expected = fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("missing expected fixture {}", expected_path.display()));

        assert_eq!(
            normalize_newlines(rendered),
            normalize_newlines(&expected),
            "render mismatch for {}",
            stem
        );
    }

    let expected_tools: BTreeSet<String> = [
        "render_table",
        "render_kv",
        "render_stats",
        "render_progress",
        "render_tree",
        "render_diff",
        "render_logs",
        "render_alerts",
        "render_list",
        "render_chart_bar",
        "render_sparkline",
        "render_timeline",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();

    assert_eq!(covered_tools, expected_tools);
}

#[test]
fn renderer_error_contracts() {
    let errors_dir = fixtures_root().join("errors");

    for case_path in list_json_files(&errors_dir) {
        let case_raw = fs::read_to_string(&case_path).expect("read error fixture file");
        let case: ErrorFixtureCase = serde_json::from_str(&case_raw).expect("parse error fixture");

        let err = workmesh_render::dispatch_tool(&case.tool, &case.arguments)
            .expect_err("fixture expected an error");

        match (case.error_kind.as_str(), err) {
            ("invalid_argument", RenderError::InvalidArgument(message)) => {
                assert!(
                    message.contains(&case.error_contains),
                    "expected invalid_argument to contain '{}', got '{}', fixture {}",
                    case.error_contains,
                    message,
                    case_path.display()
                );
            }
            ("not_found", RenderError::NotFound(message)) => {
                assert!(
                    message.contains(&case.error_contains),
                    "expected not_found to contain '{}', got '{}', fixture {}",
                    case.error_contains,
                    message,
                    case_path.display()
                );
            }
            (expected, actual) => {
                panic!(
                    "unexpected error kind for fixture {}: expected {}, got {:?}",
                    case_path.display(),
                    expected,
                    actual
                );
            }
        }
    }
}
