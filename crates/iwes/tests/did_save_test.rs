use indoc::indoc;
use lsp_types::{
    DidSaveTextDocumentParams, Position, Range, SymbolInformation, TextDocumentIdentifier,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
};

mod fixture;
use crate::fixture::*;

#[test]
#[allow(deprecated)]
fn did_save_test_once() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.did_save_text_document(DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        text: Some("# updated".to_string()),
    });

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: String::default(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            kind: lsp_types::SymbolKind::NAMESPACE,
            location: lsp_types::Location {
                uri: uri(1),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            name: "updated".to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}

#[test]
#[allow(deprecated)]
fn new_file() {
    let fixture = Fixture::new();

    fixture.did_save_text_document(DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri(2) },
        text: Some("# test".to_string()),
    });

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: String::default(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            kind: lsp_types::SymbolKind::NAMESPACE,
            location: lsp_types::Location {
                uri: uri(2),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            name: "test".to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}

#[test]
#[allow(deprecated)]
fn did_save_test_two_times() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.did_save_text_document(DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        text: Some("# updated".to_string()),
    });

    fixture.did_save_text_document(DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        text: Some("# updated again".to_string()),
    });

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: String::default(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            kind: lsp_types::SymbolKind::NAMESPACE,
            location: lsp_types::Location {
                uri: uri(1),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            name: "updated again".to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}
