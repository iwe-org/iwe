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
        [
          {
            "key": "doc1",
            "title": "Document One",
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Document Two",
            "includedBy": []
          }
        ]
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
        - key: doc1
          title: Document One
          includedBy: []
        - key: doc2
          title: Document Two
          includedBy: []
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
        [
          {
            "key": "authentication",
            "title": "User Authentication",
            "includedBy": []
          }
        ]
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
        [
          {
            "key": "doc1",
            "title": "Doc One",
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Doc Two",
            "includedBy": []
          }
        ]
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
        [
          {
            "key": "child1",
            "title": "Child One",
            "includedBy": [
              {
                "key": "source",
                "title": "Source",
                "sectionPath": [
                  "Source"
                ]
              }
            ]
          },
          {
            "key": "child2",
            "title": "Child Two",
            "includedBy": [
              {
                "key": "source",
                "title": "Source",
                "sectionPath": [
                  "Source"
                ]
              }
            ]
          }
        ]
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
        [
          {
            "key": "doc1",
            "title": "Document 1",
            "includedBy": []
          },
          {
            "key": "doc10",
            "title": "Document 10",
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Document 2",
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_limit_in_markdown() {
    let dir = setup_workspace();

    write(dir.path().join("a.md"), "# A\n\nA body.").unwrap();
    write(dir.path().join("b.md"), "# B\n\nB body.").unwrap();
    write(dir.path().join("c.md"), "# C\n\nC body.").unwrap();
    write(dir.path().join("d.md"), "# D\n\nD body.").unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--limit", "2", "-f", "markdown"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #a
        ---
        title: A
        ---

        # A

        A body.
        ````

        ````markdown #b
        ---
        title: B
        ---

        # B

        B body.
        ````
    "};

    assert_eq!(stdout, expected);
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
            ````markdown #test-doc
            ---
            title: Test Document
            ---

            # Test Document

            Content.
            ````
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
        [
          {
            "key": "child",
            "title": "Child",
            "includedBy": [
              {
                "key": "parent",
                "title": "Parent",
                "sectionPath": [
                  "Parent"
                ]
              }
            ]
          },
          {
            "key": "parent",
            "title": "Parent",
            "includedBy": []
          }
        ]
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
        [
          {
            "key": "child",
            "title": "Child",
            "includedBy": [
              {
                "key": "parent",
                "title": "Parent",
                "sectionPath": [
                  "Parent"
                ]
              }
            ]
          },
          {
            "key": "parent",
            "title": "Parent",
            "includedBy": []
          }
        ]
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
        [
          {
            "key": "child1",
            "title": "Child One",
            "includedBy": [
              {
                "key": "hub",
                "title": "Hub",
                "sectionPath": [
                  "Hub"
                ]
              }
            ]
          },
          {
            "key": "child2",
            "title": "Child Two",
            "includedBy": [
              {
                "key": "hub",
                "title": "Hub",
                "sectionPath": [
                  "Hub"
                ]
              }
            ]
          },
          {
            "key": "hub",
            "title": "Hub",
            "includedBy": []
          },
          {
            "key": "referrer",
            "title": "Referrer",
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_empty_workspace() {
    let dir = setup_workspace();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        []
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
        []
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
        []
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
        [
          {
            "key": "test",
            "title": "Test",
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_markdown_with_query() {
    let dir = setup_workspace();

    write(dir.path().join("match.md"), "# Match\n\nMatching content.").unwrap();
    write(dir.path().join("other.md"), "# Other\n\nOther content.").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["match", "-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #match
        ---
        title: Match
        ---

        # Match

        Matching content.
        ````
    "};

    assert_eq!(stdout, expected);
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
        [
          {
            "key": "doc1",
            "title": "Doc One",
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_markdown_multi_doc_stream() {
    let dir = setup_workspace();

    write(dir.path().join("alpha.md"), "# Alpha\n\nAlpha body.").unwrap();
    write(dir.path().join("beta.md"), "# Beta\n\nBeta body.").unwrap();

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #alpha
        ---
        title: Alpha
        ---

        # Alpha

        Alpha body.
        ````

        ````markdown #beta
        ---
        title: Beta
        ---

        # Beta

        Beta body.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_markdown_includes_parent_edges() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [child](child)
        "},
    )
    .unwrap();

    write(dir.path().join("child.md"), "# Child\n\nChild body.").unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["-f", "markdown", "-k", "child"],
    );

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

        Child body.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_markdown_empty_results() {
    let dir = setup_workspace();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "");
}
