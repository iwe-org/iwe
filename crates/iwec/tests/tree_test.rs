
use crate::fixture::Fixture;
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

fn root_keys(output: &serde_json::Value) -> Vec<String> {
    let mut v: Vec<String> = output
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["key"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[tokio::test]
async fn tree_selector_uses_selected_set_as_roots() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
        ("b", "# B\n\n[X](x)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_tree", json!({"in": ["a", "b"]}))
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(root_keys(&output), vec!["x"]);
}

#[tokio::test]
async fn tree_explicit_key_intersects_selector() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
    ])
    .await;

    // Explicit Y, selector A's subtree → empty intersection.
    let result = f
        .call_tool("iwe_tree", json!({"keys": ["y"], "in": ["a"]}))
        .await;
    let output = Fixture::result_json(&result);
    assert!(output.as_array().unwrap().is_empty());
}
