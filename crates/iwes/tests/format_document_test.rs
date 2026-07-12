use diwe::config::{FormattingOptions, MarkdownOptions, RefsText};
use indoc::indoc;

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
    assert_formatted_normalized(
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
    assert_formatted_normalized(
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
fn preserve_ref_titles_by_default() {
    assert_formatted(
        indoc! {"
            # test

            [something else](2)
            _
            # new
            "},
        indoc! {"
            # test

            [something else](2)
        "},
    );
}

#[test]
fn preserve_link_titles_by_default() {
    assert_formatted(
        indoc! {"
            # test

            link to [something else](2)
            _
            # new
            "},
        indoc! {"
            # test

            link to [something else](2)
        "},
    );
}

#[test]
fn update_ref_titles_after_change() {
    assert_formatted_normalized_after_change(
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
    assert_formatted_normalized_after_change(
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
#[test]
fn format_document_not_in_graph_returns_no_edits() {
    Fixture::with("# test\n").format_document(uri(999).to_document_formatting_params(), vec![]);
}

fn assert_formatted(source: &str, formatted: &str) {
    Fixture::with(source).format_document(
        uri(1).to_document_formatting_params(),
        vec![formatted.to_text_edit_full()],
    );
}

fn assert_formatted_normalized(source: &str, formatted: &str) {
    Fixture::with_options(
        source,
        MarkdownOptions {
            refs_text: RefsText::Normalize,
            ..Default::default()
        },
    )
    .format_document(
        uri(1).to_document_formatting_params(),
        vec![formatted.to_text_edit_full()],
    );
}

fn assert_formatted_normalized_after_change(source: &str, change: &str, formatted: &str) {
    Fixture::with_options(
        source,
        MarkdownOptions {
            refs_text: RefsText::Normalize,
            ..Default::default()
        },
    )
    .did_change_text_document(uri(2).to_did_change_params(2, change.to_string()))
    .format_document(
        uri(1).to_document_formatting_params(),
        vec![formatted.to_text_edit_full()],
    );
}

fn assert_formatted_with_extension(source: &str, formatted: &str) {
    Fixture::with_options(
        source,
        diwe::config::MarkdownOptions {
            refs_extension: ".md".to_string(),
            ..Default::default()
        },
    )
    .format_document(
        uri(1).to_document_formatting_params(),
        vec![formatted.to_text_edit_full()],
    );
}

#[test]
fn format_wraps_and_preserves_breaks() {
    Fixture::with_options(
        "alpha beta gamma delta epsilon zeta eta theta\\\niota kappa lambda mu nu xi omicron pi rho\n",
        MarkdownOptions {
            formatting: FormattingOptions {
                wrap_column: Some(40),
                preserve_line_breaks: Some(true),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .format_document(
        uri(1).to_document_formatting_params(),
        vec![indoc! {"
            alpha beta gamma delta epsilon zeta eta
            theta\\
            iota kappa lambda mu nu xi omicron pi
            rho
        "}
        .to_text_edit_full()],
    );
}

#[test]
fn format_preserves_newlines() {
    Fixture::with_options(
        "first line\nsecond line\nthird line\n",
        MarkdownOptions {
            formatting: FormattingOptions {
                preserve_newlines: Some(true),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .format_document(
        uri(1).to_document_formatting_params(),
        vec![indoc! {"
            first line
            second line
            third line
        "}
        .to_text_edit_full()],
    );
}
