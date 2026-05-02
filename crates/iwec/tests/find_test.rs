
use crate::fixture::Fixture;
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

    assert_eq!(output.as_array().unwrap().len(), 2);
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

    assert_eq!(output.as_array().unwrap().len(), 2);
    let titles: Vec<&str> = output
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["title"].as_str().unwrap())
        .collect();
    assert!(titles.iter().all(|t| t.to_lowercase().contains("rust")));
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

    assert_eq!(output.as_array().unwrap().len(), 2);
}

fn keys(output: &serde_json::Value) -> Vec<String> {
    let mut v: Vec<String> = output
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[tokio::test]
async fn find_selector_in_intersects_two_parents() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
        ("b", "# B\n\n[X](x)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_find", json!({"in": ["a", "b"]}))
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(keys(&output), vec!["x"]);
}

#[tokio::test]
async fn find_selector_in_any_unions_parents() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n"),
        ("b", "# B\n\n[Y](y)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
        ("z", "# Z\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_find", json!({"in_any": ["a", "b"]}))
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(keys(&output), vec!["x", "y"]);
}

#[tokio::test]
async fn find_selector_not_in_subtracts() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
        ("archive", "# Archive\n\n[Y](y)\n"),
        ("x", "# X\n"),
        ("y", "# Y\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_find",
            json!({"in": ["a"], "not_in": ["archive"]}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(keys(&output), vec!["x"]);
}

#[tokio::test]
async fn find_selector_per_key_depth() {
    // a → b → c. Per-key depth `a:1` keeps only direct children → {b}.
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[B](b)\n"),
        ("b", "# B\n\n[C](c)\n"),
        ("c", "# C\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_find",
            json!({"in": [{"key": "a", "depth": 1}]}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(keys(&output), vec!["b"]);
}

#[tokio::test]
async fn find_selector_max_depth() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[B](b)\n"),
        ("b", "# B\n\n[C](c)\n"),
        ("c", "# C\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_find", json!({"in": ["a"], "max_depth": 1}))
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(keys(&output), vec!["b"]);
}

#[tokio::test]
async fn find_with_replacement_projection() {
    let f = Fixture::with_documents(vec![
        ("doc", "---\npriority: 5\n---\n# Doc\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_find",
            json!({"project": "title=$title,priority"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    let item = &output.as_array().unwrap()[0];
    assert_eq!(item["title"], "Doc");
    assert_eq!(item["priority"], 5);
    assert!(item.get("key").is_none(), "key should not appear under explicit project");
    assert!(item.get("includedBy").is_none());
}

#[tokio::test]
async fn find_with_additive_projection_extends_default() {
    let f = Fixture::with_documents(vec![
        ("doc", "# Doc\n\nBody text.\n"),
    ])
    .await;

    let result = f
        .call_tool("iwe_find", json!({"add_fields": "body=$content"}))
        .await;
    let output = Fixture::result_json(&result);
    let item = &output.as_array().unwrap()[0];
    assert_eq!(item["key"], "doc");
    assert_eq!(item["title"], "Doc");
    assert!(item["body"].as_str().unwrap().contains("# Doc"));
}

#[tokio::test]
async fn find_project_and_add_fields_mutually_exclusive() {
    let f = Fixture::with_documents(vec![("doc", "# Doc\n")]).await;

    let err = f
        .try_call_tool(
            "iwe_find",
            json!({"project": "title", "add_fields": "body=$content"}),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("mutually exclusive"), "got: {err}");
}

#[tokio::test]
async fn find_invalid_projection_is_user_error() {
    let f = Fixture::with_documents(vec![("doc", "# Doc\n")]).await;

    let err = f
        .try_call_tool("iwe_find", json!({"project": "$bogus"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("$bogus"), "got: {err}");
}

#[tokio::test]
async fn find_selector_combines_with_query() {
    let f = Fixture::with_documents(vec![
        ("a", "# A\n\n[Design notes](design)\n\n[Random](random)\n"),
        ("design", "# Design notes\n"),
        ("random", "# Random\n"),
        ("design2", "# Design 2\n"),
    ])
    .await;

    // Selector limits to A's subtree; query further filters to "design".
    let result = f
        .call_tool(
            "iwe_find",
            json!({"in": ["a"], "query": "design"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(keys(&output), vec!["design"]);
}
