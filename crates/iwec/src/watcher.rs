use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use diwe::config::Format;
use diwe::watcher::{start_poll_watcher, start_watcher, FsChange};
use liwe::graph::Graph;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;

async fn apply_change(graph: &Arc<Mutex<Graph>>, change: FsChange) {
    let mut g = graph.lock().await;
    match change {
        FsChange::Update(key, content) => {
            tracing::debug!("file changed: key={}", key);
            g.update_document(key, content);
        }
        FsChange::Remove(key) => {
            tracing::debug!("file removed: key={}", key);
            g.remove_document(key);
        }
    }
}

fn spawn_apply_task<W: Send + 'static>(
    graph: Arc<Mutex<Graph>>,
    watcher: W,
    mut receiver: UnboundedReceiver<FsChange>,
) {
    tokio::spawn(async move {
        let _watcher = watcher;
        while let Some(change) = receiver.recv().await {
            apply_change(&graph, change).await;
        }
    });
}

pub fn start(graph: Arc<Mutex<Graph>>, base_path: PathBuf, format: Format) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<FsChange>();
    if let Some(watcher) = start_watcher(base_path, format, move |change| {
        let _ = tx.send(change);
    }) {
        spawn_apply_task(graph, watcher, rx);
    }
}

pub fn start_polling(
    graph: Arc<Mutex<Graph>>,
    base_path: PathBuf,
    format: Format,
    interval: Duration,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<FsChange>();
    if let Some(watcher) = start_poll_watcher(base_path, format, interval, move |change| {
        let _ = tx.send(change);
    }) {
        spawn_apply_task(graph, watcher, rx);
    }
}
