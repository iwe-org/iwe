use std::path::{Path, PathBuf};
use std::sync::Arc;

use liwe::graph::Graph;
use liwe::model::Key;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use tokio::sync::Mutex;

fn path_to_key(path: &Path, base_path: &Path) -> Option<Key> {
    if path.extension().is_none_or(|ext| ext != "md") {
        return None;
    }

    let relative = path.strip_prefix(base_path).ok()?;
    let key_str = relative
        .with_extension("")
        .to_string_lossy()
        .to_string();

    Some(Key::name(&key_str))
}

pub fn start(graph: Arc<Mutex<Graph>>, base_path: PathBuf) {
    start_with_config(graph, base_path, None);
}

pub fn start_with_config(
    graph: Arc<Mutex<Graph>>,
    base_path: PathBuf,
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
            handle_event(&graph, &base_path, event).await;
        }
    });
}

pub fn start_polling(
    graph: Arc<Mutex<Graph>>,
    base_path: PathBuf,
    interval: std::time::Duration,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let config = notify::Config::default().with_poll_interval(interval);
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
            handle_event(&graph, &base_path, event).await;
        }
    });
}

async fn handle_event(graph: &Arc<Mutex<Graph>>, base_path: &Path, event: Event) {
    for path in &event.paths {
        let Some(key) = path_to_key(path, base_path) else {
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
