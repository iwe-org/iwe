use crate::fixture::Fixture;
use liwe::model::config::Configuration;
use serde_json::json;

#[tokio::test]
async fn normalize_returns_counts() {
    let f = Fixture::with_documents(vec![
        ("1", "# Doc one\n\nSome text\n"),
        ("2", "# Doc two\n\nMore text\n"),
    ])
    .await;

    let result = f.call_tool("iwe_normalize", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 2);
    assert!(output["normalized"].is_u64());
}

#[tokio::test]
async fn normalize_empty_graph() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f.call_tool("iwe_normalize", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 0);
    assert_eq!(output["normalized"], 0);
}

#[tokio::test]
async fn normalize_rewrites_unnormalized_file_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    std::fs::write(base.join("doc.md"), "# Title\n\n\n* one\n* two\n").unwrap();

    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    let result = f.call_tool("iwe_normalize", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 1);
    assert_eq!(output["normalized"], 1);

    let on_disk = std::fs::read_to_string(base.join("doc.md")).unwrap();
    assert_eq!(on_disk, "# Title\n\n- one\n- two\n");
}

#[tokio::test]
async fn normalize_leaves_already_normalized_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    std::fs::write(base.join("doc.md"), "# Title\n\n- one\n- two\n").unwrap();

    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    let result = f.call_tool("iwe_normalize", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 1);
    assert_eq!(output["normalized"], 0);
}
