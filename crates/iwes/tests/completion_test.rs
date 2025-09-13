use indoc::indoc;

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
