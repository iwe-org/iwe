use indoc::indoc;
use lsp_types::{Position, Range};

mod fixture;
use crate::fixture::*;

#[test]
fn delete_block_reference_no_other_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
        "},
        2,
        vec![(
            1,
            indoc! {"
                # title a
            "},
        )],
    );
}

#[test]
fn delete_multiple_block_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)

            [title b](2)
            _
            # title b

            some content
        "},
        2,
        vec![(
            1,
            indoc! {"
                # title a
            "},
        )],
    );
}

#[test]
fn delete_updates_other_files() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
            _
            # title c

            [title b](2)
        "},
        2,
        vec![
            (
                1,
                indoc! {"
                # title a
            "},
            ),
            (
                3,
                indoc! {"
                # title c
            "},
            ),
        ],
    );
}

#[test]
fn delete_updates_inline_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
            _
            # title c

            inline link to [title b](2)
        "},
        2,
        vec![
            (
                1,
                indoc! {"
                # title a
            "},
            ),
            (
                3,
                indoc! {"
                # title c

                inline link to title b
            "},
            ),
        ],
    );
}

#[test]
fn delete_updates_all_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
            _
            # title c

            [title b](2)

            ## subtitle

            [title b](2)

            inline link to [title b](2)

            inline link to [title b](2) 2
        "},
        2,
        vec![
            (
                1,
                indoc! {"
                # title a
            "},
            ),
            (
                3,
                indoc! {"
                # title c

                ## subtitle

                inline link to title b

                inline link to title b 2
            "},
            ),
        ],
    );
}

#[test]
fn delete_non_block_reference_no_action() {
    assert_no_delete_action(
        indoc! {"
            # title a

            Some regular content here.
        "},
        0,
    );
}

fn assert_deleted(source: &str, line: u32, expected_edits: Vec<(u32, &str)>) {
    let fixture = Fixture::with(source);

    let mut operations = vec![uri(2).to_delete_file()];

    for (uri_num, expected_text) in expected_edits {
        operations.push(uri(uri_num).to_edit(expected_text));
    }

    fixture.code_action(
        uri(1).to_code_action_params(
            Range::new(Position::new(line, 0), Position::new(line, 0)),
            "refactor.delete",
        ),
        operations
            .to_workspace_edit()
            .to_code_action("Delete", "refactor.delete"),
    )
}

fn assert_no_delete_action(source: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.no_code_action(uri(1).to_code_action_params(
        Range::new(Position::new(line, 0), Position::new(line, 0)),
        "refactor.delete",
    ))
}
