use indoc::indoc;
use lsp_types::{CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier};

mod fixture;
use crate::fixture::*;

#[test]
fn change_to_ordered() {
    assert_list_change(
        indoc! {"
            - test
            - test2
            "},
        0,
        indoc! {"
            1.  test
            2.  test2
        "},
        "Change to ordered list",
    );
}

#[test]
fn change_to_bullet() {
    assert_list_change(
        indoc! {"
            1.  test
            2.  test2
            "},
        0,
        indoc! {"
            - test
            - test2
        "},
        "Change to bullet list",
    );
}

fn assert_list_change(source: &str, line: u32, expected: &str, title: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.rewrite.list.type"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        vec![uri(1).to_edit(expected)]
            .to_workspace_edit()
            .to_code_action(title, "refactor.rewrite.list.type"),
    )
}
