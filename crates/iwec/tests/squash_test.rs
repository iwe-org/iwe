
use crate::fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn squash_expands_references() {
    let f = Fixture::with_documents(vec![
        ("1", "# Parent\n\n[Child](2)\n"),
        ("2", "# Child\n\nChild content here\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_squash", json!({"key": "1", "depth": 2}))
        .await;
    let text = Fixture::result_text(&result);

    assert!(text.contains("Parent"));
    assert!(text.contains("Child content here"));
}

#[tokio::test]
async fn squash_single_document() {
    let f = Fixture::with_documents(vec![("1", "# Solo\n\nJust text\n")]).await;

    let result = f.call_tool("iwe_squash", json!({"key": "1"})).await;
    let text = Fixture::result_text(&result);

    assert!(text.contains("Solo"));
    assert!(text.contains("Just text"));
}

#[tokio::test]
async fn squash_not_found() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f
        .try_call_tool("iwe_squash", json!({"key": "nonexistent"}))
        .await;
    assert!(result.is_err());
}
