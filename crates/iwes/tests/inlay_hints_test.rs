use indoc::indoc;

mod fixture;
use crate::fixture::*;

#[test]
fn single_ref() {
    assert_inlay_hints(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            "},
        "↖header hint",
    );
}

#[test]
fn non_existent_ref() {
    assert_no_hints(
        indoc! {"
            # test

            [test](test)
            "},
        "1",
    );
}

#[test]
fn non_existent_key() {
    assert_no_hints(
        indoc! {"
        "},
        "not-a-key",
    );
}

#[test]
fn single_multiple_refs_from_same_key() {
    assert_inlay_hints(
        indoc! {"
            # test
            _
            # header hint

            [test](1)

            [test](1)

            "},
        "↖header hint",
    );
}

#[test]
fn no_refs() {
    assert_no_hints(
        indoc! {"
            # test
            _
            # header hint
            "},
        "1",
    );
}

#[test]
fn multiple_refs() {
    assert_multiple_hints(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            _
            # header hint 2

            [test](1)
            "},
        "↖header hint",
        "↖header hint 2",
    );
}

#[test]
fn block_reference() {
    assert_no_hints(
        indoc! {"
            para

            [test](2)
            _
            # test
            "},
        "1",
    );
}

#[test]
fn block_reference_2() {
    assert_inlay_hint_at(
        indoc! {"
            para

            [test](2)
            _
            # test
            _
            # from

            [test](2)
            "},
        "↖from",
        2,
    );
}

#[test]
fn block_reference_multiple_from_the_same_key() {
    assert_inlay_hint_at(
        indoc! {"
            para

            [test](2)
            _
            # test
            _
            # from

            [test](2)

            [test](2)
            "},
        "↖from",
        2,
    );
}

fn assert_inlay_hint_at(source: &str, hint_text: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        uri(1).to_inlay_hint_params(),
        vec![inlay_hint(hint_text, line, 120)],
    )
}

fn assert_inlay_hints(source: &str, hint_text: &str) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        uri(1).to_inlay_hint_params(),
        vec![inlay_hint(hint_text, 0, 120)],
    )
}

fn assert_no_hints(source: &str, key: &str) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(uri_from(key).to_inlay_hint_params(), vec![])
}

fn assert_multiple_hints(source: &str, hint_text: &str, hint_text2: &str) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        uri(1).to_inlay_hint_params(),
        vec![
            inlay_hint(hint_text, 0, 120),
            inlay_hint(hint_text2, 0, 120),
        ],
    )
}
