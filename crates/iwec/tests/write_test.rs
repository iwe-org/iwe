mod fixture;

use fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn create_document() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .call_tool("iwe_create", json!({"title": "My New Document"}))
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "my-new-document");

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["my-new-document"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    assert_eq!(docs[0]["title"], "My New Document");
}

#[tokio::test]
async fn create_with_content() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .call_tool(
            "iwe_create",
            json!({"title": "Note", "content": "Some body text"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "note");

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["note"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    let content = docs[0]["content"].as_str().unwrap();
    assert!(content.contains("Some body text"));
}

#[tokio::test]
async fn create_duplicate_fails() {
    let f = Fixture::with_documents(vec![("test", "# Test\n")]).await;

    let result = f
        .try_call_tool("iwe_create", json!({"title": "test"}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn update_document() {
    let f = Fixture::with_documents(vec![("1", "# Original\n\nOld content\n")]).await;

    let result = f
        .call_tool(
            "iwe_update",
            json!({"key": "1", "content": "# Updated title\n\nNew content\n"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "1");
    assert_eq!(output["previous_title"], "Original");
    assert_eq!(output["new_title"], "Updated title");

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    let content = docs[0]["content"].as_str().unwrap();
    assert!(content.contains("New content"));
}

#[tokio::test]
async fn update_not_found() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool(
            "iwe_update",
            json!({"key": "nonexistent", "content": "# X\n"}),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delete_document() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n"),
    ])
    .await;

    let result = f.call_tool("iwe_delete", json!({"key": "2"})).await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["removes"].as_array().unwrap().len(), 1);
    assert_eq!(output["removes"][0], "2");
    assert!(!output["updates"].as_array().unwrap().is_empty());

    let find = f.call_tool("iwe_find", json!({})).await;
    let find_output = Fixture::result_json(&find);
    assert_eq!(find_output.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_dry_run() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f
        .call_tool("iwe_delete", json!({"key": "1", "dry_run": true}))
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["removes"][0], "1");

    let find = f.call_tool("iwe_find", json!({})).await;
    let find_output = Fixture::result_json(&find);
    assert_eq!(find_output.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_not_found() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool("iwe_delete", json!({"key": "nonexistent"}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn rename_document() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_rename",
            json!({"old_key": "2", "new_key": "child-renamed"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert!(!output["creates"].as_array().unwrap().is_empty());
    assert!(!output["removes"].as_array().unwrap().is_empty());

    let find = f
        .call_tool("iwe_find", json!({"query": "child-renamed"}))
        .await;
    let find_output = Fixture::result_json(&find);
    assert_eq!(find_output.as_array().unwrap().len(), 1);

    let old = f
        .call_tool("iwe_find", json!({"query": "2"}))
        .await;
    let old_output = Fixture::result_json(&old);
    let has_old = old_output
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["key"] == "2");
    assert!(!has_old);
}

#[tokio::test]
async fn rename_dry_run() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f
        .call_tool(
            "iwe_rename",
            json!({"old_key": "1", "new_key": "renamed", "dry_run": true}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert!(!output["creates"].as_array().unwrap().is_empty());

    let find = f.call_tool("iwe_find", json!({})).await;
    let find_output = Fixture::result_json(&find);
    let keys: Vec<&str> = find_output
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"1"));
    assert!(!keys.contains(&"renamed"));
}

#[tokio::test]
async fn rename_not_found() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool(
            "iwe_rename",
            json!({"old_key": "nonexistent", "new_key": "new"}),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn round_trip_create_retrieve_update_delete() {
    let f = Fixture::with_documents(vec![]).await;

    let create = f
        .call_tool("iwe_create", json!({"title": "Temp Doc", "content": "v1"}))
        .await;
    let key = Fixture::result_json(&create)["key"]
        .as_str()
        .unwrap()
        .to_string();

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": [key], "depth": 0, "backlinks": false}),
        )
        .await;
    let content = Fixture::result_json(&retrieve)[0]["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(content.contains("v1"));

    f.call_tool(
        "iwe_update",
        json!({"key": key, "content": "# Temp Doc\n\nv2\n"}),
    )
    .await;

    let retrieve2 = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": [key], "depth": 0, "backlinks": false}),
        )
        .await;
    let content2 = Fixture::result_json(&retrieve2)[0]["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(content2.contains("v2"));

    f.call_tool("iwe_delete", json!({"key": key})).await;

    let find = f.call_tool("iwe_find", json!({})).await;
    assert_eq!(Fixture::result_json(&find).as_array().unwrap().len(), 0);
}
