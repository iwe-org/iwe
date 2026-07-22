use std::fs::{create_dir_all, read_to_string, write, File};
use std::path::Path;
use std::process::{Command, Output};

use diwe::config::{ActionDefinition, Configuration, LinkType};
use liwe::model::config::{Format, RefsPath, WikiLinkPath};
use tempfile::TempDir;

fn run_init(work_dir: &Path, args: &[&str]) -> Output {
    Command::new(crate::common::get_iwe_binary_path())
        .arg("init")
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("Failed to execute iwe init")
}

fn note(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    create_dir_all(path.parent().expect("note has a parent directory"))
        .expect("Should create note directory");
    write(&path, content).expect("Should write note");
}

fn written_config(root: &Path) -> Configuration {
    let text = read_to_string(root.join(".iwe").join("config.toml")).expect("Should read config");
    toml::from_str(&text).expect("config.toml parses as a Configuration")
}

fn config_text(root: &Path) -> String {
    read_to_string(root.join(".iwe").join("config.toml")).expect("Should read config")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("Valid UTF-8 stdout")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("Valid UTF-8 stderr")
}

fn wiki_vault(root: &Path) {
    note(
        root,
        "notes/index.md",
        "# Index\n\nSee [[projects/roadmap]].\n",
    );
    note(
        root,
        "notes/daily-log.md",
        "# Daily Log\n\nSee [[index]].\n",
    );
    note(
        root,
        "notes/projects/roadmap.md",
        "# Roadmap\n\nBack to [[index]].\n",
    );
}

#[test]
fn init_writes_a_parsable_config_in_an_empty_directory() {
    let temp = TempDir::new().expect("Failed to create temp directory");

    let output = run_init(temp.path(), &[]);

    assert_eq!(Some(0), output.status.code());
    assert!(temp.path().join(".iwe").is_dir());

    let config = written_config(temp.path());
    assert_eq!("", config.library.path);
    assert_eq!(Format::Markdown, config.format);
    assert_eq!(Some(3), config.version);
}

#[test]
fn init_exits_with_two_when_already_initialized() {
    let temp = TempDir::new().expect("Failed to create temp directory");

    assert_eq!(Some(0), run_init(temp.path(), &[]).status.code());

    let output = run_init(temp.path(), &[]);

    assert_eq!(Some(2), output.status.code());
    assert_eq!(
        "already initialized — inspect .iwe/config.toml\n",
        stderr_of(&output)
    );
}

#[test]
fn init_exits_with_two_when_the_marker_is_a_file() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    File::create(temp.path().join(".iwe")).expect("Should create .iwe file");

    let output = run_init(temp.path(), &[]);

    assert_eq!(Some(2), output.status.code());
    assert_eq!(
        ".iwe already exists and is not a directory\n",
        stderr_of(&output)
    );
}

#[test]
fn init_detects_the_library_directory() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    run_init(temp.path(), &[]);

    assert_eq!("notes", written_config(temp.path()).library.path);
}

#[test]
fn init_ignores_root_meta_files_when_choosing_the_library() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());
    note(temp.path(), "README.md", "# Readme\n");
    note(temp.path(), "CONTRIBUTING.md", "# Contributing\n");

    run_init(temp.path(), &[]);

    assert_eq!("notes", written_config(temp.path()).library.path);
}

#[test]
fn init_detects_wiki_links() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    run_init(temp.path(), &[]);

    assert_eq!(
        Some(LinkType::WikiLink),
        written_config(temp.path()).completion.link_format
    );
}

#[test]
fn init_detects_markdown_links_that_carry_an_extension() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "# One\n\nSee [Two](two.md).\n");
    note(temp.path(), "two.md", "# Two\n\nSee [Three](three.md).\n");
    note(temp.path(), "three.md", "# Three\n\nSee [One](one.md).\n");

    run_init(temp.path(), &[]);

    let config = written_config(temp.path());
    assert_eq!(Some(LinkType::Markdown), config.completion.link_format);
    assert_eq!(".md", config.markdown.refs_extension);
}

#[test]
fn init_detects_markdown_links_without_an_extension() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "# One\n\nSee [Two](two).\n");
    note(temp.path(), "two.md", "# Two\n\nSee [Three](three).\n");
    note(temp.path(), "three.md", "# Three\n\nSee [One](one).\n");

    run_init(temp.path(), &[]);

    assert_eq!("", written_config(temp.path()).markdown.refs_extension);
}

#[test]
fn init_detects_absolute_link_paths() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(
        temp.path(),
        "notes/one/deep.md",
        "# Deep\n\nSee [Other](/two/other.md).\n",
    );
    note(temp.path(), "notes/two/other.md", "# Other\n");
    note(temp.path(), "notes/two/extra.md", "# Extra\n");

    run_init(temp.path(), &[]);

    assert_eq!(
        RefsPath::Absolute,
        written_config(temp.path()).markdown.refs_path
    );
}

#[test]
fn init_detects_bare_wiki_links_as_short_paths() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "notes/one.md", "# One\n\nSee [[two]].\n");
    note(temp.path(), "notes/two.md", "# Two\n\nSee [[three]].\n");
    note(temp.path(), "notes/three.md", "# Three\n\nSee [[one]].\n");

    run_init(temp.path(), &[]);

    assert_eq!(
        WikiLinkPath::Short,
        written_config(temp.path()).markdown.wiki_link_path
    );
}

#[test]
fn init_detects_the_dominant_list_token() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "# One\n\n* alpha\n* beta\n");
    note(temp.path(), "two.md", "# Two\n\n* gamma\n* delta\n");

    run_init(temp.path(), &[]);

    assert_eq!(
        Some("*".to_string()),
        written_config(temp.path()).markdown.formatting.list_token
    );
}

#[test]
fn init_leaves_formatting_unset_when_the_corpus_matches_the_defaults() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "# One\n\n- alpha\n- beta\n");

    run_init(temp.path(), &[]);

    assert_eq!(
        None,
        written_config(temp.path()).markdown.formatting.list_token
    );
}

#[test]
fn init_detects_the_daily_note_date_format() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "2024-01-01.md", "# Jan 01, 2024\n");
    note(temp.path(), "2024-01-02.md", "# Jan 02, 2024\n");
    note(temp.path(), "2024-01-03.md", "# Jan 03, 2024\n");

    run_init(temp.path(), &[]);

    let config = written_config(temp.path());
    assert_eq!(Some("%Y-%m-%d".to_string()), config.library.date_format);
    assert_eq!(Some("%b %d, %Y".to_string()), config.markdown.date_format);
}

#[test]
fn init_propagates_the_detected_link_format_to_generated_actions() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    run_init(temp.path(), &[]);

    let config = written_config(temp.path());
    let link_types: Vec<Option<LinkType>> = ["extract", "extract_all", "link"]
        .iter()
        .map(|name| match config.actions.get(*name) {
            Some(ActionDefinition::Extract(action)) => action.link_type.clone(),
            Some(ActionDefinition::ExtractAll(action)) => action.link_type.clone(),
            Some(ActionDefinition::Link(action)) => action.link_type.clone(),
            _ => None,
        })
        .collect();

    assert_eq!(
        vec![
            Some(LinkType::WikiLink),
            Some(LinkType::WikiLink),
            Some(LinkType::WikiLink)
        ],
        link_types
    );
}

#[test]
fn init_defaults_skips_detection() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    let output = run_init(temp.path(), &["--defaults"]);

    assert_eq!(Some(0), output.status.code());

    let config = written_config(temp.path());
    assert_eq!("", config.library.path);
    assert_eq!(Some(LinkType::Markdown), config.completion.link_format);
}

#[test]
fn init_dry_run_writes_nothing() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    let output = run_init(temp.path(), &["--dry-run"]);

    assert_eq!(Some(0), output.status.code());
    assert_eq!(false, temp.path().join(".iwe").exists());

    let stdout = stdout_of(&output);
    let last_line = stdout
        .lines()
        .last()
        .expect("dry run prints a closing line");
    assert_eq!("dry run — nothing written", last_line);
}

#[test]
fn init_overrides_win_over_detection() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    run_init(
        temp.path(),
        &["--link-format", "markdown", "--library", "."],
    );

    let config = written_config(temp.path());
    assert_eq!(Some(LinkType::Markdown), config.completion.link_format);
    assert_eq!(".", config.library.path);
}

#[test]
fn init_writes_evidence_comments_above_detected_values() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    run_init(temp.path(), &[]);

    let text = config_text(temp.path());
    let commented: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("# detected:"))
        .collect();

    assert_eq!(
        vec![
            "# detected: 3 of 3 files live under notes/",
            "# detected: 3 wiki links vs 0 markdown links",
            "# detected: 3 of 3 filenames use slug style",
        ],
        commented
    );
}

#[test]
fn init_output_is_byte_identical_across_runs() {
    let first = TempDir::new().expect("Failed to create temp directory");
    let second = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(first.path());
    wiki_vault(second.path());

    run_init(first.path(), &[]);
    run_init(second.path(), &[]);

    assert_eq!(config_text(first.path()), config_text(second.path()));
}

#[test]
fn init_json_reports_settings_confidence_and_churn() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    let output = run_init(temp.path(), &["--json"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout_of(&output)).expect("report is valid JSON");

    assert_eq!(&serde_json::json!(true), &report["written"]);
    assert_eq!(
        &serde_json::json!(".iwe/config.toml"),
        &report["config_path"]
    );
    assert_eq!(
        &serde_json::json!("notes"),
        &report["settings"]["library.path"]
    );
    assert_eq!(
        &serde_json::json!("wiki"),
        &report["settings"]["completion.link_format"]
    );
    assert_eq!(
        &serde_json::json!("detected"),
        &report["confidence"]["completion.link_format"]
    );
    assert_eq!(&serde_json::json!(3), &report["evidence"]["scanned_files"]);
    assert_eq!(&serde_json::json!(3), &report["evidence"]["wiki_links"]);
    assert_eq!(&serde_json::json!(0), &report["normalize_churn"]["changed"]);
}

#[test]
fn init_json_reports_failure_when_already_initialized() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    run_init(temp.path(), &[]);

    let output = run_init(temp.path(), &["--json"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout_of(&output)).expect("report is valid JSON");

    assert_eq!(Some(2), output.status.code());
    assert_eq!(
        serde_json::json!({
            "written": false,
            "config_path": ".iwe/config.toml",
            "error": "already initialized",
        }),
        report
    );
}

#[test]
fn init_warns_about_crlf_and_setext_headers() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "One\r\n===\r\n\r\nSome text.\r\n");

    let output = run_init(temp.path(), &["--json"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout_of(&output)).expect("report is valid JSON");

    assert_eq!(
        serde_json::json!([
            "1 file uses CRLF line endings — normalize writes LF",
            "1 setext header will be rewritten as ATX headers",
        ]),
        report["warnings"]
    );
}

#[test]
fn init_reports_unresolved_links() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "# One\n\nSee [Missing](missing).\n");

    let output = run_init(temp.path(), &["--json"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout_of(&output)).expect("report is valid JSON");

    assert_eq!(
        &serde_json::json!(1),
        &report["evidence"]["unresolved_links"]
    );
    assert_eq!(
        serde_json::json!(["1 link resolves to nothing (one → missing)"]),
        report["notes"]
    );
}

#[test]
fn init_prints_agent_snippets_without_a_terminal() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());
    note(temp.path(), "AGENTS.md", "# House rules\n\nBe careful.\n");

    let output = run_init(temp.path(), &[]);

    let agents = read_to_string(temp.path().join("AGENTS.md")).expect("Should read AGENTS.md");
    let stdout = stdout_of(&output);
    let headings: Vec<&str> = stdout
        .lines()
        .filter(|line| line.ends_with("AGENTS.md:") || line.ends_with(".mcp.json:"))
        .collect();

    assert_eq!("# House rules\n\nBe careful.\n", agents);
    assert_eq!(false, temp.path().join(".mcp.json").exists());
    assert_eq!(
        vec![
            "add this section to AGENTS.md:",
            "register the MCP server in .mcp.json:",
        ],
        headings
    );
}

#[test]
fn init_describes_the_detected_conventions_to_agents() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());
    note(temp.path(), "AGENTS.md", "# House rules\n");

    let output = run_init(temp.path(), &[]);

    let stdout = stdout_of(&output);
    let bullets: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("- Documents live") || line.starts_with("- Link between"))
        .collect();

    assert_eq!(
        vec![
            "- Documents live under notes/ and are addressed by key — the path without the extension.",
            "- Link between documents with wiki links, for example `[[projects/roadmap]]`.",
        ],
        bullets
    );
}

#[test]
fn init_leaves_an_existing_mcp_config_untouched_without_a_terminal() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());
    note(
        temp.path(),
        ".mcp.json",
        "{\n  \"mcpServers\": {\n    \"other\": { \"command\": \"other\" }\n  }\n}\n",
    );

    run_init(temp.path(), &[]);

    let text = read_to_string(temp.path().join(".mcp.json")).expect("Should read .mcp.json");

    assert_eq!(
        "{\n  \"mcpServers\": {\n    \"other\": { \"command\": \"other\" }\n  }\n}\n",
        text
    );
}

#[test]
fn init_leaves_agent_files_alone_without_an_agent_surface() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());

    run_init(temp.path(), &[]);

    assert_eq!(false, temp.path().join("AGENTS.md").exists());
    assert_eq!(false, temp.path().join(".mcp.json").exists());
}

#[test]
fn init_auto_prints_agent_snippets_instead_of_writing_them() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    wiki_vault(temp.path());
    note(temp.path(), "AGENTS.md", "# House rules\n");

    let output = run_init(temp.path(), &["--auto"]);
    let agents = read_to_string(temp.path().join("AGENTS.md")).expect("Should read AGENTS.md");
    let stdout = stdout_of(&output);
    let headings: Vec<&str> = stdout
        .lines()
        .filter(|line| line.ends_with("AGENTS.md:") || line.ends_with(".mcp.json:"))
        .collect();

    assert_eq!("# House rules\n", agents);
    assert_eq!(false, temp.path().join(".mcp.json").exists());
    assert_eq!(
        vec![
            "add this section to AGENTS.md:",
            "register the MCP server in .mcp.json:",
        ],
        headings
    );
}

#[test]
fn init_measures_lower_churn_for_the_detected_bundle() {
    let temp = TempDir::new().expect("Failed to create temp directory");
    note(temp.path(), "one.md", "# One\n\n* alpha\n* beta\n");
    note(temp.path(), "two.md", "# Two\n\n* gamma\n* delta\n");

    let output = run_init(temp.path(), &["--json"]);
    let report: serde_json::Value =
        serde_json::from_str(&stdout_of(&output)).expect("report is valid JSON");

    assert_eq!(&serde_json::json!(0), &report["normalize_churn"]["changed"]);
    assert_eq!(&serde_json::json!(2), &report["default_churn"]["changed"]);
}
