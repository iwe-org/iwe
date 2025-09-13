use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn no_sub_sections_to_extract() {
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

    assert_no_action(
        indoc! {"
            # test

            ## test
            "},
        2,
    );
}

#[test]
fn extract_sub_section_after_para_test() {
    assert_extracted(
        indoc! {"
            # test

            para

            ## test2
            "},
        0,
        indoc! {"
            # test

            para

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_sub_sections_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test2
            "},
        0,
        indoc! {"
            # test

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

fn assert_extracted(source: &str, line: u32, target: &str, extracted: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        uri(1).to_code_action_params(line, "refactor.extract.subsections"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract sub-sections", "refactor.extract.subsections"),
    )
}

fn assert_no_action(source: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.no_code_action(uri(1).to_code_action_params(line, "refactor.extract.subsections"))
}
