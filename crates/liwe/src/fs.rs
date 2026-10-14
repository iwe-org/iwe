use std::path::{Component, Path, PathBuf};

use ignore::WalkBuilder;
use log::error;

use crate::model::config::Format;

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
                    Component::Normal(os) => Some(os.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/");

            Some((key, path.to_path_buf()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
