use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
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
