use std::env;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_export_json_basic() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["json"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify JSON format structure
    assert!(stdout.starts_with("["));
    assert!(stdout.ends_with("]\n") || stdout.ends_with("]"));

    // Parse as JSON to ensure it's valid
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_export_graphviz_basic() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["graphviz"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Output should start with 'digraph {{', got: {}",
        stdout.chars().take(50).collect::<String>()
    );
    assert!(stdout.ends_with("}\n"));
}

#[test]
fn test_export_json_with_key_filter() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["json", "--key", "test"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Should still be valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_export_graphviz_with_key_filter() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["graphviz", "--key", "test"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Output should start with 'digraph {{', got: {}",
        stdout.chars().take(50).collect::<String>()
    );
    assert!(stdout.ends_with("}\n"));
}

#[test]
fn test_export_json_with_depth_limit() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["json", "--depth", "1"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Should still be valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_export_graphviz_with_depth_limit() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["graphviz", "--depth", "1"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Output should start with 'digraph {{', got: {}",
        stdout.chars().take(50).collect::<String>()
    );
    assert!(stdout.ends_with("}\n"));
}

#[test]
fn test_export_empty_workspace() {
    let temp_dir = setup_empty_workspace();

    // Test JSON format
    let json_output = run_export_command(&temp_dir, &["json"]);
    assert!(
        json_output.status.success(),
        "JSON export should succeed even with empty workspace"
    );

    let json_stdout = String::from_utf8(json_output.stdout).expect("Valid UTF-8 output");
    let _: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("Output should be valid JSON");

    // Test GraphViz format
    let graphviz_output = run_export_command(&temp_dir, &["graphviz"]);
    assert!(
        graphviz_output.status.success(),
        "GraphViz export should succeed even with empty workspace"
    );

    let graphviz_stdout = String::from_utf8(graphviz_output.stdout).expect("Valid UTF-8 output");
    assert!(graphviz_stdout.starts_with("digraph {"));
    assert!(graphviz_stdout.ends_with("}\n"));
}

#[test]
fn test_export_invalid_format() {
    let temp_dir = setup_test_workspace();
    let output = run_export_command(&temp_dir, &["invalid_format"]);

    assert!(
        !output.status.success(),
        "Command should fail with invalid format"
    );
}

#[test]
fn test_export_stderr_empty() {
    let temp_dir = setup_test_workspace();

    // Test both formats
    let json_output = run_export_command(&temp_dir, &["json"]);
    assert!(json_output.status.success(), "JSON export should succeed");

    let json_stderr = String::from_utf8(json_output.stderr).expect("Valid UTF-8 stderr");
    assert!(
        json_stderr.is_empty() || !json_stderr.contains("ERROR") && !json_stderr.contains("error:"),
        "JSON export stderr should not contain errors: {}",
        json_stderr
    );

    let graphviz_output = run_export_command(&temp_dir, &["graphviz"]);
    assert!(
        graphviz_output.status.success(),
        "GraphViz export should succeed"
    );

    let graphviz_stderr = String::from_utf8(graphviz_output.stderr).expect("Valid UTF-8 stderr");
    assert!(
        graphviz_stderr.is_empty()
            || !graphviz_stderr.contains("ERROR") && !graphviz_stderr.contains("error:"),
        "GraphViz export stderr should not contain errors: {}",
        graphviz_stderr
    );
}

#[test]
fn test_export_complex_workspace() {
    let temp_dir = setup_complex_test_workspace();

    // Test JSON format
    let json_output = run_export_command(&temp_dir, &["json"]);
    assert!(json_output.status.success(), "JSON export should succeed");

    let json_stdout = String::from_utf8(json_output.stdout).expect("Valid UTF-8 output");
    let json_data: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("Output should be valid JSON");

    // Should have multiple nodes
    if let serde_json::Value::Array(nodes) = json_data {
        assert!(!nodes.is_empty(), "Should have at least some nodes");
    }

    // Test GraphViz format
    let graphviz_output = run_export_command(&temp_dir, &["graphviz"]);
    assert!(
        graphviz_output.status.success(),
        "GraphViz export should succeed"
    );

    let graphviz_stdout = String::from_utf8(graphviz_output.stdout).expect("Valid UTF-8 output");
    assert!(graphviz_stdout.starts_with("digraph {"));
    assert!(graphviz_stdout.ends_with("}\n"));

    // Should contain some nodes
    assert!(
        graphviz_stdout.len() > 50,
        "Should contain substantial content"
    );
}

fn run_export_command(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = get_binary_path();

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path()).arg("export");

    for arg in args {
        cmd.arg(arg);
    }

    cmd.output().expect("Failed to execute export command")
}

fn get_binary_path() -> PathBuf {
    // In integration tests, we need to find the compiled binary
    // This looks for the binary in the target directory
    let mut binary_path = env::current_dir().expect("Failed to get current directory");

    // Go up to the workspace root if we're in a subdirectory
    while !binary_path.join("Cargo.toml").exists() || !binary_path.join("crates").exists() {
        if !binary_path.pop() {
            panic!("Could not find workspace root");
        }
    }

    binary_path.push("target");

    // Try debug first, then release
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

fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    let config_content = r#"
[library]
path = ""

[markdown]
strict_titles = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    let test_content = r#"# Test Document

This is a test document for the export command.

## Section 1

Some content here.

[Related Document](related)

## Section 2

More content with references to other documents.
"#;

    write(temp_path.join("test.md"), test_content).expect("Failed to write test file");

    let related_content = r#"# Related Document

This document is referenced by the test document.

## Related Section

Content that creates connections in the graph.
"#;

    write(temp_path.join("related.md"), related_content).expect("Failed to write related file");

    temp_dir
}

fn setup_empty_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace without any content
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    let config_content = r#"
[library]
path = ""

[markdown]
strict_titles = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    temp_dir
}

fn setup_complex_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    let config_content = r#"
[library]
path = ""

[markdown]
strict_titles = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    // Create multiple interconnected documents
    let doc1 = r#"# Main Document

This is the main document with multiple sections.

## Introduction

Introduction to the topic with [references](reference).

## Core Concepts

Core concepts explained with links to [details](details) and [examples](examples).

## Conclusion

Final thoughts linking back to [introduction](#introduction).
"#;

    let doc2 = r#"# Reference Document

Reference material for the main document.

## Background

Background information that supports the main content.

## Technical Details

Technical details that connect to [examples](examples).
"#;

    let doc3 = r#"# Details Document

Detailed explanations of core concepts.

## Deep Dive

In-depth analysis with connections to [main document](main).

## Advanced Topics

Advanced material building on [core concepts](main#core-concepts).
"#;

    let doc4 = r#"# Examples Document

Practical examples and use cases.

## Basic Examples

Simple examples that demonstrate [core concepts](details).

## Advanced Examples

Complex examples that tie everything together.
"#;

    write(temp_path.join("main.md"), doc1).expect("Failed to write main file");
    write(temp_path.join("reference.md"), doc2).expect("Failed to write reference file");
    write(temp_path.join("details.md"), doc3).expect("Failed to write details file");
    write(temp_path.join("examples.md"), doc4).expect("Failed to write examples file");

    temp_dir
}
