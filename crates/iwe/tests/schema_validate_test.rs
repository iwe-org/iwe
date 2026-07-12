use diwe::config::{Configuration, LibraryOptions, MarkdownOptions, Patterns, SchemaBinding};
use indoc::indoc;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn validate_text_reports_violations_and_hint() {
    let temp_dir = setup_basic();
    let output = run_validate(&temp_dir, &[]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        docs/one: required section \"Two\" is missing
          hint: keep two after one
        docs/one › Extra: unexpected section
    "};
    assert_eq!(stdout, expected);
}

#[test]
fn validate_json_reports_violations() {
    let temp_dir = setup_basic();
    let output = run_validate(&temp_dir, &["-f", "json"]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    assert_eq!(
        parsed,
        json!([
            {
                "key": "docs/one",
                "schema": "alpha",
                "violations": [
                    {
                        "breadcrumb": [],
                        "message": "required section \"Two\" is missing",
                        "hint": "keep two after one",
                        "schemaPath": "/sections/1/minContains",
                        "keyword": "minContains"
                    },
                    {
                        "breadcrumb": ["Extra"],
                        "message": "unexpected section",
                        "hint": null,
                        "schemaPath": "/additionalSections",
                        "keyword": "additionalSections"
                    }
                ]
            }
        ])
    );
}

#[test]
fn validate_clean_document_produces_no_output() {
    let temp_dir = setup_basic();
    let output = run_validate(&temp_dir, &["-k", "docs/clean"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert_eq!(stdout, "");
}

#[test]
fn validate_unbound_document_produces_no_output() {
    let temp_dir = setup_basic();
    let output = run_validate(&temp_dir, &["-k", "other"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert_eq!(stdout, "");
}

#[test]
fn validate_document_bound_to_two_schemas_reports_each() {
    let temp_dir = setup_two_schemas();
    let output = run_validate(&temp_dir, &["-f", "json"]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    assert_eq!(
        parsed,
        json!([
            {
                "key": "docs/one",
                "schema": "alpha",
                "violations": [
                    {
                        "breadcrumb": [],
                        "message": "required section \"One\" is missing",
                        "hint": null,
                        "schemaPath": "/sections/0/minContains",
                        "keyword": "minContains"
                    }
                ]
            },
            {
                "key": "docs/one",
                "schema": "beta",
                "violations": [
                    {
                        "breadcrumb": [],
                        "message": "required section \"Two\" is missing",
                        "hint": null,
                        "schemaPath": "/sections/0/minContains",
                        "keyword": "minContains"
                    }
                ]
            }
        ])
    );
}

#[test]
fn validate_missing_schema_file_exits_two() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe/schemas")).unwrap();
    write_config(temp_path, binding("ghost", "docs/**"));
    create_dir_all(temp_path.join("docs")).unwrap();
    write(temp_path.join("docs/one.md"), "# Body\n").unwrap();

    let output = run_validate(&temp_dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "error: schema 'ghost': .iwe/schemas/ghost.yaml not found\n"
    );
}

#[test]
fn validate_uncompilable_schema_exits_two() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe/schemas")).unwrap();
    write_config(temp_path, binding("alpha", "docs/**"));
    write(
        temp_path.join(".iwe/schemas/alpha.yaml"),
        "sections:\n  - minContains: -1\n",
    )
    .unwrap();
    create_dir_all(temp_path.join("docs")).unwrap();
    write(temp_path.join("docs/one.md"), "# Body\n").unwrap();

    let output = run_validate(&temp_dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(
        stderr,
        "error: schema 'alpha' /sections/0/minContains: minContains must not be negative\n"
    );
}

#[test]
fn validate_without_schemas_produces_no_output() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe")).unwrap();
    write_config(temp_path, HashMap::new());
    write(temp_path.join("other.md"), "# Body\n").unwrap();

    let output = run_validate(&temp_dir, &[]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert_eq!(stdout, "");
}

#[test]
fn validate_reports_block_violation() {
    let temp_dir = setup_blocks();
    let output = run_validate(&temp_dir, &[]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        docs/one › Notes › blocks[1]: unexpected block
    "};
    assert_eq!(stdout, expected);
}

#[test]
fn validate_against_explicit_schema_file_bypasses_config() {
    let temp_dir = setup_explicit_schema();
    let output = run_validate(
        &temp_dir,
        &["-k", "docs/one", "--schema-file", "myschema.yaml"],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        docs/one: required section \"Two\" is missing
        docs/one › Extra: unexpected section
    "};
    assert_eq!(stdout, expected);
}

#[test]
fn explain_prints_the_binding_trace() {
    let temp_dir = setup_explicit_schema();
    let output = run_validate(
        &temp_dir,
        &[
            "-k",
            "docs/one",
            "--schema-file",
            "myschema.yaml",
            "--explain",
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        docs/one  [schema: myschema]
        # One  ->  sections[0]
          paragraph \"text\"  ->  additional
        # Extra  ->  additional

    "};
    assert_eq!(stdout, expected);
}

#[test]
fn validate_against_explicit_schema_file_json_uses_file_stem() {
    let temp_dir = setup_explicit_schema();
    let output = run_validate(
        &temp_dir,
        &[
            "-k",
            "docs/one",
            "--schema-file",
            "myschema.yaml",
            "-f",
            "json",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");
    assert_eq!(
        parsed,
        json!([
            {
                "key": "docs/one",
                "schema": "myschema",
                "violations": [
                    {
                        "breadcrumb": [],
                        "message": "required section \"Two\" is missing",
                        "hint": null,
                        "schemaPath": "/sections/1/minContains",
                        "keyword": "minContains"
                    },
                    {
                        "breadcrumb": ["Extra"],
                        "message": "unexpected section",
                        "hint": null,
                        "schemaPath": "/additionalSections",
                        "keyword": "additionalSections"
                    }
                ]
            }
        ])
    );
}

#[test]
fn validate_against_missing_schema_file_exits_two() {
    let temp_dir = setup_explicit_schema();
    let output = run_validate(
        &temp_dir,
        &["-k", "docs/one", "--schema-file", "ghost.yaml"],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(stderr, "error: schema file not found: ghost.yaml\n");
}

fn setup_explicit_schema() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe")).unwrap();
    create_dir_all(temp_path.join("docs")).unwrap();

    write_config(temp_path, HashMap::new());

    write(
        temp_path.join("myschema.yaml"),
        indoc! {"
            sections:
              - header: { const: One }
              - header: { const: Two }
            additionalSections: false
        "},
    )
    .unwrap();

    write(
        temp_path.join("docs/one.md"),
        indoc! {"
            # One

            text

            # Extra
        "},
    )
    .unwrap();

    temp_dir
}

fn setup_blocks() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe/schemas")).unwrap();
    create_dir_all(temp_path.join("docs")).unwrap();

    write_config(temp_path, binding("alpha", "docs/**"));

    write(
        temp_path.join(".iwe/schemas/alpha.yaml"),
        indoc! {"
            sections:
              - header: { const: Notes }
                blocks:
                  - type: paragraph
                additionalBlocks: false
        "},
    )
    .unwrap();

    write(
        temp_path.join("docs/one.md"),
        indoc! {"
            # Notes

            a paragraph

            - a list item
        "},
    )
    .unwrap();

    temp_dir
}

fn setup_basic() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe/schemas")).unwrap();
    create_dir_all(temp_path.join("docs")).unwrap();

    write_config(temp_path, binding("alpha", "docs/**"));

    write(
        temp_path.join(".iwe/schemas/alpha.yaml"),
        indoc! {"
            sections:
              - header: { const: One }
              - header: { const: Two }
                description: keep two after one
            additionalSections: false
        "},
    )
    .unwrap();

    write(
        temp_path.join("docs/one.md"),
        indoc! {"
            # One

            text

            # Extra
        "},
    )
    .unwrap();

    write(
        temp_path.join("docs/clean.md"),
        indoc! {"
            # One

            # Two
        "},
    )
    .unwrap();

    write(temp_path.join("other.md"), "# Body\n").unwrap();

    temp_dir
}

fn setup_two_schemas() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe/schemas")).unwrap();
    create_dir_all(temp_path.join("docs")).unwrap();

    let mut schemas = HashMap::new();
    schemas.insert(
        "alpha".to_string(),
        SchemaBinding {
            r#match: Patterns::One("docs/**".to_string()),
        },
    );
    schemas.insert(
        "beta".to_string(),
        SchemaBinding {
            r#match: Patterns::One("docs/**".to_string()),
        },
    );
    write_config(temp_path, schemas);

    write(
        temp_path.join(".iwe/schemas/alpha.yaml"),
        "sections:\n  - header: { const: One }\n",
    )
    .unwrap();
    write(
        temp_path.join(".iwe/schemas/beta.yaml"),
        "sections:\n  - header: { const: Two }\n",
    )
    .unwrap();

    write(temp_path.join("docs/one.md"), "# Extra\n").unwrap();

    temp_dir
}

fn binding(name: &str, pattern: &str) -> HashMap<String, SchemaBinding> {
    let mut schemas = HashMap::new();
    schemas.insert(
        name.to_string(),
        SchemaBinding {
            r#match: Patterns::One(pattern.to_string()),
        },
    );
    schemas
}

fn write_config(path: &std::path::Path, schemas: HashMap<String, SchemaBinding>) {
    let config = Configuration {
        library: LibraryOptions {
            path: "".to_string(),
            ..Default::default()
        },
        markdown: MarkdownOptions {
            refs_extension: "".to_string(),
            ..Default::default()
        },
        schemas,
        ..Default::default()
    };
    let config_content = toml::to_string(&config).expect("Failed to serialize config");
    write(path.join(".iwe/config.toml"), config_content).unwrap();
}

fn run_validate(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = crate::common::get_iwe_binary_path();
    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path())
        .arg("schema")
        .arg("validate");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output()
        .expect("Failed to execute schema validate command")
}
