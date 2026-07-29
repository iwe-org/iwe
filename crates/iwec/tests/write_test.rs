use crate::fixture::Fixture;
use diwe::config::Configuration;
use serde_json::json;

#[tokio::test]
async fn create_writes_the_document_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    let document = "---\ntags:\n- demo\ntype: note\n---\n\n# Note\n\nBody\n";
    f.call_tool("iwe_create", json!({"key": "note", "content": document}))
        .await;

    let on_disk = std::fs::read_to_string(base.join("note.md")).unwrap();
    assert_eq!(on_disk, document);
}

#[tokio::test]
async fn create_keeps_frontmatter_at_the_first_byte() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    let document = "---\ntype: person\n---\n\n# Ada Lovelace\n\nBody\n";
    f.call_tool(
        "iwe_create",
        json!({"key": "people/ada", "content": document}),
    )
    .await;

    let on_disk = std::fs::read_to_string(base.join("people/ada.md")).unwrap();
    assert_eq!(on_disk, document);
}

#[tokio::test]
async fn create_does_not_add_a_title_heading() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    f.call_tool(
        "iwe_create",
        json!({"key": "note", "content": "Just a paragraph.\n"}),
    )
    .await;

    let on_disk = std::fs::read_to_string(base.join("note.md")).unwrap();
    assert_eq!(on_disk, "Just a paragraph.\n");
}

#[tokio::test]
async fn create_document() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .call_tool(
            "iwe_create",
            json!({"key": "my-new-document", "content": "# My New Document\n"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "my-new-document");
    assert_eq!(output["created"], true);

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
async fn create_with_subdirectory_key() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .call_tool(
            "iwe_create",
            json!({"key": "people/ada", "content": "# Ada\n"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "people/ada");

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["people/ada"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    assert_eq!(docs[0]["title"], "Ada");
}

#[tokio::test]
async fn create_with_key_collision_fails() {
    let f = Fixture::with_documents(vec![("existing", "# Existing\n")]).await;

    let result = f
        .try_call_tool(
            "iwe_create",
            json!({"key": "existing", "content": "# New\n"}),
        )
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: Document 'existing' already exists"
    );
}

#[tokio::test]
async fn create_fails_on_a_file_the_graph_has_not_seen() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    std::fs::write(base.join("note.md"), "# On disk\n").unwrap();

    let result = f
        .try_call_tool("iwe_create", json!({"key": "note", "content": "# New\n"}))
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: Document 'note' already exists"
    );

    let on_disk = std::fs::read_to_string(base.join("note.md")).unwrap();
    assert_eq!(on_disk, "# On disk\n");
}

#[tokio::test]
async fn create_skips_a_file_the_graph_has_not_seen() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    std::fs::write(base.join("note.md"), "# On disk\n").unwrap();

    let result = f
        .call_tool(
            "iwe_create",
            json!({"key": "note", "content": "# New\n", "if_exists": "skip"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "note");
    assert_eq!(output["created"], false);

    let on_disk = std::fs::read_to_string(base.join("note.md")).unwrap();
    assert_eq!(on_disk, "# On disk\n");
}

#[tokio::test]
async fn create_with_skip_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), Configuration::default()).await;

    f.call_tool("iwe_create", json!({"key": "note", "content": "# First\n"}))
        .await;

    let result = f
        .call_tool(
            "iwe_create",
            json!({"key": "note", "content": "# Second\n", "if_exists": "skip"}),
        )
        .await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "note");
    assert_eq!(output["created"], false);

    let on_disk = std::fs::read_to_string(base.join("note.md")).unwrap();
    assert_eq!(on_disk, "# First\n");
}

#[tokio::test]
async fn create_schema_marks_key_and_content_required() {
    let f = Fixture::with_documents(vec![]).await;

    let tools = f.list_tools().await;
    let create = tools
        .tools
        .iter()
        .find(|tool| tool.name == "iwe_create")
        .expect("iwe_create tool to be listed");

    assert_eq!(
        create.input_schema.get("required"),
        Some(&json!(["key", "content"]))
    );
}

#[tokio::test]
async fn create_with_empty_key_fails() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool("iwe_create", json!({"key": "", "content": "# New\n"}))
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: Key must not be empty"
    );
}

#[tokio::test]
async fn create_with_extension_key_fails() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool(
            "iwe_create",
            json!({"key": "note.md", "content": "# New\n"}),
        )
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: Key 'note.md' must not include a file extension"
    );
}

#[tokio::test]
async fn create_without_key_fails() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool("iwe_create", json!({"content": "# New\n"}))
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: 'key' is required: it is the created document's stable identity"
    );
}

#[tokio::test]
async fn create_without_content_fails() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f.try_call_tool("iwe_create", json!({"key": "note"})).await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: 'content' is required: pass the complete document, frontmatter and title heading included"
    );
}

#[tokio::test]
async fn create_with_blank_content_fails() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool("iwe_create", json!({"key": "note", "content": "  \n"}))
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: 'content' is required: pass the complete document, frontmatter and title heading included"
    );
}

#[tokio::test]
async fn create_with_content_and_template_fails() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool(
            "iwe_create",
            json!({"key": "note", "content": "# New\n", "template": "daily"}),
        )
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: 'content' and 'template' are mutually exclusive: content mode writes the document you pass, template mode composes it from a named template"
    );
}

#[tokio::test]
async fn create_with_template_is_not_yet_supported() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool(
            "iwe_create",
            json!({"key": "note", "template": "daily", "variables": {"title": "Note"}}),
        )
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: template mode is not yet supported; pass the complete document in 'content'"
    );
}

#[tokio::test]
async fn create_with_frontmatter_parameter_is_not_yet_supported() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .try_call_tool(
            "iwe_create",
            json!({"key": "note", "content": "# Note\n", "frontmatter": {"type": "note"}}),
        )
        .await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mcp error: -32602: template mode is not yet supported; pass the complete document in 'content'"
    );
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
    let f =
        Fixture::with_documents(vec![("1", "# Root\n\n[Child](2)\n"), ("2", "# Child\n")]).await;

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
    let f =
        Fixture::with_documents(vec![("1", "# Root\n\n[Child](2)\n"), ("2", "# Child\n")]).await;

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
        .call_tool("iwe_find", json!({"fuzzy": "child-renamed"}))
        .await;
    let find_output = Fixture::result_json(&find);
    let has_renamed = find_output
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["key"] == "child-renamed");
    assert!(has_renamed);

    let old = f.call_tool("iwe_find", json!({"fuzzy": "2"})).await;
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
        .call_tool(
            "iwe_create",
            json!({"key": "temp-doc", "content": "# Temp Doc\n\nv1\n"}),
        )
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
