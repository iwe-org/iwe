use diwe::config::{Configuration, LibraryOptions, MarkdownOptions, Patterns, SchemaBinding};
use indoc::indoc;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

const CLEAN: &str = indoc! {"
    # Summary

    # Tasks
"};

#[test]
fn update_strict_blocks_violating_body_overwrite() {
    let temp = setup();
    let output = run_update(
        temp.path(),
        &["-k", "docs/one", "--content", "# Summary\n", "--strict"],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(
        stderr,
        indoc! {"
            error: --strict blocked the write: schema validation failed
            docs/one: required section 'Tasks' missing
        "}
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        CLEAN
    );
}

#[test]
fn update_strict_allows_clean_body_overwrite() {
    let temp = setup();
    let new_content = "# Summary\n\nmore\n\n# Tasks\n";
    let output = run_update(
        temp.path(),
        &["-k", "docs/one", "--content", new_content, "--strict"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        new_content
    );
}

#[test]
fn update_without_strict_allows_violation() {
    let temp = setup();
    let output = run_update(temp.path(), &["-k", "docs/one", "--content", "# Summary\n"]);

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        "# Summary\n"
    );
}

#[test]
fn update_strict_dry_run_does_not_gate() {
    let temp = setup();
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "docs/one",
            "--content",
            "# Summary\n",
            "--strict",
            "--dry-run",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        CLEAN
    );
}

#[test]
fn update_strict_blocks_violating_section_delete() {
    let temp = setup();
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "docs/one",
            "--delete",
            r#"{ $section: Tasks, expect: 1 }"#,
            "--strict",
            "--expect",
            "1",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(
        stderr,
        indoc! {"
            error: --strict blocked the write: schema validation failed
            docs/one: required section 'Tasks' missing
        "}
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        CLEAN
    );
}

#[test]
fn update_strict_missing_schema_file_exits_two() {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe/schemas")).unwrap();
    write_config(temp.path(), binding("ghost", "docs/**"));
    create_dir_all(temp.path().join("docs")).unwrap();
    write(temp.path().join("docs/one.md"), CLEAN).unwrap();

    let output = run_update(
        temp.path(),
        &["-k", "docs/one", "--content", "# Summary\n", "--strict"],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 output");
    assert_eq!(
        stderr,
        "error: schema 'ghost': .iwe/schemas/ghost.yaml not found\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        CLEAN
    );
}

#[test]
fn delete_strict_allows_clean_removal() {
    let temp = setup();
    let output = run_delete(temp.path(), &["docs/one", "--expect", "1", "--strict"]);

    assert!(output.status.success());
    assert!(!temp.path().join("docs/one.md").exists());
}

fn setup() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe/schemas")).unwrap();
    create_dir_all(temp.path().join("docs")).unwrap();
    write_config(temp.path(), binding("person", "docs/**"));
    write(
        temp.path().join(".iwe/schemas/person.yaml"),
        "sections:\n  - header: { const: Summary }\n  - header: { const: Tasks }\n",
    )
    .unwrap();
    write(temp.path().join("docs/one.md"), CLEAN).unwrap();
    temp
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

fn write_config(path: &Path, schemas: HashMap<String, SchemaBinding>) {
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
    write(
        path.join(".iwe/config.toml"),
        toml::to_string(&config).expect("config"),
    )
    .unwrap();
}

fn run_update(work_dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("update").current_dir(work_dir);
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("run iwe update")
}

fn run_delete(work_dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("delete").current_dir(work_dir);
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("run iwe delete")
}
