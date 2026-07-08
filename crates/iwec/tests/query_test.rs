use crate::fixture::Fixture;
use indoc::indoc;
use serde_json::json;

#[tokio::test]
async fn query_find_content_membership_and_blocks() {
    let f = Fixture::with_documents(vec![
        ("1", "# One\n\nalpha TODO beta\n"),
        ("2", "# Two\n\nnothing here\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_query",
            json!({
                "operation": "find",
                "document": indoc! {"
                    filter: { $content: { $text: TODO } }
                    project:
                      key: $key
                      hits: { $blocks: { $text: TODO } }
                "},
            }),
        )
        .await;

    let out = Fixture::result_json(&result);
    let docs = out.as_array().expect("find returns an array");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["key"], "1");
    assert_eq!(docs[0]["hits"][0]["text"], "alpha TODO beta");
}

#[tokio::test]
async fn query_count_by_content() {
    let f = Fixture::with_documents(vec![
        ("1", "# One\n\nalpha TODO beta\n"),
        ("2", "# Two\n\nnothing here\n"),
        ("3", "# Three\n\nanother TODO\n"),
    ])
    .await;

    let result = f
        .call_tool(
            "iwe_query",
            json!({
                "operation": "count",
                "document": "filter: { $content: { $text: TODO } }\n",
            }),
        )
        .await;

    assert_eq!(Fixture::result_json(&result)["count"], 2);
}

#[tokio::test]
async fn query_update_block_operator_applies() {
    let f = Fixture::with_documents(vec![("1", "# Roadmap\n\n## Goals\n\nShip it\n")]).await;

    let result = f
        .call_tool(
            "iwe_query",
            json!({
                "operation": "update",
                "document": indoc! {"
                    filter: { $key: '1' }
                    expect: 1
                    update:
                      $replaceText: { $header: Goals, to: Aims, expect: 1 }
                "},
            }),
        )
        .await;

    let out = Fixture::result_json(&result);
    assert_eq!(out["dry_run"], false);
    let content = out["changed"][0]["content"].as_str().expect("content");
    assert_eq!(content, "# Roadmap\n\n## Aims\n\nShip it\n");

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    assert_eq!(docs[0]["content"], "# Roadmap\n\n## Aims\n\nShip it\n");
}

#[tokio::test]
async fn query_update_strict_requires_expect() {
    let f = Fixture::with_documents(vec![("1", "# Roadmap\n\n## Goals\n\nShip it\n")]).await;

    let result = f
        .try_call_tool(
            "iwe_query",
            json!({
                "operation": "update",
                "document": indoc! {"
                    filter: { $key: '1' }
                    update:
                      $replaceText: { $header: Goals, to: Aims }
                "},
            }),
        )
        .await;

    assert!(result.is_err(), "unguarded mutation must be rejected");
}

#[tokio::test]
async fn query_update_expect_mismatch_aborts() {
    let f = Fixture::with_documents(vec![("1", "# Roadmap\n\n## Goals\n\nShip it\n")]).await;

    let result = f
        .try_call_tool(
            "iwe_query",
            json!({
                "operation": "update",
                "document": indoc! {"
                    filter: { $key: '1' }
                    expect: 1
                    update:
                      $replaceText: { $header: Goals, to: Aims, expect: 5 }
                "},
            }),
        )
        .await;

    assert!(result.is_err(), "expect mismatch must abort the operation");

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    assert_eq!(docs[0]["content"], "# Roadmap\n\n## Goals\n\nShip it\n");
}

#[tokio::test]
async fn query_update_dry_run_does_not_write() {
    let f = Fixture::with_documents(vec![("1", "# Roadmap\n\n## Goals\n\nShip it\n")]).await;

    let result = f
        .call_tool(
            "iwe_query",
            json!({
                "operation": "update",
                "dry_run": true,
                "document": indoc! {"
                    filter: { $key: '1' }
                    expect: 1
                    update:
                      $replaceText: { $header: Goals, to: Aims, expect: 1 }
                "},
            }),
        )
        .await;

    let out = Fixture::result_json(&result);
    assert_eq!(out["dry_run"], true);

    let retrieve = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["1"], "depth": 0, "backlinks": false}),
        )
        .await;
    let docs = Fixture::result_json(&retrieve);
    assert_eq!(docs[0]["content"], "# Roadmap\n\n## Goals\n\nShip it\n");
}

#[tokio::test]
async fn query_delete_removes_document() {
    let f = Fixture::with_documents(vec![("1", "# One\n\nbody\n"), ("2", "# Two\n\nbody\n")]).await;

    let result = f
        .call_tool(
            "iwe_query",
            json!({
                "operation": "delete",
                "document": "filter: { $key: '2' }\nexpect: 1\n",
            }),
        )
        .await;

    let out = Fixture::result_json(&result);
    assert_eq!(out["removes"], json!(["2"]));

    let count = f
        .call_tool(
            "iwe_query",
            json!({
                "operation": "count",
                "document": "filter: {}\n",
            }),
        )
        .await;
    assert_eq!(Fixture::result_json(&count)["count"], 1);
}

#[tokio::test]
async fn query_delete_strict_requires_expect() {
    let f = Fixture::with_documents(vec![("1", "# One\n\nbody\n"), ("2", "# Two\n\nbody\n")]).await;

    let result = f
        .try_call_tool(
            "iwe_query",
            json!({
                "operation": "delete",
                "document": "filter: { $key: '2' }\n",
            }),
        )
        .await;

    assert!(result.is_err(), "unguarded delete must be rejected");
}
