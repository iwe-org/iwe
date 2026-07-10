use diwe::config::{
    CompletionOptions, Configuration, LibraryOptions, LinkType, MarkdownOptions, RefsPath,
    WikiLinkPath,
};
use indoc::indoc;

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
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_test_with_refs_extension() {
    let config = Configuration {
        markdown: MarkdownOptions {
            refs_extension: ".md".to_string(),
            refs_path: Default::default(),
            date_format: None,
            time_format: None,
            locale: None,
            wiki_link_path: WikiLinkPath::Short,
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
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_relative_test() {
    Fixture::with_options_and_client(
        vec![
            (
                "dir/sub".to_string(),
                indoc! {"
                # sub-document
            "}
                .to_string(),
            ),
            (
                "top".to_string(),
                indoc! {"
                # top-level
            "}
                .to_string(),
            ),
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
                empty_range(2, 0),
            ),
            completion_item(
                "🔗 top-level",
                "[top-level](../top)",
                "top-level",
                "top-level",
                empty_range(2, 0),
            ),
        ]),
    );
}

#[test]
fn completion_absolute_test() {
    let config = Configuration {
        markdown: MarkdownOptions {
            refs_path: RefsPath::Absolute,
            ..Default::default()
        },
        completion: CompletionOptions {
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![
            (
                "dir/sub".to_string(),
                indoc! {"
                # sub-document
            "}
                .to_string(),
            ),
            (
                "top".to_string(),
                indoc! {"
                # top-level
            "}
                .to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        config,
        "",
        None,
    )
    .completion(
        uri_from("dir/sub").to_completion_params(2, 0),
        completion_list(vec![
            completion_item(
                "🔗 sub-document",
                "[sub-document](/dir/sub)",
                "sub-document",
                "sub-document",
                empty_range(2, 0),
            ),
            completion_item(
                "🔗 top-level",
                "[top-level](/top)",
                "top-level",
                "top-level",
                empty_range(2, 0),
            ),
        ]),
    );
}

#[test]
fn completion_relative_test_with_refs_extension() {
    let markdown_options = MarkdownOptions {
        refs_extension: ".html".to_string(),
        refs_path: Default::default(),
        date_format: None,
        time_format: None,
        locale: None,
        wiki_link_path: WikiLinkPath::Short,
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
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_after_file_deleted() {
    Fixture::with_options_and_client(
        vec![
            (
                "first".to_string(),
                indoc! {"
                # first-document
            "}
                .to_string(),
            ),
            (
                "second".to_string(),
                indoc! {"
                # second-document
            "}
                .to_string(),
            ),
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
                empty_range(2, 0),
            ),
            completion_item(
                "🔗 second-document",
                "[second-document](second)",
                "second-document",
                "second-document",
                empty_range(2, 0),
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
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_with_wikilink_format() {
    let config = diwe::config::Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
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
            "[[1]]",
            "test",
            "test",
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_with_wikilink_format_multiple_documents() {
    let config = diwe::config::Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![
            (
                "first".to_string(),
                indoc! {"
                # First Document
            "}
                .to_string(),
            ),
            (
                "second".to_string(),
                indoc! {"
                # Second Document
            "}
                .to_string(),
            ),
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
                empty_range(2, 0),
            ),
            completion_item(
                "🔗 Second Document",
                "[[second]]",
                "seconddocument",
                "Second Document",
                empty_range(2, 0),
            ),
        ]),
    );
}

#[test]
fn completion_with_wikilink_format_shortens_nested_key_to_bare_name() {
    let config = diwe::config::Configuration {
        markdown: MarkdownOptions {
            wiki_link_path: WikiLinkPath::Short,
            ..Default::default()
        },
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![
            ("other".to_string(), "# Other Doc\n".to_string()),
            (
                "deep/folder/target".to_string(),
                "# Target Doc\n".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        config,
        "",
        None,
    )
    .completion(
        uri_from("other").to_completion_params(2, 0),
        completion_list(vec![
            completion_item(
                "🔗 Other Doc",
                "[[other]]",
                "otherdoc",
                "Other Doc",
                empty_range(2, 0),
            ),
            completion_item(
                "🔗 Target Doc",
                "[[target]]",
                "targetdoc",
                "Target Doc",
                empty_range(2, 0),
            ),
        ]),
    );
}

#[test]
fn completion_with_markdown_format_explicit() {
    let config = diwe::config::Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::Markdown),
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
            "[test](1)",
            "test",
            "test",
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_with_wikilink_and_refs_extension() {
    let config = diwe::config::Configuration {
        markdown: MarkdownOptions {
            refs_extension: ".md".to_string(),
            refs_path: Default::default(),
            date_format: None,
            time_format: None,
            locale: None,
            wiki_link_path: WikiLinkPath::Short,
            formatting: Default::default(),
        },
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
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
            "[[1]]",
            "test",
            "test",
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_uses_frontmatter_title() {
    let config = diwe::config::Configuration {
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
            "}
            .to_string(),
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
            empty_range(5, 0),
        )]),
    );
}

#[test]
fn completion_fallback_to_header_when_frontmatter_missing() {
    let config = diwe::config::Configuration {
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
            "}
            .to_string(),
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
            empty_range(2, 0),
        )]),
    );
}

#[test]
fn completion_returns_empty_when_prefix_too_short() {
    let config = Configuration {
        completion: CompletionOptions {
            min_prefix_length: Some(3),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Test
                ab
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        config,
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
    let config = Configuration {
        completion: CompletionOptions {
            min_prefix_length: Some(3),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            # Test
            abc
        "},
        config,
    )
    .completion(
        uri(1).to_completion_params(1, 3),
        completion_list(vec![completion_item(
            "🔗 Test",
            "[Test](1)",
            "test",
            "Test",
            replace_range(1, 0, 3),
        )]),
    );
}

#[test]
fn completion_does_not_panic_on_multibyte_prefix() {
    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Test
                αβγ
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        Configuration::default(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 3),
        completion_list(vec![completion_item(
            "🔗 Test",
            "[Test](doc)",
            "test",
            "Test",
            replace_range(1, 0, 3),
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
            "}
            .to_string(),
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

#[test]
fn completion_with_bracket_prefix_replaces_typed_bracket() {
    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                [fo
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        no_min_prefix(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 3),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[Header](doc)",
            "header",
            "Header",
            replace_range(1, 0, 3),
        )]),
    );
}

#[test]
fn completion_with_double_bracket_prefix_emits_wiki_link() {
    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                [[fo
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        no_min_prefix(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 4),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[[doc]]",
            "header",
            "Header",
            replace_range(1, 0, 4),
        )]),
    );
}

#[test]
fn completion_with_double_bracket_prefix_overrides_markdown_link_format() {
    let config = Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::Markdown),
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                [[
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 2),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[[doc]]",
            "header",
            "Header",
            replace_range(1, 0, 2),
        )]),
    );
}

#[test]
fn completion_with_single_bracket_prefix_overrides_wiki_link_format() {
    let config = Configuration {
        completion: CompletionOptions {
            link_format: Some(LinkType::WikiLink),
            min_prefix_length: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                [
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 1),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[Header](doc)",
            "header",
            "Header",
            replace_range(1, 0, 1),
        )]),
    );
}

#[test]
fn completion_with_bracket_auto_pair_consumes_trailing_bracket() {
    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                []
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        no_min_prefix(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 1),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[Header](doc)",
            "header",
            "Header",
            replace_range(1, 0, 2),
        )]),
    );
}

#[test]
fn completion_with_double_bracket_auto_pair_consumes_trailing_brackets() {
    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                [[]]
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        no_min_prefix(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 2),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[[doc]]",
            "header",
            "Header",
            replace_range(1, 0, 4),
        )]),
    );
}

#[test]
fn completion_with_trailing_whitespace_after_bracket_inserts_at_cursor() {
    Fixture::with_options_and_client(
        vec![("doc".to_string(), "# Header\n[a \n".to_string())]
            .into_iter()
            .collect(),
        no_min_prefix(),
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 3),
        completion_list(vec![completion_item(
            "🔗 Header",
            "[Header](doc)",
            "header",
            "Header",
            empty_range(1, 3),
        )]),
    );
}

#[test]
fn completion_bracket_prefix_min_length_applies_to_query_only() {
    let config = Configuration {
        completion: CompletionOptions {
            min_prefix_length: Some(2),
            ..Default::default()
        },
        ..Default::default()
    };

    Fixture::with_options_and_client(
        vec![(
            "doc".to_string(),
            indoc! {"
                # Header
                [a
            "}
            .to_string(),
        )]
        .into_iter()
        .collect(),
        config,
        "",
        None,
    )
    .completion(
        uri_from("doc").to_completion_params(1, 2),
        completion_list(vec![]),
    );
}
