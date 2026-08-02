use std::path::{Path, PathBuf};
use std::time::Duration;

use liwe::model::Key;
use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Format;
use crate::fs::{read_md_file, PathFilter};

pub enum FsChange {
    Update(Key, String),
    Remove(Key),
}

fn path_to_key(path: &Path, base_path: &Path, format: Format) -> Option<Key> {
    if path.extension().is_none_or(|ext| ext != format.extension()) {
        return None;
    }

    let relative = path.strip_prefix(base_path).ok()?;
    let key_str = relative
        .with_extension("")
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(os) => Some(os.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    Some(Key::from_stripped(&key_str))
}

fn dispatch<H: Fn(FsChange)>(
    base_path: &Path,
    format: Format,
    filter: &PathFilter,
    event: Event,
    handler: &H,
) {
    for path in &event.paths {
        let Some(key) = path_to_key(path, base_path, format) else {
            continue;
        };

        if !filter.includes(path) {
            continue;
        }

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => match read_md_file(path) {
                Some(content) => handler(FsChange::Update(key, content)),
                None if !path.exists() => handler(FsChange::Remove(key)),
                None => {}
            },
            EventKind::Remove(_) => {
                handler(FsChange::Remove(key));
            }
            _ => {}
        }
    }
}

pub fn start_watcher(
    base_path: PathBuf,
    format: Format,
    handler: impl Fn(FsChange) + Send + 'static,
) -> Option<impl Watcher + Send> {
    let base_path = base_path.canonicalize().unwrap_or(base_path);
    let handler_base = base_path.clone();
    let filter = PathFilter::new(&base_path);
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                dispatch(&handler_base, format, &filter, event, &handler);
            }
        },
        Config::default(),
    )
    .ok()?;

    watcher.watch(&base_path, RecursiveMode::Recursive).ok()?;
    Some(watcher)
}

pub fn start_poll_watcher(
    base_path: PathBuf,
    format: Format,
    interval: Duration,
    handler: impl Fn(FsChange) + Send + 'static,
) -> Option<impl Watcher + Send> {
    let base_path = base_path.canonicalize().unwrap_or(base_path);
    let handler_base = base_path.clone();
    let filter = PathFilter::new(&base_path);
    let config = Config::default()
        .with_poll_interval(interval)
        .with_compare_contents(true);
    let mut watcher = PollWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                dispatch(&handler_base, format, &filter, event, &handler);
            }
        },
        config,
    )
    .ok()?;

    watcher.watch(&base_path, RecursiveMode::Recursive).ok()?;
    Some(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn path_to_key_uses_forward_slash_separators_for_nested_files() {
        let base = PathBuf::from("base");
        let path = base.join("sub").join("dir").join("note.md");

        let key = path_to_key(&path, &base, Format::Markdown).unwrap();

        assert_eq!(key, Key::from_stripped("sub/dir/note"));
    }

    #[test]
    fn path_to_key_ignores_non_markdown_files() {
        let base = PathBuf::from("base");
        let path = base.join("notes.txt");

        assert_eq!(path_to_key(&path, &base, Format::Markdown), None);
    }

    #[cfg(unix)]
    #[test]
    fn non_canonical_base_drops_events_that_a_canonical_base_keeps() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let linked_base = dir.path().join("linked");
        std::os::unix::fs::symlink(&real, &linked_base).unwrap();

        let event_path = real.canonicalize().unwrap().join("note.md");

        assert_eq!(
            path_to_key(&event_path, &linked_base, Format::Markdown),
            None
        );
        assert_eq!(
            path_to_key(
                &event_path,
                &linked_base.canonicalize().unwrap(),
                Format::Markdown
            ),
            Some(Key::from_stripped("note"))
        );
    }

    #[test]
    fn dispatch_reports_remove_when_a_modified_path_no_longer_exists() {
        use notify::event::{ModifyKind, RenameMode};

        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        let (tx, rx) = unbounded::<FsChange>();
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Any)))
            .add_path(base.join("note.md"));

        dispatch(
            &base,
            Format::Markdown,
            &PathFilter::new(&base),
            event,
            &move |change| {
                let _ = tx.send(change);
            },
        );

        match rx.try_recv().expect("a change") {
            FsChange::Remove(key) => assert_eq!(key, Key::from_stripped("note")),
            FsChange::Update(_, _) => panic!("expected a remove change"),
        }
    }

    #[test]
    fn dispatch_normalizes_line_endings_on_update() {
        use notify::event::ModifyKind;

        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::fs::write(base.join("note.md"), "# Title\r\n").unwrap();
        let (tx, rx) = unbounded::<FsChange>();
        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(base.join("note.md"));

        dispatch(
            &base,
            Format::Markdown,
            &PathFilter::new(&base),
            event,
            &move |change| {
                let _ = tx.send(change);
            },
        );

        match rx.try_recv().expect("a change") {
            FsChange::Update(key, content) => {
                assert_eq!(key, Key::from_stripped("note"));
                assert_eq!(content, "# Title\n");
            }
            FsChange::Remove(_) => panic!("expected an update change"),
        }
    }

    #[test]
    fn poll_watcher_reports_update_on_external_write() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().to_path_buf();

        let (tx, rx) = unbounded::<FsChange>();
        let _watcher = start_poll_watcher(
            base_path.clone(),
            Format::Markdown,
            Duration::from_millis(10),
            move |change| {
                let _ = tx.send(change);
            },
        )
        .expect("watcher to start");

        std::fs::write(base_path.join("note.md"), "# External\n").unwrap();

        let change = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("a change within timeout");

        match change {
            FsChange::Update(key, content) => {
                assert_eq!(key, Key::from_stripped("note"));
                assert_eq!(content, "# External\n");
            }
            FsChange::Remove(_) => panic!("expected an update change"),
        }
    }

    fn workspace_with_ignored_files() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        std::fs::create_dir_all(base.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(base.join(".hidden")).unwrap();
        std::fs::write(base.join(".gitignore"), "node_modules\n").unwrap();
        std::fs::write(base.join("note.md"), "# Note\n").unwrap();
        std::fs::write(base.join("node_modules/pkg/README.md"), "# Dependency\n").unwrap();
        std::fs::write(base.join(".hidden/secret.md"), "# Secret\n").unwrap();
        dir
    }

    fn dispatched_keys(base: &Path, kind: EventKind) -> Vec<Key> {
        let event = Event::new(kind)
            .add_path(base.join("node_modules/pkg/README.md"))
            .add_path(base.join(".hidden/secret.md"))
            .add_path(base.join("note.md"));

        let (tx, rx) = unbounded::<FsChange>();
        dispatch(
            base,
            Format::Markdown,
            &PathFilter::new(base),
            event,
            &move |change| {
                let _ = tx.send(change);
            },
        );

        let mut keys = Vec::new();
        while let Ok(change) = rx.try_recv() {
            keys.push(match change {
                FsChange::Update(key, _) => key,
                FsChange::Remove(key) => key,
            });
        }
        keys
    }

    #[test]
    fn dispatch_keeps_library_writes_and_drops_ignored_ones() {
        use notify::event::ModifyKind;

        let dir = workspace_with_ignored_files();

        assert_eq!(
            dispatched_keys(dir.path(), EventKind::Modify(ModifyKind::Any)),
            vec![Key::from_stripped("note")]
        );
    }

    #[test]
    fn dispatch_keeps_library_deletions_and_drops_ignored_ones() {
        use notify::event::RemoveKind;

        let dir = workspace_with_ignored_files();

        assert_eq!(
            dispatched_keys(dir.path(), EventKind::Remove(RemoveKind::Any)),
            vec![Key::from_stripped("note")]
        );
    }

    #[test]
    fn poll_watcher_ignores_writes_under_a_gitignored_directory() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().to_path_buf();
        std::fs::write(base_path.join(".gitignore"), "node_modules\n").unwrap();
        std::fs::create_dir_all(base_path.join("node_modules/pkg")).unwrap();

        let (tx, rx) = unbounded::<FsChange>();
        let _watcher = start_poll_watcher(
            base_path.clone(),
            Format::Markdown,
            Duration::from_millis(10),
            move |change| {
                let _ = tx.send(change);
            },
        )
        .expect("watcher to start");

        std::fs::write(
            base_path.join("node_modules/pkg/README.md"),
            "# Dependency\n",
        )
        .unwrap();
        std::fs::write(base_path.join("note.md"), "# Note\n").unwrap();

        let change = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("a change within timeout");

        match change {
            FsChange::Update(key, content) => {
                assert_eq!(key, Key::from_stripped("note"));
                assert_eq!(content, "# Note\n");
            }
            FsChange::Remove(_) => panic!("expected an update change"),
        }
    }

    #[test]
    fn poll_watcher_reports_remove_on_external_delete() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().to_path_buf();
        std::fs::write(base_path.join("note.md"), "# External\n").unwrap();

        let (tx, rx) = unbounded::<FsChange>();
        let _watcher = start_poll_watcher(
            base_path.clone(),
            Format::Markdown,
            Duration::from_millis(10),
            move |change| {
                let _ = tx.send(change);
            },
        )
        .expect("watcher to start");

        std::fs::remove_file(base_path.join("note.md")).unwrap();

        let key = loop {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(FsChange::Remove(key)) => break key,
                Ok(FsChange::Update(_, _)) => continue,
                Err(_) => panic!("expected a remove change within timeout"),
            }
        };

        assert_eq!(key, Key::from_stripped("note"));
    }
}
