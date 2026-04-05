mod fixture;

use fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn tree_shows_root_documents() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n"),
    ])
    .await;

    let result = f.call_tool("iwe_tree", json!({})).await;
    let output = Fixture::result_json(&result);

    let trees = output.as_array().unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0]["key"], "1");
    assert_eq!(trees[0]["title"], "Root");

    let children = trees[0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["key"], "2");
}

#[tokio::test]
async fn tree_from_specific_key() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[A](2)\n\n[B](3)\n"),
        ("2", "# A\n"),
        ("3", "# B\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_tree", json!({"keys": ["1"]}))
        .await;
    let output = Fixture::result_json(&result);

    let trees = output.as_array().unwrap();
    assert_eq!(trees.len(), 1);
    let children = trees[0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
}

#[tokio::test]
async fn tree_respects_depth_limit() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[A](2)\n"),
        ("2", "# A\n\n[B](3)\n"),
        ("3", "# B\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_tree", json!({"keys": ["1"], "depth": 2}))
        .await;
    let output = Fixture::result_json(&result);

    let root = &output.as_array().unwrap()[0];
    let children = root["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["key"], "2");
}
