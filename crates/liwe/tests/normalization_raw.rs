use std::sync::Once;

use indoc::indoc;
use liwe::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use liwe::graph::Graph;

#[test]
fn normalization_code_block() {
    compare(
        indoc! {"
        ``` rust
        fn main() {
            println!(\"Hello, world!\");
        }
        ```
        "},
        indoc! {"
        ``` rust

        fn main() {
            println!(\"Hello, world!\");
        }
        ```
        "},
    );
}

#[test]
fn normalization_raw_inline() {
    compare(
        indoc! {"
        `<buffer>`{=html}
        "},
        indoc! {"
        `<buffer>`{=html}
        "},
    );
}

#[test]
fn normalization_just_code() {
    compare(
        indoc! {"
        ``` test
        code
        ```
        "},
        indoc! {"
        ``` test
        code
        ```
        "},
    );
}

#[test]
fn normalization_inline_code() {
    compare(
        indoc! {"
        some `inline code` here
        "},
        indoc! {"
        some `inline code` here
        "},
    );
}

#[test]
fn raw_trim() {
    setup();
    compare(
        indoc! {"
        para

        ```
        raw block
        ```
    "},
        indoc! {"
        para

        ```

        raw block

        ```
    "},
    );
}

#[test]
fn raw_trim_keeps_spaces() {
    setup();
    compare(
        indoc! {"
        para

        ```
         raw block
        ```
    "},
        indoc! {"
        para

        ```

         raw block

        ```
    "},
    );
}

#[test]
fn raw_in_a_list_item() {
    setup();
    compare(
        indoc! {"
        - list item
          ``` markdown
          raw block line 1

          raw block line 2
          ```
    "},
        indoc! {"
        - list item

          ``` markdown
          raw block line 1

          raw block line 2
          ```
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
