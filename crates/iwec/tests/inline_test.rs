mod fixture;

use fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn inline_list_references() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child A](2)\n\n[Child B](3)\n"),
        ("2", "# Child A\n\nA content\n"),
        ("3", "# Child B\n\nB content\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_inline", json!({"key": "1", "list": true}))
        .await;
    let output = Fixture::result_json(&result);

    let refs = output.as_array().unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0]["key"], "2");
    assert_eq!(refs[1]["key"], "3");
}

#[tokio::test]
async fn inline_by_reference() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n\nChild content\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_inline", json!({"key": "1", "reference": "Child"}))
        .await;
    let output = Fixture::result_json(&result);

    assert!(!output["removes"].as_array().unwrap().is_empty());

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    let content = docs[0]["content"].as_str().unwrap();
    assert!(content.contains("Child content"));
}

#[tokio::test]
async fn inline_keep_target() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n\nChild content\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_inline",
            json!({"key": "1", "block": 1, "keep_target": true}),
        )
        .await;
    let output = Fixture::result_json(&result);

    assert!(output["removes"].as_array().unwrap().is_empty());

    let find = f.call_tool("iwe_find", json!({})).await;
    assert_eq!(Fixture::result_json(&find).as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn inline_dry_run() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_inline",
            json!({"key": "1", "block": 1, "dry_run": true}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert!(!output["removes"].as_array().unwrap().is_empty());

    let find = f.call_tool("iwe_find", json!({})).await;
    assert_eq!(Fixture::result_json(&find).as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn extract_then_inline_round_trip() {
    let f = Fixture::with_documents(vec![(
        "1",
        "# Root\n\n## Sub\n\nSub content\n",
    )])
    .await;

    f.call_tool(
        "iwe_extract",
        json!({"key": "1", "section": "Sub"}),
    )
    .await;

    let find = f.call_tool("iwe_find", json!({})).await;
    assert_eq!(Fixture::result_json(&find).as_array().unwrap().len(), 2);

    let list = f
        .call_tool("iwe_inline", json!({"key": "1", "list": true}))
        .await;
    let refs = Fixture::result_json(&list);
    assert_eq!(refs.as_array().unwrap().len(), 1);

    f.call_tool("iwe_inline", json!({"key": "1", "block": 1}))
        .await;

    let find2 = f.call_tool("iwe_find", json!({})).await;
    assert_eq!(Fixture::result_json(&find2).as_array().unwrap().len(), 1);

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    let content = docs[0]["content"].as_str().unwrap();
    assert!(content.contains("Sub content"));
}
