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
fn count_included_by_count_zero_matches_roots() {
    let dir = setup();
    let (stdout, _, ok) = run(dir.path(), "count", &["--included-by-count", "0"]);
    assert!(ok);
    assert_eq!(stdout.trim(), "3");
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
fn find_roots_alias_warns_and_matches_count_form() {
    let dir = setup();
    let (out_old, stderr_old, ok_old) = run(dir.path(), "find", &["--roots", "-f", "keys"]);
    assert!(ok_old);
    assert!(stderr_old.contains("--roots is deprecated"));

    let (out_new, _, ok_new) = run(
        dir.path(),
        "find",
        &["--included-by-count", "0", "-f", "keys"],
    );
    assert!(ok_new);

    let mut a: Vec<&str> = out_old.lines().collect();
    let mut b: Vec<&str> = out_new.lines().collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
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
        &["--filter", "status: draft", "--force", "--quiet"],
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
fn find_max_depth_widens_included_by_count() {
    let dir = setup();
    let (out_default, _, ok1) = run(
        dir.path(),
        "find",
        &["--included-by-count", "0", "-f", "keys"],
    );
    assert!(ok1);
    let (out_widened, _, ok2) = run(
        dir.path(),
        "find",
        &["--max-depth", "5", "--included-by-count", "0", "-f", "keys"],
    );
    assert!(ok2);
    let _ = (out_default, out_widened);
}
