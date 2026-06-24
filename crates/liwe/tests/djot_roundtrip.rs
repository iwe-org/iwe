use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::{DjotOptions, Format, FormatOptions};

fn djot_options() -> FormatOptions {
    FormatOptions::Djot(DjotOptions::default())
}

fn roundtrip(input: &str) -> String {
    let mut graph = Graph::new_with_options(djot_options());
    graph.insert_document("key".into(), input.to_string());
    graph.to_markdown(&"key".into())
}

#[test]
fn heading_and_paragraph() {
    let input = indoc! {"
        # Title

        A paragraph of text.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn nested_headers() {
    let input = indoc! {"
        # One

        ## Two

        text
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn emphasis_and_strong() {
    let input = indoc! {"
        A _word_ and a *strong* one.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn bullet_list() {
    let input = indoc! {"
        - one
        - two
        - three
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn ordered_list() {
    let input = indoc! {"
        1. one
        2. two
        3. three
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn link() {
    let input = indoc! {"
        A [link](https://example.com) here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn inline_code() {
    let input = indoc! {"
        Use `code` here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn code_block() {
    let input = indoc! {"
        ``` rust
        let x = 1;
        ```
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn block_quote() {
    let input = indoc! {"
        > quoted text
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn mark_insert_delete() {
    let input = indoc! {"
        Text with {=highlight=}, {+insert+}, and {-delete-} here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn symbol() {
    let input = indoc! {"
        A :smile: in text.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn superscript_and_subscript() {
    let input = indoc! {"
        H~2~O and e^x^ here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn span_with_class() {
    let input = indoc! {"
        A [highlighted]{.note} word.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn span_with_id_and_classes() {
    let input = indoc! {"
        A [target]{#anchor .a .b} span.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn span_with_attribute_pair() {
    let input = indoc! {"
        A [x]{lang=en} span.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn discovers_and_reads_dj_files_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("note.dj"), "# Title\n\ntext\n").unwrap();
    std::fs::write(dir.path().join("ignored.md"), "# Md\n").unwrap();

    let graph = Graph::from_path(dir.path(), false, djot_options(), None);

    assert_eq!("# Title\n\ntext\n", graph.to_markdown(&"note".into()));
    assert_eq!("", graph.to_markdown(&"ignored".into()));
}

#[test]
fn writes_dj_extension() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = liwe::model::State::new();
    state.insert("note".to_string(), "# Title\n".to_string());

    liwe::fs::write_store_at_path(&state, dir.path(), Format::Djot).unwrap();

    assert!(dir.path().join("note.dj").exists());
    assert!(!dir.path().join("note.md").exists());
}
