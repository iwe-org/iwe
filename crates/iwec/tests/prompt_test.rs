mod fixture;

use fixture::Fixture;
use rmcp::model::{PromptMessage, PromptMessageContent};
use serde_json::json;

fn prompt_text(msg: &PromptMessage) -> String {
    match &msg.content {
        PromptMessageContent::Text { text } => text.clone(),
        other => panic!("expected text content, got: {other:?}"),
    }
}

#[tokio::test]
async fn list_prompts() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let result = f.list_prompts().await;
    let names: Vec<&str> = result.prompts.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"explore"));
    assert!(names.contains(&"review"));
    assert!(names.contains(&"refactor"));
}

#[tokio::test]
async fn explore_prompt() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n"),
    ])
    .await;

    let result = f.get_prompt("explore", json!(null)).await;
    assert!(!result.messages.is_empty());

    let text = prompt_text(&result.messages[0]);
    assert!(text.contains("Statistics"));
    assert!(text.contains("Root documents"));
}

#[tokio::test]
async fn review_prompt() {
    let f = Fixture::with_documents(vec![("1", "# My Doc\n\nSome content\n")]).await;

    let result = f.get_prompt("review", json!({"key": "1"})).await;
    assert!(!result.messages.is_empty());

    let text = prompt_text(&result.messages[0]);
    assert!(text.contains("My Doc"));
}

#[tokio::test]
async fn refactor_prompt() {
    let f = Fixture::with_documents(vec![
        ("1", "# Root\n\n[Child](2)\n"),
        ("2", "# Child\n\nContent\n"),
    ])
    .await;

    let result = f.get_prompt("refactor", json!({"key": "1"})).await;
    assert!(!result.messages.is_empty());

    let text = prompt_text(&result.messages[0]);
    assert!(text.contains("restructuring"));
}
