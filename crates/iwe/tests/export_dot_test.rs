use diwe::config::{Configuration, MarkdownOptions, WorkspaceOptions};
use indoc::indoc;
use std::fs::{create_dir_all, write};
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
        workspace: WorkspaceOptions {
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
    let binary_path = crate::common::get_iwe_binary_path();

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path())
        .arg("export")
        .arg("-f")
        .arg("dot");

    for arg in args {
        cmd.arg(arg);
    }

    cmd.output().expect("Failed to execute export dot command")
}

#[test]
fn test_export_dot_rejects_unknown_key() {
    let temp_dir = setup_test_workspace();
    let output = run_export_dot_command(&temp_dir, &["-k", "missing"]);

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Error: Document 'missing' not found\n"
    );
}

fn workspace_with(docs: &[(&str, &str)]) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    create_dir_all(temp_dir.path().join(".iwe")).unwrap();
    let config = Configuration {
        workspace: WorkspaceOptions {
            path: "".to_string(),
            ..Default::default()
        },
        markdown: MarkdownOptions {
            refs_extension: "".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    write(
        temp_dir.path().join(".iwe/config.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    for (key, content) in docs {
        write(temp_dir.path().join(format!("{}.md", key)), content).unwrap();
    }
    temp_dir
}

fn body_lines(output: std::process::Output) -> Vec<String> {
    assert!(output.status.success());
    let mut lines: Vec<String> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| line.contains("label=") || line.contains("->"))
        .map(|line| {
            line.trim()
                .split(' ')
                .map(|part| part.trim_start_matches(|c: char| c.is_ascii_digit()))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    lines.sort();
    lines
}

#[test]
fn default_depth_follows_every_level() {
    let temp_dir = workspace_with(&[
        ("root", "# Root\n\n[Child](child)\n"),
        ("child", "# Child\n\n[Leaf](leaf)\n"),
        ("leaf", "# Leaf\n\ntext\n"),
    ]);

    assert_eq!(
        body_lines(run_export_dot_command(&temp_dir, &[])),
        body_lines(run_export_dot_command(&temp_dir, &["--depth", "3"]))
    );
    assert_eq!(
        body_lines(run_export_dot_command(&temp_dir, &[])),
        vec![
            " ->  [color=\"#38546c66\" arrowhead=normal penwidth=1.2]".to_string(),
            " ->  [color=\"#38546c66\" arrowhead=normal penwidth=1.2]".to_string(),
            "[label=\"Child\" fillcolor=\"#e9f9ef\" fontsize=32 fontname=Verdana color=\"#b3b3b3\" penwidth=1.5 shape=note style=filled]".to_string(),
            "[label=\"Leaf\" fillcolor=\"#e2f6f6\" fontsize=24 fontname=Verdana color=\"#b3b3b3\" penwidth=1.5 shape=note style=filled]".to_string(),
            "[label=\"Root\" fillcolor=\"#dcf7f7\" fontsize=40 fontname=Verdana color=\"#b3b3b3\" penwidth=1.5 shape=note style=filled]".to_string(),
        ]
    );
}

#[test]
fn title_ending_in_a_backslash_is_escaped() {
    let temp_dir = workspace_with(&[("root", "# Root\\\n")]);

    assert_eq!(
        body_lines(run_export_dot_command(&temp_dir, &[])),
        vec![
            "[label=\"Root\\\\\" fillcolor=\"#dcf7f7\" fontsize=24 fontname=Verdana color=\"#b3b3b3\" penwidth=1.5 shape=note style=filled]"
                .to_string()
        ]
    );
}
