use indoc::indoc;

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
        uri(1).to_code_action_params(line, "refactor.rewrite.list.type"),
        vec![uri(1).to_edit(expected)]
            .to_workspace_edit()
            .to_code_action(title, "refactor.rewrite.list.type"),
    )
}
