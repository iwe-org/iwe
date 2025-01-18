use std::sync::Once;

use indoc::indoc;
use liwe::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use liwe::graph::Graph;

#[test]
fn normalization_one_paragraph() {
    compare(
        indoc! {"
        para
        "},
        indoc! {"
        para

        "},
    );
}

#[test]
fn normalization_url() {
    compare(
        indoc! {"
        <http://some.com>
        "},
        indoc! {"
        <http://some.com>
        "},
    );
}

#[test]
fn normalization_dashes() {
    compare(
        indoc! {"
        text---text

        text--text
        "},
        indoc! {"
        text---text

        text--text
        "},
    );
}

#[test]
fn normalization_style() {
    compare(
        indoc! {"
        **bold**

        *em*

        ~~strike~~
        "},
        indoc! {"
        **bold**

        *em*

        ~~strike~~
        "},
    );
}

#[test]
fn normalization_two_paragraphs() {
    compare(
        indoc! {"
        para

        para 2
        "},
        indoc! {"
        para


        para 2
        "},
    );
}

#[test]
fn normalization_excaping() {
    compare(
        indoc! {"
        item it'q
        "},
        indoc! {"
        item it'q
        "},
    );
}

#[test]
fn normalization_math() {
    compare(
        indoc! {"
        $a$
        "},
        indoc! {"
        $a$
        "},
    );
}

#[test]
fn normalization_quoted() {
    setup();
    compare(
        indoc! {"
        > quote
        "},
        indoc! {"
        > quote
        "},
    );
}

#[test]
fn github_notes() {
    setup();
    compare(
        indoc! {"
        > [!NOTE]
        >
        > note text

        para
        "},
        indoc! {"
        > [!NOTE]
        >
        > note text

        para
        "},
    );
}

#[test]
fn normalization_rule() {
    setup();
    compare(
        indoc! {"
        ------------------------------------------------------------------------
        "},
        indoc! {"
        ---
        "},
    );
}

#[test]
fn rule_and_text() {
    setup();
    compare(
        indoc! {"
        ------------------------------------------------------------------------

        text
        "},
        indoc! {"
        ------------------------------------------------------------------------

        text
        "},
    );
}

#[test]
fn header_line_header() {
    setup();
    compare(
        indoc! {"
            # header

            ------------------------------------------------------------------------

            # header 2
        "},
        indoc! {"
            # header

            ------------------------------------------------------------------------

            # header 2
        "},
    );
}

#[test]
fn one_quote() {
    setup();
    compare(
        indoc! {"
        > q1
    "},
        indoc! {"
        > q1
    "},
    );
}

#[test]
fn definitions_list() {
    setup();
    compare(
        indoc! {"
            test [^1]

            [^1]: some link
    "},
        indoc! {"
            test [^1]

            [^1]: some link
    "},
    );
}

fn compare(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new();

    graph.from_markdown("key", denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown("key");

    println!("actual graph \n{:#?}", graph);
    println!("{}", expected);
    println!("normalized:");
    println!("{}", normalized);

    assert_str_eq!(expected, normalized);
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
