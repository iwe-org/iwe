use diwe::config::{Configuration, LibraryOptions, MarkdownOptions};
use indoc::indoc;
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_schema_markdown_output() {
    let temp_dir = setup_workspace();
    let output = run_schema(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let expected = indoc! {"
        | Field  | Types                    | Coverage | Distinct | Values |
        | ------ | ------------------------ | -------- | -------- | --- |
        | status | string (100%)            | 3 (100%) | 2        | draft (2), published (1) |
        | type   | string (100%)            | 3 (100%) | 3        | external (1), hub (1), post (1) |
        | url    | null (50%), string (50%) | 2 (67%)  | 1        | null (1) |
    "};
    assert_eq!(stdout, expected);
}

#[test]
fn test_schema_json_output() {
    let temp_dir = setup_workspace();
    let output = run_schema(&temp_dir, &["-f", "json"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    let fields = parsed.as_array().expect("top-level is array");
    assert_eq!(fields.len(), 3);

    let type_field = fields
        .iter()
        .find(|f| f["field"] == "type")
        .expect("type field");
    assert_eq!(type_field["coverage"]["count"], 3);
    assert_eq!(type_field["coverage"]["percentage"], 100.0);
    assert_eq!(type_field["types"][0]["type"], "string");
}

#[test]
fn test_schema_yaml_output() {
    let temp_dir = setup_workspace();
    let output = run_schema(&temp_dir, &["-f", "yaml"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let parsed: serde_yaml::Value = serde_yaml::from_str(&stdout).expect("Valid YAML");
    let fields = parsed.as_sequence().expect("top-level is sequence");
    assert_eq!(fields.len(), 3);
}

#[test]
fn test_schema_with_filter() {
    let temp_dir = setup_workspace();
    let output = run_schema(&temp_dir, &["--filter", "type: post"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let expected = indoc! {"
        | Field  | Types         | Coverage | Distinct | Values |
        | ------ | ------------- | -------- | -------- | --- |
        | status | string (100%) | 1 (100%) | 1        | published (1) |
        | type   | string (100%) | 1 (100%) | 1        | post (1) |
        | url    | string (100%) | 1 (100%) | 0        | --- |
    "};
    assert_eq!(stdout, expected);
}

#[test]
fn test_schema_field_drilldown() {
    let temp_dir = setup_workspace();
    let output = run_schema(&temp_dir, &["--field", "status", "-f", "json"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    let fields = parsed.as_array().expect("top-level is array");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0]["field"], "status");
}

#[test]
fn test_schema_nested_fields() {
    let temp_dir = setup_workspace_with_nested();
    let output = run_schema(&temp_dir, &["-f", "json"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    let fields = parsed.as_array().expect("top-level is array");

    let names: Vec<&str> = fields
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"engagement"));
    assert!(names.contains(&"engagement.upvotes"));
    assert!(names.contains(&"engagement.comments"));

    let engagement = fields.iter().find(|f| f["field"] == "engagement").unwrap();
    assert_eq!(engagement["types"][0]["type"], "object");
}

#[test]
fn test_schema_field_drilldown_nested() {
    let temp_dir = setup_workspace_with_nested();
    let output = run_schema(&temp_dir, &["--field", "engagement", "-f", "json"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    let fields = parsed.as_array().expect("top-level is array");

    let names: Vec<&str> = fields
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["engagement", "engagement.comments", "engagement.upvotes"]
    );
}

#[test]
fn test_schema_empty_workspace() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe")).unwrap();
    write_config(temp_path);

    let output = run_schema(&temp_dir, &["-f", "json"]);

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    let fields = parsed.as_array().expect("top-level is array");
    assert_eq!(fields.len(), 0);
}

fn setup_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe")).unwrap();
    write_config(temp_path);

    write(
        temp_path.join("post1.md"),
        indoc! {"
            ---
            type: post
            status: published
            url: https://example.com
            ---
            # Post One
        "},
    )
    .unwrap();

    write(
        temp_path.join("hub1.md"),
        indoc! {"
            ---
            type: hub
            status: draft
            ---
            # Hub
        "},
    )
    .unwrap();

    write(
        temp_path.join("ext1.md"),
        indoc! {"
            ---
            type: external
            status: draft
            url: null
            ---
            # External
        "},
    )
    .unwrap();

    temp_dir
}

fn setup_workspace_with_nested() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe")).unwrap();
    write_config(temp_path);

    write(
        temp_path.join("doc1.md"),
        indoc! {"
            ---
            type: post
            engagement:
              upvotes: 10
              comments: 5
            ---
            # Doc One
        "},
    )
    .unwrap();

    write(
        temp_path.join("doc2.md"),
        indoc! {"
            ---
            type: post
            engagement:
              upvotes: null
              comments: 3
            ---
            # Doc Two
        "},
    )
    .unwrap();

    temp_dir
}

fn write_config(path: &std::path::Path) {
    let config = Configuration {
        library: LibraryOptions {
            path: "".to_string(),
            ..Default::default()
        },
        markdown: MarkdownOptions {
            refs_extension: "".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let config_content = toml::to_string(&config).expect("Failed to serialize config");
    write(path.join(".iwe/config.toml"), config_content).unwrap();
}

fn run_schema(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = crate::common::get_iwe_binary_path();
    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path()).arg("schema");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute schema command")
}
