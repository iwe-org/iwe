use std::path::{Path, PathBuf};

use diwe::config::Configuration;
use diwe::fs::new_for_path;
use diwe::graph_from_path;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Churn {
    pub changed: usize,
    pub total: usize,
}

impl Churn {
    pub fn render(&self) -> String {
        if self.total == 0 {
            return "no files".to_string();
        }
        format!("{}/{} files", self.changed, self.total)
    }
}

fn library_path(root: &Path, config: &Configuration) -> PathBuf {
    if config.library.path.is_empty() {
        root.to_path_buf()
    } else {
        root.join(&config.library.path)
    }
}

pub fn measure(root: &Path, config: &Configuration) -> Churn {
    let library = library_path(root, config);
    if !library.exists() {
        return Churn::default();
    }

    let before = new_for_path(&library, config.format);
    if before.is_empty() {
        return Churn::default();
    }

    let graph = graph_from_path(
        &library,
        false,
        config.format_options(),
        config.library.frontmatter_document_title.clone(),
    );
    let after = graph.export();

    let changed = before
        .iter()
        .filter(|(key, content)| {
            after
                .get(*key)
                .is_some_and(|normalized| normalized != *content)
        })
        .count();

    Churn {
        changed,
        total: before.len(),
    }
}

pub fn sample_diff(root: &Path, config: &Configuration) -> Option<String> {
    let library = library_path(root, config);
    if !library.exists() {
        return None;
    }

    let before = new_for_path(&library, config.format);
    let graph = graph_from_path(
        &library,
        false,
        config.format_options(),
        config.library.frontmatter_document_title.clone(),
    );
    let after = graph.export();

    let mut keys: Vec<&String> = before
        .keys()
        .filter(|key| {
            after
                .get(*key)
                .is_some_and(|normalized| normalized != &before[*key])
        })
        .collect();
    keys.sort();

    let key = keys.first()?;
    let original = &before[*key];
    let normalized = after.get(*key)?;

    let mut output = format!("--- {} (on disk)\n+++ {} (after normalize)\n", key, key);
    for line in line_diff(original, normalized) {
        output.push_str(&line);
        output.push('\n');
    }
    Some(output)
}

fn line_diff(before: &str, after: &str) -> Vec<String> {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    let mut output = Vec::new();
    let mut shown = 0;
    let length = before_lines.len().max(after_lines.len());

    for index in 0..length {
        let original = before_lines.get(index);
        let normalized = after_lines.get(index);
        if original == normalized {
            continue;
        }
        if shown >= 12 {
            output.push("… diff truncated".to_string());
            break;
        }
        if let Some(line) = original {
            output.push(format!("-{}", line));
        }
        if let Some(line) = normalized {
            output.push(format!("+{}", line));
        }
        shown += 1;
    }

    output
}
