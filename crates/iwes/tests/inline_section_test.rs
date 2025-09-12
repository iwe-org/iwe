use std::u32;

use indoc::indoc;
use liwe::model::config::Configuration;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, CodeActionTriggerKind, DeleteFile,
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, ResourceOp, TextDocumentEdit, TextDocumentIdentifier, TextEdit,
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
fn inline_two_paras() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)
            _
            para 1

            para 2
            "},
        2,
        indoc! {"
            # test

            para 1

            para 2
            "},
    );
}

#[test]
fn inline_para_and_header() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)
            _
            para 1

            # header
            "},
        2,
        indoc! {"
            # test

            para 1

            ## header
            "},
    );
}

#[test]
fn inline_para_and_header_before_para() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)

            para 1
            _
            para 2

            # header
            "},
        2,
        indoc! {"
            # test

            para 1

            para 2

            ## header
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
fn inline_one_of_sub_level_section() {
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
    let fixture = Fixture::with_config(source, Configuration::template());

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
                Position::new(line, 0),
            ),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.inline_section"),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
        },
        CodeAction {
            title: "Inline section".to_string(),
            kind: action_kind("custom.inline_section"),
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

fn assert_no_action(source: &str, line: u32) {
    let fixture = Fixture::with_config(source, Configuration::template());

    fixture.no_code_action(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(line, 0), Position::new(line, 0)),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: CodeActionContext {
            diagnostics: Default::default(),
            only: action_kinds("custom.inline_section"),
            trigger_kind: None,
        },
    })
}
