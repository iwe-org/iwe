use std::path::PathBuf;
use std::{collections::HashMap, fs};

use ignore::WalkBuilder;
use log::error;

use crate::model::{Content, State};

pub fn write_file(key: &String, content: &Content, to: &PathBuf) -> std::io::Result<()> {
    fs::write(to.clone().join(format!("{}.md", key)), content.as_str())
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

            if !path.is_file() || !path.extension().map_or(false, |ext| ext == "md") {
                return None;
            }

            let relative_path = path.strip_prefix(base_path).ok()?;
            let key = if let Some(parent) = relative_path.parent() {
                if parent == std::path::Path::new("") {
                    to_file_name(&path.to_path_buf())
                } else {
                    format!(
                        "{}/{}",
                        parent.to_string_lossy(),
                        to_file_name(&path.to_path_buf())
                    )
                }
            } else {
                to_file_name(&path.to_path_buf())
            };

            fs::read_to_string(path).ok().map(|content| (key, content))
        })
        .collect()
}

pub fn new_from_hashmap(map: HashMap<String, String>) -> State {
    map.into_iter()
        .map(|(key, content)| (key, content))
        .collect()
}

pub fn write_store_at_path(store: &State, to: &PathBuf) -> std::io::Result<()> {
    for (key, content) in store.iter() {
        write_file(key, content, &to)?;
    }
    Ok(())
}

fn to_file_name(path: &PathBuf) -> String {
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    name.trim_end_matches(".md").to_string()
}
