use indoc::indoc;
use lsp_types::{request::HoverRequest, *};
use std::collections::HashMap;

use crate::fixture::*;

#[test]
fn hover_preview_for_wiki_link_strips_frontmatter() {
    Fixture::with_documents(vec![
        (
            "1",
            indoc! {
                "text [[2]] text"
            },
        ),
        (
            "2",
            indoc! {"
                ---
                title: Note Two
                ---
                # Heading
                Line 2
            "},
        ),
    ])
    .assert_response::<lsp_types::request::HoverRequest>(
        uri(1).to_hover_params(0, 7),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Heading\n\nLine 2\n".to_string(),
            }),
            range: None,
        }),
    );
}

#[test]
fn hover_preview_for_markdown_link() {
    let state = HashMap::from([
        (
            "1".to_string(),
            indoc! {
                "text [two](2) text"
            }
            .to_string(),
        ),
        (
            "2".to_string(),
            indoc! {"
                # Heading
                Line 2
            "}
            .to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .assert_response::<HoverRequest>(
            uri(1).to_hover_params(0, 7),
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "# Heading\n\nLine 2\n".to_string(),
                }),
                range: None,
            }),
        );
}

#[test]
fn hover_outside_link_returns_none() {
    Fixture::with_documents(vec![("1", "no links here\n"), ("2", "# Heading\n")])
        .assert_response::<HoverRequest>(uri(1).to_hover_params(0, 0), None);
}

#[test]
fn hover_missing_target_returns_none() {
    Fixture::with_documents(vec![("1", "text [[missing]] text\n")])
        .assert_response::<HoverRequest>(uri(1).to_hover_params(0, 7), None);
}

#[test]
fn hover_wiki_link_after_multibyte_text() {
    Fixture::with_documents(vec![
        ("1", "\u{03B1}\u{03B2} [[2]] text\n"),
        ("2", "# Heading\n\nLine 2\n"),
    ])
    .assert_response::<HoverRequest>(
        uri(1).to_hover_params(0, 4),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Heading\n\nLine 2\n".to_string(),
            }),
            range: None,
        }),
    );
}

#[test]
fn hover_wiki_link_after_emoji() {
    Fixture::with_documents(vec![
        ("1", "\u{1F5FA} [[2]] text\n"),
        ("2", "# Heading\n\nLine 2\n"),
    ])
    .assert_response::<HoverRequest>(
        uri(1).to_hover_params(0, 7),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Heading\n\nLine 2\n".to_string(),
            }),
            range: None,
        }),
    );
}

#[test]
fn hover_outside_link_with_multibyte_text() {
    Fixture::with_documents(vec![
        ("1", "\u{03B1}\u{03B2} [[2]] text\n"),
        ("2", "# Heading\n"),
    ])
    .assert_response::<HoverRequest>(uri(1).to_hover_params(0, 1), None);
}

#[test]
fn hover_wiki_link_resolves_target_in_another_directory() {
    Fixture::with_documents(vec![
        ("first/note", "[[target]]\n"),
        ("second/target", "# Heading\n\nLine 2\n"),
    ])
    .assert_response::<HoverRequest>(
        uri_from("first/note").to_hover_params(0, 3),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Heading\n\nLine 2\n".to_string(),
            }),
            range: None,
        }),
    );
}

#[test]
fn hover_markdown_link_resolves_relative_to_current_directory() {
    Fixture::with_documents(vec![
        ("first/note", "[target](target)\n"),
        ("first/target", "# Sibling\n"),
        ("second/target", "# Other\n"),
    ])
    .assert_response::<HoverRequest>(
        uri_from("first/note").to_hover_params(0, 3),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Sibling\n".to_string(),
            }),
            range: None,
        }),
    );
}
