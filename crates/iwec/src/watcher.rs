use std::path::{Path, PathBuf};
use std::sync::Arc;

use liwe::graph::Graph;
use liwe::model::config::Format;
use liwe::model::Key;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use tokio::sync::Mutex;

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

pub fn start(graph: Arc<Mutex<Graph>>, base_path: PathBuf, format: Format) {
    start_with_config(graph, base_path, format, None);
}

pub fn start_with_config(
    graph: Arc<Mutex<Graph>>,
    base_path: PathBuf,
    format: Format,
    config: Option<notify::Config>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let config = config.unwrap_or_default();
    let mut watcher = notify::RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        config,
    )
    .expect("filesystem watcher");

    watcher
        .watch(&base_path, RecursiveMode::Recursive)
        .expect("watch directory");

    tokio::spawn(async move {
        let _watcher = watcher;
        while let Some(event) = rx.recv().await {
            handle_event(&graph, &base_path, format, event).await;
        }
    });
}

pub fn start_polling(
    graph: Arc<Mutex<Graph>>,
    base_path: PathBuf,
    format: Format,
    interval: std::time::Duration,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let config = notify::Config::default()
        .with_poll_interval(interval)
        .with_compare_contents(true);
    let mut watcher = notify::PollWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        config,
    )
    .expect("poll watcher");

    watcher
        .watch(&base_path, RecursiveMode::Recursive)
        .expect("watch directory");

    tokio::spawn(async move {
        let _watcher = watcher;
        while let Some(event) = rx.recv().await {
            handle_event(&graph, &base_path, format, event).await;
        }
    });
}

async fn handle_event(graph: &Arc<Mutex<Graph>>, base_path: &Path, format: Format, event: Event) {
    for path in &event.paths {
        let Some(key) = path_to_key(path, base_path, format) else {
            continue;
        };

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                tracing::debug!("file changed: {} -> key={}", path.display(), key);
                let mut g = graph.lock().await;
                g.update_document(key, content);
            }
            EventKind::Remove(_) => {
                tracing::debug!("file removed: {} -> key={}", path.display(), key);
                let mut g = graph.lock().await;
                g.remove_document(key);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_to_key_uses_forward_slash_separators_for_nested_files() {
        let base = PathBuf::from("base");
        let path = base.join("sub").join("dir").join("note.md");

        let key = path_to_key(&path, &base, Format::Markdown).unwrap();

        assert_eq!(key, Key::name("sub/dir/note"));
    }
}
