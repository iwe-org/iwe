use std::fs;
use std::sync::Arc;
use std::time::Duration;

use liwe::graph::Graph;
use liwe::model::Key;
use liwe::model::config::MarkdownOptions;
use tokio::sync::Mutex;

async fn start_watcher(graph: Arc<Mutex<Graph>>, base_path: &std::path::Path) {
    iwec::watcher::start_polling(
        graph,
        base_path.to_path_buf(),
        Duration::from_millis(200),
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
}

async fn wait_for<F, Fut>(timeout: Duration, interval: Duration, mut check: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = tokio::time::Instant::now();
    loop {
        if check().await {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(interval).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watcher_picks_up_new_file() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().canonicalize().unwrap();

    let graph = Arc::new(Mutex::new(Graph::new()));
    start_watcher(graph.clone(), &base_path).await;

    fs::write(base_path.join("hello.md"), "# Hello\n\nWorld\n").unwrap();

    let g = graph.clone();
    let found = wait_for(Duration::from_secs(5), Duration::from_millis(100), || {
        let g = g.clone();
        async move {
            let g = g.lock().await;
            g.keys().iter().any(|k| k.to_string() == "hello")
        }
    })
    .await;

    assert!(found, "expected 'hello' key to appear");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watcher_picks_up_modification() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().canonicalize().unwrap();

    fs::write(base_path.join("doc.md"), "# Original\n").unwrap();

    let state = liwe::fs::new_for_path(&base_path);
    let graph = Arc::new(Mutex::new(Graph::from_state(
        state,
        false,
        MarkdownOptions::default(),
        None,
    )));
    start_watcher(graph.clone(), &base_path).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    fs::write(base_path.join("doc.md"), "# Updated\n\nNew content\n").unwrap();

    let g = graph.clone();
    let found = wait_for(Duration::from_secs(5), Duration::from_millis(100), || {
        let g = g.clone();
        async move {
            let g = g.lock().await;
            g.get_document(&Key::name("doc"))
                .map(|c| c.contains("Updated"))
                .unwrap_or(false)
        }
    })
    .await;

    assert!(found, "expected updated content");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watcher_picks_up_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().canonicalize().unwrap();

    fs::write(base_path.join("to-delete.md"), "# Delete me\n").unwrap();

    let state = liwe::fs::new_for_path(&base_path);
    let graph = Arc::new(Mutex::new(Graph::from_state(
        state,
        false,
        MarkdownOptions::default(),
        None,
    )));

    {
        let g = graph.lock().await;
        assert!(g.keys().iter().any(|k| k.to_string() == "to-delete"));
    }

    start_watcher(graph.clone(), &base_path).await;

    fs::remove_file(base_path.join("to-delete.md")).unwrap();

    let g = graph.clone();
    let found = wait_for(Duration::from_secs(5), Duration::from_millis(100), || {
        let g = g.clone();
        async move {
            let g = g.lock().await;
            !g.keys().iter().any(|k| k.to_string() == "to-delete")
        }
    })
    .await;

    assert!(found, "key should be removed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watcher_ignores_non_md_files() {
    let dir = tempfile::tempdir().unwrap();
    let base_path = dir.path().canonicalize().unwrap();

    let graph = Arc::new(Mutex::new(Graph::new()));
    start_watcher(graph.clone(), &base_path).await;

    fs::write(base_path.join("notes.txt"), "not markdown").unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let g = graph.lock().await;
    assert!(g.keys().is_empty());
}
