use indoc::indoc;

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
fn delete_multiple_inclusion_edges() {
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
fn delete_updates_reference_edges() {
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

#[test]
fn delete_inline_link() {
    Fixture::with(indoc! {"
        # title a

        Some text with [title b](2) link.
        _
        # title b

        some content
    "})
    .code_action(
        uri(1).to_code_action_params_at_position(2, 17, "refactor.delete"),
        vec![uri(2).to_delete_file(), uri(1).to_edit(indoc! {"
            # title a

            Some text with title b link.
        "})]
        .to_workspace_edit()
        .to_code_action("Delete", "refactor.delete"),
    );
}

#[test]
fn delete_inline_link_no_action_outside_link() {
    assert_no_delete_action_at_position(
        indoc! {"
            # title a

            Some text with [title b](2) link.
            _
            # title b

            some content
        "},
        2,
        5,
    );
}

fn assert_deleted(source: &str, line: u32, expected_edits: Vec<(u32, &str)>) {
    let mut operations = vec![uri(2).to_delete_file()];

    for (uri_num, expected_text) in expected_edits {
        operations.push(uri(uri_num).to_edit(expected_text));
    }

    Fixture::with(source).code_action(
        uri(1).to_code_action_params(line, "refactor.delete"),
        operations
            .to_workspace_edit()
            .to_code_action("Delete", "refactor.delete"),
    );
}

fn assert_no_delete_action(source: &str, line: u32) {
    Fixture::with(source).no_code_action(uri(1).to_code_action_params(line, "refactor.delete"));
}

fn assert_no_delete_action_at_position(source: &str, line: u32, character: u32) {
    Fixture::with(source)
        .no_code_action(uri(1).to_code_action_params_at_position(line, character, "refactor.delete"));
}
