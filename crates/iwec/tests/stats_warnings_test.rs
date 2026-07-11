use crate::fixture::Fixture;
use indoc::indoc;
use rmcp::model::CallToolResult;
use serde_json::json;

const ALPHA: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the spring of 1998 while both were studying
    analog synthesizers together. They collaborated on a modular sequencer and
    later co-founded a small workshop building custom filters for touring
    musicians across Europe and beyond.
"};

const BETA: &str = indoc! {"
    # Ada and Kai

    Ada and Kai met in Vienna in the summer of 1998 while both were studying
    analog synthesizers together. They collaborated on a modular sequencer and
    later co-founded a small workshop building custom filters for touring
    musicians across Europe and beyond.
"};

const DISTINCT: &str = indoc! {"
    # Tax Filing Checklist

    Gather every receipt, confirm the standard deduction amount, review the
    quarterly estimated payments, reconcile the brokerage statements, and submit
    the completed federal return well before the April filing deadline to avoid
    any late penalties or accrued interest charges this year.
"};

fn warnings(result: &CallToolResult) -> Vec<String> {
    Fixture::result_text_blocks(result)
        .into_iter()
        .filter(|block| block.starts_with("warning:"))
        .collect()
}

#[tokio::test]
async fn create_result_carries_orphan_and_dangling_warnings() {
    let f = Fixture::with_documents(vec![]).await;

    let result = f
        .call_tool(
            "iwe_create",
            json!({"title": "Notes", "content": "See [missing](ghost)."}),
        )
        .await;

    assert_eq!(
        warnings(&result),
        vec![
            "warning: notes › orphan: no page links here".to_string(),
            "warning: notes › dangling-link: links to missing 'ghost'".to_string(),
        ]
    );
}

#[tokio::test]
async fn findings_are_reported_once_per_session() {
    let f = Fixture::with_documents(vec![]).await;

    let first = f
        .call_tool(
            "iwe_create",
            json!({"title": "Notes", "content": "See [missing](ghost)."}),
        )
        .await;
    assert_eq!(
        warnings(&first),
        vec![
            "warning: notes › orphan: no page links here".to_string(),
            "warning: notes › dangling-link: links to missing 'ghost'".to_string(),
        ]
    );

    let second = f.call_tool("iwe_create", json!({"title": "Extra"})).await;
    assert_eq!(
        warnings(&second),
        vec!["warning: extra › orphan: no page links here".to_string()]
    );
}

#[tokio::test]
async fn per_key_stats_include_similar_pages() {
    let f = Fixture::with_documents(vec![
        ("alpha", ALPHA),
        ("beta", BETA),
        ("distinct", DISTINCT),
    ])
    .await;

    let result = f.call_tool("iwe_stats", json!({"key": "alpha"})).await;
    let output = Fixture::result_json(&result);
    assert_eq!(output["key"], "alpha");
    let similar = output["similarPages"]
        .as_array()
        .expect("similarPages array");
    assert_eq!(similar.len(), 1);
    assert_eq!(similar[0]["key"], "beta");
}

#[tokio::test]
async fn update_result_carries_similar_page_warning() {
    let f = Fixture::with_documents(vec![
        ("alpha", ALPHA),
        (
            "beta",
            "# Ada and Kai\n\nplaceholder text goes here for now.\n",
        ),
        ("distinct", DISTINCT),
    ])
    .await;

    let result = f
        .call_tool("iwe_update", json!({"key": "beta", "content": BETA}))
        .await;

    assert_eq!(
        warnings(&result),
        vec![
            "warning: alpha › orphan: no page links here".to_string(),
            "warning: beta › orphan: no page links here".to_string(),
            "warning: distinct › orphan: no page links here".to_string(),
            "warning: beta › similar-page: closely matches 'alpha' (0.94)".to_string(),
        ]
    );
}
