use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, read_dir, read_to_string, write};
use std::process::Command;
use tempfile::TempDir;

mod common;

#[test]
fn test_extract_list_sections() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main Title

            ## Section A

            Content A

            ## Section B

            Content B
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--list"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout,
        indoc! {"
            1: Main Title
            2: Section A
            3: Section B
        "}
    );
}

#[test]
fn test_extract_by_section_title() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main Title

            ## Section A

            Content A

            ## Section B

            Content B
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--section", "Section A"]);
    assert!(output.status.success(), "Extract command should succeed");

    let main_content = read_to_string(temp_path.join("main.md")).unwrap();
    assert_eq!(
        main_content,
        indoc! {"
            # Main Title

            [Section A](section-a)

            ## Section B

            Content B
        "}
    );

    let extracted_file = find_extracted_file(temp_path);
    assert!(extracted_file.is_some(), "Extracted file should exist");

    let extracted_content = read_to_string(extracted_file.unwrap()).unwrap();
    assert_eq!(
        extracted_content,
        indoc! {"
            # Section A

            Content A
        "}
    );
}

#[test]
fn test_extract_by_block_number() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main Title

            ## Section A

            Content A

            ## Section B

            Content B
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--block", "3"]);
    assert!(output.status.success());

    let main_content = read_to_string(temp_path.join("main.md")).unwrap();
    assert_eq!(
        main_content,
        indoc! {"
            # Main Title

            [Section B](section-b)

            ## Section A

            Content A
        "}
    );
}

#[test]
fn test_extract_ambiguous_section_fails() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Notes

            First notes

            ## Notes

            Second notes
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--section", "Notes"]);
    assert!(!output.status.success(), "Should fail with ambiguous section");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr,
        indoc! {"
            Error: Multiple sections match 'Notes':
              2: Notes
              3: Notes
            Use --block <n> to select a specific section.
        "}
    );
}

#[test]
fn test_extract_top_level_not_allowed() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main Title

            Content
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--block", "1"]);
    assert!(!output.status.success(), "Should fail for top-level section");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Cannot extract top-level document section\n");
}

#[test]
fn test_extract_nonexistent_key() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("a", "# Doc A"),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["nonexistent", "--block", "1"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Document 'nonexistent' not found\n");
}

#[test]
fn test_extract_section_not_found() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Section A
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--section", "Nonexistent"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: No section matches 'Nonexistent'\n");
}

#[test]
fn test_extract_block_out_of_range() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Section A
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--block", "10"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "Error: Block number 10 out of range (1-2)\n");
}

#[test]
fn test_extract_dry_run() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Section A

            Content A
        "}),
    ]);
    let temp_path = temp_dir.path();

    let original_content = read_to_string(temp_path.join("main.md")).unwrap();

    let output = run_extract_command(temp_path, &["main", "--section", "Section A", "--dry-run"]);
    assert!(output.status.success());

    let after_content = read_to_string(temp_path.join("main.md")).unwrap();
    assert_eq!(original_content, after_content, "File should not be modified");

    let file_count = read_dir(temp_path)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().extension().map(|s| s == "md").unwrap_or(false))
        .count();
    assert_eq!(file_count, 1, "No new files should be created");
}

#[test]
fn test_extract_keys_output() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Section A
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--section", "Section A", "--keys", "--dry-run"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, "section-a\nmain\n");
}

#[test]
fn test_extract_quiet_mode() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Section A
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--section", "Section A", "--quiet"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().is_empty(), "Quiet mode should suppress output");
}

#[test]
fn test_extract_preserves_subsections() {
    let temp_dir = setup_workspace_with_docs(vec![
        ("main", indoc! {"
            # Main

            ## Section A

            Content A

            ### Subsection A1

            Subsection content
        "}),
    ]);
    let temp_path = temp_dir.path();

    let output = run_extract_command(temp_path, &["main", "--block", "2"]);
    assert!(output.status.success());

    let extracted_file = find_extracted_file(temp_path);
    let extracted_content = read_to_string(extracted_file.unwrap()).unwrap();

    assert_eq!(
        extracted_content,
        indoc! {"
            # Section A

            Content A

            ## Subsection A1

            Subsection content
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

fn run_extract_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(common::get_iwe_binary_path());
    command.arg("extract").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe extract")
}

fn find_extracted_file(temp_path: &std::path::Path) -> Option<std::path::PathBuf> {
    read_dir(temp_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|s| s == "md").unwrap_or(false)
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s != "main")
                    .unwrap_or(false)
        })
        .next()
}
