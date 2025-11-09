use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

mod common;

#[test]
fn test_stats_markdown_output() {
    let temp_dir = setup_test_workspace();
    let output = run_stats_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("# Graph Statistics"),
        "Should contain title"
    );
}

#[test]
fn test_stats_csv_output() {
    let temp_dir = setup_test_workspace();
    let output = run_stats_command(&temp_dir, &["--format", "csv"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 3,
        "Should have header + at least 2 data rows"
    );

    let header = lines[0];
    let expected_headers = vec![
        "key",
        "title",
        "sections",
        "paragraphs",
        "lines",
        "words",
        "incoming_block_refs",
        "incoming_inline_refs",
        "total_incoming_refs",
        "outgoing_block_refs",
        "outgoing_inline_refs",
        "total_connections",
        "bullet_lists",
        "ordered_lists",
        "code_blocks",
        "tables",
        "quotes",
    ];

    for expected in &expected_headers {
        assert!(
            header.contains(expected),
            "Header should contain '{}', but got: {}",
            expected,
            header
        );
    }

    assert!(stdout.contains("test,"), "Should contain test document");
    assert!(
        stdout.contains("related,"),
        "Should contain related document"
    );
}

#[test]
fn test_stats_with_various_elements() {
    let temp_dir = setup_test_workspace_with_elements();
    let output = run_stats_command(&temp_dir, &["--format", "csv"]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let lines: Vec<&str> = stdout.lines().collect();

    let complex_line = lines
        .iter()
        .find(|line| line.starts_with("complex,"))
        .expect("Should find complex document in CSV");

    let parts: Vec<&str> = complex_line.split(',').collect();

    let bullet_lists = parts[12]
        .parse::<usize>()
        .expect("bullet_lists should be a number");
    let ordered_lists = parts[13]
        .parse::<usize>()
        .expect("ordered_lists should be a number");
    let code_blocks = parts[14]
        .parse::<usize>()
        .expect("code_blocks should be a number");
    let tables = parts[15]
        .parse::<usize>()
        .expect("tables should be a number");
    let quotes = parts[16]
        .parse::<usize>()
        .expect("quotes should be a number");

    assert!(bullet_lists > 0, "Should count bullet lists");
    assert!(ordered_lists > 0, "Should count ordered lists");
    assert!(code_blocks > 0, "Should count code blocks");
    assert!(tables > 0, "Should count tables");
    assert!(quotes > 0, "Should count quotes");
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

        This is a test document with multiple sections.

        ## Section 1

        Some content here with a [link to related](related).

        ## Section 2

        More content with another paragraph.

        ### Subsection 2.1

        Nested content.
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

fn setup_test_workspace_with_elements() -> TempDir {
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

    let complex_content = indoc! {"
        # Complex Document

        This document contains various markdown elements.

        ## Lists Section

        Here's a bullet list:

        - Item 1
        - Item 2
        - Item 3

        And an ordered list:

        1. First
        2. Second
        3. Third

        ## Code Section

        Here's some code:

        ```rust
        fn main() {
            println!(\"Hello, world!\");
        }
        ```

        ## Table Section

        | Column 1 | Column 2 |
        |----------|----------|
        | Data 1   | Data 2   |
        | Data 3   | Data 4   |

        ## Quote Section

        > This is a quote
        > with multiple lines
    "};

    write(temp_path.join("complex.md"), complex_content).expect("Failed to write complex file");

    temp_dir
}

fn run_stats_command(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = common::get_iwe_binary_path();

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path()).arg("stats");

    for arg in args {
        cmd.arg(arg);
    }

    cmd.output().expect("Failed to execute stats command")
}
