use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::env;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_export_dot_basic() {
    let temp_dir = setup_test_workspace();
    let output = run_export_dot_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.starts_with("digraph G {"),
        "Should start with 'digraph G {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");

    assert!(
        stdout.contains("Test Document"),
        "Should contain test document node"
    );
}

fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

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

    let config_content = toml::to_string(&config).expect("Failed to serialize config to TOML");
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    let test_content = indoc! {"
        # Test Document

        This is a test document for the dot export.

        ## Section 1

        Some content here.

        [Related Document](related)

        ## Section 2

        More content with a [link to another section](test#section-1).
    "};

    write(temp_path.join("test.md"), test_content).expect("Failed to write test file");

    let related_content = indoc! {"
        # Related Document

        This document is related to the [Test Document](test).

        ## Details

        Some additional details here.
    "};

    write(temp_path.join("related.md"), related_content).expect("Failed to write related file");

    temp_dir
}

fn run_export_dot_command(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = get_binary_path();

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path()).arg("export").arg("dot");

    for arg in args {
        cmd.arg(arg);
    }

    cmd.output().expect("Failed to execute export dot command")
}

fn get_binary_path() -> PathBuf {
    let mut binary_path = env::current_dir().expect("Failed to get current directory");

    while !binary_path.join("Cargo.toml").exists() || !binary_path.join("crates").exists() {
        if !binary_path.pop() {
            panic!("Could not find workspace root");
        }
    }

    binary_path.push("target");

    let debug_path = binary_path.join("debug").join("iwe");
    let release_path = binary_path.join("release").join("iwe");

    if debug_path.exists() {
        debug_path
    } else if release_path.exists() {
        release_path
    } else {
        panic!("Could not find iwe binary. Run 'cargo build' first.");
    }
}
