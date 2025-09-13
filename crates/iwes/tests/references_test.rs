use indoc::indoc;

mod fixture;
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
