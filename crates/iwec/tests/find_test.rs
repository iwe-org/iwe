mod fixture;

use fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn find_all_documents() {
    let f = Fixture::with_documents(vec![
        ("1", "# First document\n"),
        ("2", "# Second document\n"),
    ])
    .await;

    let result = f.call_tool("iwe_find", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 2);
    assert_eq!(output["results"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn find_by_query() {
    let f = Fixture::with_documents(vec![
        ("1", "# Rust programming\n"),
        ("2", "# Python scripting\n"),
        ("3", "# Rust macros\n"),
    ])
    .await;

    let result = f.call_tool("iwe_find", json!({"query": "rust"})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 2);
    let titles: Vec<&str> = output["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["title"].as_str().unwrap())
        .collect();
    assert!(titles.iter().all(|t| t.to_lowercase().contains("rust")));
}

#[tokio::test]
async fn find_root_documents() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root doc\n\n[child](2)\n"),
        ("2", "# Child doc\n"),
    ])
    .await;

    let result = f.call_tool("iwe_find", json!({"roots": true})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 1);
    assert_eq!(output["results"][0]["key"], "1");
}

#[tokio::test]
async fn find_with_limit() {
    let f = Fixture::with_documents(vec![
        ("1", "# Doc one\n"),
        ("2", "# Doc two\n"),
        ("3", "# Doc three\n"),
    ])
    .await;

    let result = f.call_tool("iwe_find", json!({"limit": 2})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["results"].as_array().unwrap().len(), 2);
    assert_eq!(output["total"], 3);
}
