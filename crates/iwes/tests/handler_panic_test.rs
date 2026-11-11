use lsp_server::ErrorCode;
use serde_json::json;

use crate::fixture::*;

#[test]
fn request_for_a_non_file_uri_gets_an_error_response() {
    let response = Fixture::with("# test\n").raw_response(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "untitled:Untitled-1" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    let error = response
        .response_result
        .expect_err("expected an error response");
    assert_eq!(error.code, ErrorCode::InternalError as i32);
    assert_eq!(
        error.message,
        "Panic occurred with message: URI is a file URL: ()"
    );
}
