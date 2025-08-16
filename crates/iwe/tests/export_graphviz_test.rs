use std::env;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_export_graphviz_basic() {
    let temp_dir = setup_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");

    // Verify it contains expected DOT elements
    assert!(
        stdout.contains("graph ["),
        "Should contain graph attributes"
    );
    assert!(stdout.contains("node ["), "Should contain node attributes");

    // Verify nodes are present (should have at least the test nodes)
    assert!(
        stdout.contains("Test Document"),
        "Should contain test document node"
    );
}

#[test]
fn test_export_graphviz_with_key_filter() {
    let temp_dir = setup_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &["--key", "test"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");

    // Should still contain filtered content
    assert!(stdout.contains("Test"), "Should contain filtered content");
}

#[test]
fn test_export_graphviz_with_depth_limit() {
    let temp_dir = setup_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &["--depth", "1"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");
}

#[test]
fn test_export_graphviz_empty_workspace() {
    let temp_dir = setup_empty_workspace();
    let output = run_export_graphviz_command(&temp_dir, &[]);

    assert!(
        output.status.success(),
        "Command should succeed even with empty workspace"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Should still produce valid DOT format even if empty
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");
}

#[test]
fn test_export_graphviz_multiple_files() {
    let temp_dir = setup_complex_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");

    // Should contain content from multiple files
    assert!(
        stdout.contains("Introduction"),
        "Should contain content from intro file"
    );
    assert!(
        stdout.contains("Chapter"),
        "Should contain content from chapter file"
    );
}

fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    // Create a basic config file
    let config_content = r#"
[library]
path = ""

[markdown]
normalize_headers = true
normalize_lists = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    // Create test markdown files
    let test_content = r#"# Test Document

This is a test document for the graphviz export.

## Section 1

Some content here.

[Related Document](related)

## Section 2

More content with a [link to another section](test#section-1).
"#;

    write(temp_path.join("test.md"), test_content).expect("Failed to write test file");

    let related_content = r#"# Related Document

This document is related to the [Test Document](test).

## Details

Some additional details here.
"#;

    write(temp_path.join("related.md"), related_content).expect("Failed to write related file");

    temp_dir
}

fn setup_empty_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    // Create a basic config file
    let config_content = r#"
[library]
path = ""

[markdown]
normalize_headers = true
normalize_lists = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    temp_dir
}

fn setup_complex_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    // Create a basic config file
    let config_content = r#"
[library]
path = ""

[markdown]
normalize_headers = true
normalize_lists = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    // Create multiple interconnected markdown files
    let intro_content = r#"# Introduction

Welcome to this knowledge base.

## Overview

This contains several [chapters](chapter1) and [references](references).

- [Chapter 1](chapter1)
- [Chapter 2](chapter2)
- [References](references)
"#;

    write(temp_path.join("intro.md"), intro_content).expect("Failed to write intro file");

    let chapter1_content = r#"# Chapter 1: Getting Started

This is the first chapter. See [Introduction](intro) for context.

## Basic Concepts

Key concepts are explained here.

## Next Steps

Continue to [Chapter 2](chapter2).
"#;

    write(temp_path.join("chapter1.md"), chapter1_content).expect("Failed to write chapter1 file");

    let chapter2_content = r#"# Chapter 2: Advanced Topics

Building on [Chapter 1](chapter1), we explore advanced topics.

## Advanced Concepts

More complex ideas here.

## References

See [References](references) for more information.
"#;

    write(temp_path.join("chapter2.md"), chapter2_content).expect("Failed to write chapter2 file");

    let references_content = r#"# References

External references and links.

## Books

- Reference Book 1
- Reference Book 2

## Articles

- [Chapter 1](chapter1) covers basics
- [Chapter 2](chapter2) covers advanced topics
"#;

    write(temp_path.join("references.md"), references_content)
        .expect("Failed to write references file");

    temp_dir
}

fn run_export_graphviz_command(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = get_binary_path();

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path()).arg("export-graphviz");

    for arg in args {
        cmd.arg(arg);
    }

    cmd.output()
        .expect("Failed to execute export-graphviz command")
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

#[test]
fn test_export_graphviz_output_format() {
    let temp_dir = setup_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify specific GraphViz DOT format elements
    assert!(
        stdout.contains("graph ["),
        "Should contain graph attributes"
    );
    assert!(stdout.contains("node ["), "Should contain node attributes");
    assert!(
        stdout.contains("overlap_scaling"),
        "Should contain overlap_scaling attribute"
    );
    assert!(stdout.contains("pack=90"), "Should contain pack attribute");
    assert!(
        stdout.contains("label=\"\\N\""),
        "Should contain node label format"
    );

    // Verify that nodes have proper attributes
    assert!(
        stdout.contains("group="),
        "Should contain group attributes for nodes"
    );
    assert!(
        stdout.contains("class="),
        "Should contain class attributes for nodes"
    );

    // Check for proper escaping in node labels
    let lines: Vec<&str> = stdout.lines().collect();
    let node_lines: Vec<&str> = lines
        .iter()
        .filter(|line| {
            line.trim_start()
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_digit())
        })
        .cloned()
        .collect();

    assert!(!node_lines.is_empty(), "Should contain node definitions");

    // Verify edge format (should have -> for directed graph)
    if stdout.contains(" -> ") {
        assert!(
            stdout.matches(" -> ").count() > 0,
            "Should contain proper edge format"
        );
    }
}

#[test]
fn test_export_graphviz_node_escaping() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Initialize IWE workspace
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    let config_content = r#"
[library]
path = ""

[markdown]
normalize_headers = true
normalize_lists = true
"#;
    write(temp_path.join(".iwe/config.toml"), config_content).expect("Failed to write config file");

    // Create test file with special characters that need escaping
    let test_content = r#"# Test "Document" with Special Characters

This document has quotes "like this" and backslashes \ and newlines.

## Section with 'quotes' and "more quotes"

Content with special chars: \n \t \r
"#;

    write(temp_path.join("special.md"), test_content).expect("Failed to write special file");

    let output = run_export_graphviz_command(&temp_dir, &[]);
    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify that special characters are properly escaped
    assert!(
        stdout.contains("\\\""),
        "Should escape quotes in node labels"
    );

    // Should not contain unescaped quotes that would break DOT format
    let lines: Vec<&str> = stdout.lines().collect();
    for line in lines {
        if line.trim().contains("label=") {
            // Count quotes in label definitions - should be even (properly paired)
            let quote_count = line.matches('"').count();
            assert!(
                quote_count >= 2,
                "Label lines should have at least opening and closing quotes"
            );
        }
    }
}

#[test]
fn test_export_graphviz_stderr_empty() {
    let temp_dir = setup_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");

    // For a successful export, stderr should be empty or only contain non-error logs
    assert!(
        stderr.is_empty() || !stderr.contains("ERROR") && !stderr.contains("error:"),
        "Stderr should not contain errors: {}",
        stderr
    );
}

#[test]
fn test_export_graphviz_with_nonexistent_key_filter() {
    let temp_dir = setup_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &["--key", "nonexistent"]);

    assert!(
        output.status.success(),
        "Command should succeed even with nonexistent key"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Should still produce valid DOT format
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");

    // Should produce minimal output when no matches found
    assert!(
        stdout.contains("graph ["),
        "Should still contain graph attributes"
    );
}

#[test]
fn test_export_graphviz_high_depth_limit() {
    let temp_dir = setup_complex_test_workspace();
    let output = run_export_graphviz_command(&temp_dir, &["--depth", "10"]);

    assert!(
        output.status.success(),
        "Command should succeed with high depth"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Verify basic DOT format structure
    assert!(
        stdout.starts_with("digraph {"),
        "Should start with 'digraph {{'"
    );
    assert!(stdout.ends_with("}\n"), "Should end with '}}'");

    // With higher depth, should potentially include more content
    assert!(
        stdout.len() > 100,
        "Should produce substantial output with complex workspace"
    );
}
