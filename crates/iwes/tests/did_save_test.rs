use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
#[allow(deprecated)]
fn did_save_test_once() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.did_save_text_document(uri(1).to_did_save_params(Some("# updated".to_string())));

    fixture.workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![symbol_info(
            "updated",
            lsp_types::SymbolKind::NAMESPACE,
            uri(1),
            0,
            1,
        )]),
    );
}

#[test]
#[allow(deprecated)]
fn new_file() {
    let fixture = Fixture::new();

    fixture.did_save_text_document(uri(2).to_did_save_params(Some("# test".to_string())));

    fixture.workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![symbol_info(
            "test",
            lsp_types::SymbolKind::NAMESPACE,
            uri(2),
            0,
            1,
        )]),
    );
}

#[test]
#[allow(deprecated)]
fn did_save_test_two_times() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.did_save_text_document(uri(1).to_did_save_params(Some("# updated".to_string())));

    fixture.did_save_text_document(uri(1).to_did_save_params(Some("# updated again".to_string())));

    fixture.workspace_symbols(
        workspace_symbol_params(""),
        workspace_symbol_response(vec![symbol_info(
            "updated again",
            lsp_types::SymbolKind::NAMESPACE,
            uri(1),
            0,
            1,
        )]),
    );
}
