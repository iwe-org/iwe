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
    command.arg("find").current_dir(dir);

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
fn test_find_lists_all_documents() {
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

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 2,
          "results": [
            {
              "key": "doc1",
              "title": "Document One",
              "display_title": "Document One",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            },
            {
              "key": "doc2",
              "title": "Document Two",
              "display_title": "Document Two",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_yaml_format() {
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

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "yaml"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        query: null
        limit: null
        total: 2
        results:
        - key: doc1
          title: Document One
          display_title: Document One
          is_root: true
          incoming_refs: 0
          outgoing_refs: 0
          parent_documents: []
        - key: doc2
          title: Document Two
          display_title: Document Two
          is_root: true
          incoming_refs: 0
          outgoing_refs: 0
          parent_documents: []
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_fuzzy_search() {
    let dir = setup_workspace();

    write(dir.path().join("authentication.md"), "# User Authentication\n\nAuth content.").unwrap();
    write(dir.path().join("database.md"), "# Database Config\n\nDB content.").unwrap();
    write(dir.path().join("api.md"), "# API Endpoints\n\nAPI content.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["auth", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": "auth",
          "limit": null,
          "total": 1,
          "results": [
            {
              "key": "authentication",
              "title": "User Authentication",
              "display_title": "User Authentication",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_roots_only() {
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
    write(dir.path().join("orphan.md"), "# Orphan\n\nNo references.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--roots", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 2,
          "results": [
            {
              "key": "orphan",
              "title": "Orphan",
              "display_title": "Orphan",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            },
            {
              "key": "parent",
              "title": "Parent",
              "display_title": "Parent",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 1,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_refs_to() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            [target](target)
        "},
    )
    .unwrap();

    write(
        dir.path().join("doc2.md"),
        indoc! {"
            # Doc Two

            [target](target)
        "},
    )
    .unwrap();

    write(dir.path().join("doc3.md"), "# Doc Three\n\nNo refs.").unwrap();
    write(dir.path().join("target.md"), "# Target\n\nTarget content.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--refs-to", "target", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 2,
          "results": [
            {
              "key": "doc1",
              "title": "Doc One",
              "display_title": "Doc One",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 1,
              "parent_documents": []
            },
            {
              "key": "doc2",
              "title": "Doc Two",
              "display_title": "Doc Two",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 1,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_refs_from() {
    let dir = setup_workspace();

    write(
        dir.path().join("source.md"),
        indoc! {"
            # Source

            [child1](child1)

            [child2](child2)
        "},
    )
    .unwrap();

    write(dir.path().join("child1.md"), "# Child One\n\nContent.").unwrap();
    write(dir.path().join("child2.md"), "# Child Two\n\nContent.").unwrap();
    write(dir.path().join("other.md"), "# Other\n\nNot referenced by source.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--refs-from", "source", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 2,
          "results": [
            {
              "key": "child1",
              "title": "Child One",
              "display_title": "Child One ↖Source",
              "is_root": false,
              "incoming_refs": 1,
              "outgoing_refs": 0,
              "parent_documents": [
                {
                  "key": "source",
                  "title": "Source",
                  "section_path": [
                    "Source"
                  ]
                }
              ]
            },
            {
              "key": "child2",
              "title": "Child Two",
              "display_title": "Child Two ↖Source",
              "is_root": false,
              "incoming_refs": 1,
              "outgoing_refs": 0,
              "parent_documents": [
                {
                  "key": "source",
                  "title": "Source",
                  "section_path": [
                    "Source"
                  ]
                }
              ]
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_limit() {
    let dir = setup_workspace();

    for i in 1..=10 {
        write(
            dir.path().join(format!("doc{}.md", i)),
            format!("# Document {}\n\nContent.", i),
        )
        .unwrap();
    }

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--limit", "3", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": 3,
          "total": 10,
          "results": [
            {
              "key": "doc1",
              "title": "Document 1",
              "display_title": "Document 1",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            },
            {
              "key": "doc10",
              "title": "Document 10",
              "display_title": "Document 10",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            },
            {
              "key": "doc2",
              "title": "Document 2",
              "display_title": "Document 2",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_limit_shown_in_markdown() {
    let dir = setup_workspace();

    for i in 1..=10 {
        write(
            dir.path().join(format!("doc{}.md", i)),
            format!("# Document {}\n\nContent.", i),
        )
        .unwrap();
    }

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--limit", "3", "-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    assert!(stdout.contains("Found 10 results"));
    assert!(stdout.contains("(showing 3)"));
}

#[test]
fn test_find_keys_format() {
    let dir = setup_workspace();

    write(dir.path().join("alpha.md"), "# Alpha\n\nContent.").unwrap();
    write(dir.path().join("beta.md"), "# Beta\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "keys"]);

    assert!(success, "stderr: {}", stderr);

    let keys: Vec<&str> = stdout.lines().collect();
    assert!(keys.contains(&"alpha"));
    assert!(keys.contains(&"beta"));
}

#[test]
fn test_find_markdown_format() {
    let dir = setup_workspace();

    write(dir.path().join("test-doc.md"), "# Test Document\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    assert_eq!(
        stdout,
        indoc! {"
            Found 1 results:

            Test Document   #test-doc
        "}
    );
}

#[test]
fn test_find_with_parent_documents() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            ## Section One

            [child](child)
        "},
    )
    .unwrap();

    write(dir.path().join("child.md"), "# Child\n\nChild content.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 2,
          "results": [
            {
              "key": "child",
              "title": "Child",
              "display_title": "Child ↖Parent",
              "is_root": false,
              "incoming_refs": 1,
              "outgoing_refs": 0,
              "parent_documents": [
                {
                  "key": "parent",
                  "title": "Parent",
                  "section_path": [
                    "Parent"
                  ]
                }
              ]
            },
            {
              "key": "parent",
              "title": "Parent",
              "display_title": "Parent",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 1,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_is_root_flag() {
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

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 2,
          "results": [
            {
              "key": "child",
              "title": "Child",
              "display_title": "Child ↖Parent",
              "is_root": false,
              "incoming_refs": 1,
              "outgoing_refs": 0,
              "parent_documents": [
                {
                  "key": "parent",
                  "title": "Parent",
                  "section_path": [
                    "Parent"
                  ]
                }
              ]
            },
            {
              "key": "parent",
              "title": "Parent",
              "display_title": "Parent",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 1,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_incoming_outgoing_refs() {
    let dir = setup_workspace();

    write(
        dir.path().join("hub.md"),
        indoc! {"
            # Hub

            [child1](child1)

            [child2](child2)
        "},
    )
    .unwrap();

    write(
        dir.path().join("referrer.md"),
        indoc! {"
            # Referrer

            Check out [hub](hub) for more.
        "},
    )
    .unwrap();

    write(dir.path().join("child1.md"), "# Child One\n\nContent.").unwrap();
    write(dir.path().join("child2.md"), "# Child Two\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 4,
          "results": [
            {
              "key": "child1",
              "title": "Child One",
              "display_title": "Child One ↖Hub",
              "is_root": false,
              "incoming_refs": 1,
              "outgoing_refs": 0,
              "parent_documents": [
                {
                  "key": "hub",
                  "title": "Hub",
                  "section_path": [
                    "Hub"
                  ]
                }
              ]
            },
            {
              "key": "child2",
              "title": "Child Two",
              "display_title": "Child Two ↖Hub",
              "is_root": false,
              "incoming_refs": 1,
              "outgoing_refs": 0,
              "parent_documents": [
                {
                  "key": "hub",
                  "title": "Hub",
                  "section_path": [
                    "Hub"
                  ]
                }
              ]
            },
            {
              "key": "hub",
              "title": "Hub",
              "display_title": "Hub",
              "is_root": true,
              "incoming_refs": 1,
              "outgoing_refs": 2,
              "parent_documents": []
            },
            {
              "key": "referrer",
              "title": "Referrer",
              "display_title": "Referrer",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_empty_workspace() {
    let dir = setup_workspace();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 0,
          "results": []
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_query_with_no_match() {
    let dir = setup_workspace();

    write(dir.path().join("document.md"), "# My Document\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["zzzznonexistent", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": "zzzznonexistent",
          "limit": null,
          "total": 0,
          "results": []
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_search_query_in_output() {
    let dir = setup_workspace();

    write(dir.path().join("test.md"), "# Test\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["myquery", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": "myquery",
          "limit": null,
          "total": 0,
          "results": []
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_no_query_null_in_output() {
    let dir = setup_workspace();

    write(dir.path().join("test.md"), "# Test\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 1,
          "results": [
            {
              "key": "test",
              "title": "Test",
              "display_title": "Test",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_markdown_with_query() {
    let dir = setup_workspace();

    write(dir.path().join("test.md"), "# Test Document\n\nContent.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["test", "-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    assert!(stdout.contains("for \"test\""));
}

#[test]
fn test_find_refs_to_inline_link() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            See [target](target) for details.
        "},
    )
    .unwrap();

    write(dir.path().join("target.md"), "# Target\n\nTarget content.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["--refs-to", "target", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        {
          "query": null,
          "limit": null,
          "total": 1,
          "results": [
            {
              "key": "doc1",
              "title": "Doc One",
              "display_title": "Doc One",
              "is_root": true,
              "incoming_refs": 0,
              "outgoing_refs": 0,
              "parent_documents": []
            }
          ]
        }
    "#};

    assert_eq!(stdout, expected);
}
