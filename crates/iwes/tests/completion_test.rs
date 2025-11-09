use indoc::indoc;
use liwe::model::config::MarkdownOptions;

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
