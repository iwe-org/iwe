use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::{evaluate, parse_filter_expression, Filter};
use liwe::state::from_indoc;
use pretty_assertions::assert_eq;

fn matched(docs: &str, expr: &str) -> Vec<String> {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    let filter = parse_filter_expression(expr).expect("filter parses");
    evaluate(&filter, &graph)
        .into_iter()
        .map(|k| k.to_string())
        .collect()
}

const CORPUS: &str = indoc! {"
    # alpha

    Ship the editor TODO confirm date

    ## Status

    active
    _
    # beta

    Nothing pending here

    ## Notes

    keep going
    _
    # gamma

    Another TODO waits
"};

#[test]
fn content_text_membership() {
    assert_eq!(
        matched(CORPUS, "$content: { $text: TODO }"),
        vec!["1".to_string(), "3".to_string()]
    );
}

#[test]
fn content_header_membership() {
    assert_eq!(
        matched(CORPUS, "$content: { $header: Status }"),
        vec!["1".to_string()]
    );
}

#[test]
fn content_empty_predicate_matches_any_block() {
    assert_eq!(
        matched(CORPUS, "$content: {}"),
        vec!["1".to_string(), "2".to_string(), "3".to_string()]
    );
}

#[test]
fn content_absence_via_nor() {
    assert_eq!(
        matched(CORPUS, "$nor: [ { $content: { $header: Status } } ]"),
        vec!["2".to_string(), "3".to_string()]
    );
}

#[test]
fn content_composes_under_or_with_field() {
    let docs = indoc! {"
        ---
        status: draft
        ---

        # one

        plain body
        _
        # two

        body with TODO
        _
        # three

        clean body
    "};
    assert_eq!(
        matched(
            docs,
            "$or: [ { status: draft }, { $content: { $text: TODO } } ]"
        ),
        vec!["1".to_string(), "2".to_string()]
    );
}

#[test]
fn content_scoped_membership_within_section() {
    let docs = indoc! {"
        # one

        ## Goals

        Q3 target here

        ## Other

        Q3 mention elsewhere
        _
        # two

        ## Other

        Q3 only outside goals
    "};
    assert_eq!(
        matched(docs, "$content: { $within: Goals, $text: Q3 }"),
        vec!["1".to_string()]
    );
}

#[test]
fn content_parses_to_content_filter() {
    let filter = parse_filter_expression("$content: { $text: TODO }").expect("parses");
    assert!(matches!(filter, Filter::Content(_)));
}
