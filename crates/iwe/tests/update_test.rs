use diwe::config::{Configuration, LibraryOptions, MarkdownOptions};
use indoc::indoc;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn setup(docs: Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().expect("tempdir");
    let temp_path = temp_dir.path();
    create_dir_all(temp_path.join(".iwe")).expect("mkdir .iwe");
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
    write(
        temp_path.join(".iwe").join("config.toml"),
        toml::to_string(&config).expect("config"),
    )
    .expect("write config");
    for (key, content) in docs {
        write(temp_path.join(format!("{}.md", key)), content).expect("write doc");
    }
    temp_dir
}

fn run_update(work_dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("update").current_dir(work_dir);
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("run iwe update")
}

#[test]
fn replace_text_and_set_apply_atomically() {
    let temp = setup(vec![(
        "roadmap",
        indoc! {"
            # Roadmap

            ## Status

            old status
        "},
    )]);
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "roadmap",
            "--replace-text",
            r#"{ $paragraph: "old status", from: "old status", to: reviewed, expect: 1 }"#,
            "--set",
            "reviewed=true",
        ],
    );
    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("roadmap.md")).unwrap(),
        indoc! {"
            ---
            reviewed: true
            ---

            # Roadmap

            ## Status

            reviewed
        "}
    );
}

#[test]
fn append_under_header() {
    let temp = setup(vec![(
        "notes",
        indoc! {"
            # Status

            existing
        "},
    )]);
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "notes",
            "--append",
            r#"{ $header: Status, content: "Reviewed." }"#,
        ],
    );
    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("notes.md")).unwrap(),
        indoc! {"
            # Status

            existing

            Reviewed.
        "}
    );
}

#[test]
fn append_with_bare_markdown_is_refused_with_the_mapping_shape() {
    let notes = "# Status\n\nexisting\n";
    let temp = setup(vec![("notes", notes)]);
    let output = run_update(
        temp.path(),
        &["-k", "notes", "--append", "[Title](notes/slug)"],
    );
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        indoc! {r#"
            error: invalid --append argument: deserializing from YAML containing more than one document is not supported
            hint: --append takes one YAML mapping '{ <selector>, content: <markdown> }', e.g. --append '{ $header: Notes, content: "[Title](notes/slug)" }'; quote a value that contains brackets or colons
        "#}
    );
    assert_eq!(read_to_string(temp.path().join("notes.md")).unwrap(), notes);
}

#[test]
fn block_operator_with_a_scalar_argument_is_refused_with_the_mapping_shape() {
    let notes = "# Status\n\nexisting\n";
    let temp = setup(vec![("notes", notes)]);
    let output = run_update(temp.path(), &["-k", "notes", "--delete", "Status"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        indoc! {"
            error: invalid --delete argument: expected a YAML mapping, got a string
            hint: --delete takes one YAML mapping '{ <selector> }', e.g. --delete '{ $header: Notes }'; quote a value that contains brackets or colons
        "}
    );
    assert_eq!(read_to_string(temp.path().join("notes.md")).unwrap(), notes);
}

#[test]
fn expect_violation_aborts_without_writing() {
    let original = indoc! {"
        # Doc

        drop

        drop
    "};
    let temp = setup(vec![("multi", original)]);
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "multi",
            "--delete",
            r#"{ $paragraph: { $text: drop }, expect: 1 }"#,
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        indoc! {"
            error: $delete expects 1 block, selected 2
              multi › \"drop\"
              multi › \"drop\"
            hint: narrow with $within or $matches, or raise expect
        "}
    );
    assert_eq!(
        read_to_string(temp.path().join("multi.md")).unwrap(),
        original
    );
}

#[test]
fn repeatable_key_updates_multiple() {
    let temp = setup(vec![
        ("a", "# A\n\ntodo item\n"),
        ("b", "# B\n\ntodo item\n"),
        ("c", "# C\n\nleft alone\n"),
    ]);
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "a",
            "-k",
            "b",
            "--replace-text",
            r#"{ $paragraph: "todo item", from: todo, to: done }"#,
        ],
    );
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Updated 2 document(s)\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("a.md")).unwrap(),
        "# A\n\ndone item\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("b.md")).unwrap(),
        "# B\n\ndone item\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("c.md")).unwrap(),
        "# C\n\nleft alone\n"
    );
}

#[test]
fn noop_reports_honestly_and_does_not_rewrite() {
    let original = "# C\n\nkeep\n";
    let temp = setup(vec![("c", original)]);
    let file = temp.path().join("c.md");
    let before = std::fs::metadata(&file).unwrap().modified().unwrap();
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "c",
            "--delete",
            r#"{ $paragraph: absent, expect: 0 }"#,
        ],
    );
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Matched 1 document(s), 0 changed\n"
    );
    assert_eq!(read_to_string(&file).unwrap(), original);
    let after = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert_eq!(before, after, "no-op must not rewrite the file");
}

#[test]
fn dry_run_does_not_write() {
    let original = indoc! {"
        # Doc

        para
    "};
    let temp = setup(vec![("d", original)]);
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "d",
            "--append",
            r#"{ $header: Doc, content: added }"#,
            "--dry-run",
        ],
    );
    assert!(output.status.success());
    assert_eq!(read_to_string(temp.path().join("d.md")).unwrap(), original);
}

#[test]
fn document_expect_violation_aborts_without_writing() {
    let a = "# Alpha\n\nkeep\n";
    let b = "# Beta\n\nkeep\n";
    let temp = setup(vec![("a", a), ("b", b)]);
    let output = run_update(
        temp.path(),
        &["--filter", "{}", "--set", "reviewed=true", "--expect", "1"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        indoc! {"
            error: update expects 1 document, matched 2
              a › Alpha
              b › Beta
            hint: adjust the filter or raise expect
        "}
    );
    assert_eq!(read_to_string(temp.path().join("a.md")).unwrap(), a);
    assert_eq!(read_to_string(temp.path().join("b.md")).unwrap(), b);
}

#[test]
fn strict_without_guards_aborts_without_writing() {
    let original = "# Doc\n\npara\n";
    let temp = setup(vec![("d", original)]);
    let output = run_update(
        temp.path(),
        &[
            "--filter",
            "{}",
            "--delete",
            r#"{ $paragraph: {} }"#,
            "--strict",
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        indoc! {"
            error: --strict requires an expect guard on every mutating application; missing: document-level --expect, $delete expect
            hint: state the expected count — 1 for a precision edit, '{ min: 1 }' for a bulk edit that must match, '{ min: 0 }' when zero is acceptable
        "}
    );
    assert_eq!(read_to_string(temp.path().join("d.md")).unwrap(), original);
}

#[test]
fn strict_with_all_guards_applies() {
    let temp = setup(vec![("d", "# Doc\n\npara\n")]);
    let output = run_update(
        temp.path(),
        &[
            "--filter",
            "{}",
            "--delete",
            r#"{ $paragraph: {}, expect: 1 }"#,
            "--strict",
            "--expect",
            "1",
        ],
    );
    assert!(output.status.success());
    assert_eq!(read_to_string(temp.path().join("d.md")).unwrap(), "# Doc\n");
}

#[test]
fn body_overwrite_preserves_dot_closed_frontmatter() {
    let temp = setup(vec![("d", "---\ntype: note\n...\n\n# Doc\n\npara\n")]);
    let output = run_update(temp.path(), &["-k", "d", "--content", "# Doc\n\nnew\n"]);
    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("d.md")).unwrap(),
        "---\ntype: note\n...\n\n# Doc\n\nnew\n"
    );
}

#[test]
fn body_overwrite_with_own_frontmatter_replaces_the_existing_block() {
    let temp = setup(vec![("d", "---\ntype: note\n---\n\n# Doc\n\npara\n")]);
    let output = run_update(
        temp.path(),
        &[
            "-k",
            "d",
            "--content",
            "---\ntype: page\n---\n\n# Doc\n\nnew\n",
        ],
    );
    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("d.md")).unwrap(),
        "---\ntype: page\n---\n\n# Doc\n\nnew\n"
    );
}

#[test]
fn body_overwrite_normalizes_what_it_writes() {
    let temp = setup(vec![(
        "note",
        "---\ncreated: \"2026-08-24 10:00\"\n---\n\n# Note\n\nOriginal.\n",
    )]);

    let output = run_update(
        temp.path(),
        &[
            "-k",
            "note",
            "--content",
            "#  Rewritten   note\n\nWrapped\nacross lines.\n\n* one\n",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("note.md")).expect("read"),
        "---\ncreated: \"2026-08-24 10:00\"\n---\n\n# Rewritten note\n\nWrapped across lines.\n\n- one\n"
    );
}
