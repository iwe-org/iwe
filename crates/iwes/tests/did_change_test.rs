use indoc::indoc;

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
fn did_change_reindexes_body_for_search() {
    Fixture::with(indoc! {"
            # Note A
            _
            # Note B
            "})
    .did_change_text_document(uri(2).to_did_change_params(
        1,
        "# Note B\n\nkubernetes kubernetes kubernetes\n".to_string(),
    ))
    .workspace_symbols(
        workspace_symbol_params("kubernetes"),
        workspace_symbol_response(vec![
            uri(2).to_symbol_info("Note B", lsp_types::SymbolKind::NAMESPACE, 0, 1),
            uri(1).to_symbol_info("Note A", lsp_types::SymbolKind::NAMESPACE, 0, 1),
        ]),
    );
}

#[test]
#[allow(deprecated)]
fn did_delete_removes_document_from_search() {
    Fixture::with(indoc! {"
            # One

            kubernetes
            _
            # Two

            kubernetes
            "})
    .did_delete_files(uri(2).to_file_delete_params())
    .workspace_symbols(
        workspace_symbol_params("kubernetes"),
        workspace_symbol_response(vec![uri(1).to_symbol_info(
            "One",
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
