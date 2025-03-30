use indoc::indoc;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, DeleteFile, DocumentChangeOperation,
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range, ResourceOp,
    TextDocumentEdit, TextDocumentIdentifier, TextEdit,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn inline_quote_test() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para
        "},
    );
}

#[test]
fn inline_with_content_after_ref() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)

            ## test3
            _
            # test2

            para
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para

            ## test3
        "},
    );
}

fn assert_inlined(source: &str, line: u32, inlined: &str) {
    let fixture = Fixture::with(source);

    let delete = DocumentChangeOperation::Op(ResourceOp::Delete(DeleteFile {
        uri: uri(2),
        options: None,
    }));

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                only: action_kinds("refactor.inline.reference.quote"),
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Inline quote".to_string(),
            kind: action_kind("refactor.inline.reference.quote"),
            edit: Some(lsp_types::WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    delete,
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri(1),
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit {
                            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                            new_text: inlined.to_string(),
                        })],
                    }),
                ])),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}
