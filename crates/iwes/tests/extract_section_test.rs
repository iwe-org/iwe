use indoc::indoc;
use lsp_types::{CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier};

mod fixture;
use crate::fixture::*;

#[test]
fn to_level_extract_not_allowed() {
    assert_no_action(
        indoc! {"
            # test
            "},
        0,
    );

    assert_no_action(
        indoc! {"
            # test

            # test
            "},
        2,
    );
}

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
fn extract_section() {
    assert_extracted(
        indoc! {"
            # test

            ## test2
            "},
        2,
        indoc! {"
            # test

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_helix_section() {
    assert_extracted_helix(
        indoc! {"
            # test

            ## test2
            "},
        2,
        indoc! {"
            # test

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_middle_section_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test1

            ## test2

            ## test3
        "},
        4,
        indoc! {"
            # test

            [test2](2)

            ## test1

            ## test3
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_after_list() {
    assert_extracted(
        indoc! {"
            # test

            - item1

            ## test2

            - item2
            "},
        4,
        indoc! {"
            # test

            - item1

            [test2](2)
            "},
        indoc! {"
            # test2

            - item2
        "},
    );
}

#[test]
fn extract_after_para() {
    assert_extracted(
        indoc! {"
            # test

            para1

            ## test2
            "},
        4,
        indoc! {"
            # test

            para1

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_third_level_section_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test2

            ### test3
            "},
        4,
        indoc! {"
            # test

            ## test2

            [test3](2)
            "},
        indoc! {"
            # test3
        "},
    );
}

#[test]
fn extract_one_of_sub_level_section() {
    assert_extracted(
        indoc! {"
            # test

            para

            ## test2

            - item

            ## test3

            - item
            "},
        4,
        indoc! {"
            # test

            para

            [test2](2)

            ## test3

            - item
            "},
        indoc! {"
            # test2

            - item
        "},
    );
}

#[test]
fn test_extracted_relative() {
    let fixture = Fixture::with_documents(vec![(
        "d/1",
        indoc! {"
        # test

        ## target"},
    )]);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: uri_from("d/1"),
            },
            range: Range::new(Position::new(2, 0), Position::new(2, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.extract.section"),
                trigger_kind: None,
            },
        },
        vec![
            uri_from("d/2").to_create_file(),
            uri_from("d/2").to_edit("# target\n"),
            uri_from("d/1").to_edit("# test\n\n[target](2)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "refactor.extract.section"),
    )
}

fn assert_extracted(source: &str, line: u32, target: &str, extracted: &str) {
    let fixture = Fixture::with(source);
    let action = vec![
        uri(2).to_create_file(),
        uri(2).to_edit(extracted),
        uri(1).to_edit(target),
    ]
    .to_workspace_edit()
    .to_code_action("Extract section", "refactor.extract.section");

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
        action,
    );
}

fn assert_extracted_helix(source: &str, line: u32, target: &str, extracted: &str) {
    let fixture = Fixture::with_client(source, "helix");

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 1)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.extract.section"),
                trigger_kind: None,
            },
        },
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "refactor.extract.section"),
    )
}
fn assert_no_action(source: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.no_code_action(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(line, 0), Position::new(line, 0)),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: CodeActionContext {
            diagnostics: Default::default(),
            only: action_kinds("refactor.extract.section"),
            trigger_kind: None,
        },
    })
}
