mod fixture;

use liwe::model::config::{ActionDefinition, Attach, Configuration};

fn config_with_attach() -> Configuration {
    let mut config = Configuration::default();
    config.actions.insert(
        "today".to_string(),
        ActionDefinition::Attach(Attach {
            title: "Add Date".to_string(),
            key_template: "daily".to_string(),
            document_template: "# Daily\n\n{{content}}\n".to_string(),
        }),
    );
    config
}

#[tokio::test]
async fn list_attach_actions() {
    let f = fixture::Fixture::with_documents_and_config(
        vec![("1", "# Doc")],
        config_with_attach(),
    )
    .await;

    let result = f.call_tool("iwe_attach", serde_json::json!({"list": true})).await;
    let json = fixture::Fixture::result_json(&result);
    let actions = json.as_array().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["name"], "today");
    assert_eq!(actions[0]["title"], "Add Date");
    assert_eq!(actions[0]["target_key"], "daily");
}

#[tokio::test]
async fn attach_creates_new_target() {
    let f = fixture::Fixture::with_documents_and_config(
        vec![("notes", "# My Notes")],
        config_with_attach(),
    )
    .await;

    let result = f
        .call_tool(
            "iwe_attach",
            serde_json::json!({"action": "today", "key": "notes"}),
        )
        .await;
    let json = fixture::Fixture::result_json(&result);
    let creates = json["creates"].as_array().unwrap();
    assert_eq!(creates.len(), 1);
    assert_eq!(creates[0]["key"], "daily");
    assert!(creates[0]["content"].as_str().unwrap().contains("notes"));
}

#[tokio::test]
async fn attach_appends_to_existing_target() {
    let f = fixture::Fixture::with_documents_and_config(
        vec![
            ("notes", "# My Notes"),
            ("daily", "# Daily\n"),
        ],
        config_with_attach(),
    )
    .await;

    let result = f
        .call_tool(
            "iwe_attach",
            serde_json::json!({"action": "today", "key": "notes"}),
        )
        .await;
    let json = fixture::Fixture::result_json(&result);
    let updates = json["updates"].as_array().unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0]["key"], "daily");
    assert!(updates[0]["content"].as_str().unwrap().contains("notes"));
}

#[tokio::test]
async fn attach_rejects_duplicate() {
    let f = fixture::Fixture::with_documents_and_config(
        vec![
            ("notes", "# My Notes"),
            ("daily", "# Daily\n\n[My Notes](notes)\n"),
        ],
        config_with_attach(),
    )
    .await;

    let result = f
        .try_call_tool(
            "iwe_attach",
            serde_json::json!({"action": "today", "key": "notes"}),
        )
        .await;
    assert!(result.is_err() || {
        let r = result.unwrap();
        r.is_error.unwrap_or(false)
    });
}

#[tokio::test]
async fn attach_errors_on_missing_source() {
    let f = fixture::Fixture::with_documents_and_config(
        vec![("daily", "# Daily\n")],
        config_with_attach(),
    )
    .await;

    let result = f
        .try_call_tool(
            "iwe_attach",
            serde_json::json!({"action": "today", "key": "nonexistent"}),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn attach_errors_on_unknown_action() {
    let f = fixture::Fixture::with_documents_and_config(
        vec![("notes", "# My Notes")],
        config_with_attach(),
    )
    .await;

    let result = f
        .try_call_tool(
            "iwe_attach",
            serde_json::json!({"action": "unknown", "key": "notes"}),
        )
        .await;
    assert!(result.is_err());
}
