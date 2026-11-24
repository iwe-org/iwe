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

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::fs::{set_permissions, Permissions};
    use std::os::unix::fs::PermissionsExt;
    set_permissions(path, Permissions::from_mode(mode)).expect("Should change the file mode");
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

#[cfg(unix)]
#[test]
fn a_read_only_config_with_the_old_table_name_is_migrated_in_memory() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    let original = indoc! {r#"
        version = 3

        [library]
        path = "notes"

        [markdown]
        refs_extension = ""
    "#};
    write_config(temp.path(), original);
    create_dir_all(temp.path().join("notes")).expect("Should create the document directory");
    write(temp.path().join("notes/one.md"), "# One\n").expect("Should write a document");
    let config_path = temp.path().join(".iwe/config.toml");
    set_mode(&config_path, 0o444);

    let output = run(temp.path(), &["find"]);

    set_mode(&config_path, 0o644);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("Valid UTF-8 stdout"),
        "- [One](one)\n"
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("Valid UTF-8 stderr"),
        format!(
            "warning: config file '{}' is from an older iwe version and could not be updated (Permission denied (os error 13)); using the updated settings without saving them\n",
            temp.path()
                .canonicalize()
                .expect("Should resolve the temp directory")
                .join(".iwe/config.toml")
                .display()
        )
    );
    assert_eq!(config_text(temp.path()), original);
}
