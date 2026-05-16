use indoc::indoc;
use liwe::model::config::{ActionDefinition, Attach, Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, read_to_string, write};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn attach_to_subdir_target_writes_relative_url() {
    let temp_dir = setup_workspace(
        vec![("foo/bar", "today", "# Title\n\n{{content}}\n")],
        vec![("baz/qux", "# Qux\n")],
        vec![("foo/bar", "# Bar\n")],
    );
    let temp_path = temp_dir.path();

    let output = run_attach(temp_path, &["--to", "today", "-k", "baz/qux"]);
    assert!(
        output.status.success(),
        "attach should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = read_to_string(temp_path.join("foo/bar.md")).unwrap();
    assert_eq!(
        target,
        indoc! {"
            # Bar

            [Qux](../baz/qux)
        "}
    );
}

#[test]
fn attach_to_same_dir_target_writes_bare_url() {
    let temp_dir = setup_workspace(
        vec![("foo/bar", "today", "# Title\n\n{{content}}\n")],
        vec![("foo/qux", "# Qux\n")],
        vec![("foo/bar", "# Bar\n")],
    );
    let temp_path = temp_dir.path();

    let output = run_attach(temp_path, &["--to", "today", "-k", "foo/qux"]);
    assert!(
        output.status.success(),
        "attach should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = read_to_string(temp_path.join("foo/bar.md")).unwrap();
    assert_eq!(
        target,
        indoc! {"
            # Bar

            [Qux](qux)
        "}
    );
}

#[test]
fn attach_creates_subdir_target_with_relative_url() {
    let temp_dir = setup_workspace(
        vec![("foo/bar", "today", "# Title\n\n{{content}}\n")],
        vec![("baz/qux", "# Qux\n")],
        vec![],
    );
    let temp_path = temp_dir.path();

    let output = run_attach(temp_path, &["--to", "today", "-k", "baz/qux"]);
    assert!(
        output.status.success(),
        "attach should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = read_to_string(temp_path.join("foo/bar.md")).unwrap();
    assert_eq!(
        target,
        indoc! {"
            # Title

            [Qux](../baz/qux)
        "}
    );
}

fn setup_workspace(
    actions: Vec<(&str, &str, &str)>,
    sources: Vec<(&str, &str)>,
    targets: Vec<(&str, &str)>,
) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let mut config = Configuration {
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
    for (key_template, action_name, document_template) in actions {
        config.actions.insert(
            action_name.to_string(),
            ActionDefinition::Attach(Attach {
                title: "Title".to_string(),
                key_template: key_template.to_string(),
                document_template: document_template.to_string(),
            }),
        );
    }

    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");
    let config_content = toml::to_string(&config).expect("Failed to serialize config");
    write(temp_path.join(".iwe").join("config.toml"), config_content).expect("Should write config");

    for (key, content) in sources.into_iter().chain(targets) {
        let path = temp_path.join(format!("{}.md", key));
        if let Some(parent) = path.parent() {
            create_dir_all(parent).expect("Should create parent dir");
        }
        write(path, content).expect("Should write file");
    }

    temp_dir
}

fn run_attach(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("attach").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe attach")
}
