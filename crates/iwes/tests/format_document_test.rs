use std::u32;

use indoc::indoc;
use lsp_types::{
    DidChangeTextDocumentParams, DocumentFormattingParams, Position, Range,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextEdit,
    VersionedTextDocumentIdentifier,
};

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

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
        DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            options: Default::default(),
            work_done_progress_params: Default::default(),
        },
        vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: formatted.to_string(),
        }],
    )
}

fn assert_formatted_with_extension(source: &str, formatted: &str) {
    let fixture = Fixture::with_options(
        source,
        liwe::model::config::MarkdownOptions {
            refs_extension: ".md".to_string(),
        },
    );

    fixture.format_document(
        DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            options: Default::default(),
            work_done_progress_params: Default::default(),
        },
        vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: formatted.to_string(),
        }],
    )
}

fn assert_formatted_after_change(source: &str, change: &str, formatted: &str) {
    let fixture = Fixture::with(source);

    fixture.did_change_text_document(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri(2),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: change.to_string(),
        }],
    });

    fixture.format_document(
        DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            options: Default::default(),
            work_done_progress_params: Default::default(),
        },
        vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: formatted.to_string(),
        }],
    )
}
