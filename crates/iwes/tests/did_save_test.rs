use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn did_save_test_once() {
    Fixture::with(indoc! {"
            # test
            "})
    .did_save_text_document(uri(1).to_did_save_params(Some("# updated".to_string())))
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
fn new_file() {
    Fixture::new()
        .did_save_text_document(uri(2).to_did_save_params(Some("# test".to_string())))
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
fn did_save_test_two_times() {
    Fixture::with(indoc! {"
            # test
            "})
    .did_save_text_document(uri(1).to_did_save_params(Some("# updated".to_string())))
    .did_save_text_document(uri(1).to_did_save_params(Some("# updated again".to_string())))
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
