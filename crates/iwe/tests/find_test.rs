use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
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
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Document Two",
            "references": [],
            "includes": [],
            "referencedBy": [],
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
          references: []
          includes: []
          referencedBy: []
          includedBy: []
        - key: doc2
          title: Document Two
          references: []
          includes: []
          referencedBy: []
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
            "references": [],
            "includes": [],
            "referencedBy": [],
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
            "references": [],
            "includes": [
              {
                "key": "target",
                "title": "Target",
                "sectionPath": []
              }
            ],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Doc Two",
            "references": [],
            "includes": [
              {
                "key": "target",
                "title": "Target",
                "sectionPath": []
              }
            ],
            "referencedBy": [],
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
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "source",
                "title": "Source",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "child2",
            "title": "Child Two",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "source",
                "title": "Source",
                "sectionPath": []
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
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "doc10",
            "title": "Document 10",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          },
          {
            "key": "doc2",
            "title": "Document 2",
            "references": [],
            "includes": [],
            "referencedBy": [],
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
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "parent",
                "title": "Parent",
                "sectionPath": [
                  "Section One"
                ]
              }
            ]
          },
          {
            "key": "parent",
            "title": "Parent",
            "references": [],
            "includes": [
              {
                "key": "child",
                "title": "Child",
                "sectionPath": [
                  "Section One"
                ]
              }
            ],
            "referencedBy": [],
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
            "references": [],
            "includes": [
              {
                "key": "child",
                "title": "Child",
                "sectionPath": []
              }
            ],
            "referencedBy": [],
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
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "hub",
                "title": "Hub",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "child2",
            "title": "Child Two",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [
              {
                "key": "hub",
                "title": "Hub",
                "sectionPath": []
              }
            ]
          },
          {
            "key": "hub",
            "title": "Hub",
            "references": [],
            "includes": [
              {
                "key": "child1",
                "title": "Child One",
                "sectionPath": []
              },
              {
                "key": "child2",
                "title": "Child Two",
                "sectionPath": []
              }
            ],
            "referencedBy": [
              {
                "key": "referrer",
                "title": "Referrer",
                "sectionPath": []
              }
            ],
            "includedBy": []
          },
          {
            "key": "referrer",
            "title": "Referrer",
            "references": [
              {
                "key": "hub",
                "title": "Hub",
                "sectionPath": []
              }
            ],
            "includes": [],
            "referencedBy": [],
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
            "references": [],
            "includes": [],
            "referencedBy": [],
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
            "references": [
              {
                "key": "target",
                "title": "Target",
                "sectionPath": []
              }
            ],
            "includes": [],
            "referencedBy": [],
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

#[test]
fn test_find_project_replace_drops_defaults() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            status: draft
            priority: 5
            ---
            # Doc One
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "key=$key,status,priority", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "key": "doc1",
            "status": "draft",
            "priority": 5
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_pseudo_content() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            Hello.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "k=$key,body=$content", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "k": "doc1",
            "body": "# Doc One\n\nHello.\n"
          }
        ]
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_bare_pseudo_uses_default_name() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["--project", "$content", "-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "content": "# Doc One\n\nBody.\n"
          }
        ]
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_yaml_mapping_form() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            status: draft
            ---
            # Doc One
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "--project",
            "{key: $key, status: 1}",
            "-f",
            "json",
        ],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "key": "doc1",
            "status": "draft"
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_add_fields_extends_default() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            status: draft
            ---
            # Doc One

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--add-fields", "body=$content", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "key": "doc1",
            "title": "Doc One",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [],
            "body": "# Doc One\n\nBody.\n",
            "status": "draft"
          }
        ]
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_and_add_fields_conflict() {
    let dir = setup_workspace();
    write(dir.path().join("a.md"), "# A").unwrap();

    let (_stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "key", "--add-fields", "status"],
    );

    assert!(!success, "stderr: {}", stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "expected conflict error, got: {}",
        stderr
    );
}

#[test]
fn test_find_project_unknown_pseudo_rejected() {
    let dir = setup_workspace();
    write(dir.path().join("a.md"), "# A").unwrap();

    let (_stdout, stderr, success) =
        run_iwe(dir.path(), &["--project", "$bogus"]);

    assert!(!success, "stderr: {}", stderr);
    assert!(
        stderr.contains("unknown projection source") || stderr.contains("$bogus"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn test_find_project_frontmatter_fields_trimmed_block() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            status: draft
            priority: 5
            ---
            # Doc One
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "status,priority", "-f", "markdown"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc1
        ---
        status: draft
        priority: 5
        ---

        # Doc One
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_content_renders_body_only() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "body=$content", "-f", "markdown"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc1
        # Doc One

        Body.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_key_only_emits_no_frontmatter() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["--project", "key", "-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc1
        # Doc One

        Body.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_missing_frontmatter_field_emits_null() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One
        "},
    )
    .unwrap();

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["--project", "pillar", "-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc1
        ---
        pillar: null
        ---

        # Doc One
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_default_projection_omits_empty_edges() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc1
        ---
        title: Doc One
        ---

        # Doc One

        Body.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_default_projection_renders_inclusion_edges() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [Doc One](doc1)
        "},
    )
    .unwrap();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Doc One

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) =
        run_iwe(dir.path(), &["--key", "doc1", "-f", "markdown"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #doc1
        ---
        title: Doc One
        includedBy:
        - key: parent
          title: Parent
        ---

        # Doc One

        Body.
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_yaml_form() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            status: draft
            ---
            # Doc One
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "k=$key,status", "-f", "yaml"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        - k: doc1
          status: draft
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_default_projection_user_fm_title_wins() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            title: FM Title
            ---
            # Heading Title

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "key": "doc1",
            "title": "FM Title",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_add_fields_user_fm_title_wins() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            title: FM Title
            ---
            # Heading Title

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--add-fields", "note=$key", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "key": "doc1",
            "title": "FM Title",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": [],
            "note": "doc1"
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_empty_result_json_is_open_close_bracket_with_newline() {
    let dir = setup_workspace();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "[]\n");
}

#[test]
fn test_find_empty_result_yaml_is_empty_sequence() {
    let dir = setup_workspace();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "yaml"]);

    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout, "[]\n");
}

#[test]
fn test_find_json_output_ends_with_single_newline() {
    let dir = setup_workspace();

    write(dir.path().join("a.md"), "# A\n").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);
    assert!(stdout.ends_with("]\n"), "stdout: {:?}", stdout);
    assert!(!stdout.ends_with("]\n\n"), "stdout: {:?}", stdout);
}

#[test]
fn test_find_yaml_output_no_trailing_double_newline() {
    let dir = setup_workspace();

    write(dir.path().join("a.md"), "# A\n").unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "yaml"]);

    assert!(success, "stderr: {}", stderr);
    assert!(!stdout.ends_with("\n\n"), "stdout: {:?}", stdout);
}

#[test]
fn test_find_project_edges_only_renders_as_markdown_block() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [Child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &[
            "--key",
            "child",
            "--project",
            "parents=$includedBy",
            "-f",
            "markdown",
        ],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        ````markdown #child
        ---
        parents:
        - key: parent
          title: Parent
        ---

        # Child
        ````
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_field_order_preserved_json() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Heading

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "z=$key,a=$title,m=$content", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r##"
        [
          {
            "z": "doc1",
            "a": "Heading",
            "m": "# Heading\n\nBody.\n"
          }
        ]
    "##};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_field_order_preserved_yaml() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Heading

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--project", "z=$key,a=$title,m=$content", "-f", "yaml"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {"
        - z: doc1
          a: Heading
          m: |
            # Heading

            Body.
    "};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_project_includes_edges_json_shape() {
    let dir = setup_workspace();

    write(
        dir.path().join("parent.md"),
        indoc! {"
            # Parent

            [Child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            # Child
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--key", "parent", "--project", "k=$key,inc=$includes", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "k": "parent",
            "inc": [
              {
                "key": "child",
                "title": "Child",
                "sectionPath": []
              }
            ]
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_add_fields_collision_overwrites_default_title() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            # Heading Title

            Body.
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(
        dir.path(),
        &["--add-fields", "title=$key", "-f", "json"],
    );

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "key": "doc1",
            "title": "doc1",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_default_projection_user_fm_key_wins() {
    let dir = setup_workspace();

    write(
        dir.path().join("doc1.md"),
        indoc! {"
            ---
            key: user-supplied
            ---
            # Heading
        "},
    )
    .unwrap();

    let (stdout, stderr, success) = run_iwe(dir.path(), &["-f", "json"]);

    assert!(success, "stderr: {}", stderr);

    let expected = indoc! {r#"
        [
          {
            "key": "user-supplied",
            "title": "Heading",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
          }
        ]
    "#};

    assert_eq!(stdout, expected);
}

#[test]
fn test_find_roots_flag() {
    let dir = setup_workspace();

    write(
        dir.path().join("root.md"),
        indoc! {"
            # Root

            [Child](child)
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

    write(
        dir.path().join("standalone.md"),
        indoc! {"
            # Standalone

            No links.
        "},
    )
    .unwrap();

    let (stdout, _, success) = run_iwe(dir.path(), &["--roots", "-f", "keys"]);
    assert!(success);
    assert_eq!(stdout, "root\nstandalone\n");
}

#[test]
fn test_find_roots_combined_with_filter() {
    let dir = setup_workspace();

    write(
        dir.path().join("root-a.md"),
        indoc! {"
            ---
            status: draft
            ---
            # Root A

            [Child](child)
        "},
    )
    .unwrap();

    write(
        dir.path().join("root-b.md"),
        indoc! {"
            ---
            status: published
            ---
            # Root B

            Content.
        "},
    )
    .unwrap();

    write(
        dir.path().join("child.md"),
        indoc! {"
            ---
            status: draft
            ---
            # Child

            Content.
        "},
    )
    .unwrap();

    let (stdout, _, success) = run_iwe(
        dir.path(),
        &["--roots", "--filter", "status: draft", "-f", "keys"],
    );
    assert!(success);
    assert_eq!(stdout, "root-a\n");
}
