use crate::fixture::Fixture;
use diwe::config::{Configuration, Patterns, SchemaBinding};
use rmcp::model::ErrorData;
use rmcp::ServiceError;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use tempfile::TempDir;

const CLEAN: &str = "# Summary\n\n# Tasks\n";

const PERSON_SCHEMA: &str =
    "sections:\n  - header: { const: Summary }\n  - header: { const: Tasks }\n";

#[tokio::test]
async fn update_violating_change_is_rejected_and_not_written() {
    let dir = setup(PERSON_SCHEMA);
    let base = dir.path();
    let f = Fixture::with_path(base.to_str().unwrap(), config("person", "docs/**")).await;

    let err = f
        .try_call_tool(
            "iwe_update",
            json!({ "key": "docs/one", "content": "# Summary\n" }),
        )
        .await
        .unwrap_err();

    let error = mcp_error(err);
    assert_eq!(
        error.message,
        "schema validation failed; change rejected:\ndocs/one: required section \"Tasks\" is missing\n"
    );
    assert_eq!(
        error.data,
        Some(json!({
            "violations": [
                {
                    "key": "docs/one",
                    "schema": "person",
                    "violations": [
                        {
                            "breadcrumb": [],
                            "message": "required section \"Tasks\" is missing",
                            "hint": null,
                            "schemaPath": "/sections/1/minContains",
                            "keyword": "minContains"
                        }
                    ]
                }
            ]
        }))
    );

    assert_eq!(read_to_string(base.join("docs/one.md")).unwrap(), CLEAN);
}

#[tokio::test]
async fn update_clean_change_is_written() {
    let dir = setup(PERSON_SCHEMA);
    let base = dir.path();
    let f = Fixture::with_path(base.to_str().unwrap(), config("person", "docs/**")).await;

    let new_content = "# Summary\n\nmore\n\n# Tasks\n";
    f.call_tool(
        "iwe_update",
        json!({ "key": "docs/one", "content": new_content }),
    )
    .await;

    assert_eq!(
        read_to_string(base.join("docs/one.md")).unwrap(),
        new_content
    );
}

#[tokio::test]
async fn create_violating_document_is_rejected_and_not_written() {
    let dir = setup(PERSON_SCHEMA);
    let base = dir.path();
    let f = Fixture::with_path(base.to_str().unwrap(), config("person", "docs/**")).await;

    let err = f
        .try_call_tool("iwe_create", json!({ "key": "docs/ada", "title": "Ada" }))
        .await
        .unwrap_err();

    let error = mcp_error(err);
    assert_eq!(
        error.message,
        "schema validation failed; change rejected:\ndocs/ada: required section \"Summary\" is missing\ndocs/ada: required section \"Tasks\" is missing\n"
    );
    assert!(!base.join("docs/ada.md").exists());
}

#[tokio::test]
async fn missing_schema_file_is_a_configuration_error() {
    let dir = TempDir::new().unwrap();
    let base = dir.path();
    create_dir_all(base.join(".iwe/schemas")).unwrap();
    create_dir_all(base.join("docs")).unwrap();
    write(base.join("docs/one.md"), CLEAN).unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), config("ghost", "docs/**")).await;

    let err = f
        .try_call_tool(
            "iwe_update",
            json!({ "key": "docs/one", "content": "# Summary\n" }),
        )
        .await
        .unwrap_err();

    let error = mcp_error(err);
    assert_eq!(
        error.message,
        "schema configuration error: schema 'ghost': .iwe/schemas/ghost.yaml not found"
    );
    assert_eq!(read_to_string(base.join("docs/one.md")).unwrap(), CLEAN);
}

#[tokio::test]
async fn normalize_is_not_gated_by_a_pre_existing_violation() {
    let dir = TempDir::new().unwrap();
    let base = dir.path();
    create_dir_all(base.join(".iwe/schemas")).unwrap();
    write(base.join(".iwe/schemas/person.yaml"), PERSON_SCHEMA).unwrap();
    create_dir_all(base.join("docs")).unwrap();
    write(base.join("docs/one.md"), "# Summary\n").unwrap();
    let f = Fixture::with_path(base.to_str().unwrap(), config("person", "docs/**")).await;

    let result = f.call_tool("iwe_normalize", json!({})).await;
    let output = Fixture::result_json(&result);

    assert_eq!(output["total"], 1);
}

fn setup(schema: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let base = dir.path();
    create_dir_all(base.join(".iwe/schemas")).unwrap();
    write(base.join(".iwe/schemas/person.yaml"), schema).unwrap();
    create_dir_all(base.join("docs")).unwrap();
    write(base.join("docs/one.md"), CLEAN).unwrap();
    dir
}

fn config(name: &str, pattern: &str) -> Configuration {
    let mut schemas = HashMap::new();
    schemas.insert(
        name.to_string(),
        SchemaBinding {
            r#match: Patterns::One(pattern.to_string()),
        },
    );
    Configuration {
        schemas,
        ..Default::default()
    }
}

fn mcp_error(err: ServiceError) -> ErrorData {
    match err {
        ServiceError::McpError(error) => error,
        other => panic!("expected McpError, got: {other:?}"),
    }
}
