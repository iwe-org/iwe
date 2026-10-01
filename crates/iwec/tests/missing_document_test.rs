use crate::fixture::Fixture;
use rmcp::model::ErrorCode;
use rmcp::ServiceError;
use serde_json::json;

/// Assert the failure is the documented MCP error for a key that is not in the
/// graph: `invalid_params` naming every key that could not be resolved.
fn assert_missing_document_error(error: ServiceError, expected_keys: &[&str]) {
    match error {
        ServiceError::McpError(data) => {
            assert_eq!(
                data.code,
                ErrorCode::INVALID_PARAMS,
                "missing document must be reported as invalid params, got {:?}",
                data
            );
            for key in expected_keys {
                assert!(
                    data.message.contains(&format!("'{key}'")),
                    "error message {:?} should name the missing key {key}",
                    data.message
                );
            }
        }
        other => panic!("expected an MCP error, got {other:?}"),
    }
}

/// A key that is not in the graph must not be echoed back as a document with
/// empty content — the caller has to learn that the key does not exist.
#[tokio::test]
async fn retrieve_rejects_an_unknown_document_key() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n\nbody\n")]).await;

    let error = f
        .try_call_tool("iwe_retrieve", json!({"keys": ["nope"], "depth": 0}))
        .await
        .expect_err("unknown key must not be returned as a document");

    assert_missing_document_error(error, &["nope"]);
}

#[tokio::test]
async fn retrieve_rejects_unknown_keys_in_a_batch_and_names_them() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n\nbody\n")]).await;

    let error = f
        .try_call_tool(
            "iwe_retrieve",
            json!({"keys": ["1", "ghost", "phantom"], "depth": 0}),
        )
        .await
        .expect_err("a batch containing unknown keys must not fabricate documents");

    assert_missing_document_error(error, &["ghost", "phantom"]);
}

/// Search seeds and selectors are a candidate set, not named documents, so an
/// empty match stays a successful empty result.
#[tokio::test]
async fn retrieve_search_without_matches_stays_successful() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n\nbody\n")]).await;

    let result = f
        .call_tool(
            "iwe_retrieve",
            json!({"keys": ["nope"], "search": "nothing-matches-this", "depth": 0}),
        )
        .await;

    assert_eq!(Fixture::result_json(&result).as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn review_prompt_rejects_an_unknown_key() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let error = f
        .try_get_prompt("review", json!({"key": "nope"}))
        .await
        .expect_err("review prompt must not describe a document that does not exist");

    assert_missing_document_error(error, &["nope"]);
}

#[tokio::test]
async fn refactor_prompt_rejects_an_unknown_key() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let error = f
        .try_get_prompt("refactor", json!({"key": "nope"}))
        .await
        .expect_err("refactor prompt must not analyse a document that does not exist");

    assert_missing_document_error(error, &["nope"]);
}

#[tokio::test]
async fn read_resource_reports_a_missing_document() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let error = f
        .try_read_resource("iwe://documents/nope")
        .await
        .expect_err("missing document resource must not resolve");

    match error {
        ServiceError::McpError(data) => {
            assert_eq!(data.code, ErrorCode::RESOURCE_NOT_FOUND);
            assert!(
                data.message.contains("Document 'nope' not found"),
                "unexpected message {:?}",
                data.message
            );
        }
        other => panic!("expected an MCP error, got {other:?}"),
    }
}

#[tokio::test]
async fn read_resource_reports_an_unknown_uri() {
    let f = Fixture::with_documents(vec![("1", "# Doc\n")]).await;

    let error = f
        .try_read_resource("iwe://not-a-resource")
        .await
        .expect_err("unknown resource uri must not resolve");

    match error {
        ServiceError::McpError(data) => {
            assert_eq!(data.code, ErrorCode::RESOURCE_NOT_FOUND);
            assert!(
                data.message.contains("Unknown resource"),
                "unexpected message {:?}",
                data.message
            );
        }
        other => panic!("expected an MCP error, got {other:?}"),
    }
}
