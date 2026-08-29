use diwe::config::{
    Configuration, LibraryOptions, MarkdownOptions, NoteTemplate, Patterns, SchemaBinding,
};
use indoc::indoc;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

fn setup() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe")).expect("mkdir .iwe");
    write_config(temp.path(), Configuration::default());
    temp
}

fn write_config(path: &Path, mut config: Configuration) {
    config.library.path = "".to_string();
    config.markdown.refs_extension = "".to_string();
    write(
        path.join(".iwe/config.toml"),
        toml::to_string(&config).expect("config"),
    )
    .expect("write config");
}

fn with_template(name: &str, template: NoteTemplate) -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe")).expect("mkdir .iwe");
    let mut config = Configuration::default();
    config.templates.insert(name.to_string(), template);
    write_config(temp.path(), config);
    temp
}

fn run(work_dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command.arg("create").current_dir(work_dir);
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("run iwe create")
}

fn run_piped(work_dir: &Path, args: &[&str], stdin_content: &str) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command
        .arg("create")
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for arg in args {
        command.arg(arg);
    }
    let mut child = command.spawn().expect("spawn iwe create");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin_content.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("run iwe create")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("Valid UTF-8 output")
}

#[test]
fn content_mode_writes_the_document_verbatim() {
    let temp = setup();
    let document = "# Overview\n\nFirst paragraph.\n";
    let output = run(temp.path(), &["projects/overview", "--content", document]);

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("projects/overview.md")).unwrap(),
        document
    );
}

#[test]
fn content_mode_keeps_frontmatter_at_the_first_byte() {
    let temp = setup();
    let document = "---\ntype: person\ntags:\n- pioneer\n---\n\n# Ada Lovelace\n\nBody.\n";
    let output = run(temp.path(), &["people/ada", "--content", document]);

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("people/ada.md")).unwrap(),
        document
    );
}

#[test]
fn content_mode_reads_piped_input() {
    let temp = setup();
    let document = "---\ntype: note\n---\n\n# Piped\n\nBody.\n";
    let output = run_piped(temp.path(), &["notes/piped"], document);

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("notes/piped.md")).unwrap(),
        document
    );
}

#[test]
fn content_mode_reads_stdin_for_a_dash() {
    let temp = setup();
    let document = "# Dash\n\nBody.\n";
    let output = run_piped(temp.path(), &["notes/dash", "--content", "-"], document);

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("notes/dash.md")).unwrap(),
        document
    );
}

#[test]
fn content_mode_prints_the_created_path() {
    let temp = setup();
    let output = run(temp.path(), &["note", "--content", "# Note\n"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "{}\n",
            temp.path()
                .canonicalize()
                .unwrap()
                .join("note.md")
                .display()
        )
    );
}

#[test]
fn content_mode_fails_when_the_document_exists() {
    let temp = setup();
    write(temp.path().join("note.md"), "# Existing\n").unwrap();
    let output = run(temp.path(), &["note", "--content", "# Note\n"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "Error: Document 'note' already exists\n"
    );
    assert_eq!(
        read_to_string(temp.path().join("note.md")).unwrap(),
        "# Existing\n"
    );
}

#[test]
fn content_mode_skip_leaves_the_document_untouched() {
    let temp = setup();
    write(temp.path().join("note.md"), "# Existing\n").unwrap();
    let output = run(
        temp.path(),
        &["note", "--content", "# Note\n", "--if-exists", "skip"],
    );

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        read_to_string(temp.path().join("note.md")).unwrap(),
        "# Existing\n"
    );
}

#[test]
fn content_mode_rejects_suffix() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["note", "--content", "# Note\n", "--if-exists", "suffix"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "error: --if-exists suffix is template-mode only; an explicit key is the document's identity, so pick fail or skip\n"
    );
}

#[test]
fn content_mode_rejects_override() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["note", "--content", "# Note\n", "--if-exists", "override"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "error: --if-exists override is not available in content mode; use `iwe update` to replace an existing document\n"
    );
}

#[test]
fn content_mode_requires_a_key() {
    let temp = setup();
    let output = run(temp.path(), &["--content", "# Note\n"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "error: content mode needs an explicit key: iwe create <key> --content '<document>'\n"
    );
}

#[test]
fn content_mode_requires_content() {
    let temp = setup();
    let output = run(temp.path(), &["note", "--content", "   \n"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "error: content mode needs a document: pass --content '<document>' or pipe it on stdin\n"
    );
}

#[test]
fn content_mode_rejects_a_key_with_an_extension() {
    let temp = setup();
    let output = run(temp.path(), &["note.md", "--content", "# Note\n"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "Error: Key 'note.md' must not include a file extension\n"
    );
}

#[test]
fn template_variables_require_a_template() {
    let temp = setup();
    let output = run(temp.path(), &["note", "--var", "title=X"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: the following required arguments were not provided:
              --template <NAME>

            Usage: iwe create --template <NAME> --var <NAME=VALUE> <KEY>

            For more information, try '--help'.
        "}
    );
}

#[test]
fn frontmatter_flags_require_a_template() {
    let temp = setup();
    let output = run(temp.path(), &["note", "--set", "type=note"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: the following required arguments were not provided:
              --template <NAME>

            Usage: iwe create --template <NAME> --set <FIELD=VALUE> <KEY>

            For more information, try '--help'.
        "}
    );
}

#[test]
fn content_mode_rejects_template_variables() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "note",
            "--content",
            "# Note\n",
            "--template",
            "default",
            "--var",
            "title=X",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "error: --content and --template are mutually exclusive: content mode writes the document you pass, template mode composes it from a named template\n"
    );
}

#[test]
fn content_and_template_are_mutually_exclusive() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["note", "--content", "# Note\n", "--template", "daily"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "error: --content and --template are mutually exclusive: content mode writes the document you pass, template mode composes it from a named template\n"
    );
}

#[test]
fn content_mode_strict_blocks_a_violating_document() {
    let temp = setup_with_schema();
    let output = run(
        temp.path(),
        &["docs/one", "--content", "# Summary\n", "--strict"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: --strict blocked the write: schema validation failed
            docs/one: required section \"Tasks\" is missing
        "}
    );
    assert!(!temp.path().join("docs/one.md").exists());
}

#[test]
fn content_mode_strict_allows_a_clean_document() {
    let temp = setup_with_schema();
    let document = "# Summary\n\n# Tasks\n";
    let output = run(
        temp.path(),
        &["docs/one", "--content", document, "--strict"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("docs/one.md")).unwrap(),
        document
    );
}

#[test]
fn template_mode_strict_blocks_a_violating_render() {
    let temp = setup_with_schema();
    let output = run(
        temp.path(),
        &[
            "docs/one",
            "--template",
            "default",
            "--var",
            "title=Summary",
            "--strict",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: --strict blocked the write: schema validation failed
            docs/one: required section \"Tasks\" is missing
        "}
    );
    assert!(!temp.path().join("docs/one.md").exists());
}

fn setup_with_schema() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    create_dir_all(temp.path().join(".iwe/schemas")).unwrap();
    create_dir_all(temp.path().join("docs")).unwrap();
    let mut schemas = HashMap::new();
    schemas.insert(
        "person".to_string(),
        SchemaBinding {
            r#match: Patterns::One("docs/**".to_string()),
        },
    );
    write_config(
        temp.path(),
        Configuration {
            library: LibraryOptions::default(),
            markdown: MarkdownOptions::default(),
            schemas,
            ..Default::default()
        },
    );
    write(
        temp.path().join(".iwe/schemas/person.yaml"),
        "sections:\n  - header: { const: Summary }\n  - header: { const: Tasks }\n",
    )
    .unwrap();
    temp
}

#[test]
fn template_mode_renders_the_stock_template() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["--template", "default", "--var", "title=My Note"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("my-note.md")).unwrap(),
        "# My Note\n"
    );
}

#[test]
fn template_mode_uses_a_named_template() {
    let temp = with_template(
        "meeting",
        NoteTemplate {
            key_template: "meetings/{{slug}}".to_string(),
            document_template: "# {{title}}\n\n## Notes\n\n{{body}}".to_string(),
        },
    );
    let output = run(
        temp.path(),
        &[
            "--template",
            "meeting",
            "--var",
            "title=Sync",
            "--var",
            "body=Agreed.",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("meetings/sync.md")).unwrap(),
        "# Sync\n\n## Notes\n\nAgreed.\n"
    );
}

#[test]
fn template_mode_ignores_piped_stdin() {
    let temp = setup();
    let output = run_piped(
        temp.path(),
        &["--template", "default", "--var", "title=Piped"],
        "Piped prose.",
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("piped.md")).unwrap(),
        "# Piped\n"
    );
}

#[test]
fn template_mode_binds_the_legacy_content_variable_to_the_body() {
    let temp = with_template(
        "legacy",
        NoteTemplate {
            key_template: "{{slug}}".to_string(),
            document_template: "# {{title}}\n\n{{content}}".to_string(),
        },
    );
    let output = run(
        temp.path(),
        &[
            "-t",
            "legacy",
            "--var",
            "title=Legacy",
            "--var",
            "body=Prose.",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("legacy.md")).unwrap(),
        "# Legacy\n\nProse.\n"
    );
}

#[test]
fn template_mode_accepts_content_as_a_body_spelling() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "--template",
            "default",
            "--var",
            "title=Alias",
            "--var",
            "content=Prose.",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("alias.md")).unwrap(),
        "# Alias\n\nProse.\n"
    );
}

#[test]
fn template_mode_takes_typed_variables_from_vars_yaml() {
    let temp = with_template(
        "meeting",
        NoteTemplate {
            key_template: "meetings/{{slug}}".to_string(),
            document_template:
                "# {{title}}\n\n{% for name in attendees %}- {{name}}\n{% endfor %}\n{{body}}"
                    .to_string(),
        },
    );
    let output = run(
        temp.path(),
        &[
            "-t",
            "meeting",
            "--vars-yaml",
            "title: Sync\nattendees: [ada, alan]\nbody: Agreed.\n",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("meetings/sync.md")).unwrap(),
        "# Sync\n\n- ada\n- alan\n\nAgreed.\n"
    );
}

#[test]
fn template_mode_var_overrides_a_vars_yaml_field() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--vars-yaml",
            "title: From mapping\nbody: Prose.\n",
            "--var",
            "title=From flag",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "# From flag\n\nProse.\n"
    );
}

#[test]
fn template_mode_var_overrides_a_vars_json_field() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--vars-json",
            "{\"title\": \"From mapping\", \"body\": \"Prose.\"}",
            "--var",
            "title=From flag",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "# From flag\n\nProse.\n"
    );
}

#[test]
fn template_mode_var_wins_over_a_later_bulk_mapping() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=From flag",
            "--vars-yaml",
            "title: From mapping\nbody: Prose.\n",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "# From flag\n\nProse.\n"
    );
}

#[test]
fn template_mode_rejects_an_empty_vars_yaml_argument() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["doc", "--template", "default", "--vars-yaml", ""],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: --vars-yaml requires a YAML mapping, got an empty value\n"
    );
}

#[test]
fn template_mode_rejects_a_non_mapping_vars_yaml_argument() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["doc", "--template", "default", "--vars-yaml", "[a, b]"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: invalid --vars-yaml: expected a YAML mapping\n"
    );
}

#[test]
fn template_mode_rejects_a_null_vars_yaml_value() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "--template",
            "default",
            "--vars-yaml",
            "title: Title\nbody: null",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: variable 'body' is null, which would render as the text \"none\"; pass '' for an empty value\n"
    );
}

#[test]
fn template_mode_rejects_a_null_vars_json_value() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "--template",
            "default",
            "--vars-json",
            r#"{"title": "Title", "body": null}"#,
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: variable 'body' is null, which would render as the text \"none\"; pass '' for an empty value\n"
    );
}

#[test]
fn template_mode_hints_at_vars_yaml_for_mapping_shaped_var_input() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["doc", "--template", "default", "--var", "title: Title"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: invalid --var assignment 'title: Title': expected NAME=VALUE; use --vars-yaml to pass a whole YAML mapping\n"
    );
}

#[test]
fn template_mode_uses_var_values_verbatim() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--var",
            "body=## Notes",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "# Title\n\n## Notes\n"
    );
}

#[test]
fn template_mode_treats_a_var_value_as_a_string() {
    let temp = with_template(
        "typed",
        NoteTemplate {
            key_template: "{{slug}}".to_string(),
            document_template: "# {{title}}\n\n{% if draft %}Draft{% else %}Final{% endif %}\n"
                .to_string(),
        },
    );
    let output = run(
        temp.path(),
        &[
            "-t",
            "typed",
            "--var",
            "title=Typed",
            "--var",
            "draft=false",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("typed.md")).unwrap(),
        "# Typed\n\nDraft\n"
    );
}

fn typed_template() -> TempDir {
    with_template(
        "typed",
        NoteTemplate {
            key_template: "{{slug}}".to_string(),
            document_template: "# {{title}}\n\n{% if draft %}Draft{% else %}Final{% endif %}, {{ count + 1 }}\n\n{% for tag in tags %}- {{tag}}\n{% endfor %}\n{{body}}"
                .to_string(),
        },
    )
}

const TYPED_DOCUMENT: &str = "# Typed\n\nFinal, 42\n\n- one\n- two\n\nLine one. Line two.\n";

#[test]
fn template_mode_keeps_value_types_from_vars_yaml() {
    let temp = typed_template();
    let output = run(
        temp.path(),
        &[
            "-t",
            "typed",
            "--vars-yaml",
            "title: Typed\ndraft: false\ncount: 41\ntags: [one, two]\nbody: |\n  Line one.\n  Line two.\n",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("typed.md")).unwrap(),
        TYPED_DOCUMENT
    );
}

#[test]
fn template_mode_keeps_value_types_from_vars_json() {
    let temp = typed_template();
    let output = run(
        temp.path(),
        &[
            "-t",
            "typed",
            "--vars-json",
            "{\"title\": \"Typed\", \"draft\": false, \"count\": 41, \"tags\": [\"one\", \"two\"], \"body\": \"Line one.\\nLine two.\\n\"}",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("typed.md")).unwrap(),
        TYPED_DOCUMENT
    );
}

#[test]
fn template_mode_rejects_an_empty_vars_json_argument() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["doc", "--template", "default", "--vars-json", ""],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: --vars-json requires a JSON object, got an empty value\n"
    );
}

#[test]
fn template_mode_rejects_a_non_object_vars_json_argument() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["doc", "--template", "default", "--vars-json", "[1, 2]"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: invalid --vars-json: expected a JSON object\n"
    );
}

#[test]
fn template_mode_rejects_malformed_vars_json() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--vars-json",
            "{\"title\": }",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: invalid --vars-json: expected value at line 1 column 11\n"
    );
}

#[test]
fn template_mode_derives_a_key_from_a_non_string_title() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["--template", "default", "--var", "title=2026"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("2026.md")).unwrap(),
        "# 2026\n"
    );
}

#[test]
fn template_mode_rejects_both_bulk_variable_forms() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--vars-yaml",
            "title: One\n",
            "--vars-json",
            "{\"title\": \"Two\"}",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: the argument '--vars-yaml <YAML>' cannot be used with '--vars-json <JSON>'

            Usage: iwe create --template <NAME> --vars-yaml <YAML> <KEY>

            For more information, try '--help'.
        "}
    );
}

#[test]
fn template_mode_writes_frontmatter_above_the_render() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--set",
            "type=note",
            "--set",
            "tags=[demo]",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "---\ntype: note\ntags:\n- demo\n---\n\n# Title\n"
    );
}

#[test]
fn template_mode_applies_set_fields_in_command_line_order() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--set",
            "status=draft",
            "--set",
            "type=note",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "---\nstatus: draft\ntype: note\n---\n\n# Title\n"
    );
}

#[test]
fn template_mode_replaces_a_repeated_set_field_in_place() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--set",
            "type=note",
            "--set",
            "status=draft",
            "--set",
            "type=page",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "---\ntype: page\nstatus: draft\n---\n\n# Title\n"
    );
}

#[test]
fn template_mode_last_set_for_a_field_wins() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--set",
            "status=draft",
            "--set",
            "status=final",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "---\nstatus: final\n---\n\n# Title\n"
    );
}

#[test]
fn template_mode_drops_reserved_frontmatter_fields() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--set",
            "_internal=1",
            "--set",
            "type=note",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "---\ntype: note\n---\n\n# Title\n"
    );
}

#[test]
fn template_mode_has_no_bulk_frontmatter_flag() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--frontmatter",
            "type: note",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: unexpected argument '--frontmatter' found

              tip: to pass '--frontmatter' as a value, use '-- --frontmatter'

            Usage: iwe create --template <NAME> --var <NAME=VALUE> <KEY>

            For more information, try '--help'.
        "}
    );
}

#[test]
fn template_mode_rejects_a_mapping_shaped_set_input() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--set",
            "type: note",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: invalid --set assignment 'type: note': expected FIELD=VALUE\n"
    );
}

#[test]
fn template_mode_rejects_frontmatter_flags_against_a_frontmatter_template() {
    let temp = with_template(
        "fronted",
        NoteTemplate {
            key_template: "{{slug}}".to_string(),
            document_template: "---\nkind: page\n---\n\n# {{title}}\n".to_string(),
        },
    );
    let output = run(
        temp.path(),
        &[
            "doc",
            "-t",
            "fronted",
            "--var",
            "title=Title",
            "--set",
            "type=note",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "Error: the document already begins with a frontmatter block, it would be written twice; drop the frontmatter fields, or pass the complete document as content\n"
    );
}

#[test]
fn template_mode_keeps_a_frontmatter_template_without_the_flags() {
    let temp = with_template(
        "fronted",
        NoteTemplate {
            key_template: "{{slug}}".to_string(),
            document_template: "---\nkind: page\n---\n\n# {{title}}\n".to_string(),
        },
    );
    let output = run(
        temp.path(),
        &["doc", "-t", "fronted", "--var", "title=Title"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "---\nkind: page\n---\n\n# Title\n"
    );
}

#[test]
fn template_mode_rejects_a_reserved_variable_name() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--var",
            "slug=x",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: 'slug' is computed by iwe and cannot be set as a template variable (reserved: slug, today, now, id)\n"
    );
}

#[test]
fn template_mode_rejects_body_and_content_together() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--var",
            "title=Title",
            "--var",
            "body=One",
            "--var",
            "content=Two",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: 'body' and 'content' name the same variable; pass only one\n"
    );
}

#[test]
fn template_mode_writes_a_body_with_a_title_heading_verbatim() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--vars-yaml",
            "title: Title\nbody: |\n  # Other\n\n  Prose.\n",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "# Title\n\n# Other\n\nProse.\n"
    );
}

#[test]
fn template_mode_accepts_a_body_with_a_section_heading() {
    let temp = setup();
    let output = run(
        temp.path(),
        &[
            "doc",
            "--template",
            "default",
            "--vars-yaml",
            "title: Title\nbody: |\n  ## Section\n\n  Prose.\n",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("doc.md")).unwrap(),
        "# Title\n\n## Section\n\nProse.\n"
    );
}

#[test]
fn template_mode_suffixes_a_derived_key_that_exists() {
    let temp = setup();
    write(temp.path().join("my-note.md"), "# Existing\n").unwrap();
    let output = run(
        temp.path(),
        &["--template", "default", "--var", "title=My Note"],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("my-note-1.md")).unwrap(),
        "# My Note\n"
    );
}

#[test]
fn template_mode_fails_on_an_explicit_key_that_exists() {
    let temp = setup();
    write(temp.path().join("doc.md"), "# Existing\n").unwrap();
    let output = run(
        temp.path(),
        &["doc", "--template", "default", "--var", "title=Title"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr_of(&output), "Error: Document 'doc' already exists\n");
}

#[test]
fn template_mode_reports_an_empty_derived_key() {
    let temp = setup();
    let output = run(temp.path(), &["--template", "default"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "Error: Generated key is empty. Set the title with --var title=VALUE, or pass an explicit key.\n"
    );
}

#[test]
fn template_mode_reports_an_unknown_template() {
    let temp = setup();
    let output = run(
        temp.path(),
        &["--template", "ghost", "--var", "title=Title"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        "Error: Template 'ghost' not found in configuration\n"
    );
}

#[test]
fn template_mode_requires_a_template_name() {
    let temp = setup();
    let output = run(temp.path(), &["doc", "--template", "--var", "title=Title"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        indoc! {"
            error: a value is required for '--template <NAME>' but none was supplied

            For more information, try '--help'.
        "}
    );
}

#[test]
fn template_mode_rejects_an_empty_template_name() {
    let temp = setup();
    let output = run(temp.path(), &["doc", "--template", "", "--var", "title=X"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr_of(&output),
        "error: --template needs a template name, got an empty value\n"
    );
}

#[test]
fn content_mode_normalizes_the_body_and_keeps_the_frontmatter_verbatim() {
    let temp = setup();
    let sloppy = indoc! {"
        ---
        created: \"2026-08-24 10:00\"
        ---

        #  Sloppy   title

        A paragraph that is hard wrapped
        across two source lines.

        * one
        * two

        1) first
        3) third
    "};

    let output = run_piped(temp.path(), &["sloppy", "--content", "-"], sloppy);

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("sloppy.md")).unwrap(),
        indoc! {"
            ---
            created: \"2026-08-24 10:00\"
            ---

            # Sloppy title

            A paragraph that is hard wrapped across two source lines.

            - one
            - two

            1. first
            2. third
        "}
    );
}

#[test]
fn template_mode_normalizes_the_rendered_body() {
    let temp = with_template(
        "note",
        NoteTemplate {
            key_template: "{{slug}}".to_string(),
            document_template: "# {{title}}\n\n{{body}}".to_string(),
        },
    );
    let output = run(
        temp.path(),
        &[
            "--template",
            "note",
            "--var",
            "title=Sloppy   title",
            "--var",
            "body=Wrapped\nacross lines.\n\n* one\n* two\n\n1) first\n3) third",
        ],
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("sloppy-title.md")).unwrap(),
        indoc! {"
            # Sloppy title

            Wrapped across lines.

            - one
            - two

            1. first
            2. third
        "}
    );
}

#[test]
fn content_mode_normalizes_a_document_with_no_frontmatter() {
    let temp = setup();
    let output = run_piped(
        temp.path(),
        &["plain", "--content", "-"],
        "#   Plain\n\nWrapped\nacross lines.\n",
    );

    assert!(output.status.success());
    assert_eq!(
        read_to_string(temp.path().join("plain.md")).unwrap(),
        "# Plain\n\nWrapped across lines.\n"
    );
}
