pub mod agents;
pub mod detect;
pub mod evidence;
pub mod fit;
pub mod probe;
pub mod report;
pub mod screen;
pub mod settings;
pub mod writer;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::init::detect::detect;
use crate::init::evidence::scan;
use crate::init::fit::{measure, Churn};
use crate::init::probe::probe;
use crate::init::report::{
    epilogue, notes, render_settings, summarize, summary_line, warnings, Report,
};
use crate::init::screen::{run, Outcome};
use crate::init::settings::{defaults, to_configuration, Confidence, SettingId, Settings, Value};
use crate::init::writer::render;

pub const IWE_MARKER: &str = ".iwe";
pub const CONFIG_FILE_NAME: &str = "config.toml";

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_ALREADY_INITIALIZED: i32 = 2;

#[derive(Debug, Default, Clone)]
pub struct Overrides {
    pub library: Option<String>,
    pub link_format: Option<String>,
    pub refs_extension: Option<String>,
    pub format: Option<String>,
    pub date_format: Option<String>,
}

impl Overrides {
    fn entries(&self) -> Vec<(SettingId, String)> {
        let mut entries = Vec::new();
        if let Some(value) = &self.library {
            entries.push((SettingId::LibraryPath, value.clone()));
        }
        if let Some(value) = &self.link_format {
            entries.push((SettingId::LinkFormat, value.clone()));
        }
        if let Some(value) = &self.refs_extension {
            entries.push((SettingId::RefsExtension, value.clone()));
        }
        if let Some(value) = &self.format {
            entries.push((SettingId::Format, value.clone()));
        }
        if let Some(value) = &self.date_format {
            entries.push((SettingId::KeyDateFormat, value.clone()));
        }
        entries
    }

    fn is_empty(&self) -> bool {
        self.entries().is_empty()
    }
}

#[derive(Debug, Default, Clone)]
pub struct InitOptions {
    pub auto: bool,
    pub dry_run: bool,
    pub use_defaults: bool,
    pub json: bool,
    pub overrides: Overrides,
}

pub fn init_library(root: &Path, options: &InitOptions) -> i32 {
    let marker = root.join(IWE_MARKER);
    if marker.exists() {
        return already_initialized(root, &marker, options);
    }

    let probes = probe(root);

    let (mut chosen, base_defaults, evidence) = if options.use_defaults {
        let evidence = evidence::Evidence::default();
        (defaults(), defaults(), evidence)
    } else {
        let evidence = scan(root);
        let detected = detect(&evidence, &probes);
        (detected, defaults(), evidence)
    };

    apply_overrides(&mut chosen, &options.overrides);

    let interactive = !options.auto
        && !options.dry_run
        && !options.use_defaults
        && !options.json
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal();

    let mut confirmed = false;
    if interactive {
        let detected = chosen.clone();
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut output = std::io::stdout();
        match run(
            root,
            &evidence,
            &detected,
            &base_defaults,
            &mut input,
            &mut output,
        ) {
            Outcome::Write(selected) => {
                chosen = selected;
                confirmed = true;
            }
            Outcome::Quit => {
                println!("nothing written");
                return EXIT_OK;
            }
        }
    }

    let config = to_configuration(&chosen);
    let churn = measure(root, &config);
    let default_churn = if options.use_defaults {
        churn
    } else {
        measure(root, &to_configuration(&base_defaults))
    };

    let rendered = render(&chosen);
    let warnings = warnings(&evidence, &chosen, &probes);
    let notes = notes(&evidence);

    if options.dry_run {
        return finish_dry_run(
            root,
            options,
            &chosen,
            &evidence,
            &rendered,
            churn,
            default_churn,
            warnings,
            notes,
        );
    }

    if let Err(error) = std::fs::create_dir(&marker) {
        eprintln!("failed to create {}: {}", marker.display(), error);
        return EXIT_ERROR;
    }
    if let Err(error) = std::fs::write(marker.join(CONFIG_FILE_NAME), &rendered) {
        eprintln!("failed to write the configuration: {}", error);
        return EXIT_ERROR;
    }

    let mut artifacts = Vec::new();
    if chosen.agents_enabled() {
        artifacts = write_agent_artifacts(root, &chosen, !confirmed);
    }

    if options.json {
        let mut notes = notes;
        notes.extend(artifacts.iter().cloned());
        let report = Report {
            written: true,
            config_path: config_path_display(root, &marker),
            settings: chosen.values(),
            confidence: chosen.confidences(),
            evidence: summarize(&evidence),
            normalize_churn: churn,
            default_churn,
            warnings,
            notes,
        };
        print_json(&report);
        return EXIT_OK;
    }

    if !options.use_defaults && !interactive {
        println!("{}", summary_line(&evidence));
    }
    for warning in &warnings {
        println!("warning: {}", warning);
    }
    for note in &notes {
        println!("note: {}", note);
    }
    println!("initialized {}", config_path_display(root, &marker));
    for artifact in &artifacts {
        println!("{}", artifact);
    }
    if !options.use_defaults && churn.total > 0 {
        println!("iwe normalize would rewrite {}", churn.render());
    }
    for line in epilogue(&probes) {
        println!("{}", line);
    }

    EXIT_OK
}

#[allow(clippy::too_many_arguments)]
fn finish_dry_run(
    root: &Path,
    options: &InitOptions,
    chosen: &Settings,
    evidence: &evidence::Evidence,
    rendered: &str,
    churn: Churn,
    default_churn: Churn,
    warnings: Vec<String>,
    notes: Vec<String>,
) -> i32 {
    let marker = root.join(IWE_MARKER);

    if options.json {
        let report = Report {
            written: false,
            config_path: config_path_display(root, &marker),
            settings: chosen.values(),
            confidence: chosen.confidences(),
            evidence: summarize(evidence),
            normalize_churn: churn,
            default_churn,
            warnings,
            notes,
        };
        print_json(&report);
        return EXIT_OK;
    }

    println!("{}", summary_line(evidence));
    println!();
    println!("{}", render_settings(chosen));
    println!(
        "normalize churn — detected {} · defaults {}",
        churn.render(),
        default_churn.render()
    );
    for warning in &warnings {
        println!("warning: {}", warning);
    }
    for note in &notes {
        println!("note: {}", note);
    }
    println!();
    println!("{}", rendered);
    println!("dry run — nothing written");

    EXIT_OK
}

fn already_initialized(root: &Path, marker: &Path, options: &InitOptions) -> i32 {
    if options.json {
        let payload = serde_json::json!({
            "written": false,
            "config_path": config_path_display(root, marker),
            "error": "already initialized",
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("report serializes")
        );
        return EXIT_ALREADY_INITIALIZED;
    }

    if marker.is_dir() {
        eprintln!(
            "already initialized — inspect {}",
            config_path_display(root, marker)
        );
    } else {
        eprintln!("{} already exists and is not a directory", IWE_MARKER);
    }
    EXIT_ALREADY_INITIALIZED
}

fn apply_overrides(settings: &mut Settings, overrides: &Overrides) {
    if overrides.is_empty() {
        return;
    }
    for (id, value) in overrides.entries() {
        settings.set(
            id,
            Value::text(value),
            Confidence::Overridden,
            "set on the command line",
        );
    }
}

fn write_agent_artifacts(root: &Path, settings: &Settings, print_only: bool) -> Vec<String> {
    if print_only {
        return vec![
            "add this section to AGENTS.md:".to_string(),
            agents::instructions(settings),
            "register the MCP server in .mcp.json:".to_string(),
            agents::mcp_snippet(),
        ];
    }

    let mut lines = Vec::new();
    match agents::write_instructions(root, settings) {
        Ok(message) => lines.push(message),
        Err(error) => lines.push(format!("warning: {}", error)),
    }
    match agents::register_mcp(root) {
        Ok(message) => lines.push(message),
        Err(error) => lines.push(format!("warning: {}", error)),
    }
    lines
}

fn config_path_display(root: &Path, marker: &Path) -> String {
    let full = marker.join(CONFIG_FILE_NAME);
    full.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| full.display().to_string())
}

fn print_json(report: &Report) {
    println!(
        "{}",
        serde_json::to_string_pretty(report).expect("report serializes")
    );
}

pub fn current_root() -> PathBuf {
    std::env::current_dir().expect("to get current dir")
}
