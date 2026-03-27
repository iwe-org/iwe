use indoc::indoc;
use liwe::model::config::{CompletionOptions, LibraryOptions, LinkType, MarkdownOptions};

mod fixture;
use crate::fixture::*;

#[test]
fn completion_test() {
    Fixture::with(indoc! {"
            # test
            "})
    .completion(
        uri(1).to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 test",
            "[test](1)",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_test_with_refs_extension() {
    let markdown_options = MarkdownOptions {
        refs_extension: ".md".to_string(),
        date_format: None,
        locale: None,
    };

    Fixture::with_options(
        indoc! {"
            # test
            "},
        markdown_options,
    )
    .completion(
        uri(1).to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 test",
            "[test](1.md)",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_relative_test() {
    Fixture::with_documents(vec![
        (
            "dir/sub",
            indoc! {"
            # sub-document
            "},
        ),
        (
            "top",
            indoc! {"
                # top-level
                "},
        ),
    ])
    .completion(
        uri_from("dir/sub").to_completion_params(2, 0),
        completion_list(vec![
            completion_item(
                "🔗 sub-document",
                "[sub-document](sub)",
                "sub-document",
                "sub-document",
            ),
            completion_item(
                "🔗 top-level",
                "[top-level](../top)",
                "top-level",
                "top-level",
            ),
        ]),
    );
}

#[test]
fn completion_relative_test_with_refs_extension() {
    let markdown_options = MarkdownOptions {
        refs_extension: ".html".to_string(),
        date_format: None,
        locale: None,
    };

    let config = liwe::model::config::Configuration {
        markdown: markdown_options,
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            # test
            "},
        config,
    )
    .completion(
        uri(1).to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 test",
            "[test](1.html)",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_after_file_deleted() {
    Fixture::with_documents(vec![
        (
            "first",
            indoc! {"
            # first-document
            "},
        ),
        (
            "second",
            indoc! {"
            # second-document
            "},
        ),
    ])
    .completion(
        uri_from("first").to_completion_params(2, 0),
        completion_list(vec![
            completion_item(
                "🔗 first-document",
                "[first-document](first)",
                "first-document",
                "first-document",
            ),
            completion_item(
                "🔗 second-document",
                "[second-document](second)",
                "second-document",
                "second-document",
            ),
        ]),
    )
    .did_delete_files(uri_from("second").to_file_delete_params())
    .completion(
        uri_from("first").to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 first-document",
            "[first-document](first)",
            "first-document",
            "first-document",
        )]),
    );
}

#[test]
fn completion_with_wikilink_format() {
    let config = liwe::model::config::Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
        },
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            # test
            "},
        config,
    )
    .completion(
        uri(1).to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 test",
            "[[1]]",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_with_wikilink_format_multiple_documents() {
    let config = liwe::model::config::Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![
            ("first".to_string(), "# First Document\n".to_string()),
            ("second".to_string(), "# Second Document\n".to_string()),
        ]
        .into_iter()
        .collect(),
        config,
        "",
    )
    .completion(
        uri_from("first").to_completion_params(2, 0),
        completion_list(vec![
            completion_item(
                "🔗 First Document",
                "[[first]]",
                "firstdocument",
                "First Document",
            ),
            completion_item(
                "🔗 Second Document",
                "[[second]]",
                "seconddocument",
                "Second Document",
            ),
        ]),
    );
}

#[test]
fn completion_with_markdown_format_explicit() {
    let config = liwe::model::config::Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::Markdown),
        },
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            # test
            "},
        config,
    )
    .completion(
        uri(1).to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 test",
            "[test](1)",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_with_wikilink_and_refs_extension() {
    let config = liwe::model::config::Configuration {
        markdown: MarkdownOptions {
            refs_extension: ".md".to_string(),
            date_format: None,
            locale: None,
        },
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
        },
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            # test
            "},
        config,
    )
    .completion(
        uri(1).to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 test",
            "[[1]]",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_uses_frontmatter_title() {
    let config = liwe::model::config::Configuration {
        library: LibraryOptions {
            frontmatter_document_title: Some("title".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            "---\ntitle: Custom Title\n---\n\n# Header\n".to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
    )
    .completion(
        uri_from("doc").to_completion_params(5, 0),
        completion_list(vec![completion_item(
            "🔗 Custom Title",
            "[Custom Title](doc)",
            "customtitle",
            "Custom Title",
        )]),
    );
}

#[test]
fn completion_fallback_to_header_when_frontmatter_missing() {
    let config = liwe::model::config::Configuration {
        library: LibraryOptions {
            frontmatter_document_title: Some("title".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![("doc".to_string(), "# Header Title\n".to_string())]
            .into_iter()
            .collect(),
        config,
        "",
    )
    .completion(
        uri_from("doc").to_completion_params(2, 0),
        completion_list(vec![completion_item(
            "🔗 Header Title",
            "[Header Title](doc)",
            "headertitle",
            "Header Title",
        )]),
    );
}
