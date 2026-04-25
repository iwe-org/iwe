use indoc::indoc;
use liwe::model::config::{
    CompletionOptions, Configuration, LibraryOptions, LinkType, MarkdownOptions,
};

mod fixture;
use crate::fixture::*;

fn no_min_prefix() -> Configuration {
    Configuration {
        completion: CompletionOptions {
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn completion_test() {
    Fixture::with_config(
        indoc! {"
            # test
            "},
        no_min_prefix(),
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
fn completion_test_with_refs_extension() {
    let config = Configuration {
        markdown: MarkdownOptions {
            refs_extension: ".md".to_string(),
            date_format: None,
            time_format: None,
            locale: None,
            formatting: Default::default(),
        },
        completion: CompletionOptions {
            min_prefix_length: Some(0),
            ..Default::default()
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
            "[test](1.md)",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_relative_test() {
    Fixture::with_options_and_client(
        vec![
            ("dir/sub".to_string(), indoc! {"
                # sub-document
            "}.to_string()),
            ("top".to_string(), indoc! {"
                # top-level
            "}.to_string()),
        ]
        .into_iter()
        .collect(),
        no_min_prefix(),
        "",
        None,
    )
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
        time_format: None,
        locale: None,
        formatting: Default::default(),
    };

    let config = Configuration {
        markdown: markdown_options,
        completion: CompletionOptions {
            min_prefix_length: Some(0),
            ..Default::default()
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
            "[test](1.html)",
            "test",
            "test",
        )]),
    );
}

#[test]
fn completion_after_file_deleted() {
    Fixture::with_options_and_client(
        vec![
            ("first".to_string(), indoc! {"
                # first-document
            "}.to_string()),
            ("second".to_string(), indoc! {"
                # second-document
            "}.to_string()),
        ]
        .into_iter()
        .collect(),
        no_min_prefix(),
        "",
        None,
    )
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
            min_prefix_length: Some(0),
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
            min_prefix_length: Some(0),
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![
            ("first".to_string(), indoc! {"
                # First Document
            "}.to_string()),
            ("second".to_string(), indoc! {"
                # Second Document
            "}.to_string()),
        ]
        .into_iter()
        .collect(),
        config,
        "",
        None,
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
            min_prefix_length: Some(0),
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
            time_format: None,
            locale: None,
            formatting: Default::default(),
        },
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
            min_prefix_length: Some(0),
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
        completion: CompletionOptions {
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                ---
                title: Custom Title
                ---

                # Header
            "}.to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
        None,
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
        completion: CompletionOptions {
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header Title
            "}.to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
        None,
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

#[test]
fn completion_returns_empty_when_prefix_too_short() {
    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Test
                ab
            "}.to_string(),
        )]
        .into_iter()
        .collect(),
        Configuration::default(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 2),
        completion_list(vec![]),
    );
}

#[test]
fn completion_returns_results_when_prefix_long_enough() {
    Fixture::with_config(
        indoc! {"
            # Test
            abc
        "},
        Configuration::default(),
    )
    .completion(
        uri(1).to_completion_params(1, 3),
        completion_list(vec![completion_item(
            "🔗 Test",
            "[Test](1)",
            "test",
            "Test",
        )]),
    );
}

#[test]
fn completion_respects_custom_min_prefix_length() {
    let config = Configuration {
        completion: CompletionOptions {
            min_prefix_length: Some(5),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Test
                abcd
            "}.to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 4),
        completion_list(vec![]),
    );
}
