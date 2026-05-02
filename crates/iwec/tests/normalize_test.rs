
use crate::fixture::Fixture;
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
