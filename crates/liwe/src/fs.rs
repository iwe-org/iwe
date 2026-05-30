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

    walk_md_paths(base_path)
        .into_iter()
        .filter_map(|(key, path)| {
            fs::read_to_string(&path)
                .ok()
                .map(|content| (key, sanitize_content(content)))
        })
        .collect()
}

pub fn walk_md_paths(base_path: &Path) -> Vec<(String, PathBuf)> {
    if !base_path.exists() {
        error!("path doesn't exist");
        return Vec::new();
    }

    WalkBuilder::new(base_path)
        .follow_links(false)
        .hidden(true)
        .require_git(false)
        .build()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if !path.is_file() || path.extension().is_none_or(|ext| ext != "md") {
                return None;
            }

            let relative_path = path.strip_prefix(base_path).ok()?;
            let key = relative_path
                .with_extension("")
                .components()
                .filter_map(|c| match c {
                    std::path::Component::Normal(os) => Some(os.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/");

            Some((key, path.to_path_buf()))
        })
        .collect()
}

pub fn read_md_file(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(sanitize_content)
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

fn sanitize_content(content: String) -> String {
    let content = content
        .strip_prefix('\u{FEFF}')
        .map(|s| s.to_string())
        .unwrap_or(content);
    content.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_content_strips_crlf() {
        assert_eq!("a\nb\nc\n", sanitize_content("a\r\nb\r\nc\r\n".into()));
    }

    #[test]
    fn walk_md_paths_uses_forward_slash_separators_for_nested_files() {
        let base = tempfile::tempdir().unwrap();
        let nested = base.path().join("sub").join("dir");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("note.md"), "# note\n").unwrap();

        let keys = walk_md_paths(base.path())
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        assert_eq!(keys, vec!["sub/dir/note".to_string()]);
    }
}
