mod fixture;

use fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn stats_aggregate() {
    let f = Fixture::with_documents(vec![
        ("1", "# Doc one\n\nSome text\n"),
        ("2", "# Doc two\n\n[Ref to one](1)\n"),
    ])
    .await;

    let result = f.call_tool("iwe_stats", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["totalDocuments"], 2);
    assert!(output["totalNodes"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn stats_per_document() {
    let f = Fixture::with_documents(vec![
        ("1", "# My doc\n\nParagraph one\n\nParagraph two\n"),
    ])
    .await;

    let result = f.call_tool("iwe_stats", json!({"key": "1"})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["key"], "1");
    assert_eq!(output["title"], "My doc");
    assert!(output["paragraphs"].as_u64().unwrap() >= 2);
    assert!(output["lines"].as_u64().unwrap() > 0);
    assert!(output["words"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn stats_not_found() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f
        .try_call_tool("iwe_stats", json!({"key": "nonexistent"}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn stats_broken_links() {
    let f = Fixture::with_documents(vec![
        ("1", "# Doc\n\n[Missing](nonexistent)\n"),
    ])
    .await;

    let result = f.call_tool("iwe_stats", json!({})).await;
    let output = Fixture::result_json(&result);

    assert!(output["brokenLinkCount"].as_u64().unwrap() >= 1);
}
