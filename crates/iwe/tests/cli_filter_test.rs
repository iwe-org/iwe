use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

mod common;

fn setup() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path();
    create_dir_all(path.join(".iwe")).unwrap();
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
        path.join(".iwe/config.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();

    write(
        path.join("a.md"),
        indoc! {"
            ---
            status: draft
            priority: 3
            ---
            # Doc A
        "},
    )
    .unwrap();
    write(
        path.join("b.md"),
        indoc! {"
            ---
            status: draft
            priority: 7
            ---
            # Doc B
        "},
    )
    .unwrap();
    write(
        path.join("c.md"),
        indoc! {"
            ---
            status: published
            priority: 5
            ---
            # Doc C
        "},
    )
    .unwrap();
    write(
        path.join("d.md"),
        indoc! {"
            # Doc D

            [Doc A](a)
        "},
    )
    .unwrap();

    dir
}

fn run(dir: &Path, subcmd: &str, args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new(common::get_iwe_binary_path());
    cmd.arg(subcmd).current_dir(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run iwe");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

#[test]
fn count_total() {
    let dir = setup();
    let (stdout, _, ok) = run(dir.path(), "count", &[]);
    assert!(ok);
    assert_eq!(stdout.trim(), "4");
}

#[test]
fn count_filter_status_draft() {
    let dir = setup();
    let (stdout, _, ok) = run(dir.path(), "count", &["--filter", "status: draft"]);
    assert!(ok);
    assert_eq!(stdout.trim(), "2");
}


#[test]
fn find_filter_status_draft_returns_two_keys() {
    let dir = setup();
    let (stdout, _, ok) = run(
        dir.path(),
        "find",
        &["--filter", "status: draft", "-f", "keys"],
    );
    assert!(ok);
    let keys: Vec<&str> = stdout.lines().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"a"));
    assert!(keys.contains(&"b"));
}

#[test]
fn find_combined_fuzzy_and_filter() {
    let dir = setup();
    let (stdout, _, ok) = run(
        dir.path(),
        "find",
        &["A", "--filter", "status: draft", "-f", "keys"],
    );
    assert!(ok);
    let keys: Vec<&str> = stdout.lines().collect();
    assert_eq!(keys, vec!["a"]);
}

#[test]
fn find_in_alias_warns_and_works() {
    let dir = setup();
    let (_, stderr, ok) = run(dir.path(), "find", &["--in", "d:5", "-f", "keys"]);
    assert!(ok);
    assert!(
        stderr.contains("--in is deprecated"),
        "expected deprecation warning, got: {}",
        stderr
    );
}

#[test]
fn update_set_with_filter_writes_back() {
    let dir = setup();
    let (_, _, ok) = run(
        dir.path(),
        "update",
        &["--filter", "status: draft", "--set", "reviewed=true"],
    );
    assert!(ok);

    let a = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    let b = std::fs::read_to_string(dir.path().join("b.md")).unwrap();
    let c = std::fs::read_to_string(dir.path().join("c.md")).unwrap();
    assert!(a.contains("reviewed: true"));
    assert!(b.contains("reviewed: true"));
    assert!(!c.contains("reviewed: true"));
}

#[test]
fn update_set_yaml_value_typing() {
    let dir = setup();
    let (_, _, ok) = run(
        dir.path(),
        "update",
        &[
            "-k",
            "a",
            "--set",
            "priority=10",
            "--set",
            "reviewed=true",
            "--set",
            "tag=urgent",
        ],
    );
    assert!(ok);
    let body = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(body.contains("priority: 10"));
    assert!(body.contains("reviewed: true"));
    assert!(body.contains("tag: urgent"));
}

#[test]
fn delete_filter_matches_multi_doc() {
    let dir = setup();
    let (_, _, ok) = run(
        dir.path(),
        "delete",
        &["--filter", "status: draft", "--quiet"],
    );
    assert!(ok);
    assert!(!dir.path().join("a.md").exists());
    assert!(!dir.path().join("b.md").exists());
    assert!(dir.path().join("c.md").exists());
}

#[test]
fn delete_f_keys_matches_legacy_keys_flag() {
    let dir_a = setup();
    let dir_b = setup();
    let (out_new, _, ok_new) = run(
        dir_a.path(),
        "delete",
        &["a", "--dry-run", "-f", "keys"],
    );
    let (out_old, _, ok_old) = run(
        dir_b.path(),
        "delete",
        &["a", "--dry-run", "--keys"],
    );
    assert!(ok_new);
    assert!(ok_old);
    assert_eq!(out_new, out_old, "-f keys must match --keys output exactly");
    assert!(out_new.contains("a"));
}

#[test]
fn rename_f_keys_matches_legacy_keys_flag() {
    let dir_a = setup();
    let dir_b = setup();
    let (out_new, _, ok_new) = run(
        dir_a.path(),
        "rename",
        &["a", "renamed-a", "--dry-run", "-f", "keys"],
    );
    let (out_old, _, ok_old) = run(
        dir_b.path(),
        "rename",
        &["a", "renamed-a", "--dry-run", "--keys"],
    );
    assert!(ok_new);
    assert!(ok_old);
    assert_eq!(out_new, out_old);
}

#[test]
fn find_max_depth_widens_includes_anchor() {
    let dir = setup();
    let (out_default, _, ok1) = run(
        dir.path(),
        "find",
        &["--includes", "a", "-f", "keys"],
    );
    assert!(ok1);
    let (out_widened, _, ok2) = run(
        dir.path(),
        "find",
        &["--max-depth", "3", "--includes", "a", "-f", "keys"],
    );
    assert!(ok2);
    let _ = (out_default, out_widened);
}

#[test]
fn find_max_distance_widens_references_anchor() {
    let dir = setup();
    let (out_default, _, ok1) = run(
        dir.path(),
        "find",
        &["--references", "a", "-f", "keys"],
    );
    assert!(ok1);
    let (out_widened, _, ok2) = run(
        dir.path(),
        "find",
        &["--max-distance", "2", "--references", "a", "-f", "keys"],
    );
    assert!(ok2);
    let _ = (out_default, out_widened);
}

#[test]
fn update_set_reserved_top_level_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", "_hidden=1"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("ReservedPrefix"),
        "expected reserved-prefix error, got: {}",
        stderr
    );
    let body = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(!body.contains("_hidden"));
}

#[test]
fn update_set_reserved_dotted_segment_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", "author._hidden=1"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("ReservedPrefix"),
        "expected reserved-prefix error, got: {}",
        stderr
    );
    let body = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(!body.contains("_hidden"));
    assert!(!body.contains("author:"));
}

#[test]
fn update_set_reserved_in_nested_value_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", "author={_hidden: 1}"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("ReservedPrefix"),
        "expected reserved-prefix error, got: {}",
        stderr
    );
    let body = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(!body.contains("_hidden"));
}

#[test]
fn update_set_whitespace_segment_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", " foo=1"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("InvalidPathSegment"),
        "expected invalid-path-segment error, got: {}",
        stderr
    );
}

#[test]
fn update_set_control_char_segment_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", "foo\tbar=1"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("InvalidPathSegment"),
        "expected invalid-path-segment error, got: {}",
        stderr
    );
}

#[test]
fn update_set_prefix_overlap_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", "x=1", "--set", "x.y=2"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("SetUnsetConflict"),
        "expected set/unset conflict error, got: {}",
        stderr
    );
}

#[test]
fn update_set_unset_overlap_rejected() {
    let dir = setup();
    let (_, stderr, ok) = run(
        dir.path(),
        "update",
        &["-k", "a", "--set", "x=1", "--unset", "x.y"],
    );
    assert!(!ok);
    assert!(
        stderr.contains("SetUnsetConflict"),
        "expected set/unset conflict error, got: {}",
        stderr
    );
}

