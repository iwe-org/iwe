use std::path::{Path, PathBuf};
use std::time::Duration;

use liwe::model::Key;
use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Format;

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

fn dispatch<H: Fn(FsChange)>(base_path: &Path, format: Format, event: Event, handler: &H) {
    for path in &event.paths {
        let Some(key) = path_to_key(path, base_path, format) else {
            continue;
        };

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                if let Ok(content) = std::fs::read_to_string(path) {
                    handler(FsChange::Update(key, content));
                }
            }
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
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                dispatch(&handler_base, format, event, &handler);
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
    let config = Config::default()
        .with_poll_interval(interval)
        .with_compare_contents(true);
    let mut watcher = PollWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                dispatch(&handler_base, format, event, &handler);
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
