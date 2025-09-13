use indoc::indoc;
use lsp_types::{Position, Range};

mod fixture;
use crate::fixture::*;

#[test]
fn unwrap_single_item_list_test() {
    assert_sections(
        indoc! {"
            - test
            "},
        0,
        "# test\n",
    );
}

#[test]
fn keep_frontmatter() {
    assert_sections(
        indoc! {"
            ---
            frontmatter: true
            ---

            - test
            "},
        4,
        indoc! {"
            ---
            frontmatter: true
            ---

            # test
            "},
    );
}

#[test]
fn unwrap_list_with_items_test() {
    assert_sections(
        indoc! {"
            - test
              - test2
            "},
        0,
        indoc! {"
            # test

            - test2
        "},
    );
}

#[test]
fn unwrap_list_takes_top_level_list() {
    assert_sections(
        indoc! {"
            - test
              - test2
            "},
        1,
        indoc! {"
            # test

            - test2
        "},
    );
}

#[test]
fn unwrap_list_after_para_test() {
    assert_sections(
        indoc! {"
            para

            - test
            "},
        2,
        indoc! {"
            para

            # test
        "},
    );
}

#[test]
fn unwrap_list_between_para_and_para_test() {
    assert_sections(
        indoc! {"
            para

            - test

            para2
            "},
        2,
        indoc! {"
            para

            # test

            para2
        "},
    );
}

#[test]
fn unwrap_list_with_items_after_para_test() {
    assert_sections(
        indoc! {"
            para

            - test
              - test2
            "},
        2,
        indoc! {"
            para

            # test

            - test2
        "},
    );
}

#[test]
fn unwrap_sub_list_test() {
    assert_sections(
        indoc! {"
            # test

            - test2
            "},
        2,
        indoc! {"
            # test

            ## test2
        "},
    );
}

#[test]
fn unwrap_middle_list_test() {
    assert_sections(
        indoc! {"
            # test

            - test2

            # test3
            "},
        2,
        indoc! {"
            # test

            ## test2

            # test3
        "},
    );
}

#[test]
fn unwrap_list_prior_to_level_two_header_test() {
    assert_sections(
        indoc! {"
            # test

            - test2

            ## test3
            "},
        2,
        indoc! {"
            # test

            ## test2

            ## test3
        "},
    );
}

fn assert_sections(source: &str, line: u32, expected: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        uri(1).to_code_action_params(
            Range::new(Position::new(line, 0), Position::new(line, 0)),
            "refactor.rewrite.list.section",
        ),
        vec![uri(1).to_edit(expected)]
            .to_workspace_edit()
            .to_code_action("List to sections", "refactor.rewrite.list.section"),
    )
}
