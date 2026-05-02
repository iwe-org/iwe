use indoc::indoc;
use lsp_types::{request::FoldingRangeRequest, FoldingRange, FoldingRangeKind, FoldingRangeParams};

use crate::fixture::*;

fn folding_range_params(key: u32) -> FoldingRangeParams {
    FoldingRangeParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri(key) },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    }
}

fn folding_range(start_line: u32, end_line: u32, collapsed_text: Option<String>) -> FoldingRange {
    FoldingRange {
        start_line,
        start_character: None,
        end_line,
        end_character: None,
        kind: Some(FoldingRangeKind::Region),
        collapsed_text,
    }
}

#[test]
fn folding_range_for_nested_sections() {
    Fixture::with(indoc! {"
        # test

        ## Section 1

        Content for section 1

        ## Section 2

        Content for section 2
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 8, Some("# test".to_string())),
            folding_range(2, 4, Some("## Section 1".to_string())),
            folding_range(6, 8, Some("## Section 2".to_string())),
        ]),
    );
}

#[test]
fn folding_range_for_code_blocks() {
    Fixture::with(indoc! {"
        # test

        ```rust
        fn main() {
            println!(\"Hello\");
        }
        ```
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 5, Some("# test".to_string())),
            folding_range(2, 5, Some("rust".to_string())),
        ]),
    );
}


#[test]
fn folding_range_for_quotes() {
    Fixture::with(indoc! {"
        # test

        > This is a quote
        > that spans multiple lines
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![folding_range(0, 2, Some("# test".to_string()))]),
    );
}

#[test]
fn folding_range_empty_document() {
    Fixture::with(indoc! {"
    "})
    .assert_response::<FoldingRangeRequest>(folding_range_params(1), Some(vec![]));
}

#[test]
fn folding_range_single_line_section() {
    Fixture::with(indoc! {"
        # test
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![folding_range(0, 0, Some("# test".to_string()))]),
    );
}

#[test]
fn folding_range_last_section_without_content() {
    Fixture::with(indoc! {"
        # test

        Some content

        ## test2

        More content

        ## test3

        Even more content
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 10, Some("# test".to_string())),
            folding_range(4, 6, Some("## test2".to_string())),
            folding_range(8, 10, Some("## test3".to_string())),
        ]),
    );
}

#[test]
fn folding_range_for_tables() {
    Fixture::with(indoc! {"
        # test

        | Header 1 | Header 2 |
        |----------|----------|
        | Cell 1   | Cell 2   |
        | Cell 3   | Cell 4   |
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 4, Some("# test".to_string())),
            folding_range(2, 4, None),
        ]),
    );
}

#[test]
fn folding_range_for_lists() {
    Fixture::with(indoc! {"
        # test

        - Item 1
        - Item 2
        - Item 3
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 4, Some("# test".to_string())),
            folding_range(2, 4, Some("- Item 1".to_string())),
            folding_range(2, 2, Some("## Item 1".to_string())),
            folding_range(3, 3, Some("## Item 2".to_string())),
            folding_range(4, 4, Some("## Item 3".to_string())),
        ]),
    );
}

#[test]
fn folding_range_last_section_no_trailing_newline() {
    Fixture::with("# Crypto course\n\n1.  Find a crypto course.\n\n## Test\n\ntest content\n\n## test2\n\ntest content")
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 10, Some("# Crypto course".to_string())),
            folding_range(2, 2, Some("## Find a crypto course.".to_string())),
            folding_range(4, 6, Some("## Test".to_string())),
            folding_range(8, 10, Some("## test2".to_string())),
        ]),
    );
}

#[test]
fn folding_range_last_section_with_reference_and_nested_list() {
    Fixture::with(indoc! {"
        # Header

        ## Last Section

        [Reference](link)

        - test
          - nested
        - test
    "})
    .assert_response::<FoldingRangeRequest>(
        folding_range_params(1),
        Some(vec![
            folding_range(0, 8, Some("# Header".to_string())),
            folding_range(2, 8, Some("## Last Section".to_string())),
            folding_range(6, 8, Some("- test".to_string())),
            folding_range(6, 7, Some("### test".to_string())),
            folding_range(7, 7, Some("#### nested".to_string())),
            folding_range(8, 8, Some("### test".to_string())),
        ]),
    );
}
