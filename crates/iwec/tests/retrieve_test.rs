mod fixture;

use fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn retrieve_single_document() {
    let f = Fixture::with_documents(vec![("1", "# Hello world\n\nSome content\n")]).await;

    let result = f.call_tool("iwe_retrieve", json!({"keys": ["1"]})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["documents"].as_array().unwrap().len(), 1);
    assert_eq!(output["documents"][0]["key"], "1");
    assert_eq!(output["documents"][0]["title"], "Hello world");
    assert!(output["documents"][0]["content"]
        .as_str()
        .unwrap()
        .contains("Some content"));
}

#[tokio::test]
async fn retrieve_with_depth_expansion() {
    let f = Fixture::with_documents(vec![
        ("1", "# Parent\n\n[Child](2)\n"),
        ("2", "# Child\n\nChild content\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_retrieve", json!({"keys": ["1"], "depth": 1}))
        .await;
    let output = Fixture::result_json(&result);

    let docs = output["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 2);

    let keys: Vec<&str> = docs.iter().map(|d| d["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"1"));
    assert!(keys.contains(&"2"));
}

#[tokio::test]
async fn retrieve_no_content() {
    let f = Fixture::with_documents(vec![("1", "# Title\n\nLots of text here\n")]).await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "no_content": true, "depth": 0}),
        )
        .await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["documents"][0]["key"], "1");
    assert_eq!(output["documents"][0]["title"], "Title");
    assert_eq!(output["documents"][0]["content"], "");
}

#[tokio::test]
async fn retrieve_multiple_keys() {
    let f = Fixture::with_documents(vec![
        ("1", "# First\n"),
        ("2", "# Second\n"),
        ("3", "# Third\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1", "3"], "depth": 0, "backlinks": false}),
        )
        .await;
    let output = Fixture::result_json(&result);

    let docs = output["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 2);
}

#[tokio::test]
async fn retrieve_with_backlinks() {
    let f = Fixture::with_documents(vec![
        ("1", "# Doc A\n\nSee [Doc B](2) for more\n"),
        ("2", "# Doc B\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["2"], "depth": 0, "backlinks": true}),
        )
        .await;
    let output = Fixture::result_json(&result);

    let backlinks = output["documents"][0]["backlinks"].as_array().unwrap();
    assert_eq!(backlinks.len(), 1);
    assert_eq!(backlinks[0]["key"], "1");
}

fn doc_keys(output: &serde_json::Value) -> Vec<String> {
    let mut v: Vec<String> = output["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["key"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[tokio::test]
async fn retrieve_selector_only_uses_selected_set() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
        ("b", "# B\n\n[X](x)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
    ])
    .await;

    // No explicit keys; selector intersection picks {x}.
    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"in": ["a", "b"], "depth": 0, "context": 0, "backlinks": false}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(doc_keys(&output), vec!["x"]);
}

#[tokio::test]
async fn retrieve_explicit_keys_intersected_with_selector() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
        ("z", "# Z\n"),
    ])
    .await;

    // Explicit asks for [x, y, z]; selector limits to A's subtree {x, y};
    // intersection is {x, y}.
    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["x", "y", "z"], "in": ["a"], "depth": 0, "context": 0, "backlinks": false}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(doc_keys(&output), vec!["x", "y"]);
}

#[tokio::test]
async fn retrieve_empty_intersection_yields_empty() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n"),
        ("b", "# B\n\n[Y](y)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["x"], "in": ["b"], "depth": 0, "context": 0, "backlinks": false}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert!(output["documents"].as_array().unwrap().is_empty());
}
