use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Sort};
use lsp_types::{CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier};

mod fixture;
use crate::fixture::*;

#[test]
fn sort_simple_list() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    let fixture = Fixture::with_config(
        indoc! {"
            - zebra
            - apple
            - banana
            "},
        configuration,
    );

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.sort"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        vec![uri(1).to_edit(indoc! {"
                - apple
                - banana
                - zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    )
}

#[test]
fn sort_not_offered_when_already_sorted_ascending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort A-Z".into(),
            reverse: Some(false),
        }),
    );

    let fixture = Fixture::with_config(
        indoc! {"
            - apple
            - banana
            - zebra
            "},
        configuration,
    );

    fixture.no_code_action(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        context: CodeActionContext {
            diagnostics: Default::default(),
            only: action_kinds("custom.sort"),
            trigger_kind: None,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    })
}

#[test]
fn sort_not_offered_when_already_sorted_descending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort Z-A".into(),
            reverse: Some(true),
        }),
    );

    let fixture = Fixture::with_config(
        indoc! {"
            - zebra
            - banana
            - apple
            "},
        configuration,
    );

    fixture.no_code_action(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        context: CodeActionContext {
            diagnostics: Default::default(),
            only: action_kinds("custom.sort"),
            trigger_kind: None,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    })
}

#[test]
fn sort_offered_when_partially_sorted() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort A-Z".into(),
            reverse: Some(false),
        }),
    );

    let fixture = Fixture::with_config(
        indoc! {"
            - apple
            - zebra
            - banana
            "},
        configuration,
    );

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.sort"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        vec![uri(1).to_edit(indoc! {"
                - apple
                - banana
                - zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort A-Z", "custom.sort"),
    )
}

#[test]
fn sort_list_descending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort Descending".into(),
            reverse: Some(true),
        }),
    );

    let fixture = Fixture::with_config(
        indoc! {"
            - zebra
            - apple
            - banana
            "},
        configuration,
    );

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.sort"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        vec![uri(1).to_edit(indoc! {"
                - zebra
                - banana
                - apple
                "})]
        .to_workspace_edit()
        .to_code_action("Sort Descending", "custom.sort"),
    )
}

#[test]
fn sort_ordered_list() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    let fixture = Fixture::with_config(
        indoc! {"
            1. zebra
            2. apple
            3. banana
            "},
        configuration,
    );

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.sort"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        vec![uri(1).to_edit(indoc! {"
                1.  apple
                2.  banana
                3.  zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    )
}
