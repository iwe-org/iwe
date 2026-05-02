use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, read_to_string, write};
use std::process::Command;
use tempfile::TempDir;


#[test]
fn test_delete_basic() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [Link to B](b)
        "}),
        ("b", indoc! {"
            # Doc B

            Content here
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b"]);
    assert!(output.status.success(), "Delete command should succeed");

    assert!(!temp_path.join("b.md").exists(), "File should be deleted");

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(a_content, "# Doc A\n");
}

#[test]
fn test_delete_removes_multiple_inclusion_edges() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [Link 1](b)

            [Link 2](b)
        "}),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b"]);
    assert!(output.status.success());

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(a_content, "# Doc A\n");
}

#[test]
fn test_delete_updates_reference_edges() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            Some text with [inline link](b) in it.
        "}),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b"]);
    assert!(output.status.success());

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(
        a_content,
        indoc! {"
            # Doc A

            Some text with Doc B in it.
        "}
    );
}

#[test]
fn test_delete_updates_multiple_files() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [link](target)
        "}),
        ("b", indoc! {"
            # Doc B

            [another link](target)
        "}),
        ("target", "# Target"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["target"]);
    assert!(output.status.success());

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    let b_content = read_to_string(temp_path.join("b.md")).unwrap();

    assert_eq!(a_content, "# Doc A\n");
    assert_eq!(b_content, "# Doc B\n");
}

#[test]
fn test_delete_nonexistent_key() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "# Doc A"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["nonexistent"]);
    assert!(!output.status.success(), "Should fail for nonexistent key");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Document 'nonexistent' not found\n");
}

#[test]
fn test_delete_dry_run() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [link](b)
        "}),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b", "--dry-run"]);
    assert!(output.status.success());

    assert!(temp_path.join("b.md").exists(), "File should still exist");

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(
        a_content,
        indoc! {"
            # Doc A

            [link](b)
        "}
    );
}

#[test]
fn test_delete_keys_output() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "[link](b)"),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b", "--keys", "--dry-run"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, "a\nb\n");
}

#[test]
fn test_delete_quiet_mode() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "[link](b)"),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b", "--quiet"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().is_empty(), "Quiet mode should suppress output");
}

#[test]
fn test_delete_preserves_other_content() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            Some content before.

            [link](b)

            Some content after.

            ## Section

            More content.
        "}),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_delete_command(temp_path, &["b"]);
    assert!(output.status.success());

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(
        a_content,
        indoc! {"
            # Doc A

            Some content before.

            Some content after.

            ## Section

            More content.
        "}
    );
}

fn setup_workspace_with_docs(docs: Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    for (key, content) in docs {
        write(temp_path.join(format!("{}.md", key)), content).expect("Should write file");
    }

    temp_dir
}

fn setup_iwe_config(temp_path: &std::path::Path) {
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

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
    write(temp_path.join(".iwe").join("config.toml"), config_content).expect("Should write config");
}

fn run_delete_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("delete").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe delete")
}
