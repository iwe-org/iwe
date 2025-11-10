use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn basic_prepare_rename() {
    assert_prepare_rename(
        indoc! {"
            [text text](key)
            "},
        "key",
    );
}

#[test]
fn basic_rename() {
    assert_rename(
        indoc! {"
            [link text](1)
            _
            # file 2
            "},
        indoc! {"
            [link text](new_name)
        "},
    );
}

#[test]
fn rename_to_an_existing_key() {
    assert_rename_error(
        indoc! {"
            [](1)
            _
            # file 2
            "},
        "The file name 2 is already taken",
        lsp_types::Position::new(0, 0),
        "2",
    );
}

#[test]
fn rename_both_references() {
    assert_rename(
        indoc! {"
            [first link](1)

            [second link](1)
            _
            # file 2
            "},
        indoc! {"
            [first link](new_name)

            [second link](new_name)
        "},
    );
}

#[test]
fn rename_updates_affected_files() {
    assert_rename_updates_second_file(
        indoc! {"
            [my link](1)
            _
            # file 2

            [another reference](1)
            "},
        indoc! {"
            [my link](new_name)
        "},
        indoc! {"
            # file 2

            [another reference](new_name)
        "},
    );
}

#[test]
fn rename_inline_references() {
    assert_rename_at(
        indoc! {"
            # title

            [inline link](1) text
            "},
        indoc! {"
            # title

            [title](new_name) text
        "},
        lsp_types::Position::new(2, 0),
        "new_name",
    );
}

#[test]
fn rename_with_empty_link_text() {
    assert_rename(
        indoc! {"
            [](1)
            _
            # file 2
            "},
        indoc! {"
            [](new_name)
        "},
    );
}

fn assert_prepare_rename(source: &str, _: &str) {
    Fixture::with(source).prepare_rename(
        uri(1).to_text_document_position_params(0, 0),
        prepare_rename_response(
            lsp_types::Range::new(
                lsp_types::Position::new(0, 12),
                lsp_types::Position::new(0, 15),
            ),
            "key".to_string(),
        ),
    );
}
fn assert_rename(source: &str, expected: &str) {
    assert_rename_at(source, expected, lsp_types::Position::new(0, 0), "new_name");
}

fn assert_rename_at(source: &str, expected: &str, position: lsp_types::Position, new_name: &str) {
    let new_uri = uri_from(new_name);

    Fixture::with(source).rename(
        uri(1).to_rename_params(position.line, position.character, new_name.to_string()),
        vec![
            uri(1).to_delete_file(),
            new_uri.clone().to_create_file(),
            new_uri.to_edit_with_range(
                expected,
                lsp_types::Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(0, 0),
                ),
            ),
        ]
        .to_workspace_edit(),
    );
}

fn assert_rename_error(
    source: &str,
    expected: &str,
    position: lsp_types::Position,
    new_name: &str,
) {
    Fixture::with(source).rename_err(
        uri(1).to_rename_params(position.line, position.character, new_name.to_string()),
        response_error(1, expected.to_string()),
    );
}

fn assert_rename_updates_second_file(source: &str, expected1: &str, expected2: &str) {
    let new_uri = uri_from("new_name");

    Fixture::with(source).rename(
        uri(1).to_rename_params(0, 0, "new_name".to_string()),
        vec![
            uri(2).to_edit(expected2),
            uri(1).to_delete_file(),
            new_uri.clone().to_create_file(),
            new_uri.to_edit_with_range(
                expected1,
                lsp_types::Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(0, 0),
                ),
            ),
        ]
        .to_workspace_edit(),
    );
}
