use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

mod common;

fn setup_workspace() -> TempDir {
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
    write(temp_path.join(".iwe").join("config.toml"), config_content)
        .expect("Should write config file");

    temp_dir
}

fn run_iwe(dir: &std::path::Path, args: &[&str]) -> (String, String, bool) {
    let mut command = Command::new(common::get_iwe_binary_path());
    command.arg("retrieve").current_dir(dir);

    for arg in args {
        command.arg(arg);
    }

    let output = command.output().expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[test]
fn test_retrieve_basic_document() {
    let dir = setup_workspace();

    write(
        dir.path().join("test-doc.md"),
        indoc! {"
            # Test Document

            This is some content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "test-doc", "-d", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: test-doc
          title: Test Document
        ---

        # Test Document

        This is some content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_nonexistent_document() {
    let dir = setup_workspace();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "nonexistent"]);

    assert!(!success);
    assert_eq!(stdout, "");
    assert_eq!(stderr, "Error: Document 'nonexistent' not found\n");
}

#[test]
fn test_retrieve_json_format() {
    let dir = setup_workspace();

    write(
        dir.path().join("test-doc.md"),
        indoc! {"
            # Test Document

            Content here.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "test-doc", "-d", "0", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "test-doc",
              "title": "Test Document",
              "content": "# Test Document\n\nContent here.\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_yaml_format() {
    let dir = setup_workspace();

    write(
        dir.path().join("test-doc.md"),
        indoc! {"
            # Test Document

            Content here.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "test-doc", "-d", "0", "-f", "yaml"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        documents:
        - key: test-doc
          title: Test Document
          content: |
            # Test Document

            Content here.
          parent_documents: []
          child_documents: []
          backlinks: []
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_with_parent_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent Document

            ## Overview

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child Document

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: child
          title: Child Document
          parents:
          - key: parent
            title: Parent Document
        ---

        # Child Document

        Child content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_with_backlinks() {
    let dir = setup_workspace();

    write(
        dir.path().join("referrer.md"),
        indoc! {"
            # Referrer Document

            This text mentions [target](target) inline.
        "},
    )
    .unwrap();

    write(
        dir.path().join("target.md"),
        indoc! {"
            # Target Document

            Target content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "target", "-d", "0", "-b"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: target
          title: Target Document
          back-links:
          - key: referrer
            title: Referrer Document
        ---

        # Target Document

        Target content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_with_both_parent_and_backlinks() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("referrer.md"),
        indoc! {"
            # Referrer

            See also [child](child) for details.
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-b", "-c", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: child
          title: Child
          parents:
          - key: parent
            title: Parent
          back-links:
          - key: referrer
            title: Referrer
        ---

        # Child

        Child content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_depth_zero_excludes_referenced_docs() {
    let dir = setup_workspace();

    write(
        dir.path().join("root.md"),
        indoc! {"
            # Root

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "root", "-d", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: root
          title: Root
        ---

        # Root

        [Child](child)


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_depth_one_includes_referenced_docs() {
    let dir = setup_workspace();

    write(
        dir.path().join("root.md"),
        indoc! {"
            # Root

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "root", "-d", "1"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: root
          title: Root
        ---

        # Root

        [Child](child)


        ---
        document:
          key: child
          title: Child
          parents:
          - key: root
            title: Root
        ---

        # Child

        Child content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_depth_two_includes_nested_refs() {
    let dir = setup_workspace();

    write(
        dir.path().join("level0.md"),
        indoc! {"
            # Level Zero

            [level1](level1)
        "},
    )
    .unwrap();

    write(
        dir.path().join("level1.md"),
        indoc! {"
            # Level One

            [level2](level2)
        "},
    )
    .unwrap();

    write(
        dir.path().join("level2.md"),
        indoc! {"
            # Level Two

            Final content.
        "},
    )
    .unwrap();

    let (stdout_d1, stderr, success) = run_iwe(dir.path(), &["-k", "level0", "-d", "1"]);
    assert!(success, "stderr: {}", stderr);

    let expected_d1 = indoc! {"
        ---
        document:
          key: level0
          title: Level Zero
        ---

        # Level Zero

        [Level One](level1)


        ---
        document:
          key: level1
          title: Level One
          parents:
          - key: level0
            title: Level Zero
        ---

        # Level One

        [Level Two](level2)


    "};

    assert_eq!(stdout_d1, expected_d1);

    let (stdout_d2, stderr, success) = run_iwe(dir.path(), &["-k", "level0", "-d", "2"]);
    assert!(success, "stderr: {}", stderr);

    let expected_d2 = indoc! {"
        ---
        document:
          key: level0
          title: Level Zero
        ---

        # Level Zero

        [Level One](level1)


        ---
        document:
          key: level1
          title: Level One
          parents:
          - key: level0
            title: Level Zero
        ---

        # Level One

        [Level Two](level2)


        ---
        document:
          key: level2
          title: Level Two
          parents:
          - key: level1
            title: Level One
        ---

        # Level Two

        Final content.


    "};

    assert_eq!(stdout_d2, expected_d2);
}

#[test]
fn test_retrieve_context_one_level() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent Document

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child Document

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "1"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: child
          title: Child Document
          parents:
          - key: parent
            title: Parent Document
        ---

        # Child Document

        Child content.


        ---
        document:
          key: parent
          title: Parent Document
        ---

        # Parent Document

        [Child Document](child)


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_two_levels() {
    let dir = setup_workspace();

    write(
        dir.path().join("grandparent.md"),
        indoc! {"
            # Grandparent

            [parent](parent)
        "},
    )
    .unwrap();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "2"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: child
          title: Child
          parents:
          - key: parent
            title: Parent
        ---

        # Child

        Child content.


        ---
        document:
          key: parent
          title: Parent
          parents:
          - key: grandparent
            title: Grandparent
        ---

        # Parent

        [Child](child)


        ---
        document:
          key: grandparent
          title: Grandparent
        ---

        # Grandparent

        [Parent](parent)


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_bidirectional() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [middle](middle)
        "},
    )
    .unwrap();

    write(
        dir.path().join("middle.md"),
        indoc! {"
            # Middle

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "middle", "-d", "1", "-c", "1"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: middle
          title: Middle
          parents:
          - key: parent
            title: Parent
        ---

        # Middle

        [Child](child)


        ---
        document:
          key: child
          title: Child
          parents:
          - key: middle
            title: Middle
        ---

        # Child

        Child content.


        ---
        document:
          key: parent
          title: Parent
        ---

        # Parent

        [Middle](middle)


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_with_inline_links() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            # Document

            This text mentions [another](another) inline.
        "},
    )
    .unwrap();

    write(
        dir.path().join("another.md"),
        indoc! {"
            # Another Document

            Some content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0", "-l"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: doc
          title: Document
        ---

        # Document

        This text mentions [Another Document](another) inline.


        ---
        document:
          key: another
          title: Another Document
          back-links:
          - key: doc
            title: Document
        ---

        # Another Document

        Some content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_json_format() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "1", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "child",
              "title": "Child",
              "content": "# Child\n\nContent.\n",
              "parent_documents": [
                {
                  "key": "parent",
                  "title": "Parent",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "parent",
              "title": "Parent",
              "content": "# Parent\n\n[Child](child)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_no_parents() {
    let dir = setup_workspace();

    write(
        dir.path().join("orphan.md"),
        indoc! {"
            # Orphan Document

            No parents here.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "orphan", "-d", "0", "-c", "1"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: orphan
          title: Orphan Document
        ---

        # Orphan Document

        No parents here.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_links_with_depth_and_context() {
    let dir = setup_workspace();

    write(
        dir.path().join("grandparent.md"),
        indoc! {"
            # Grandparent

            [parent](parent)
        "},
    )
    .unwrap();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [middle](middle)
        "},
    )
    .unwrap();

    write(
        dir.path().join("middle.md"),
        indoc! {"
            # Middle

            [child](child)

            See also [related](related) for more info.
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    write(
        dir.path().join("related.md"),
        indoc! {"
            # Related

            Related content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "middle", "-d", "1", "-c", "1", "-l"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: middle
          title: Middle
          parents:
          - key: parent
            title: Parent
        ---

        # Middle

        [Child](child)

        See also [Related](related) for more info.


        ---
        document:
          key: child
          title: Child
          parents:
          - key: middle
            title: Middle
        ---

        # Child

        Child content.


        ---
        document:
          key: parent
          title: Parent
          parents:
          - key: grandparent
            title: Grandparent
        ---

        # Parent

        [Middle](middle)


        ---
        document:
          key: related
          title: Related
          back-links:
          - key: middle
            title: Middle
        ---

        # Related

        Related content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_deduplication_same_doc_multiple_paths() {
    let dir = setup_workspace();

    write(
        dir.path().join("root.md"),
        indoc! {"
            # Root

            [a](a)

            [b](b)
        "},
    )
    .unwrap();

    write(
        dir.path().join("a.md"),
        indoc! {"
            # A

            [common](common)
        "},
    )
    .unwrap();

    write(
        dir.path().join("b.md"),
        indoc! {"
            # B

            [common](common)
        "},
    )
    .unwrap();

    write(
        dir.path().join("common.md"),
        indoc! {"
            # Common

            Shared content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "root", "-d", "2", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "root",
              "title": "Root",
              "content": "# Root\n\n[A](a)\n\n[B](b)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "a",
              "title": "A",
              "content": "# A\n\n[Common](common)\n",
              "parent_documents": [
                {
                  "key": "root",
                  "title": "Root",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "b",
              "title": "B",
              "content": "# B\n\n[Common](common)\n",
              "parent_documents": [
                {
                  "key": "root",
                  "title": "Root",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "common",
              "title": "Common",
              "content": "# Common\n\nShared content.\n",
              "parent_documents": [
                {
                  "key": "a",
                  "title": "A",
                  "section_path": []
                },
                {
                  "key": "b",
                  "title": "B",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_multiple_inline_links() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            # Document

            Check [first](first) and [second](second) for details.
        "},
    )
    .unwrap();

    write(
        dir.path().join("first.md"),
        indoc! {"
            # First

            First content.
        "},
    )
    .unwrap();

    write(
        dir.path().join("second.md"),
        indoc! {"
            # Second

            Second content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0", "-l"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: doc
          title: Document
        ---

        # Document

        Check [First](first) and [Second](second) for details.


        ---
        document:
          key: first
          title: First
          back-links:
          - key: doc
            title: Document
        ---

        # First

        First content.


        ---
        document:
          key: second
          title: Second
          back-links:
          - key: doc
            title: Document
        ---

        # Second

        Second content.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_links_without_flag_excludes_inline_refs() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            # Document

            See [another](another) inline.
        "},
    )
    .unwrap();

    write(
        dir.path().join("another.md"),
        indoc! {"
            # Another

            Content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: doc
          title: Document
        ---

        # Document

        See [Another](another) inline.


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_all_document_types_json() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [main](main)
        "},
    )
    .unwrap();

    write(
        dir.path().join("main.md"),
        indoc! {"
            # Main

            [child](child)

            Also see [linked](linked).
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    write(
        dir.path().join("linked.md"),
        indoc! {"
            # Linked

            Linked content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "main", "-d", "1", "-c", "1", "-l", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "main",
              "title": "Main",
              "content": "# Main\n\n[Child](child)\n\nAlso see [Linked](linked).\n",
              "parent_documents": [
                {
                  "key": "parent",
                  "title": "Parent",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "child",
              "title": "Child",
              "content": "# Child\n\nChild content.\n",
              "parent_documents": [
                {
                  "key": "main",
                  "title": "Main",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "parent",
              "title": "Parent",
              "content": "# Parent\n\n[Main](main)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "linked",
              "title": "Linked",
              "content": "# Linked\n\nLinked content.\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": [
                {
                  "key": "main",
                  "title": "Main",
                  "section_path": []
                }
              ]
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_multiple_parents() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent1.md"),
        indoc! {"
            # Parent One

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("parent2.md"),
        indoc! {"
            # Parent Two

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Shared child.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "1", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "child",
              "title": "Child",
              "content": "# Child\n\nShared child.\n",
              "parent_documents": [
                {
                  "key": "parent1",
                  "title": "Parent One",
                  "section_path": []
                },
                {
                  "key": "parent2",
                  "title": "Parent Two",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "parent1",
              "title": "Parent One",
              "content": "# Parent One\n\n[Child](child)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "parent2",
              "title": "Parent Two",
              "content": "# Parent Two\n\n[Child](child)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_includes_parents_of_sub_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("main.md"),
        indoc! {"
            # Main Document

            [a](a)
        "},
    )
    .unwrap();

    write(
        dir.path().join("a.md"),
        indoc! {"
            # Document A

            Content of A.
        "},
    )
    .unwrap();

    write(
        dir.path().join("parent2.md"),
        indoc! {"
            # Parent Two

            [a](a)
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "main", "-d", "1", "-c", "1", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "main",
              "title": "Main Document",
              "content": "# Main Document\n\n[Document A](a)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "a",
              "title": "Document A",
              "content": "# Document A\n\nContent of A.\n",
              "parent_documents": [
                {
                  "key": "main",
                  "title": "Main Document",
                  "section_path": []
                },
                {
                  "key": "parent2",
                  "title": "Parent Two",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "parent2",
              "title": "Parent Two",
              "content": "# Parent Two\n\n[Document A](a)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_sub_document_parents_without_depth() {
    let dir = setup_workspace();

    write(
        dir.path().join("main.md"),
        indoc! {"
            # Main Document

            [a](a)
        "},
    )
    .unwrap();

    write(
        dir.path().join("a.md"),
        indoc! {"
            # Document A

            Content of A.
        "},
    )
    .unwrap();

    write(
        dir.path().join("parent2.md"),
        indoc! {"
            # Parent Two

            [a](a)
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "main", "-d", "0", "-c", "1", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "main",
              "title": "Main Document",
              "content": "# Main Document\n\n[Document A](a)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_context_only_direct_sub_document_parents() {
    let dir = setup_workspace();

    write(
        dir.path().join("main.md"),
        indoc! {"
            # Main

            [level1](level1)
        "},
    )
    .unwrap();

    write(
        dir.path().join("level1.md"),
        indoc! {"
            # Level 1

            [level2](level2)
        "},
    )
    .unwrap();

    write(
        dir.path().join("level2.md"),
        indoc! {"
            # Level 2

            Final content.
        "},
    )
    .unwrap();

    write(
        dir.path().join("other-parent.md"),
        indoc! {"
            # Other Parent

            [level2](level2)
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "main", "-d", "2", "-c", "1", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "main",
              "title": "Main",
              "content": "# Main\n\n[Level 1](level1)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "level1",
              "title": "Level 1",
              "content": "# Level 1\n\n[Level 2](level2)\n",
              "parent_documents": [
                {
                  "key": "main",
                  "title": "Main",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "level2",
              "title": "Level 2",
              "content": "# Level 2\n\nFinal content.\n",
              "parent_documents": [
                {
                  "key": "level1",
                  "title": "Level 1",
                  "section_path": []
                },
                {
                  "key": "other-parent",
                  "title": "Other Parent",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_dry_run_basic() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            # Test Document

            Line one.
            Line two.
            Line three.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0", "--dry-run"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        documents: 1
        lines: 3
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_dry_run_multiple_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "1", "--dry-run"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        documents: 2
        lines: 6
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_multiple_keys() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Document One

            Content one.
        "},
    )
    .unwrap();

    write(
        dir.path().join("doc2.md"),
        indoc! {"
            # Document Two

            Content two.
        "},
    )
    .unwrap();

    write(
        dir.path().join("doc3.md"),
        indoc! {"
            # Document Three

            Content three.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc1", "-k", "doc2", "-k", "doc3", "-d", "0", "-c", "0", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "doc1",
              "title": "Document One",
              "content": "# Document One\n\nContent one.\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "doc2",
              "title": "Document Two",
              "content": "# Document Two\n\nContent two.\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "doc3",
              "title": "Document Three",
              "content": "# Document Three\n\nContent three.\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_multiple_keys_with_deduplication() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Document One

            [shared](shared)
        "},
    )
    .unwrap();

    write(
        dir.path().join("doc2.md"),
        indoc! {"
            # Document Two

            [shared](shared)
        "},
    )
    .unwrap();

    write(
        dir.path().join("shared.md"),
        indoc! {"
            # Shared

            Shared content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc1", "-k", "doc2", "-d", "1", "-c", "0", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "doc1",
              "title": "Document One",
              "content": "# Document One\n\n[Shared](shared)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "shared",
              "title": "Shared",
              "content": "# Shared\n\nShared content.\n",
              "parent_documents": [
                {
                  "key": "doc1",
                  "title": "Document One",
                  "section_path": []
                },
                {
                  "key": "doc2",
                  "title": "Document Two",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "doc2",
              "title": "Document Two",
              "content": "# Document Two\n\n[Shared](shared)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_exclude_single_key() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "1", "-c", "0", "-e", "child", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "parent",
              "title": "Parent",
              "content": "# Parent\n\n[Child](child)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_exclude_multiple_keys() {
    let dir = setup_workspace();

    write(
        dir.path().join("root.md"),
        indoc! {"
            # Root

            [a](a)

            [b](b)

            [c](c)
        "},
    )
    .unwrap();

    write(dir.path().join("a.md"), "# A\n\nContent A.").unwrap();
    write(dir.path().join("b.md"), "# B\n\nContent B.").unwrap();
    write(dir.path().join("c.md"), "# C\n\nContent C.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "root", "-d", "1", "-c", "0", "-e", "a", "-e", "c", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "root",
              "title": "Root",
              "content": "# Root\n\n[A](a)\n\n[B](b)\n\n[C](c)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "b",
              "title": "B",
              "content": "# B\n\nContent B.\n",
              "parent_documents": [
                {
                  "key": "root",
                  "title": "Root",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_exclude_main_document() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            # Document

            Content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0", "-c", "0", "-e", "doc", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "documents": []
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_no_content_flag() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            # Document

            This is the content that should be excluded.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0", "-c", "0", "--no-content", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "documents": [
            {
              "key": "doc",
              "title": "Document",
              "content": "",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_no_content_with_multiple_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "1", "-c", "0", "--no-content", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "documents": [
            {
              "key": "parent",
              "title": "Parent",
              "content": "",
              "parent_documents": [],
              "child_documents": [
                {
                  "key": "child",
                  "title": "Child"
                }
              ],
              "backlinks": []
            },
            {
              "key": "child",
              "title": "Child",
              "content": "",
              "parent_documents": [
                {
                  "key": "parent",
                  "title": "Parent",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_no_content_preserves_parent_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "0", "--no-content", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "documents": [
            {
              "key": "child",
              "title": "Child",
              "content": "",
              "parent_documents": [
                {
                  "key": "parent",
                  "title": "Parent",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_no_content_multiple_children() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child1](child1)

            [child2](child2)

            [child3](child3)
        "},
    )
    .unwrap();

    write(dir.path().join("child1.md"), "# Child One\n\nContent 1.").unwrap();
    write(dir.path().join("child2.md"), "# Child Two\n\nContent 2.").unwrap();
    write(dir.path().join("child3.md"), "# Child Three\n\nContent 3.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "0", "-c", "0", "--no-content"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: parent
          title: Parent
        ---



    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_keys_format_output() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "1", "-c", "0", "-f", "keys"]);

    assert!(success, "stderr: {}", stderr);

    let keys: Vec<&str> = stdout.lines().collect();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], "parent");
    assert_eq!(keys[1], "child");
}

#[test]
fn test_retrieve_keys_format_with_context() {
    let dir = setup_workspace();

    write(
        dir.path().join("grandparent.md"),
        indoc! {"
            # Grandparent

            [parent](parent)
        "},
    )
    .unwrap();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "child", "-d", "0", "-c", "1", "-f", "keys"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        child
        parent
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_default_depth() {
    let dir = setup_workspace();

    write(
        dir.path().join("root.md"),
        indoc! {"
            # Root

            [child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child

            Child content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "root", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "root",
              "title": "Root",
              "content": "# Root\n\n[Child](child)\n",
              "parent_documents": [],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "child",
              "title": "Child",
              "content": "# Child\n\nChild content.\n",
              "parent_documents": [
                {
                  "key": "root",
                  "title": "Root",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_cyclic_references() {
    let dir = setup_workspace();

    write(
        dir.path().join("a.md"),
        indoc! {"
            # Document A

            [b](b)
        "},
    )
    .unwrap();

    write(
        dir.path().join("b.md"),
        indoc! {"
            # Document B

            [a](a)
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "a", "-d", "2", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "a",
              "title": "Document A",
              "content": "# Document A\n\n[Document B](b)\n",
              "parent_documents": [
                {
                  "key": "b",
                  "title": "Document B",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            },
            {
              "key": "b",
              "title": "Document B",
              "content": "# Document B\n\n[Document A](a)\n",
              "parent_documents": [
                {
                  "key": "a",
                  "title": "Document A",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_parent_annotation_excludes_current_document() {
    let dir = setup_workspace();

    write(
        dir.path().join("main.md"),
        indoc! {"
            # Main

            [Shared](shared)

            [Exclusive](exclusive)
        "},
    )
    .unwrap();

    write(
        dir.path().join("other.md"),
        indoc! {"
            # Other

            [Shared](shared)
        "},
    )
    .unwrap();

    write(
        dir.path().join("shared.md"),
        indoc! {"
            # Shared

            Shared content.
        "},
    )
    .unwrap();

    write(
        dir.path().join("exclusive.md"),
        indoc! {"
            # Exclusive

            Exclusive content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "main", "-d", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ---
        document:
          key: main
          title: Main
        ---

        # Main

        [Shared](shared) <- [Other](other)

        [Exclusive](exclusive)


    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_self_referencing_document() {
    let dir = setup_workspace();

    write(
        dir.path().join("self.md"),
        indoc! {"
            # Self Reference

            [self](self)
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "self", "-d", "1", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        {
          "documents": [
            {
              "key": "self",
              "title": "Self Reference",
              "content": "# Self Reference\n\n[Self Reference](self)\n",
              "parent_documents": [
                {
                  "key": "self",
                  "title": "Self Reference",
                  "section_path": []
                }
              ],
              "child_documents": [],
              "backlinks": []
            }
          ]
        }
    "##};

    assert_eq!(stdout, expected);
}

