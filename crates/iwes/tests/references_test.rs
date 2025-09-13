use indoc::indoc;
use lsp_types::{
    Location, PartialResultParams, Position, Range, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams,
};

mod fixture;
use crate::fixture::*;

#[test]
fn single_reference() {
    let fixture = Fixture::with(indoc! {"
        # doc1

        [target](3)
        _
        # doc2

        [target](3)
        _
        # target
        "});

    fixture.references(
        ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 1),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: ReferenceContext {
                include_declaration: false,
            },
        },
        vec![Location {
            uri: uri(2),
            range: Range::new(Position::new(2, 0), Position::new(3, 0)),
        }],
    );
}

#[test]
fn two_references() {
    let fixture = Fixture::with(indoc! {"
        # doc1

        [target](4)
        _
        # doc2

        [target](4)
        _
        # doc3

        [target](4)
        _
        # target
        "});

    fixture.references(
        ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 1),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: ReferenceContext {
                include_declaration: false,
            },
        },
        vec![
            Location {
                uri: uri(2),
                range: Range::new(Position::new(2, 0), Position::new(3, 0)),
            },
            Location {
                uri: uri(3),
                range: Range::new(Position::new(2, 0), Position::new(3, 0)),
            },
        ],
    );
}

#[test]
fn link() {
    let fixture = Fixture::with(indoc! {"
        # header 1

        text and link [target](2)
        _
        # target
        "});

    fixture.references(
        ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 15),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: ReferenceContext {
                include_declaration: false,
            },
        },
        vec![],
    );
}
