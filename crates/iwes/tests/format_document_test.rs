use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn basic_format() {
    assert_formatted(
        indoc! {"
            # test


            # test2
            "},
        indoc! {"
            # test

            # test2
        "},
    );
}

#[test]
fn metadata_format() {
    assert_formatted(
        indoc! {"
            ---
            key: value
            ---

            # test
            "},
        indoc! {"
            ---
            key: value
            ---

            # test
        "},
    );
}

#[test]
fn update_ref_titles() {
    assert_formatted(
        indoc! {"
            # test

            [something else](2)
            _
            # new
            "},
        indoc! {"
            # test

            [new](2)
        "},
    );
}

#[test]
fn custom_ref_text() {
    assert_formatted(
        indoc! {"
            # test

            [[2|something else]]
            _
            # new
            "},
        indoc! {"
            # test

            [[2|something else]]
        "},
    );
}

#[test]
fn format_extension() {
    assert_formatted_with_extension(
        indoc! {"
            # test

            [title](2.md)
            _
            # title
            "},
        indoc! {"
            # test

            [title](2.md)
        "},
    );
}

#[test]
fn format_extension_inline() {
    assert_formatted_with_extension(
        indoc! {"
            # test

            test [title](2.md)
            _
            # title
            "},
        indoc! {"
            # test

            test [title](2.md)
        "},
    );
}

#[test]
fn update_link_titles() {
    assert_formatted(
        indoc! {"
            # test

            link to [something else](2)
            _
            # new
            "},
        indoc! {"
            # test

            link to [new](2)
        "},
    );
}

#[test]
fn update_ref_titles_after_change() {
    assert_formatted_after_change(
        indoc! {"
            # test

            [title](2)
            _
            # title
            "},
        "# updated",
        indoc! {"
            # test

            [updated](2)
        "},
    );
}

#[test]
fn update_ref_titles_after_new_file_change() {
    assert_formatted_after_change(
        indoc! {"
            # test

            [title](2)
            "},
        "# updated",
        indoc! {"
            # test

            [updated](2)
        "},
    );
}
fn assert_formatted(source: &str, formatted: &str) {
    let fixture = Fixture::with(source);

    fixture.format_document(
        uri(1).to_document_formatting_params(),
        vec![text_edit_full(formatted)],
    )
}

fn assert_formatted_with_extension(source: &str, formatted: &str) {
    let fixture = Fixture::with_options(
        source,
        liwe::model::config::MarkdownOptions {
            refs_extension: ".md".to_string(),
            ..Default::default()
        },
    );

    fixture.format_document(
        uri(1).to_document_formatting_params(),
        vec![text_edit_full(formatted)],
    )
}

fn assert_formatted_after_change(source: &str, change: &str, formatted: &str) {
    let fixture = Fixture::with(source);

    fixture.did_change_text_document(uri(2).to_did_change_params(2, change.to_string()));

    fixture.format_document(
        uri(1).to_document_formatting_params(),
        vec![text_edit_full(formatted)],
    )
}
