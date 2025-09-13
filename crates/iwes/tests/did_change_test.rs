use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
#[allow(deprecated)]
fn did_change_test_once() {
    Fixture::with(indoc! {"
            # test
            "})
    .did_change_text_document(uri(1).to_did_change_params(1, "# updated".to_string()))
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![uri(1).to_symbol_info(
            "updated",
            lsp_types::SymbolKind::NAMESPACE,
            0,
            1,
        )]),
    );
}

#[test]
#[allow(deprecated)]
fn new_file() {
    Fixture::new()
        .did_change_text_document(uri(2).to_did_change_params(1, "# test".to_string()))
        .workspace_symbols(
            workspace_symbol_params(""),
            workspace_symbol_response(vec![uri(2).to_symbol_info(
                "test",
                lsp_types::SymbolKind::NAMESPACE,
                0,
                1,
            )]),
        );
}

#[test]
#[allow(deprecated)]
fn did_change_test_two_times() {
    Fixture::with(indoc! {"
            # test
            "})
    .did_change_text_document(uri(1).to_did_change_params(1, "# updated".to_string()))
    .did_change_text_document(uri(1).to_did_change_params(1, "# updated again".to_string()))
    .workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![uri(1).to_symbol_info(
            "updated again",
            lsp_types::SymbolKind::NAMESPACE,
            0,
            1,
        )]),
    );
}
