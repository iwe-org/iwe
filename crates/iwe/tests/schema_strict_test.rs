use diwe::config::{Configuration, MarkdownOptions, Patterns, SchemaBinding, WorkspaceOptions};
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

const STAGED: &str = indoc! {"
    ---
    stage: a
    ---

    # One
"};

const HUB: &str = indoc! {"
    # Index

    [One](one)
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
            docs/one: required section \"Tasks\" is missing
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
fn update_strict_dry_run_blocks_violating_body_overwrite() {
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

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        indoc! {"
            error: --strict blocked the write: schema validation failed
            docs/one: required section \"Tasks\" is missing
        "}
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        CLEAN
    );
}

#[test]
fn update_strict_dry_run_allows_clean_body_overwrite() {
    let temp = setup();
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "docs/one",
            "--content",
            "# Summary\n\n# Tasks\n\nmore\n",
            "--strict",
            "--dry-run",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Would update 'docs/one' (25 bytes)\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        CLEAN
    );
}

#[test]
fn update_strict_dry_run_blocks_violating_set() {
    let temp = setup_frontmatter();
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "docs/one",
            "--set",
            "stage=other",
            "--expect",
            "1",
            "--strict",
            "--dry-run",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        indoc! {r#"
            error: --strict blocked the write: schema validation failed
            docs/one › frontmatter › stage: "other" is not one of "a" or "b"
        "#}
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        STAGED
    );
}

#[test]
fn update_strict_dry_run_without_expect_still_validates() {
    let temp = setup_frontmatter();
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "docs/one",
            "--set",
            "stage=b",
            "--strict",
            "--dry-run",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Would update 1 document(s)\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        STAGED
    );
}

#[test]
fn update_dry_run_without_strict_does_not_validate() {
    let temp = setup_frontmatter();
    let output = run_update(
        temp.path(),
        &["-k", "docs/one", "--set", "stage=other", "--dry-run"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        STAGED
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
            docs/one: required section \"Tasks\" is missing
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
fn delete_strict_dry_run_blocks_violating_reference_cleanup() {
    let temp = setup_hub();
    let output = run_delete(temp.path(), &["docs/one", "--strict", "--dry-run"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        indoc! {"
            error: --strict blocked the write: schema validation failed
            docs/hub › Index: required block paragraph is missing
        "}
    );
    assert!(temp.path().join("docs/one.md").exists());
    assert_eq!(
        read_to_string(temp.path().join("docs/hub.md")).unwrap(),
        HUB
    );
}

#[test]
fn delete_strict_allows_clean_removal() {
    let temp = setup();
    let output = run_delete(temp.path(), &["docs/one", "--expect", "1", "--strict"]);

    assert!(output.status.success());
    assert!(!temp.path().join("docs/one.md").exists());
}

fn setup_frontmatter() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe/schemas")).unwrap();
    create_dir_all(temp.path().join("docs")).unwrap();
    write_config(temp.path(), binding("staged", "docs/**"));
    write(
        temp.path().join(".iwe/schemas/staged.yaml"),
        "frontmatter:\n  type: object\n  properties:\n    stage: { enum: [a, b] }\n",
    )
    .unwrap();
    write(temp.path().join("docs/one.md"), STAGED).unwrap();
    temp
}

fn setup_hub() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe/schemas")).unwrap();
    create_dir_all(temp.path().join("docs")).unwrap();
    write_config(temp.path(), binding("hub", "docs/hub"));
    write(
        temp.path().join(".iwe/schemas/hub.yaml"),
        "sections:\n  - header: { const: Index }\n    blocks:\n      - type: paragraph\n",
    )
    .unwrap();
    write(temp.path().join("docs/hub.md"), HUB).unwrap();
    write(temp.path().join("docs/one.md"), "# One\n").unwrap();
    temp
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
        workspace: WorkspaceOptions {
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
