use std::sync::Once;

use indoc::indoc;
use liwe::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use liwe::graph::Graph;

#[test]
fn normalization_single_header() {
    normalize(indoc! {"
        # header 1
        "});
}

#[test]
fn normalization_two_headers() {
    normalize(indoc! {"
        # header 1

        # header 2
        "});
}

#[test]
fn normalization_header_and_text() {
    normalize(indoc! {"
        # header-1

        item

        # header-2
        "});
}

#[test]
fn normalization_two_headers_different_levels() {
    normalize(indoc! {"
        # header 1

        ## header 2
        "});
}

#[test]
fn normalization_three_headers_different_levels() {
    normalize(indoc! {"
        # header 1

        ## header 2

        # header 1
        "});
}

#[test]
fn normalization_three_headers_different_levels_extra_level() {
    normalize_to(
        indoc! {"
        # header 1

        ## header 3

        # header 1
        "},
        indoc! {"
        # header 1

        ### header 3

        # header 1
        "},
    );
}

#[test]
fn normalization_paragraph() {
    normalize(indoc! {"
        para

        para
        "});
}

#[test]
fn normalization_paragraph_and_headers() {
    normalize(indoc! {"
        para

        # header 1

        para
        "});
}

#[test]
fn normalization_paragraph_and_nested_headers() {
    normalize(indoc! {"
        # header 1

        ## header 2

        para
        "});
}

#[test]
fn normalization_header_zero_level() {
    normalize_to(
        indoc! {"
        # header-1

        # header-2

        # header-2
        "},
        indoc! {"
        ### header-1

        ## header-2

        # header-2
        "},
    );
}

#[test]
fn normalization_headers_to_same_zero_level() {
    normalize_to(
        indoc! {"
        # header-1

        # header-2

        # header-2
        "},
        indoc! {"
        ## header-1

        ## header-2

        ## header-2
        "},
    );
}

#[test]
fn normalization_header_and_quate_zero_level() {
    normalize_to(
        indoc! {"
            # header-1

            > q

            ## header-2
            "},
        indoc! {"
            # header-1

            > q

            ## header-2
            "},
    );
}

#[test]
fn normalization_other() {
    normalize(indoc! {"
            # header

            > q

            - list

            > q

            - list
        "});
}

#[test]
fn normalization_headers_and_lists() {
    normalize(indoc! {"
            # header

            - list

            # header
        "});
}

#[test]
fn normalization_headers_and_para() {
    normalize(indoc! {"
            # header

            para

            # header

            para
        "});
}

#[test]
fn normalization_headers_2() {
    normalize(indoc! {"
            # header-1

            ## header-2

            ## header-3

            - list

            # header-4
        "});
}

#[test]
fn normalization_headers_3() {
    normalize_to(
        indoc! {"
             # header-1

             ## header-2

             ## header-3
            "},
        indoc! {"
            # header-1

            ### header-2

            ## header-3
        "},
    );
}

#[test]
fn normalization_headers_4() {
    normalize_to(
        indoc! {"
             # header-1

             ## header-2

             ## header-3
            "},
        indoc! {"
            # header-1

            ### header-2

            ### header-3
        "},
    );
}

#[test]
fn normalization_happy_path() {
    normalize(indoc! {"
             para

             # header-1

             para

             ## header-2

             parr

             # header-3

             para
            "});
}

#[test]
fn normalization_headers_5() {
    normalize(indoc! {"
        # header

        ## header-2

        para

        # header-3

        - list-item-1

        # header-4

        - list-item-2
        "});
}

#[test]
fn multiple_headers() {
    setup();
    normalize(indoc! {"
            # header-1

            ## header-2

            para-1

            ## header-3

            para-2

            ## header-4

            para-3

            ## header-5

            para-4
    "});
}

#[test]
fn empty_document() {
    normalize(indoc! {""});
}

#[test]
fn empty_line_document() {
    normalize_to(indoc! {""}, indoc! {"\n \n "});
}

fn normalize_to(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new();

    graph.from_markdown("key".into(), denormalized, MarkdownReader::new());
    let normalized = graph.to_markdown(&"key".into());

    println!("{:#?}", graph);

    println!("denormalized:");
    println!("{}", denormalized);
    println!();

    if expected != denormalized {
        println!("expected:");
        println!("{}", expected);
        println!();
    }

    println!("normalized:");
    println!("{}", normalized);
    println!();

    assert_str_eq!(expected, normalized);
}

fn normalize(text: &str) {
    normalize_to(text, text)
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
