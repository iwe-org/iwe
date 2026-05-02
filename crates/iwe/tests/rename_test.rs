use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, read_to_string, write};
use std::process::Command;
use tempfile::TempDir;


#[test]
fn test_rename_basic() {
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

    let output = run_rename_command(temp_path, &["b", "renamed-b"]);
    assert!(output.status.success(), "Rename command should succeed");

    assert!(!temp_path.join("b.md").exists(), "Old file should be deleted");
    assert!(temp_path.join("renamed-b.md").exists(), "New file should exist");

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(
        a_content,
        indoc! {"
            # Doc A

            [Doc B](renamed-b)
        "}
    );
}

#[test]
fn test_rename_multiple_references() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [first link](b)

            [second link](b)
        "}),
        ("b", indoc! {"
            # Doc B
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["b", "new-name"]);
    assert!(output.status.success());

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    assert_eq!(
        a_content,
        indoc! {"
            # Doc A

            [Doc B](new-name)

            [Doc B](new-name)
        "}
    );
}

#[test]
fn test_rename_updates_multiple_files() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [link](target)
        "}),
        ("b", indoc! {"
            # Doc B

            [another link](target)
        "}),
        ("target", indoc! {"
            # Target
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["target", "new-target"]);
    assert!(output.status.success());

    let a_content = read_to_string(temp_path.join("a.md")).unwrap();
    let b_content = read_to_string(temp_path.join("b.md")).unwrap();

    assert_eq!(
        a_content,
        indoc! {"
            # Doc A

            [Target](new-target)
        "}
    );
    assert_eq!(
        b_content,
        indoc! {"
            # Doc B

            [Target](new-target)
        "}
    );
}

#[test]
fn test_rename_nonexistent_key() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "# Doc A"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["nonexistent", "new-name"]);
    assert!(!output.status.success(), "Should fail for nonexistent key");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Document 'nonexistent' not found\n");
}

#[test]
fn test_rename_to_existing_key() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "# Doc A"),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["a", "b"]);
    assert!(!output.status.success(), "Should fail when target exists");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Document 'b' already exists\n");
}

#[test]
fn test_rename_dry_run() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", indoc! {"
            # Doc A

            [link](b)
        "}),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["b", "new-name", "--dry-run"]);
    assert!(output.status.success());

    assert!(temp_path.join("b.md").exists(), "Original file should still exist");
    assert!(!temp_path.join("new-name.md").exists(), "New file should not be created");

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
fn test_rename_keys_output() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "[link](b)"),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["b", "new-name", "--keys", "--dry-run"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, "new-name\na\nb\n");
}

#[test]
fn test_rename_quiet_mode() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "[link](b)"),
        ("b", "# Doc B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["b", "new-name", "--quiet"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().is_empty(), "Quiet mode should suppress output");
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

#[test]
fn test_rename_empty_key_rejected() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "# Doc A"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_rename_command(temp_path, &["a", ""]);
    assert!(!output.status.success(), "Should fail for empty key");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Invalid target: Key cannot be empty\n");

    assert!(temp_path.join("a.md").exists(), "Original file should still exist");
    assert!(!temp_path.join(".md").exists(), "Phantom .md should not be created");
}

#[test]
fn test_rename_cleans_empty_directory() {
    let temp_dir = setup_workspace_with_docs(vec![]);
    let temp_path = temp_dir.path();

    std::fs::create_dir_all(temp_path.join("sub")).unwrap();
    write(temp_path.join("sub").join("child.md"), "# Child").unwrap();

    let output = run_rename_command(temp_path, &["sub/child", "child"]);
    assert!(output.status.success());

    assert!(!temp_path.join("sub").join("child.md").exists(), "Old file removed");
    assert!(temp_path.join("child.md").exists(), "New file created");
    assert!(!temp_path.join("sub").exists(), "Empty directory should be cleaned up");
}

fn run_rename_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("rename").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe rename")
}
