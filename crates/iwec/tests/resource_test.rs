mod fixture;

use fixture::Fixture;
use liwe::model::config::{ActionDefinition, Attach, Configuration};
use rmcp::model::ResourceContents;

#[tokio::test]
async fn list_resources_includes_documents() {
    let f = Fixture::with_documents(vec![
        ("1", "# Doc one\n"),
        ("2", "# Doc two\n"),
    ])
    .await;

    let result = f.list_resources().await;
    assert!(result.resources.len() >= 5);

    let uris: Vec<&str> = result
        .resources
        .iter()
        .map(|r| r.raw.uri.as_str())
        .collect();
    assert!(uris.contains(&"iwe://tree"));
    assert!(uris.contains(&"iwe://stats"));
    assert!(uris.contains(&"iwe://documents/1"));
    assert!(uris.contains(&"iwe://documents/2"));
}

#[tokio::test]
async fn read_document_resource() {
    let f = Fixture::with_documents(vec![("1", "# Hello\n\nWorld\n")]).await;

    let result = f.read_resource("iwe://documents/1").await;
    let text = match &result.contents[0] {
        ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("expected text resource"),
    };
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
}

#[tokio::test]
async fn read_tree_resource() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n"),
    ])
    .await;

    let result = f.read_resource("iwe://tree").await;
    let text = match &result.contents[0] {
        ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("expected text resource"),
    };
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(json.as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn read_stats_resource() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f.read_resource("iwe://stats").await;
    let text = match &result.contents[0] {
        ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("expected text resource"),
    };
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["total_documents"], 1);
}

#[tokio::test]
async fn list_resources_includes_config() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f.list_resources().await;
    let uris: Vec<&str> = result
        .resources
        .iter()
        .map(|r| r.raw.uri.as_str())
        .collect();
    assert!(uris.contains(&"iwe://config"));
}

#[tokio::test]
async fn read_config_resource() {
    let mut config = Configuration::default();
    config.actions.insert(
        "today".to_string(),
        ActionDefinition::Attach(Attach {
            title: "Add Date".to_string(),
            key_template: "daily".to_string(),
            document_template: "# Daily\n\n{{content}}\n".to_string(),
        }),
    );

    let f = Fixture::with_documents_and_config(vec![("1", "# Doc\n")], config).await;

    let result = f.read_resource("iwe://config").await;
    let text = match &result.contents[0] {
        ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("expected text resource"),
    };
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(json["markdown"].is_object());
    assert!(json["library"].is_object());
    assert!(json["actions"].is_array());

    let actions = json["actions"].as_array().unwrap();
    let attach_action = actions.iter().find(|a| a["name"] == "today").unwrap();
    assert_eq!(attach_action["action_type"], "attach");
    assert_eq!(attach_action["title"], "Add Date");
    assert_eq!(attach_action["target_key"], "daily");
}
