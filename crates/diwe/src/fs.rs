use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::{collections::HashMap, fs};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{Match, WalkBuilder};
use log::error;
use rayon::prelude::*;

use liwe::model::config::Format;
use liwe::model::{Content, State};
use liwe::operations::Changes;

pub fn key_escapes_workspace(key: &str) -> bool {
    Path::new(key).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

pub fn workspace_document_path(
    base_path: &Path,
    key: &str,
    extension: &str,
) -> std::io::Result<PathBuf> {
    if key_escapes_workspace(key) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("key '{key}' escapes the workspace"),
        ));
    }
    Ok(base_path.join(format!("{key}.{extension}")))
}

pub fn write_file(
    key: &String,
    content: &Content,
    to: &Path,
    format: Format,
) -> std::io::Result<()> {
    let file_path = workspace_document_path(to, key, format.extension())?;
    write_file_if_changed(&file_path, content.as_str()).map(|_| ())
}

pub fn write_file_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    if fs::read_to_string(path).is_ok_and(|current| current == content) {
        return Ok(false);
    }
    fs::write(path, content)?;
    Ok(true)
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

pub struct PathFilter {
    base_path: PathBuf,
    global: Gitignore,
    per_directory: Mutex<HashMap<PathBuf, Gitignore>>,
}

impl PathFilter {
    pub fn new(base_path: &Path) -> PathFilter {
        let (global, _) = Gitignore::global();
        PathFilter {
            base_path: base_path.to_path_buf(),
            global,
            per_directory: Mutex::new(HashMap::new()),
        }
    }

    pub fn includes(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.base_path) else {
            return false;
        };

        let components: Vec<_> = relative.components().collect();

        if components.iter().any(|component| match component {
            std::path::Component::Normal(name) => name.to_string_lossy().starts_with('.'),
            _ => true,
        }) {
            return false;
        }

        let mut directory = self.base_path.clone();
        for component in &components[..components.len().saturating_sub(1)] {
            directory = directory.join(component);
            if self.is_ignored(&directory, true) {
                return false;
            }
        }

        !self.is_ignored(path, false)
    }

    fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let mut directory = path.parent();
        while let Some(current) = directory {
            if !current.starts_with(&self.base_path) {
                break;
            }
            match self.matched_in(current, path, is_dir) {
                Match::Ignore(_) => return true,
                Match::Whitelist(_) => return false,
                Match::None => {}
            }
            if current == self.base_path {
                break;
            }
            directory = current.parent();
        }

        matches!(
            matched_under_root(&self.global, path, is_dir),
            Match::Ignore(_)
        )
    }

    fn matched_in(&self, directory: &Path, path: &Path, is_dir: bool) -> Match<()> {
        let mut cache = self.per_directory.lock().expect("filter cache to lock");
        let matcher = cache
            .entry(directory.to_path_buf())
            .or_insert_with(|| build_directory_gitignore(directory));
        matched_under_root(matcher, path, is_dir)
    }
}

fn matched_under_root(matcher: &Gitignore, path: &Path, is_dir: bool) -> Match<()> {
    if matcher.is_empty() || !path.starts_with(matcher.path()) {
        return Match::None;
    }
    matcher.matched(path, is_dir).map(|_| ())
}

fn build_directory_gitignore(directory: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(directory);
    builder.add(directory.join(".gitignore"));
    builder.add(directory.join(".ignore"));
    builder.add(directory.join(".git/info/exclude"));
    builder.build().unwrap_or_else(|_| Gitignore::empty())
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
        let file_path = workspace_document_path(base_path, key.as_str(), extension)?;
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
        prune_empty_dirs(file_path.parent(), base_path);
    }

    for (key, markdown) in &changes.creates {
        let file_path = workspace_document_path(base_path, key.as_str(), extension)?;
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_file_if_changed(&file_path, markdown)?;
    }

    for (key, markdown) in &changes.updates {
        let file_path = workspace_document_path(base_path, key.as_str(), extension)?;
        write_file_if_changed(&file_path, markdown)?;
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
    use liwe::model::Key;

    #[test]
    fn sanitize_content_strips_crlf() {
        assert_eq!("a\nb\nc\n", sanitize_content("a\r\nb\r\nc\r\n".into()));
    }

    fn modified_at(path: &Path) -> std::time::SystemTime {
        std::fs::metadata(path).unwrap().modified().unwrap()
    }

    fn backdate(path: &Path) -> std::time::SystemTime {
        let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(past)
            .unwrap();
        modified_at(path)
    }

    #[test]
    fn identical_content_is_not_rewritten() {
        let base = tempfile::tempdir().unwrap();
        let path = base.path().join("note.md");
        std::fs::write(&path, "# note\n").unwrap();
        let stamp = backdate(&path);

        let wrote = write_file_if_changed(&path, "# note\n").unwrap();

        assert!(!wrote);
        assert_eq!(modified_at(&path), stamp);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# note\n");
    }

    #[test]
    fn different_content_is_written() {
        let base = tempfile::tempdir().unwrap();
        let path = base.path().join("note.md");
        std::fs::write(&path, "# note\n").unwrap();
        let stamp = backdate(&path);

        let wrote = write_file_if_changed(&path, "# other\n").unwrap();

        assert!(wrote);
        assert_ne!(modified_at(&path), stamp);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# other\n");
    }

    #[test]
    fn a_missing_file_is_written() {
        let base = tempfile::tempdir().unwrap();
        let path = base.path().join("note.md");

        let wrote = write_file_if_changed(&path, "# note\n").unwrap();

        assert!(wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# note\n");
    }

    #[test]
    fn a_file_that_is_not_text_is_written() {
        let base = tempfile::tempdir().unwrap();
        let path = base.path().join("note.md");
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();

        let wrote = write_file_if_changed(&path, "# note\n").unwrap();

        assert!(wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# note\n");
    }

    fn workspace_with_gitignore(ignore_body: &str) -> tempfile::TempDir {
        let base = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(base.path().join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(base.path().join("docs/drafts")).unwrap();
        std::fs::write(base.path().join(".gitignore"), ignore_body).unwrap();
        std::fs::write(base.path().join("note.md"), "# Note\n").unwrap();
        std::fs::write(base.path().join("docs/guide.md"), "# Guide\n").unwrap();
        std::fs::write(base.path().join("docs/drafts/wip.md"), "# Wip\n").unwrap();
        std::fs::write(base.path().join("node_modules/pkg/README.md"), "# Pkg\n").unwrap();
        base
    }

    fn walked_keys(base: &Path) -> Vec<String> {
        let mut keys: Vec<String> = walk_md_paths(base, Format::Markdown)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        keys.sort();
        keys
    }

    fn filtered_keys(base: &Path) -> Vec<String> {
        let filter = PathFilter::new(base);
        let mut keys: Vec<String> = walk_all_md_paths(base)
            .into_iter()
            .filter(|path| filter.includes(path))
            .map(|path| {
                path.strip_prefix(base)
                    .unwrap()
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        keys.sort();
        keys
    }

    fn walk_all_md_paths(base: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![base.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "md") {
                    found.push(path);
                }
            }
        }
        found
    }

    #[test]
    fn path_filter_agrees_with_walk_on_gitignored_directory() {
        let base = workspace_with_gitignore("node_modules\n");

        assert_eq!(
            walked_keys(base.path()),
            vec![
                "docs/drafts/wip".to_string(),
                "docs/guide".into(),
                "note".into()
            ]
        );
        assert_eq!(filtered_keys(base.path()), walked_keys(base.path()));
    }

    #[test]
    fn path_filter_agrees_with_walk_on_nested_gitignore() {
        let base = workspace_with_gitignore("node_modules\n");
        std::fs::write(base.path().join("docs/.gitignore"), "drafts\n").unwrap();

        assert_eq!(
            walked_keys(base.path()),
            vec!["docs/guide".to_string(), "note".into()]
        );
        assert_eq!(filtered_keys(base.path()), walked_keys(base.path()));
    }

    #[test]
    fn path_filter_agrees_with_walk_on_whitelisted_path() {
        let base = workspace_with_gitignore("docs\n!docs/guide.md\n");

        assert_eq!(
            walked_keys(base.path()),
            vec!["node_modules/pkg/README".to_string(), "note".into()]
        );
        assert_eq!(filtered_keys(base.path()), walked_keys(base.path()));
    }

    #[test]
    fn path_filter_rejects_hidden_and_outside_paths() {
        let base = workspace_with_gitignore("node_modules\n");
        let filter = PathFilter::new(base.path());

        assert!(!filter.includes(&base.path().join(".iwe/config.md")));
        assert!(!filter.includes(Path::new("/elsewhere/note.md")));
        assert!(filter.includes(&base.path().join("note.md")));
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

    #[test]
    fn a_nested_key_stays_inside_the_workspace() {
        assert!(!key_escapes_workspace("note"));
        assert!(!key_escapes_workspace("people/ada"));
        assert!(!key_escapes_workspace("./note"));
    }

    #[test]
    fn a_parent_segment_escapes_the_workspace() {
        assert!(key_escapes_workspace(".."));
        assert!(key_escapes_workspace("../note"));
        assert!(key_escapes_workspace("people/../../note"));
    }

    #[test]
    fn an_absolute_key_escapes_the_workspace() {
        assert!(key_escapes_workspace("/note"));
    }

    #[cfg(windows)]
    #[test]
    fn a_windows_path_escapes_the_workspace() {
        assert!(key_escapes_workspace("..\\note"));
        assert!(key_escapes_workspace("\\note"));
        assert!(key_escapes_workspace("C:\\note"));
        assert!(key_escapes_workspace("C:note"));
    }

    #[test]
    fn a_nested_key_joins_onto_the_workspace() {
        let path = workspace_document_path(Path::new("workspace"), "people/ada", "md").unwrap();

        assert_eq!(path, Path::new("workspace").join("people/ada.md"));
    }

    #[test]
    fn an_escaping_key_does_not_join_onto_the_workspace() {
        let error = workspace_document_path(Path::new("workspace"), "../note", "md").unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "key '../note' escapes the workspace");
    }

    fn workspace_beside_a_victim() -> (tempfile::TempDir, PathBuf) {
        let base = tempfile::tempdir().unwrap();
        let workspace = base.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(base.path().join("victim.md"), "# Victim\n").unwrap();
        (base, workspace)
    }

    #[test]
    fn write_file_refuses_a_key_outside_the_workspace() {
        let (base, workspace) = workspace_beside_a_victim();

        let error = write_file(
            &"../victim".to_string(),
            &"# Escaped\n".to_string(),
            &workspace,
            Format::Markdown,
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "key '../victim' escapes the workspace");
        assert_eq!(
            std::fs::read_to_string(base.path().join("victim.md")).unwrap(),
            "# Victim\n"
        );
    }

    #[test]
    fn apply_changes_refuses_a_create_outside_the_workspace() {
        let (base, workspace) = workspace_beside_a_victim();
        let changes = Changes::new().create(Key::name("../escaped"), "# Escaped\n".to_string());

        let error = apply_changes(&changes, &workspace, Format::Markdown).unwrap_err();

        assert_eq!(error.to_string(), "key '../escaped' escapes the workspace");
        assert!(!base.path().join("escaped.md").exists());
    }

    #[test]
    fn apply_changes_refuses_an_update_outside_the_workspace() {
        let (base, workspace) = workspace_beside_a_victim();
        let changes = Changes::new().update(Key::name("../victim"), "# Replaced\n".to_string());

        let error = apply_changes(&changes, &workspace, Format::Markdown).unwrap_err();

        assert_eq!(error.to_string(), "key '../victim' escapes the workspace");
        assert_eq!(
            std::fs::read_to_string(base.path().join("victim.md")).unwrap(),
            "# Victim\n"
        );
    }

    #[test]
    fn apply_changes_refuses_a_remove_outside_the_workspace() {
        let (base, workspace) = workspace_beside_a_victim();
        let changes = Changes::new().remove(Key::name("../victim"));

        let error = apply_changes(&changes, &workspace, Format::Markdown).unwrap_err();

        assert_eq!(error.to_string(), "key '../victim' escapes the workspace");
        assert_eq!(
            std::fs::read_to_string(base.path().join("victim.md")).unwrap(),
            "# Victim\n"
        );
    }
}
