use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn single_reference() {
    let fixture = Fixture::with(indoc! {"
        # doc1

        [target](3)
        _
        # doc2

        [target](3)
        _
        # target
        "});

    fixture.references(
        uri(1).to_reference_params(2, 1, false),
        vec![location(uri(2), 2, 3)],
    );
}

#[test]
fn two_references() {
    let fixture = Fixture::with(indoc! {"
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
        "});

    fixture.references(
        uri(1).to_reference_params(2, 1, false),
        vec![location(uri(2), 2, 3), location(uri(3), 2, 3)],
    );
}

#[test]
fn link() {
    let fixture = Fixture::with(indoc! {"
        # header 1

        text and link [target](2)
        _
        # target
        "});

    fixture.references(uri(1).to_reference_params(2, 15, false), vec![]);
}
