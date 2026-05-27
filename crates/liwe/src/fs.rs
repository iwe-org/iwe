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
            let key = relative_key(relative_path);

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

fn to_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().trim_end_matches(".md").to_string())
        .unwrap_or_default()
}

fn relative_key(path: &Path) -> String {
    let parent = path.parent().unwrap_or(Path::new(""));
    let file_name = to_file_name(path);

    if parent.as_os_str().is_empty() {
        return file_name;
    }

    let parent_key = parent
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    format!("{}/{}", parent_key, file_name)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn walk_md_paths_normalizes_nested_keys_to_forward_slashes() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let nested = temp_dir.path().join("sub").join("dir");

        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(nested.join("note.md"), "# title").expect("write note");

        let mut paths = walk_md_paths(temp_dir.path());
        paths.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(
            paths,
            vec![("sub/dir/note".to_string(), nested.join("note.md"))]
        );
    }
}
