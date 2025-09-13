use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn wrap_single_section() {
    assert_list(
        indoc! {"
            # test
            "},
        0,
        "- test\n",
    );
}

#[test]
fn wrap_parent_section() {
    assert_list(
        indoc! {"
            # test

            ## test2
            "},
        0,
        indoc! {"
            - test
              # test2
        "},
    );
}

#[test]
fn wrap_section_with_para() {
    assert_list(
        indoc! {"
            # test

            test2
            "},
        0,
        indoc! {"
            - test

              test2
        "},
    );
}

#[test]
fn wrap_nested_section() {
    assert_list(
        indoc! {"
            # test

            ## test2

            "},
        2,
        indoc! {"
            # test

            - test2
        "},
    );
}

#[test]
fn wrap_list_after_para_test() {
    assert_list(
        indoc! {"
            para

            # test
            "},
        2,
        indoc! {"
            para

            - test
        "},
    );
}

#[test]
fn wrap_list_after_para_with_para_test() {
    assert_list(
        indoc! {"
            para

            # test

            para2
            "},
        2,
        indoc! {"
            para

            - test

              para2
        "},
    );
}

#[test]
fn wrap_list_something() {
    assert_list(
        indoc! {"
            # test1

            para

            ## test2

            para2
            "},
        4,
        indoc! {"
            # test1

            para

            - test2

              para2
        "},
    );
}

fn assert_list(source: &str, line: u32, expected: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        uri(1).to_code_action_params(line, "refactor.rewrite.section.list"),
        vec![uri(1).to_edit(expected)]
            .to_workspace_edit()
            .to_code_action("Section to list", "refactor.rewrite.section.list"),
    )
}
