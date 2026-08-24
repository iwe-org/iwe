use lsp_server::ErrorCode;
use serde_json::json;

use crate::fixture::*;

#[test]
fn unknown_method_returns_method_not_found_without_hanging() {
    let response = Fixture::with("# test\n").raw_response(
        "workspace/executeCommand",
        json!({ "command": "generate", "arguments": [] }),
    );

    let error = response
        .response_result
        .expect_err("expected an error response");
    assert_eq!(error.code, ErrorCode::MethodNotFound as i32);
    assert_eq!(error.message, "unhandled method: workspace/executeCommand");
}
