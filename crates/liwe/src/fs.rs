use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};

use ignore::WalkBuilder;
use log::error;

use crate::model::{Content, State};

pub fn write_file(key: &String, content: &Content, to: &Path) -> std::io::Result<()> {
    fs::write(to.join(format!("{}.md", key)), content.as_str())
}

pub fn new_for_path(base_path: &PathBuf) -> State {
    if !base_path.exists() {
        error!("path doesn't exist");
        return State::new();
    }

    WalkBuilder::new(base_path)
        .follow_links(false)
        .hidden(false)
        .require_git(false)
        .build()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if !path.is_file() || path.extension().is_none_or(|ext| ext != "md") {
                return None;
            }

            let relative_path = path.strip_prefix(base_path).ok()?;
            let key = if let Some(parent) = relative_path.parent() {
                if parent == std::path::Path::new("") {
                    to_file_name(path)
                } else {
                    format!("{}/{}", parent.to_string_lossy(), to_file_name(path))
                }
            } else {
                to_file_name(path)
            };

            fs::read_to_string(path).ok().map(|content| (key, content))
        })
        .collect()
}

pub fn new_from_hashmap(map: HashMap<String, String>) -> State {
    map.into_iter().collect()
}

pub fn write_store_at_path(store: &State, to: &Path) -> std::io::Result<()> {
    for (key, content) in store.iter() {
        write_file(key, content, to)?;
    }
    Ok(())
}

fn to_file_name(path: &Path) -> String {
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    name.trim_end_matches(".md").to_string()
}
