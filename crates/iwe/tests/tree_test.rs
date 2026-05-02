use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;


#[test]
fn test_tree_default_format_is_markdown() {
    let temp_dir = setup_workspace_with_linked_documents();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &[]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        - [Main Document](main)
          - [Child Document](child)
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_markdown_format() {
    let temp_dir = setup_workspace_with_linked_documents();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "markdown"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        - [Main Document](main)
          - [Child Document](child)
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_json_format() {
    let temp_dir = setup_workspace_with_linked_documents();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "json"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {r#"
        [
          {
            "key": "main",
            "title": "Main Document",
            "children": [
              {
                "key": "child",
                "title": "Child Document",
                "children": []
              }
            ]
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_yaml_format() {
    let temp_dir = setup_workspace_with_linked_documents();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "yaml"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        - key: main
          title: Main Document
          children:
          - key: child
            title: Child Document
            children: []
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_keys_format() {
    let temp_dir = setup_workspace_with_linked_documents();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        main
        	child
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_with_key_filter() {
    let temp_dir = setup_workspace_with_multiple_roots();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-k", "root-a", "-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        root-a
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_with_multiple_key_filters() {
    let temp_dir = setup_workspace_with_multiple_roots();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-k", "root-a", "-k", "root-b", "-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        root-a
        root-b
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_with_depth_limit() {
    let temp_dir = setup_workspace_with_deep_nesting();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["--depth", "2", "-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        level1
        	level2
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_invalid_key() {
    let temp_dir = setup_workspace_with_linked_documents();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-k", "nonexistent-doc"]);

    assert!(!output.status.success(), "Tree should fail with invalid key");

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");
    let expected = "Error: Document 'nonexistent-doc' not found\n";

    assert_eq!(stderr, expected);
}

#[test]
fn test_tree_empty_workspace() {
    let temp_dir = setup_empty_workspace();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &[]);
    assert!(output.status.success(), "Tree should succeed with empty workspace");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert_eq!(stdout, "");
}

#[test]
fn test_tree_multiple_roots() {
    let temp_dir = setup_workspace_with_multiple_roots();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "markdown"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        - [Root A](root-a)
        - [Root B](root-b)
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_deep_nesting() {
    let temp_dir = setup_workspace_with_deep_nesting();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "markdown"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        - [Level 1](level1)
          - [Level 2](level2)
            - [Level 3](level3)
              - [Level 4](level4)
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_shared_child_multiple_parents() {
    let temp_dir = setup_workspace_with_shared_child();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "markdown"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        - [Parent A](parent-a)
          - [Shared Child](shared-child)
        - [Parent B](parent-b)
          - [Shared Child](shared-child)
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_circular_inclusion_no_roots() {
    let temp_dir = setup_workspace_with_circular_inclusion();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = "";

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_circular_inclusion_with_key_filter() {
    let temp_dir = setup_workspace_with_circular_inclusion();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-k", "doc-a", "-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        doc-a
        	doc-b
        		doc-c
        			doc-a
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_self_referencing_document() {
    let temp_dir = setup_workspace_with_self_reference();
    let temp_path = temp_dir.path();

    let output = run_tree_command(temp_path, &["-f", "keys"]);
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let expected = indoc! {"
        self-ref
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_tree_project_user_frontmatter() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let parent = indoc! {"
        ---
        pillar: ai-memory
        status: published
        ---

        # Parent

        [child](child)
    "};
    let child = indoc! {"
        ---
        pillar: ai-memory
        status: draft
        ---

        # Child

        Child content.
    "};

    write(temp_path.join("parent.md"), parent).expect("Should write parent");
    write(temp_path.join("child.md"), child).expect("Should write child");

    let output = run_tree_command(
        temp_path,
        &["-k", "parent", "--project", "pillar,status", "-f", "json"],
    );
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let parent_node = &parsed[0];
    assert_eq!(parent_node["key"], "parent");
    assert_eq!(parent_node["title"], "Parent");
    assert_eq!(parent_node["pillar"], "ai-memory");
    assert_eq!(parent_node["status"], "published");
    let children = parent_node["children"].as_array().expect("children is array");
    assert_eq!(children.len(), 1);
    let child_node = &children[0];
    assert_eq!(child_node["pillar"], "ai-memory");
    assert_eq!(child_node["status"], "draft");
    assert!(child_node["children"].is_array(), "children always present");
}

#[test]
fn test_tree_project_pseudo_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    write(
        temp_path.join("root.md"),
        indoc! {"
            # Root

            [child](child)
        "},
    )
    .unwrap();
    write(
        temp_path.join("child.md"),
        indoc! {"
            # Child

            Child body.
        "},
    )
    .unwrap();

    let output = run_tree_command(
        temp_path,
        &["-k", "root", "--project", "body=$content", "-f", "json"],
    );
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let root = &parsed[0];
    assert_eq!(root["key"], "root");
    assert_eq!(root["title"], "Root");
    assert!(
        root["body"]
            .as_str()
            .unwrap_or_default()
            .contains("# Root"),
        "expected body to contain root content, got: {:?}",
        root["body"]
    );
    assert!(root["children"].is_array());
    let child = &root["children"][0];
    assert!(child["body"]
        .as_str()
        .unwrap_or_default()
        .contains("# Child"));
}

#[test]
fn test_tree_project_pseudo_counts() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    write(
        temp_path.join("root.md"),
        indoc! {"
            # Root

            [a](a)

            [b](b)
        "},
    )
    .unwrap();
    write(temp_path.join("a.md"), "# A").unwrap();
    write(temp_path.join("b.md"), "# B").unwrap();

    let output = run_tree_command(
        temp_path,
        &[
            "-k",
            "root",
            "--project",
            "n=$includesCount",
            "-f",
            "json",
        ],
    );
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed[0]["n"], 2);
}

#[test]
fn test_tree_add_fields_extends_default_user_fm() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    write(
        temp_path.join("doc.md"),
        indoc! {"
            ---
            status: draft
            ---
            # Doc
        "},
    )
    .unwrap();

    let output = run_tree_command(
        temp_path,
        &["-k", "doc", "--add-fields", "status", "-f", "json"],
    );
    assert!(output.status.success(), "Tree command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed[0]["key"], "doc");
    assert_eq!(parsed[0]["title"], "Doc");
    assert_eq!(parsed[0]["status"], "draft");
}

#[test]
fn test_tree_project_and_add_fields_conflict() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    write(temp_path.join("doc.md"), "# Doc").unwrap();

    let output = run_tree_command(
        temp_path,
        &[
            "-k",
            "doc",
            "--project",
            "key",
            "--add-fields",
            "status",
        ],
    );
    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8");
    assert!(!output.status.success(), "expected conflict error");
    assert!(
        stderr.contains("cannot be used with"),
        "unexpected stderr: {}",
        stderr
    );
}

fn setup_workspace_with_self_reference() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let self_ref = indoc! {"
        # Self Reference

        This document references itself: [Self Reference](self-ref)
    "};

    write(temp_path.join("self-ref.md"), self_ref).expect("Should write self-ref");

    temp_dir
}

fn setup_workspace_with_circular_inclusion() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let doc_a = indoc! {"
        # Doc A

        [Doc B](doc-b)
    "};

    let doc_b = indoc! {"
        # Doc B

        [Doc C](doc-c)
    "};

    let doc_c = indoc! {"
        # Doc C

        [Doc A](doc-a)
    "};

    write(temp_path.join("doc-a.md"), doc_a).expect("Should write doc-a");
    write(temp_path.join("doc-b.md"), doc_b).expect("Should write doc-b");
    write(temp_path.join("doc-c.md"), doc_c).expect("Should write doc-c");

    temp_dir
}

fn setup_workspace_with_shared_child() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let parent_a = indoc! {"
        # Parent A

        [Shared Child](shared-child)
    "};

    let parent_b = indoc! {"
        # Parent B

        [Shared Child](shared-child)
    "};

    let shared_child = indoc! {"
        # Shared Child

        This document has multiple parents.
    "};

    write(temp_path.join("parent-a.md"), parent_a).expect("Should write parent-a");
    write(temp_path.join("parent-b.md"), parent_b).expect("Should write parent-b");
    write(temp_path.join("shared-child.md"), shared_child).expect("Should write shared-child");

    temp_dir
}

fn setup_workspace_with_linked_documents() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let main_content = indoc! {"
        # Main Document

        Some content here.

        [Child Document](child)
    "};

    let child_content = indoc! {"
        # Child Document

        Child content here.
    "};

    write(temp_path.join("main.md"), main_content).expect("Should write main");
    write(temp_path.join("child.md"), child_content).expect("Should write child");

    temp_dir
}

fn setup_workspace_with_multiple_roots() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let root_a = indoc! {"
        # Root A

        Content A.
    "};

    let root_b = indoc! {"
        # Root B

        Content B.
    "};

    write(temp_path.join("root-a.md"), root_a).expect("Should write root-a");
    write(temp_path.join("root-b.md"), root_b).expect("Should write root-b");

    temp_dir
}

fn setup_workspace_with_deep_nesting() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let level1 = indoc! {"
        # Level 1

        [Level 2](level2)
    "};

    let level2 = indoc! {"
        # Level 2

        [Level 3](level3)
    "};

    let level3 = indoc! {"
        # Level 3

        [Level 4](level4)
    "};

    let level4 = indoc! {"
        # Level 4

        Final level.
    "};

    write(temp_path.join("level1.md"), level1).expect("Should write level1");
    write(temp_path.join("level2.md"), level2).expect("Should write level2");
    write(temp_path.join("level3.md"), level3).expect("Should write level3");
    write(temp_path.join("level4.md"), level4).expect("Should write level4");

    temp_dir
}

fn setup_empty_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

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

    let config_content = toml::to_string(&config).expect("Failed to serialize config to TOML");

    write(temp_path.join(".iwe").join("config.toml"), config_content)
        .expect("Should write config file");
}

fn run_tree_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("tree").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe tree")
}
