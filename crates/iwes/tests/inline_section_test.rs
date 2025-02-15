use std::u32;

use indoc::indoc;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionOrCommand, CodeActionParams, CodeActionTriggerKind,
    DeleteFile, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, ResourceOp, TextDocumentEdit,
    TextDocumentIdentifier, TextEdit,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn no_action_on_list() {
    assert_no_action(
        indoc! {"
            - test
            "},
        0,
    );
}

#[test]
fn inline_basic_section() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)
            _
            # test2
            "},
        2,
        indoc! {"
            # test

            ## test2
            "},
    );
}

#[test]
fn inline_after_other_refs() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)

            [test3](3)
            _
            # test2
            "},
        2,
        indoc! {"
            # test

            [test3](3)

            ## test2
            "},
    );
}

#[test]
fn inline_middle_section_test() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)

            ## test1

            ## test3
            _
            # test2
            "},
        2,
        indoc! {"
            # test

            ## test2

            ## test1

            ## test3
        "},
    );
}

#[test]
fn inline_after_list() {
    assert_inlined(
        indoc! {"
            # test

            - item1

            [test2](2)
            _
            # test2

            - item2
            "},
        4,
        indoc! {"
            # test

            - item1

            ## test2

            - item2
            "},
    );
}

#[test]
fn inline_after_para() {
    assert_inlined(
        indoc! {"
            # test

            para1

            [test2](2)
            _
            # test2
            "},
        4,
        indoc! {"
            # test

            para1

            ## test2
            "},
    );
}

#[test]
fn inline_third_level_section_test() {
    assert_inlined(
        indoc! {"
            # test

            ## test2

            [test3](2)
            _
            # test3
            "},
        4,
        indoc! {"
            # test

            ## test2

            ### test3
            "},
    );
}

#[test]
fn extract_one_of_sub_level_section() {
    assert_inlined(
        indoc! {"
            # test

            para

            [test2](2)

            ## test3

            - item
            _
            # test2

            - item
            "},
        4,
        indoc! {"
            # test

            para

            ## test2

            - item

            ## test3

            - item
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
            range: Range::new(
                Position::new(line, 0),
                // helix editor provides range even if nothing selected.
                Position::new(line + 1, 1),
            ),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.inline.reference.section"),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
        },
        vec![CodeActionOrCommand::CodeAction(CodeAction {
            title: "Inline section".to_string(),
            kind: action_kind("refactor.inline.reference.section"),
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
        })],
    )
}

fn assert_no_action(source: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.extract.section"),
                trigger_kind: None,
            },
        },
        vec![],
    )
}
