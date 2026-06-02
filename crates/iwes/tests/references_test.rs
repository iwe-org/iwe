use indoc::indoc;

use crate::fixture::*;

#[test]
fn single_reference() {
    Fixture::with(indoc! {"
        # doc1

        [target](3)
        _
        # doc2

        [target](3)
        _
        # target
        "})
    .references(
        uri(1).to_reference_params(2, 1, false),
        vec![uri(2).to_location(2, 3)],
    );
}

#[test]
fn two_references() {
    Fixture::with(indoc! {"
        # doc1

        [target](4)
        _
        # doc2

        [target](4)
        _
        # doc3

        [target](4)
        _
        # target
        "})
    .references(
        uri(1).to_reference_params(2, 1, false),
        vec![uri(2).to_location(2, 3), uri(3).to_location(2, 3)],
    );
}

#[test]
fn link() {
    Fixture::with(indoc! {"
        # header 1

        text and link [target](2)
        _
        # target
        "})
    .references(uri(1).to_reference_params(2, 15, false), vec![]);
}

#[test]
fn wiki_link_reference_resolves_target_in_another_directory() {
    Fixture::with_documents(vec![
        ("first/note", "[[target]]\n"),
        ("third/note", "[[target]]\n"),
        ("second/target", "# target\n"),
    ])
    .references(
        uri_from("first/note").to_reference_params(0, 3, false),
        vec![uri_from("third/note").to_location(0, 1)],
    );
}
