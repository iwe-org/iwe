use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};

use ignore::WalkBuilder;
use log::error;
use rayon::prelude::*;

use liwe::model::config::Format;
use liwe::model::{Content, State};
use liwe::operations::Changes;

pub fn write_file(
    key: &String,
    content: &Content,
    to: &Path,
    format: Format,
) -> std::io::Result<()> {
    fs::write(
        to.join(format!("{}.{}", key, format.extension())),
        content.as_str(),
    )
}

pub fn new_for_path(base_path: &PathBuf, format: Format) -> State {
    if !base_path.exists() {
        error!("path doesn't exist");
        return State::new();
    }

    walk_md_paths(base_path, format)
        .into_par_iter()
        .filter_map(|(key, path)| {
            fs::read_to_string(&path)
                .ok()
                .map(|content| (key, sanitize_content(content)))
        })
        .collect()
}

pub fn walk_md_paths(base_path: &Path, format: Format) -> Vec<(String, PathBuf)> {
    if !base_path.exists() {
        error!("path doesn't exist");
        return Vec::new();
    }

    let extension = format.extension();

    WalkBuilder::new(base_path)
        .follow_links(false)
        .hidden(true)
        .require_git(false)
        .build()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if !path.is_file() || path.extension().is_none_or(|ext| ext != extension) {
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

pub fn write_store_at_path(store: &State, to: &Path, format: Format) -> std::io::Result<()> {
    for (key, content) in store.iter() {
        write_file(key, content, to, format)?;
    }
    Ok(())
}

pub fn apply_changes(changes: &Changes, base_path: &Path, format: Format) -> std::io::Result<()> {
    let extension = format.extension();

    for key in &changes.removes {
        let file_path = base_path.join(format!("{}.{}", key, extension));
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
        prune_empty_dirs(file_path.parent(), base_path);
    }

    for (key, markdown) in &changes.creates {
        let file_path = base_path.join(format!("{}.{}", key, extension));
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, markdown)?;
    }

    for (key, markdown) in &changes.updates {
        let file_path = base_path.join(format!("{}.{}", key, extension));
        fs::write(&file_path, markdown)?;
    }

    Ok(())
}

fn prune_empty_dirs(start: Option<&Path>, base_path: &Path) {
    let mut dir = start.map(|p| p.to_path_buf());
    while let Some(parent) = dir {
        if parent == base_path || !parent.starts_with(base_path) {
            break;
        }
        if parent.read_dir().map_or(false, |mut d| d.next().is_none()) {
            let _ = fs::remove_dir(&parent);
            dir = parent.parent().map(|p| p.to_path_buf());
        } else {
            break;
        }
    }
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

        let keys = walk_md_paths(base.path(), Format::Markdown)
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        assert_eq!(keys, vec!["sub/dir/note".to_string()]);
    }
}
