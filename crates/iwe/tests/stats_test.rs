use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;


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
        "includedByCount",
        "referencedByCount",
        "incomingEdgesCount",
        "includesCount",
        "referencesCount",
        "totalEdgesCount",
        "bulletLists",
        "orderedLists",
        "codeBlocks",
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

#[test]
fn test_stats_broken_links_local_included() {
    let temp_dir = setup_test_workspace_with_broken_links();
    let output = run_stats_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("## Broken Links"),
        "Should contain Broken Links section"
    );
    assert!(
        stdout.contains("-> nonexistent"),
        "Should report broken link to nonexistent document"
    );
    assert!(
        stdout.contains("-> also-missing"),
        "Should report broken block reference to also-missing document"
    );
}

#[test]
fn test_stats_broken_links_external_excluded() {
    let temp_dir = setup_test_workspace_with_broken_links();
    let output = run_stats_command(&temp_dir, &[]);

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        !stdout.contains("https://example.com"),
        "Should not report external links as broken"
    );
    assert!(
        !stdout.contains("http://example.org"),
        "Should not report external links as broken"
    );
}

fn setup_test_workspace_with_broken_links() -> TempDir {
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

    let doc_with_broken_inline = indoc! {"
        # Document With Links

        This has a [broken link](nonexistent) and an [external link](https://example.com).

        Also links to [another external](http://example.org) site.
    "};

    write(temp_path.join("doc-with-links.md"), doc_with_broken_inline)
        .expect("Failed to write doc file");

    let doc_with_broken_block = indoc! {"
        # Document With Block Ref

        Some content here.

        [also-missing](also-missing)
    "};

    write(temp_path.join("doc-with-block-ref.md"), doc_with_broken_block)
        .expect("Failed to write doc file");

    temp_dir
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

#[test]
fn test_stats_broken_inline_links_in_table() {
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

    let doc = indoc! {"
        # Document With Table

        | Name | Link |
        |------|------|
        | A    | [missing](nonexistent) |
        | B    | [valid](existing) |

        This paragraph has a [broken link](also-missing) after the table.
    "};

    write(temp_path.join("doc.md"), doc).expect("Failed to write doc file");
    write(temp_path.join("existing.md"), "# Existing\n").expect("Failed to write existing file");

    let output = run_stats_command(&temp_dir, &[]);
    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("-> nonexistent"),
        "Should detect broken inline link inside table cell"
    );
    assert!(
        stdout.contains("-> also-missing"),
        "Should detect broken inline link after the table"
    );
    assert!(
        !stdout.contains("-> existing"),
        "Should not report valid link as broken"
    );
}

#[test]
fn test_stats_per_doc_default_format_outputs_json() {
    let temp_dir = setup_test_workspace();
    let output = run_stats_command(&temp_dir, &["-k", "test"]);

    assert!(output.status.success(), "Should succeed with default format");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(parsed["key"], "test");
}

#[test]
fn test_stats_per_doc_csv_format_outputs_json() {
    let temp_dir = setup_test_workspace();
    let output = run_stats_command(&temp_dir, &["-k", "test", "-f", "csv"]);

    assert!(output.status.success(), "Should succeed with csv format");
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(parsed["key"], "test");
}

fn run_stats_command(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary_path = crate::common::get_iwe_binary_path();

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(temp_dir.path()).arg("stats");

    for arg in args {
        cmd.arg(arg);
    }

    cmd.output().expect("Failed to execute stats command")
}
