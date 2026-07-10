use crate::fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn retrieve_single_document() {
    let f = Fixture::with_documents(vec![("1", "# Hello world\n\nSome content\n")]).await;

    let result = f.call_tool("iwe_retrieve", json!({"keys": ["1"]})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output.as_array().unwrap().len(), 1);
    assert_eq!(output[0]["key"], "1");
    assert_eq!(output[0]["title"], "Hello world");
    assert!(output[0]["content"]
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

    let docs = output.as_array().unwrap();
    assert_eq!(docs.len(), 2);

    let keys: Vec<&str> = docs.iter().map(|d| d["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"1"));
    assert!(keys.contains(&"2"));
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

    let docs = output.as_array().unwrap();
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

    let backlinks = output[0]["referencedBy"].as_array().unwrap();
    assert_eq!(backlinks.len(), 1);
    assert_eq!(backlinks[0]["key"], "1");
}

fn doc_keys(output: &serde_json::Value) -> Vec<String> {
    let mut v: Vec<String> = output
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
    assert!(output.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn retrieve_has_no_default_limit() {
    let docs: Vec<(String, String)> = (1..=51)
        .map(|i| (i.to_string(), format!("# Doc {i}\n")))
        .collect();
    let doc_refs: Vec<(&str, &str)> = docs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let keys: Vec<String> = (1..=51).map(|i| i.to_string()).collect();
    let f = Fixture::with_documents(doc_refs).await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": keys, "depth": 0, "context": 0, "backlinks": false}),
        )
        .await;

    let output = Fixture::result_json(&result);
    assert_eq!(output.as_array().unwrap().len(), 51);

    let blocks = Fixture::result_text_blocks(&result);
    assert_eq!(blocks.len(), 1);
}

#[tokio::test]
async fn retrieve_max_documents_bounds_and_notes() {
    let docs: Vec<(String, String)> = (1..=51)
        .map(|i| (i.to_string(), format!("# Doc {i}\n")))
        .collect();
    let doc_refs: Vec<(&str, &str)> = docs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let keys: Vec<String> = (1..=51).map(|i| i.to_string()).collect();
    let f = Fixture::with_documents(doc_refs).await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": keys, "backlinks": false, "max_documents": 2}),
        )
        .await;

    let output = Fixture::result_json(&result);
    assert_eq!(output.as_array().unwrap().len(), 2);

    let blocks = Fixture::result_text_blocks(&result);
    assert_eq!(blocks.len(), 2);
    let note: serde_json::Value = serde_json::from_str(&blocks[1]).expect("note is JSON");
    assert_eq!(note["truncated"], json!(true));
    assert_eq!(note["emitted"], json!(2));
    assert_eq!(note["matched"], json!(51));
    assert_eq!(note["clipped"], json!([]));
    assert_eq!(note.get("budget"), None);
}

#[tokio::test]
async fn retrieve_limit_caps_seeds_without_a_note() {
    let docs: Vec<(String, String)> = (1..=51)
        .map(|i| (i.to_string(), format!("# Doc {i}\n")))
        .collect();
    let doc_refs: Vec<(&str, &str)> = docs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let keys: Vec<String> = (1..=51).map(|i| i.to_string()).collect();
    let f = Fixture::with_documents(doc_refs).await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": keys, "backlinks": false, "limit": 2}),
        )
        .await;

    let output = Fixture::result_json(&result);
    assert_eq!(output.as_array().unwrap().len(), 2);

    let blocks = Fixture::result_text_blocks(&result);
    assert_eq!(blocks.len(), 1);
}

#[tokio::test]
async fn retrieve_expand_conflicts_with_deprecated_alias() {
    let f = Fixture::with_documents(vec![("notes", "# Notes\n")]).await;

    let result = f
        .try_call_tool(
            "iwe_retrieve",
            json!({"keys": ["notes"], "expand": {"includes": 1}, "depth": 1}),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn retrieve_max_document_tokens_clips_content_and_notes() {
    let f = Fixture::with_documents(vec![(
        "notes",
        "# Notes\n\nalpha beta gamma delta epsilon zeta eta theta\n",
    )])
    .await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["notes"], "depth": 0, "context": 0, "backlinks": false, "max_document_tokens": 4}),
        )
        .await;

    let output = Fixture::result_json(&result);
    assert_eq!(
        output[0]["content"].as_str().unwrap(),
        "# Notes\n\nalpha\n\n⋯ truncated (9 tokens omitted)"
    );

    let blocks = Fixture::result_text_blocks(&result);
    assert_eq!(blocks.len(), 2);
    let note: serde_json::Value = serde_json::from_str(&blocks[1]).expect("note is JSON");
    assert_eq!(note["truncated"], json!(true));
    assert_eq!(note["emitted"], json!(1));
    assert_eq!(note["matched"], json!(1));
    assert_eq!(note["clipped"], json!(["notes"]));
    assert_eq!(note["tokens"], json!(13));
    assert_eq!(note.get("budget"), None);
}
