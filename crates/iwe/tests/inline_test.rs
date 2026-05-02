use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, read_to_string, write};
use std::process::Command;
use tempfile::TempDir;


#[test]
fn test_inline_list_references() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Architecture](arch)

            [Database](db)
        "}),
        ("arch", "# Architecture"),
        ("db", "# Database"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--list"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout,
        indoc! {"
            1: [Architecture](arch)
            2: [Database](db)
        "}
    );
}

#[test]
fn test_inline_by_reference() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Architecture](arch)
        "}),
        ("arch", indoc! {"
            # Architecture

            Architecture content here.
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "arch"]);
    assert!(output.status.success());

    assert!(!temp_path.join("arch.md").exists(), "Target file should be deleted");

    let index_content = read_to_string(temp_path.join("index.md")).unwrap();
    assert_eq!(
        index_content,
        indoc! {"
            # Index

            ## Architecture

            Architecture content here.
        "}
    );
}

#[test]
fn test_inline_by_block_number() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [First](first)

            [Second](second)
        "}),
        ("first", "# First\n\nFirst content"),
        ("second", "# Second\n\nSecond content"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--block", "1"]);
    assert!(output.status.success());

    assert!(!temp_path.join("first.md").exists(), "First file should be deleted");
    assert!(temp_path.join("second.md").exists(), "Second file should remain");

    let index_content = read_to_string(temp_path.join("index.md")).unwrap();
    assert_eq!(
        index_content,
        indoc! {"
            # Index

            [Second](second)

            ## First

            First content
        "}
    );
}

#[test]
fn test_inline_keep_target() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Architecture](arch)
        "}),
        ("arch", indoc! {"
            # Architecture

            Architecture content.
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "arch", "--keep-target"]);
    assert!(output.status.success());

    assert!(temp_path.join("arch.md").exists(), "Target file should be kept");

    let index_content = read_to_string(temp_path.join("index.md")).unwrap();
    assert_eq!(
        index_content,
        indoc! {"
            # Index

            ## Architecture

            Architecture content.
        "}
    );
}

#[test]
fn test_inline_as_quote() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Quote](quote)
        "}),
        ("quote", indoc! {"
            Quote paragraph 1

            Quote paragraph 2
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "quote", "--as-quote"]);
    assert!(output.status.success());

    let index_content = read_to_string(temp_path.join("index.md")).unwrap();
    assert_eq!(
        index_content,
        indoc! {"
            # Index

            > Quote paragraph 1
            >
            > Quote paragraph 2
        "}
    );
}

#[test]
fn test_inline_cleans_other_references() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Target](target)
        "}),
        ("other", indoc! {"
            # Other

            [Also refs target](target)
        "}),
        ("target", "# Target\n\nContent"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "target"]);
    assert!(output.status.success());

    let other_content = read_to_string(temp_path.join("other.md")).unwrap();
    assert_eq!(other_content, "# Other\n");
}

#[test]
fn test_inline_ambiguous_reference_fails() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Notes A](notes-a)

            [Notes B](notes-b)
        "}),
        ("notes-a", "# Notes A"),
        ("notes-b", "# Notes B"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "notes"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr,
        indoc! {"
            Error: Multiple references match 'notes':
              1: [Notes A](notes-a)
              2: [Notes B](notes-b)
            Use --block <n> to select a specific reference.
        "}
    );
}

#[test]
fn test_inline_nonexistent_key() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "# Doc A"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["nonexistent", "--block", "1"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Document 'nonexistent' not found\n");
}

#[test]
fn test_inline_reference_not_found() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Link](target)
        "}),
        ("target", "# Target"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "nonexistent"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: No reference matches 'nonexistent'\n");
}

#[test]
fn test_inline_block_out_of_range() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Link](target)
        "}),
        ("target", "# Target"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--block", "10"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Block number 10 out of range (1-1)\n");
}

#[test]
fn test_inline_dry_run() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            [Target](target)
        "}),
        ("target", "# Target\n\nContent"),
    ]);
    let temp_path = temp_dir.path();

    let original_content = read_to_string(temp_path.join("index.md")).unwrap();

    let output = run_inline_command(temp_path, &["index", "--reference", "target", "--dry-run"]);
    assert!(output.status.success());

    let after_content = read_to_string(temp_path.join("index.md")).unwrap();
    assert_eq!(original_content, after_content, "File should not be modified");
    assert!(temp_path.join("target.md").exists(), "Target should still exist");
}

#[test]
fn test_inline_keys_output() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", "# Index\n\n[Target](target)"),
        ("target", "# Target"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "target", "--keys", "--dry-run"]);
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(output.status.success(), "Command failed: {}", stderr);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, "index\ntarget\n");
}

#[test]
fn test_inline_quiet_mode() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", "# Index\n\n[Target](target)"),
        ("target", "# Target"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--reference", "target", "--quiet"]);
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(output.status.success(), "Command failed: {}", stderr);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().is_empty(), "Quiet mode should suppress output");
}

#[test]
fn test_inline_no_references() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("index", indoc! {"
            # Index

            Just regular content, no references.
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_inline_command(temp_path, &["index", "--list"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().is_empty(), "Should have no references to list");
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

fn run_inline_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("inline").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe inline")
}
