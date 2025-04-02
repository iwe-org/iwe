use std::sync::Once;

use indoc::indoc;
use liwe::graph::{Graph, GraphContext};
use liwe::model::config::MarkdownOptions;

#[test]
fn squash_text() {
    squash(
        indoc! {"
            [](2)
            _
            text
            "},
        indoc! {"
            text
            "},
    );
}

#[test]
fn squash_invalid_ref_text() {
    squash(
        indoc! {"
            [](2)
            "},
        indoc! {"
            [](2)
            "},
    );
}

#[test]
fn squash_after_text() {
    squash(
        indoc! {"
            text 1

            [](2)

            _
            text 2
            "},
        indoc! {"
            text 1

            text 2
            "},
    );
}

#[test]
fn squash_before_text() {
    squash(
        indoc! {"
            [](2)

            text 1
            _
            text 2
            "},
        indoc! {"
            text 2

            text 1
            "},
    );
}

#[test]
fn squash_no_text() {
    squash(
        indoc! {"
            # file 1 title

            [file 2 title](2)

            _
            # file 2 title
            "},
        indoc! {"
            # file 1 title

            ## file 2 title
            "},
    );
}

#[test]
fn squash_single_child_with_text() {
    squash(
        indoc! {"
            # file 1 title

            [file 2 title](2)

            _
            # file 2 title

            text 2
            "},
        indoc! {"
            # file 1 title

            ## file 2 title

            text 2
            "},
    );
}

#[test]
fn squash_single_next_with_text() {
    squash(
        indoc! {"
            # file 1 title

            text

            [file 2 title](2)

            _
            # file 2 title

            text
            "},
        indoc! {"
            # file 1 title

            text

            ## file 2 title

            text
            "},
    );
}

#[test]
fn squash_two_refs() {
    squash(
        indoc! {"
            [](2)

            [](3)
            _
            # title 1
            _
            # title 2
            "},
        indoc! {"
            # title 1

            # title 2
            "},
    );
}

#[test]
fn squash_depth_counter() {
    squash(
        indoc! {"
            # title 1

            [title 2](2)

            text

            [title 2](2)

            _
            # title 2

            [title 3](3)

            _
            # title 3
            "},
        indoc! {"
            # title 1

            ## title 2

            ### title 3

            text

            ## title 2

            ### title 3
            "},
    );
}

#[test]
fn squash_three_refs() {
    squash(
        indoc! {"
            [](2)

            [](3)

            [](4)
            _
            # title 1
            _
            # title 2
            _
            # title 3
            "},
        indoc! {"
            # title 1

            # title 2

            # title 3
            "},
    );
}

#[test]
fn squash_two_levels() {
    squash(
        indoc! {"
            # file 1 title

            [file 2 title](2)

            _
            # file 2 title

            text 2

            [file 3 title](3)
            _
            # file 3 title
            "},
        indoc! {"
            # file 1 title

            ## file 2 title

            text 2

            ### file 3 title
            "},
    );
}

#[test]
fn squash_infinite_recursion() {
    squash(
        indoc! {"
            # file 1 title

            text 1

            [file 2 title](2)

            _
            # file 2 title

            text 2

            [file 1 title](1)
            "},
        indoc! {"
            # file 1 title

            text 1

            ## file 2 title

            text 2

            ### file 1 title

            text 1

            [file 2 title](2)
            "},
    );
}

fn squash(source: &str, expected: &str) {
    setup();

    let graph = &Graph::import(
        &source
            .split("\n_\n")
            .enumerate()
            .map(|(index, text)| (format!("{}", index + 1), text.trim().to_string()))
            .collect(),
        MarkdownOptions::default(),
    );
    let mut patch = Graph::new();
    let document_tree = graph.squash(&"1".into(), 2);
    patch.build_key_from_iter(&"1".into(), document_tree.iter());

    eprintln!("graph {:#?}", graph);
    eprintln!("patch {:#?}", patch);

    assert_eq!(expected, patch.export_key(&"1".into()).unwrap());
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
