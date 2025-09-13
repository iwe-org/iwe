use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Inline, InlineType};
use lsp_types::{
    CodeActionContext, CodeActionParams, CodeActionTriggerKind, Position, Range,
    TextDocumentIdentifier,
};

mod fixture;
use crate::fixture::*;

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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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
    assert_inlined_remove(
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

#[test]
fn inline_section_default_removes_all_references() {
    assert_inlined_remove_target(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            some content
            _
            # test3

            [test2](2)

            inline link to [test2](2)
            "},
        2,
        indoc! {"
            # test

            ## test2

            some content
            "},
        indoc! {"
            # test3

            inline link to test2
            "},
    );
}

#[test]
fn inline_with_keep_target_true_basic_section() {
    assert_inlined_keep(
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
fn inline_with_keep_target_true_keeps_other_references() {
    assert_inlined_keep(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            some content
            _
            # test3

            [test2](2)

            inline link to [test2](2)
            "},
        2,
        indoc! {"
            # test

            ## test2

            some content
            "},
    );
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

fn assert_inlined_remove(source: &str, line: u32, inlined: &str) {
    let fixture = Fixture::with_config(source, Configuration::template());

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.inline_section"),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
        },
        vec![uri(2).to_delete_file(), uri(1).to_edit(inlined)]
            .to_workspace_edit()
            .to_code_action("Inline section", "custom.inline_section"),
    )
}

fn assert_inlined_remove_target(source: &str, line: u32, inlined: &str, additional_updates: &str) {
    let fixture = Fixture::with_config(source, Configuration::template());

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.inline_section"),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
        },
        vec![
            uri(2).to_delete_file(),
            uri(1).to_edit(inlined),
            uri(3).to_edit(additional_updates),
        ]
        .to_workspace_edit()
        .to_code_action("Inline section", "custom.inline_section"),
    )
}

fn assert_inlined_keep(source: &str, line: u32, inlined: &str) {
    let mut config = Configuration::template();
    config.actions.insert(
        "inline_section_keep".into(),
        BlockAction::Inline(Inline {
            title: "Inline section (keep target)".into(),
            inline_type: InlineType::Section,
            keep_target: Some(true),
        }),
    );

    let fixture = Fixture::with_config(source, config);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.inline_section_keep"),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
        },
        vec![uri(1).to_edit(inlined)]
            .to_workspace_edit()
            .to_code_action("Inline section (keep target)", "custom.inline_section_keep"),
    )
}
