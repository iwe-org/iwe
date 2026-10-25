use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use diwe::config::Format;
use diwe::fs::{read_md_file, workspace_document_path};
use diwe::watcher::{start_poll_watcher, start_watcher, FsChange};
use liwe::graph::Graph;
use liwe::model::Key;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;

pub fn reload_document_from_disk(graph: &mut Graph, base_path: &Path, format: Format, key: &Key) {
    let Ok(path) = workspace_document_path(base_path, key.as_str(), format.extension()) else {
        return;
    };
    match read_md_file(&path) {
        Some(content) => {
            if graph.get_document(key).as_deref() != Some(content.as_str()) {
                tracing::debug!("file changed: key={}", key);
                graph.update_document(key.clone(), content);
            }
        }
        None if !path.exists() && graph.has_key(key) => {
            tracing::debug!("file removed: key={}", key);
            graph.remove_document(key.clone());
        }
        None => {}
    }
}

fn changed_key(change: FsChange) -> Key {
    match change {
        FsChange::Update(key, _) => key,
        FsChange::Remove(key) => key,
    }
}

async fn reload_changed_documents(
    graph: &RwLock<Graph>,
    base_path: &Path,
    format: Format,
    changes: Vec<FsChange>,
) {
    let mut seen = HashSet::new();
    let keys: Vec<Key> = changes
        .into_iter()
        .map(changed_key)
        .filter(|key| seen.insert(key.clone()))
        .collect();
    let mut graph = graph.write().await;
    for key in &keys {
        reload_document_from_disk(&mut graph, base_path, format, key);
    }
}

fn spawn_apply_task<W: Send + 'static>(
    graph: Arc<RwLock<Graph>>,
    base_path: PathBuf,
    format: Format,
    watcher: W,
    mut receiver: UnboundedReceiver<FsChange>,
) {
    tokio::spawn(async move {
        let _watcher = watcher;
        while let Some(change) = receiver.recv().await {
            let mut changes = vec![change];
            while let Ok(change) = receiver.try_recv() {
                changes.push(change);
            }
            reload_changed_documents(&graph, &base_path, format, changes).await;
        }
    });
}

pub fn start(graph: Arc<RwLock<Graph>>, base_path: PathBuf, format: Format) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<FsChange>();
    if let Some(watcher) = start_watcher(base_path.clone(), format, move |change| {
        let _ = tx.send(change);
    }) {
        spawn_apply_task(graph, base_path, format, watcher, rx);
    }
}

pub fn start_polling(
    graph: Arc<RwLock<Graph>>,
    base_path: PathBuf,
    format: Format,
    interval: Duration,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<FsChange>();
    if let Some(watcher) = start_poll_watcher(base_path.clone(), format, interval, move |change| {
        let _ = tx.send(change);
    }) {
        spawn_apply_task(graph, base_path, format, watcher, rx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with(key: &str, content: &str) -> RwLock<Graph> {
        let mut graph = Graph::new();
        graph.insert_document(Key::name(key), content.to_string());
        RwLock::new(graph)
    }

    #[tokio::test]
    async fn a_stale_update_takes_the_newer_contents_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# Newer\n").unwrap();
        let graph = graph_with("note", "# Newer\n");

        reload_changed_documents(
            &graph,
            dir.path(),
            Format::Markdown,
            vec![FsChange::Update(Key::name("note"), "# Older\n".to_string())],
        )
        .await;

        assert_eq!(
            graph.read().await.get_document(&Key::name("note")),
            Some("# Newer\n".to_string())
        );
    }

    #[tokio::test]
    async fn a_remove_for_a_recreated_file_keeps_the_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# Recreated\n").unwrap();
        let graph = graph_with("note", "# Original\n");

        reload_changed_documents(
            &graph,
            dir.path(),
            Format::Markdown,
            vec![FsChange::Remove(Key::name("note"))],
        )
        .await;

        assert_eq!(
            graph.read().await.get_document(&Key::name("note")),
            Some("# Recreated\n".to_string())
        );
    }

    #[tokio::test]
    async fn an_update_for_a_missing_file_removes_the_note() {
        let dir = tempfile::tempdir().unwrap();
        let graph = graph_with("note", "# Note\n");

        reload_changed_documents(
            &graph,
            dir.path(),
            Format::Markdown,
            vec![FsChange::Update(Key::name("note"), "# Note\n".to_string())],
        )
        .await;

        let graph = graph.read().await;
        assert_eq!(graph.get_document(&Key::name("note")), None);
        assert!(!graph.has_key(&Key::name("note")));
    }

    #[tokio::test]
    async fn an_update_for_a_directory_keeps_the_note() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("note.md")).unwrap();
        let graph = graph_with("note", "# Note\n");

        reload_changed_documents(
            &graph,
            dir.path(),
            Format::Markdown,
            vec![FsChange::Update(Key::name("note"), "# Other\n".to_string())],
        )
        .await;

        assert_eq!(
            graph.read().await.get_document(&Key::name("note")),
            Some("# Note\n".to_string())
        );
    }

    #[tokio::test]
    async fn several_events_for_one_key_match_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# Latest\n").unwrap();
        let graph = graph_with("note", "# First\n");

        reload_changed_documents(
            &graph,
            dir.path(),
            Format::Markdown,
            vec![
                FsChange::Update(Key::name("note"), "# Second\n".to_string()),
                FsChange::Remove(Key::name("note")),
                FsChange::Update(Key::name("note"), "# Third\n".to_string()),
            ],
        )
        .await;

        let graph = graph.read().await;
        assert_eq!(
            graph.get_document(&Key::name("note")),
            Some("# Latest\n".to_string())
        );
        assert_eq!(graph.keys(), vec![Key::name("note")]);
    }
}
