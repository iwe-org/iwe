use indoc::indoc;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run(work_dir: &Path, args: &[&str]) -> Output {
    Command::new(crate::common::get_iwe_binary_path())
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("run iwe command")
}

fn write_config(root: &Path, contents: &str) {
    create_dir_all(root.join(".iwe")).expect("Should create the marker directory");
    write(root.join(".iwe/config.toml"), contents).expect("Should write the config");
}

fn config_text(root: &Path) -> String {
    read_to_string(root.join(".iwe/config.toml")).expect("Should read the config")
}

#[test]
fn a_config_with_the_old_table_name_is_migrated_in_place() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    write_config(
        temp.path(),
        indoc! {r#"
            version = 3

            [library]
            path = "notes"
            date_format = "%Y-%m-%d"

            [markdown]
            refs_extension = ""
        "#},
    );
    create_dir_all(temp.path().join("notes")).expect("Should create the document directory");
    write(temp.path().join("notes/one.md"), "# One\n").expect("Should write a document");

    let output = run(temp.path(), &["find"]);

    assert!(output.status.success());
    assert_eq!(
        config_text(temp.path()),
        indoc! {r#"
            version = 4

            [workspace]
            path = "notes"
            date_format = "%Y-%m-%d"

            [markdown]
            refs_extension = ""
        "#}
    );
}

#[test]
fn a_config_with_the_old_table_name_still_locates_the_documents() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    write_config(
        temp.path(),
        indoc! {r#"
            version = 3

            [library]
            path = "notes"

            [markdown]
            refs_extension = ""
        "#},
    );
    create_dir_all(temp.path().join("notes")).expect("Should create the document directory");
    write(temp.path().join("notes/one.md"), "# One\n").expect("Should write a document");

    let output = run(temp.path(), &["find"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("Valid UTF-8 stdout"),
        "- [One](one)\n"
    );
}
