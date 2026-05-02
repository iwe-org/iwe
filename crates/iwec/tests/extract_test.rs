
use crate::fixture::Fixture;
use serde_json::json;

#[tokio::test]
async fn extract_list_sections() {
    let f = Fixture::with_documents(vec![(
        "1",
        "# Root\n\n## Section A\n\nContent A\n\n## Section B\n\nContent B\n",
    )])
    .await;

    let result = f
        .call_tool("iwe_extract", json!({"key": "1", "list": true}))
        .await;
    let output = Fixture::result_json(&result);

    let sections = output.as_array().unwrap();
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0]["title"], "Root");
    assert_eq!(sections[1]["title"], "Section A");
    assert_eq!(sections[2]["title"], "Section B");
}

#[tokio::test]
async fn extract_by_section_title() {
    let f = Fixture::with_documents(vec![(
        "1",
        "# Root\n\n## Section A\n\nContent A\n\n## Section B\n\nContent B\n",
    )])
    .await;

    let result = f
        .call_tool(
            "iwe_extract",
            json!({"key": "1", "section": "Section A"}),
        )
        .await;
    let output = Fixture::result_json(&result);

    assert!(!output["creates"].as_array().unwrap().is_empty());
    assert!(!output["updates"].as_array().unwrap().is_empty());

    let created_key = output["creates"][0]["key"].as_str().unwrap();
    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": [created_key], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    let content = docs[0]["content"].as_str().unwrap();
    assert!(content.contains("Content A"));
}

#[tokio::test]
async fn extract_by_block_number() {
    let f = Fixture::with_documents(vec![(
        "1",
        "# Root\n\n## Section A\n\nContent A\n\n## Section B\n\nContent B\n",
    )])
    .await;

    let result = f
        .call_tool("iwe_extract", json!({"key": "1", "block": 3}))
        .await;
    let output = Fixture::result_json(&result);

    let created_content = output["creates"][0]["content"].as_str().unwrap();
    assert!(created_content.contains("Content B"));
}

#[tokio::test]
async fn extract_dry_run() {
    let f = Fixture::with_documents(vec![(
        "1",
        "# Root\n\n## Sub\n\nText\n",
    )])
    .await;

    let result = f
        .call_tool(
            "iwe_extract",
            json!({"key": "1", "section": "Sub", "dry_run": true}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert!(!output["creates"].as_array().unwrap().is_empty());

    let find = f.call_tool("iwe_find", json!({})).await;
    assert_eq!(Fixture::result_json(&find).as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn extract_not_found() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool("iwe_extract", json!({"key": "nonexistent", "block": 1}))
        .await;
    assert!(result.is_err());
}
