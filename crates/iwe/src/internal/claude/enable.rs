use std::env::set_current_dir;
use std::path::{Path, PathBuf};

use chrono::Local;
use diwe::config::load_config;

use crate::init::{current_root, init_library, InitOptions, Overrides};
use crate::internal::claude::hook::store::{
    library_path_of, state_directory, workspace_root, SESSIONS_PREFIX,
};
use crate::new::{write_document, ContentOptions, DocumentCreator, IfExists};

const STARTER_BODY: &str = include_str!("../../../templates/claude/enable/starter.md");
const TYPED_BODY: &str = include_str!("../../../templates/claude/enable/typed.md");
const QUERIES_BODY: &str = include_str!("../../../templates/claude/enable/queries.md");
const TYPED_CONFIG: &str = include_str!("../../../templates/claude/enable/typed.toml");

const TYPED_SCHEMAS: [(&str, &str); 4] = [
    (
        "learning.yaml",
        include_str!("../../../templates/claude/enable/schemas/learning.yaml"),
    ),
    (
        "decision.yaml",
        include_str!("../../../templates/claude/enable/schemas/decision.yaml"),
    ),
    (
        "gotcha.yaml",
        include_str!("../../../templates/claude/enable/schemas/gotcha.yaml"),
    ),
    (
        "topic.yaml",
        include_str!("../../../templates/claude/enable/schemas/topic.yaml"),
    ),
];

pub struct EnableOptions {
    pub typed: bool,
    pub queries: bool,
    pub body: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub schemas: Vec<PathBuf>,
    pub root: Option<PathBuf>,
}

pub fn enable_memory(options: &EnableOptions) -> i32 {
    let body = match &options.body {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(body) => Some(body),
            Err(_) => {
                eprintln!("error: {} is not a readable file", path.display());
                return 1;
            }
        },
        None => None,
    };
    if body.is_some() && options.typed {
        eprintln!("error: --typed writes its own policy body; drop one of --typed/--body");
        return 1;
    }
    if options.typed && (options.config.is_some() || !options.schemas.is_empty()) {
        eprintln!("error: --typed installs its own ontology; drop --typed or --config/--schema");
        return 1;
    }

    let config_text = match &options.config {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => {
                eprintln!("error: {} is not a readable file", path.display());
                return 1;
            }
        },
        None => String::new(),
    };
    let mut schema_files: Vec<(String, String)> = Vec::new();
    for path in &options.schemas {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            eprintln!("error: {} has no file name", path.display());
            return 1;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => schema_files.push((name, content)),
            Err(_) => {
                eprintln!("error: {} is not a readable file", path.display());
                return 1;
            }
        }
    }

    if let Some(root) = &options.root {
        if !root.is_dir() || set_current_dir(root).is_err() {
            eprintln!("error: {} is not a directory", root.display());
            return 1;
        }
    }

    if !Path::new("iwe.toml").is_file() && !Path::new(".iwe").is_dir() {
        let code = init_library(
            &current_root(),
            &InitOptions {
                auto: false,
                dry_run: false,
                use_defaults: true,
                json: false,
                okf: false,
                overrides: Overrides::default(),
            },
        );
        if code != 0 {
            return 1;
        }
        println!(
            "initialized the iwe workspace at {}",
            current_root().display()
        );

        if let Ok(config) = std::fs::read_to_string(".iwe/config.toml") {
            if std::fs::write(".iwe/config.toml", iso_date_formats(&config)).is_ok() {
                println!("set ISO date and time formats so timestamps compare");
            }
        }
    }

    let config = match load_config() {
        Ok(config) => config,
        Err(_) => {
            eprintln!("error: this workspace's configuration does not parse");
            return 1;
        }
    };
    let library = library_path_of(&config);
    let extension = config.format.extension();

    if library.join(format!("MEMORY.{}", extension)).is_file() {
        eprintln!("already memory-enabled — inspect the policy with `iwe retrieve -k MEMORY`");
        return 2;
    }

    if options.typed {
        let schemas: Vec<(String, String)> = TYPED_SCHEMAS
            .iter()
            .map(|(name, content)| (name.to_string(), content.to_string()))
            .collect();
        if let Some(code) = install_ontology(
            "typed ontology",
            TYPED_CONFIG,
            &schemas,
            &[
                "the typed ontology would overwrite them; enable memory without --typed",
                "and describe the types this store already has in the policy instead",
            ],
        ) {
            return code;
        }
    } else if !config_text.is_empty() || !schema_files.is_empty() {
        if let Some(code) = install_ontology(
            "composed ontology",
            &config_text,
            &schema_files,
            &["drop the clashing tables from --config/--schema, or describe the existing types in the policy instead"],
        ) {
            return code;
        }
    }

    let policy = match &body {
        Some(body) => body.as_str(),
        None if options.typed => TYPED_BODY,
        None => STARTER_BODY,
    };
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let document = format!("---\ncreated: \"{}\"\n---\n\n{}", now, policy);

    let config = match load_config() {
        Ok(config) => config,
        Err(_) => {
            eprintln!("error: this workspace's configuration does not parse");
            return 1;
        }
    };
    if !create_document(&config, "MEMORY", &document) {
        return 1;
    }
    println!("wrote the MEMORY.md policy document — memory is on for this workspace");
    println!(
        "session records will land under {}",
        library.join(SESSIONS_PREFIX).display()
    );
    println!(
        "capture chunks stay out of the graph, under {}",
        state_directory(&workspace_root()).display()
    );

    if options.queries && !library.join(format!("queries.{}", extension)).is_file() {
        if !create_document(&config, "queries", QUERIES_BODY) {
            return 1;
        }
        println!("wrote the queries cookbook");
    }

    println!("nothing was committed; review it as a normal diff");
    0
}

fn create_document(config: &diwe::config::Configuration, key: &str, content: &str) -> bool {
    let creator = DocumentCreator::new(config, library_path_of(config));
    let prepared = creator.prepare_content(ContentOptions {
        key: key.to_string(),
        content: content.to_string(),
        if_exists: IfExists::Fail,
    });
    let prepared = match prepared {
        Ok(Some(prepared)) => prepared,
        Ok(None) => return true,
        Err(error) => {
            eprintln!("error: {}", error);
            return false;
        }
    };
    match write_document(&prepared) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("error: {}", error);
            false
        }
    }
}

fn install_ontology(
    label: &str,
    config_text: &str,
    schemas: &[(String, String)],
    remedy: &[&str],
) -> Option<i32> {
    let path = Path::new(".iwe/config.toml");
    let before = match std::fs::read_to_string(path) {
        Ok(config) => config,
        Err(_) => {
            eprintln!("error: no .iwe/config.toml to extend");
            return Some(1);
        }
    };

    let mut clashes = Vec::new();
    for table in table_names(config_text) {
        if before
            .lines()
            .any(|line| line.trim() == format!("[{}]", table))
        {
            clashes.push(table);
        }
    }
    for (name, _) in schemas {
        if Path::new(".iwe/schemas").join(name).is_file() {
            clashes.push(format!(".iwe/schemas/{}", name));
        }
    }
    if !clashes.is_empty() {
        eprintln!(
            "error: this workspace already defines: {}",
            clashes.join(" ")
        );
        for line in remedy {
            eprintln!("error: {}", line);
        }
        return Some(1);
    }

    if !config_text.trim().is_empty() {
        let appended = format!("{}{}", before, config_text);
        if std::fs::write(path, &appended).is_err() {
            eprintln!("error: could not write .iwe/config.toml");
            return Some(1);
        }
        if load_config().is_err() {
            std::fs::write(path, &before).ok();
            eprintln!(
                "error: the {} did not parse against this configuration, rolled back",
                label
            );
            return Some(1);
        }
        println!("appended the {} to .iwe/config.toml", label);
    }

    if !schemas.is_empty() {
        if std::fs::create_dir_all(".iwe/schemas").is_err() {
            eprintln!("error: could not create .iwe/schemas");
            return Some(1);
        }
        for (name, content) in schemas {
            if std::fs::write(Path::new(".iwe/schemas").join(name), content).is_err() {
                eprintln!("error: could not write .iwe/schemas/{}", name);
                return Some(1);
            }
            println!("wrote .iwe/schemas/{}", name);
        }
    }

    None
}

fn table_names(config: &str) -> Vec<String> {
    config
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
                return None;
            }
            let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
            (!inner.is_empty()).then(|| inner.to_string())
        })
        .collect()
}

fn iso_date_formats(config: &str) -> String {
    let mut section = String::new();
    let mut out: Vec<String> = Vec::new();
    for line in config.lines() {
        if line.starts_with('[') {
            section = line.trim().to_string();
        }
        if (section == "[markdown]" || section == "[library]")
            && line.trim_start().starts_with("date_format = ")
        {
            out.push("date_format = \"%Y-%m-%d\"".to_string());
            out.push("time_format = \"%Y-%m-%d %H:%M\"".to_string());
        } else {
            out.push(line.to_string());
        }
    }
    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}
