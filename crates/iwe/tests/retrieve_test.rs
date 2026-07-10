use diwe::config::{Configuration, LibraryOptions, MarkdownOptions};
use indoc::indoc;
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

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
    let mut command = Command::new(crate::common::get_iwe_binary_path());
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
        ````markdown #test-doc
        ---
        title: Test Document
        ---

        # Test Document

        This is some content.
        ````
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

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["-k", "test-doc", "-d", "0", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "test-doc",
            "title": "Test Document",
            "content": "# Test Document\n\nContent here.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["-k", "test-doc", "-d", "0", "-f", "yaml"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        - key: test-doc
          title: Test Document
          content: |
            # Test Document

            Content here.
          references: []
          includes: []
          referencedBy: []
          includedBy: []
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
        ````markdown #child
        ---
        title: Child Document
        includedBy:
        - key: parent
          title: Parent Document
          sectionPath:
          - Overview
        ---

        # Child Document

        Child content.
        ````
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
        ````markdown #target
        ---
        title: Target Document
        referencedBy:
        - key: referrer
          title: Referrer Document
        ---

        # Target Document

        Target content.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_backlinks_false_excludes_incoming_references() {
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "target", "-d", "0", "--backlinks", "false"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #target
        ---
        title: Target Document
        ---

        # Target Document

        Target content.
        ````
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

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["-k", "child", "-d", "0", "-b", "-c", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #child
        ---
        title: Child
        referencedBy:
        - key: referrer
          title: Referrer
        includedBy:
        - key: parent
          title: Parent
        ---

        # Child

        Child content.
        ````
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
        ````markdown #root
        ---
        title: Root
        ---

        # Root

        [Child](child)
        ````
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
        ````markdown #root
        ---
        title: Root
        ---

        # Root

        [Child](child)
        ````

        ````markdown #child
        ---
        title: Child
        includedBy:
        - key: root
          title: Root
        ---

        # Child

        Child content.
        ````
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
        ````markdown #level0
        ---
        title: Level Zero
        ---

        # Level Zero

        [Level One](level1)
        ````

        ````markdown #level1
        ---
        title: Level One
        includedBy:
        - key: level0
          title: Level Zero
        ---

        # Level One

        [Level Two](level2)
        ````
    "};

    assert_eq!(stdout_d1, expected_d1);

    let (stdout_d2, stderr, success) = run_iwe(dir.path(), &["-k", "level0", "-d", "2"]);
    assert!(success, "stderr: {}", stderr);

    let expected_d2 = indoc! {"
        ````markdown #level0
        ---
        title: Level Zero
        ---

        # Level Zero

        [Level One](level1)
        ````

        ````markdown #level1
        ---
        title: Level One
        includedBy:
        - key: level0
          title: Level Zero
        ---

        # Level One

        [Level Two](level2)
        ````

        ````markdown #level2
        ---
        title: Level Two
        includedBy:
        - key: level1
          title: Level One
        ---

        # Level Two

        Final content.
        ````
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
        ````markdown #child
        ---
        title: Child Document
        includedBy:
        - key: parent
          title: Parent Document
        ---

        # Child Document

        Child content.
        ````

        ````markdown #parent
        ---
        title: Parent Document
        ---

        # Parent Document

        [Child Document](child)
        ````
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
        ````markdown #child
        ---
        title: Child
        includedBy:
        - key: parent
          title: Parent
        ---

        # Child

        Child content.
        ````

        ````markdown #parent
        ---
        title: Parent
        includedBy:
        - key: grandparent
          title: Grandparent
        ---

        # Parent

        [Child](child)
        ````

        ````markdown #grandparent
        ---
        title: Grandparent
        ---

        # Grandparent

        [Parent](parent)
        ````
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
        ````markdown #middle
        ---
        title: Middle
        includedBy:
        - key: parent
          title: Parent
        ---

        # Middle

        [Child](child)
        ````

        ````markdown #child
        ---
        title: Child
        includedBy:
        - key: middle
          title: Middle
        ---

        # Child

        Child content.
        ````

        ````markdown #parent
        ---
        title: Parent
        ---

        # Parent

        [Middle](middle)
        ````
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
        ````markdown #doc
        ---
        title: Document
        references:
        - key: another
          title: Another Document
        ---

        # Document

        This text mentions [Another Document](another) inline.
        ````

        ````markdown #another
        ---
        title: Another Document
        referencedBy:
        - key: doc
          title: Document
        ---

        # Another Document

        Some content.
        ````
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "child", "-d", "0", "-c", "1", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "child",
            "title": "Child",
            "content": "# Child\n\nContent.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "parent",
                "title": "Parent",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "parent",
            "title": "Parent",
            "content": "# Parent\n\n[Child](child)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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
        ````markdown #orphan
        ---
        title: Orphan Document
        ---

        # Orphan Document

        No parents here.
        ````
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

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["-k", "middle", "-d", "1", "-c", "1", "-l"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #middle
        ---
        title: Middle
        references:
        - key: related
          title: Related
        includedBy:
        - key: parent
          title: Parent
        ---

        # Middle

        [Child](child)

        See also [Related](related) for more info.
        ````

        ````markdown #child
        ---
        title: Child
        includedBy:
        - key: middle
          title: Middle
        ---

        # Child

        Child content.
        ````

        ````markdown #parent
        ---
        title: Parent
        includedBy:
        - key: grandparent
          title: Grandparent
        ---

        # Parent

        [Middle](middle)
        ````

        ````markdown #related
        ---
        title: Related
        referencedBy:
        - key: middle
          title: Middle
        ---

        # Related

        Related content.
        ````
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
        [
          {
            "key": "root",
            "title": "Root",
            "content": "# Root\n\n[A](a)\n\n[B](b)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "a",
            "title": "A",
            "content": "# A\n\n[Common](common)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "root",
                "title": "Root",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "b",
            "title": "B",
            "content": "# B\n\n[Common](common)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "root",
                "title": "Root",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "common",
            "title": "Common",
            "content": "# Common\n\nShared content.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "a",
                "title": "A",
                "sectionPath": []
              },
              {
                "key": "b",
                "title": "B",
                "sectionPath": []
              }
            ]
          }
        ]
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
        ````markdown #doc
        ---
        title: Document
        references:
        - key: first
          title: First
        - key: second
          title: Second
        ---

        # Document

        Check [First](first) and [Second](second) for details.
        ````

        ````markdown #first
        ---
        title: First
        referencedBy:
        - key: doc
          title: Document
        ---

        # First

        First content.
        ````

        ````markdown #second
        ---
        title: Second
        referencedBy:
        - key: doc
          title: Document
        ---

        # Second

        Second content.
        ````
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
        ````markdown #doc
        ---
        title: Document
        ---

        # Document

        See [Another](another) inline.
        ````
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "main", "-d", "1", "-c", "1", "-l", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "main",
            "title": "Main",
            "content": "# Main\n\n[Child](child)\n\nAlso see [Linked](linked).\n",
            "references": [
              {
                "key": "linked",
                "title": "Linked",
                "sectionPath": []
              }
            ],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "parent",
                "title": "Parent",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "child",
            "title": "Child",
            "content": "# Child\n\nChild content.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "main",
                "title": "Main",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "parent",
            "title": "Parent",
            "content": "# Parent\n\n[Main](main)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "linked",
            "title": "Linked",
            "content": "# Linked\n\nLinked content.\n",
            "references": [],
            "includes": [],
            "referencedBy": [
              {
                "key": "main",
                "title": "Main",
                "sectionPath": []
              }
            ],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "child", "-d", "0", "-c", "1", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "child",
            "title": "Child",
            "content": "# Child\n\nShared child.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "parent1",
                "title": "Parent One",
                "sectionPath": []
              },
              {
                "key": "parent2",
                "title": "Parent Two",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "parent1",
            "title": "Parent One",
            "content": "# Parent One\n\n[Child](child)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "parent2",
            "title": "Parent Two",
            "content": "# Parent Two\n\n[Child](child)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "main", "-d", "1", "-c", "1", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "main",
            "title": "Main Document",
            "content": "# Main Document\n\n[Document A](a)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "a",
            "title": "Document A",
            "content": "# Document A\n\nContent of A.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "main",
                "title": "Main Document",
                "sectionPath": []
              },
              {
                "key": "parent2",
                "title": "Parent Two",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "parent2",
            "title": "Parent Two",
            "content": "# Parent Two\n\n[Document A](a)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "main", "-d", "0", "-c", "1", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "main",
            "title": "Main Document",
            "content": "# Main Document\n\n[Document A](a)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "main", "-d", "2", "-c", "1", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "main",
            "title": "Main",
            "content": "# Main\n\n[Level 1](level1)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "level1",
            "title": "Level 1",
            "content": "# Level 1\n\n[Level 2](level2)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "main",
                "title": "Main",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "level2",
            "title": "Level 2",
            "content": "# Level 2\n\nFinal content.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "level1",
                "title": "Level 1",
                "sectionPath": []
              },
              {
                "key": "other-parent",
                "title": "Other Parent",
                "sectionPath": []
              }
            ]
          }
        ]
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_children_flag_populates_includes() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(dir.path().join("child.md"), "# Child\n\nChild content.").unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "parent", "-d", "0", "--children", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON output");
    let parent = &parsed[0];
    assert_eq!(parent["key"], "parent");
    let includes = parent["includes"].as_array().expect("includes is array");
    assert_eq!(includes.len(), 1);
    assert_eq!(includes[0]["key"], "child");
    assert_eq!(includes[0]["title"], "Child");
    assert!(parent["content"].as_str().unwrap().contains("# Parent"));
}

#[test]
fn test_retrieve_without_children_does_not_populate_includes() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(dir.path().join("child.md"), "# Child\n\nChild content.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "0", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON output");
    let parent = &parsed[0];
    assert!(parent["content"].as_str().unwrap().contains("# Parent"));
    let includes = parent["includes"].as_array().expect("includes is array");
    assert!(includes.is_empty());
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k", "doc1", "-k", "doc2", "-k", "doc3", "-d", "0", "-c", "0", "-f", "json",
        ],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "doc1",
            "title": "Document One",
            "content": "# Document One\n\nContent one.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Document Two",
            "content": "# Document Two\n\nContent two.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "doc3",
            "title": "Document Three",
            "content": "# Document Three\n\nContent three.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k", "doc1", "-k", "doc2", "-d", "1", "-c", "0", "-f", "json",
        ],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "doc1",
            "title": "Document One",
            "content": "# Document One\n\n[Shared](shared)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "shared",
            "title": "Shared",
            "content": "# Shared\n\nShared content.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "doc1",
                "title": "Document One",
                "sectionPath": []
              },
              {
                "key": "doc2",
                "title": "Document Two",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "doc2",
            "title": "Document Two",
            "content": "# Document Two\n\n[Shared](shared)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k", "parent", "-d", "1", "-c", "0", "-e", "child", "-f", "json",
        ],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "parent",
            "title": "Parent",
            "content": "# Parent\n\n[Child](child)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k", "root", "-d", "1", "-c", "0", "-e", "a", "-e", "c", "-f", "json",
        ],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "root",
            "title": "Root",
            "content": "# Root\n\n[A](a)\n\n[B](b)\n\n[C](c)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "b",
            "title": "B",
            "content": "# B\n\nContent B.\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "root",
                "title": "Root",
                "sectionPath": []
              }
            ]
          }
        ]
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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "doc", "-d", "0", "-c", "0", "-e", "doc", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        []
    "#};

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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "parent", "-d", "1", "-c", "0", "-f", "keys"],
    );

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

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "child", "-d", "0", "-c", "1", "-f", "keys"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        child
        parent
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_default_is_doc_only() {
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
        [
          {
            "key": "root",
            "title": "Root",
            "content": "# Root\n\n[Child](child)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
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
        [
          {
            "key": "a",
            "title": "Document A",
            "content": "# Document A\n\n[Document B](b)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "b",
                "title": "Document B",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "b",
            "title": "Document B",
            "content": "# Document B\n\n[Document A](a)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "a",
                "title": "Document A",
                "sectionPath": []
              }
            ]
          }
        ]
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
        ````markdown #main
        ---
        title: Main
        ---

        # Main

        [Shared](shared) <- [Other](other)

        [Exclusive](exclusive)
        ````
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
        [
          {
            "key": "self",
            "title": "Self Reference",
            "content": "# Self Reference\n\n[Self Reference](self)\n",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "self",
                "title": "Self Reference",
                "sectionPath": []
              }
            ]
          }
        ]
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_frontmatter_not_duplicated_in_body() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc.md"),
        indoc! {"
            ---
            status: draft
            tags:
              - rust
            ---
            # My Document

            Body text here.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "doc", "-d", "0"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc
        ---
        title: My Document
        ---

        # My Document

        Body text here.
        ````
    "};

    assert_eq!(stdout, expected);

    let body_start = stdout.find("---\n\n").expect("end of frontmatter");
    let body = &stdout[body_start + 5..];
    assert!(
        !body.contains("---"),
        "source frontmatter leaked into body: {}",
        stdout
    );
}

#[test]
fn test_retrieve_markdown_children_populates_includes() {
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

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-k", "parent", "-d", "0", "--children"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #parent
        ---
        title: Parent
        includes:
        - key: child
          title: Child
        ---

        # Parent

        [Child](child)
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_retrieve_max_documents_warns_when_truncating() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        "# Doc One\n\nfirst body content here\n",
    )
    .unwrap();
    write(
        dir.path().join("doc2.md"),
        "# Doc Two\n\nsecond body content here\n",
    )
    .unwrap();
    write(
        dir.path().join("doc3.md"),
        "# Doc Three\n\nthird body content here\n",
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k",
            "doc1",
            "-k",
            "doc2",
            "-k",
            "doc3",
            "-f",
            "keys",
            "--max-documents",
            "2",
        ],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "doc1\ndoc2\n");
    assert_eq!(
        stderr,
        "warning: output truncated — returned 2/3 documents; ~18 tokens. Narrow with --filter or raise --max-documents.\n"
    );
}

#[test]
fn test_retrieve_max_document_tokens_truncates_body() {
    let dir = setup_workspace();

    write(
        dir.path().join("notes.md"),
        "# Notes\n\nalpha beta gamma delta epsilon zeta eta theta\n",
    )
    .unwrap();

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["-k", "notes", "--max-document-tokens", "4"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #notes
        ---
        title: Notes
        ---

        # Notes

        alpha

        ⋯ truncated (9 tokens omitted)
        ````
    "};

    assert_eq!(stdout, expected);
    assert_eq!(
        stderr,
        "warning: output truncated — returned 1/1 documents, 1 clipped to --max-document-tokens; ~13 tokens. Narrow with --filter or raise --max-document-tokens.\n"
    );
}

#[test]
fn test_retrieve_max_tokens_drops_trailing_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        "# Doc One\n\nfirst body content here\n",
    )
    .unwrap();
    write(
        dir.path().join("doc2.md"),
        "# Doc Two\n\nsecond body content here\n",
    )
    .unwrap();
    write(
        dir.path().join("doc3.md"),
        "# Doc Three\n\nthird body content here\n",
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k",
            "doc1",
            "-k",
            "doc2",
            "-k",
            "doc3",
            "-f",
            "keys",
            "--max-tokens",
            "12",
        ],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "doc1\n");
    assert_eq!(
        stderr,
        "warning: output truncated — returned 1/3 documents; ~9 tokens (budget 12). Narrow with --filter or raise --max-documents/--max-tokens.\n"
    );
}

#[test]
fn test_retrieve_max_tokens_zero_disables_budget() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        "# Doc One\n\nfirst body content here\n",
    )
    .unwrap();
    write(
        dir.path().join("doc2.md"),
        "# Doc Two\n\nsecond body content here\n",
    )
    .unwrap();
    write(
        dir.path().join("doc3.md"),
        "# Doc Three\n\nthird body content here\n",
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k",
            "doc1",
            "-k",
            "doc2",
            "-k",
            "doc3",
            "-f",
            "keys",
            "--max-tokens",
            "0",
        ],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "doc1\ndoc2\ndoc3\n");
    assert_eq!(stderr, "");
}

#[test]
fn test_retrieve_max_tokens_counts_edges() {
    let dir = setup_workspace();

    write(dir.path().join("hub1.md"), "# Hub One\n\nx\n").unwrap();
    write(dir.path().join("hub2.md"), "# Hub Two\n\ny\n").unwrap();
    write(
        dir.path().join("refa.md"),
        "# Referencing Document Alpha With A Fairly Long Title\n\n[Hub One](hub1) [Hub Two](hub2)\n",
    )
    .unwrap();
    write(
        dir.path().join("refb.md"),
        "# Referencing Document Beta With A Fairly Long Title\n\n[Hub One](hub1) [Hub Two](hub2)\n",
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k",
            "hub1",
            "-k",
            "hub2",
            "-f",
            "keys",
            "--max-tokens",
            "40",
        ],
    );
    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "hub1\n");
    assert_eq!(
        stderr,
        "warning: output truncated — returned 1/2 documents; ~56 tokens (budget 40). Narrow with --filter or raise --max-documents/--max-tokens.\n"
    );

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k",
            "hub1",
            "-k",
            "hub2",
            "--backlinks",
            "false",
            "-f",
            "keys",
            "--max-tokens",
            "40",
        ],
    );
    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "hub1\nhub2\n");
    assert_eq!(stderr, "");
}

#[test]
fn test_retrieve_expand_matches_legacy_depth_context() {
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

    let (expand_out, _, expand_ok) = run_iwe(
        dir.path(),
        &[
            "-k",
            "child",
            "--expand-includes",
            "1",
            "--expand-included-by",
            "1",
            "-f",
            "json",
        ],
    );
    let (legacy_out, _, legacy_ok) = run_iwe(
        dir.path(),
        &["-k", "child", "-d", "1", "-c", "1", "-f", "json"],
    );

    assert!(expand_ok);
    assert!(legacy_ok);
    assert_eq!(expand_out, legacy_out);
}

#[test]
fn test_retrieve_expand_referenced_by_pulls_referencing_docs() {
    let dir = setup_workspace();

    write(
        dir.path().join("source.md"),
        indoc! {"
            # Source

            This mentions [target](target) inline.
        "},
    )
    .unwrap();
    write(
        dir.path().join("target.md"),
        indoc! {"
            # Target

            Target content.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "target", "--expand-referenced-by", "1", "-f", "keys"],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "target\nsource\n");
}

#[test]
fn test_retrieve_expand_references_transitive() {
    let dir = setup_workspace();

    write(dir.path().join("a.md"), "# A\n\nSee [b](b) next.\n").unwrap();
    write(dir.path().join("b.md"), "# B\n\nSee [c](c) next.\n").unwrap();
    write(dir.path().join("c.md"), "# C\n\nThe end.\n").unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "a", "--expand-references", "2", "-f", "keys"],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "a\nb\nc\n");
}

#[test]
fn test_retrieve_expand_unbounded_zero() {
    let dir = setup_workspace();

    write(dir.path().join("l0.md"), "# L0\n\n[l1](l1)\n").unwrap();
    write(dir.path().join("l1.md"), "# L1\n\n[l2](l2)\n").unwrap();
    write(dir.path().join("l2.md"), "# L2\n\nLeaf.\n").unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-k", "l0", "--expand-includes", "0", "-f", "keys"],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "l0\nl1\nl2\n");
}

#[test]
fn test_retrieve_expand_non_integer_errors() {
    let dir = setup_workspace();
    write(dir.path().join("doc.md"), "# Doc\n\nBody.\n").unwrap();

    let (stdout, _, success) = run_iwe(dir.path(), &["-k", "doc", "--expand-includes", "x"]);

    assert!(!success);
    assert_eq!(stdout, "");
}

#[test]
fn test_retrieve_expand_conflicts_with_deprecated_alias() {
    let dir = setup_workspace();
    write(dir.path().join("doc.md"), "# Doc\n\nBody.\n").unwrap();

    let (stdout, _, success) = run_iwe(
        dir.path(),
        &["-k", "doc", "-d", "1", "--expand-includes", "1"],
    );

    assert!(!success);
    assert_eq!(stdout, "");
}

#[test]
fn test_retrieve_limit_caps_seeds_before_expansion() {
    let dir = setup_workspace();
    write(dir.path().join("a.md"), "# A\n").unwrap();
    write(dir.path().join("b.md"), "# B\n").unwrap();
    write(dir.path().join("c.md"), "# C\n").unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "-k", "a", "-k", "b", "-k", "c", "--limit", "2", "-f", "keys",
        ],
    );

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "a\nb\n");
}

#[test]
fn test_retrieve_max_documents_caps_after_expansion() {
    let dir = setup_workspace();
    write(dir.path().join("l0.md"), "# L0\n\n[l1](l1)\n").unwrap();
    write(dir.path().join("l1.md"), "# L1\n\n[l2](l2)\n").unwrap();
    write(dir.path().join("l2.md"), "# L2\n\nLeaf.\n").unwrap();

    let (stdout, _, success) = run_iwe(
        dir.path(),
        &[
            "-k",
            "l0",
            "--expand-includes",
            "0",
            "--max-documents",
            "2",
            "-f",
            "keys",
        ],
    );

    assert!(success);
    assert_eq!(stdout, "l0\nl1\n");
}

#[test]
fn test_retrieve_lexical_seeds() {
    let dir = setup_workspace();

    write(
        dir.path().join("ownership.md"),
        "# Ownership\n\nThe borrow checker enforces ownership rules.\n",
    )
    .unwrap();
    write(
        dir.path().join("pasta.md"),
        "# Pasta\n\nBoil water and add salt.\n",
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--lexical", "borrow", "-f", "keys"]);

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "ownership\n");
}
