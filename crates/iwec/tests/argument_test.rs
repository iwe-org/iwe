use crate::fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn retrieve_refuses_a_misspelled_argument() {
    let f = Fixture::with_documents(vec![("first", "# First\n")]).await;

    let result = f
        .try_call_tool("iwe_retrieve", json!({"key": "first"}))
        .await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: iwe_retrieve does not accept argument(s) key; accepted: backlinks, children, context, depth, exclude, expand, frontmatter, fuzzy, in, in_any, keys, limit, links, max_depth, max_document_tokens, max_documents, max_tokens, not_in, search"
    );
}

#[tokio::test]
async fn delete_refuses_an_undeclared_argument_and_keeps_the_document() {
    let f = Fixture::with_documents(vec![("first", "# First\n")]).await;

    let result = f
        .try_call_tool("iwe_delete", json!({"key": "first", "force": true}))
        .await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: iwe_delete does not accept argument(s) force; accepted: dry_run, key"
    );
    let remaining = f
        .call_tool("iwe_retrieve", json!({"keys": ["first"], "depth": 0}))
        .await;
    assert_eq!(
        Fixture::result_json(&remaining).as_array().unwrap().len(),
        1
    );
}

#[tokio::test]
async fn retrieve_accepts_flattened_selector_arguments() {
    let f = Fixture::with_documents(vec![
        ("first", "# First\n\n[Second](second)\n"),
        ("second", "# Second\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["second"], "in": ["first"], "backlinks": false}),
        )
        .await;

    let keys: Vec<String> = Fixture::result_json(&result)
        .as_array()
        .unwrap()
        .iter()
        .map(|document| document["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(keys, vec!["second".to_string()]);
}
