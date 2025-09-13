use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Inline, InlineType};
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

#[test]
fn inline_quote_default_removes_all_references() {
    assert_inlined_remove_target(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para content
            _
            # test3

            [test2](2)

            inline link to [test2](2) text
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para content
            "},
        indoc! {"
            # test3

            inline link to test2 text
            "},
    );
}

#[test]
fn inline_quote_with_keep_target_true_basic() {
    assert_inlined_with_keep_target(
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
fn inline_quote_with_keep_target_true_keeps_other_references() {
    assert_inlined_with_keep_target(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para content
            _
            # test3

            [test2](2)

            inline link to [test2](2) text
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para content
            "},
    );
}

fn assert_inlined_with_keep_target(source: &str, line: u32, inlined: &str) {
    let mut config = Configuration::template();
    config.actions.insert(
        "inline_quote_keep".into(),
        BlockAction::Inline(Inline {
            title: "Inline quote (keep target)".into(),
            inline_type: InlineType::Quote,
            keep_target: Some(true),
        }),
    );

    let fixture = Fixture::with_config(source, config);

    let mut operations = vec![];

    operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier {
            uri: uri(1),
            version: None,
        },
        edits: vec![OneOf::Left(TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: inlined.to_string(),
        })],
    }));

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                only: action_kinds("custom.inline_quote_keep"),
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Inline quote (keep target)".to_string(),
            kind: action_kind("custom.inline_quote_keep"),
            edit: Some(lsp_types::WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(operations)),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}

fn assert_inlined(source: &str, line: u32, inlined: &str) {
    let fixture = Fixture::with_config(source, Configuration::template());

    let mut operations = vec![DocumentChangeOperation::Op(ResourceOp::Delete(
        DeleteFile {
            uri: uri(2),
            options: None,
        },
    ))];

    operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier {
            uri: uri(1),
            version: None,
        },
        edits: vec![OneOf::Left(TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: inlined.to_string(),
        })],
    }));

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                only: action_kinds("custom.inline_quote"),
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Inline quote".to_string(),
            kind: action_kind("custom.inline_quote"),
            edit: Some(lsp_types::WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(operations)),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}

fn assert_inlined_remove_target(source: &str, line: u32, inlined: &str, additional_updates: &str) {
    let fixture = Fixture::with_config(source, Configuration::template());

    let mut operations = vec![DocumentChangeOperation::Op(ResourceOp::Delete(
        DeleteFile {
            uri: uri(2),
            options: None,
        },
    ))];

    operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier {
            uri: uri(1),
            version: None,
        },
        edits: vec![OneOf::Left(TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: inlined.to_string(),
        })],
    }));

    operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier {
            uri: uri(3),
            version: None,
        },
        edits: vec![OneOf::Left(TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: additional_updates.to_string(),
        })],
    }));

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                only: action_kinds("custom.inline_quote"),
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Inline quote".to_string(),
            kind: action_kind("custom.inline_quote"),
            edit: Some(lsp_types::WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(operations)),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}
