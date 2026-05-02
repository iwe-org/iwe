use std::sync::Once;

use indoc::indoc;
use liwe::graph::Graph;
use liwe::markdown::MarkdownReader;
use liwe::model::config::{FormattingOptions, MarkdownOptions};
use pretty_assertions::assert_str_eq;

#[test]
fn default_emphasis_uses_asterisk() {
    compare(
        indoc! {"
        *italic*
        "},
        "*italic*",
        FormattingOptions::default(),
    );
}

#[test]
fn underscore_emphasis() {
    compare(
        indoc! {"
        _italic_
        "},
        "*italic*",
        FormattingOptions {
            emphasis_token: Some("_".into()),
            ..Default::default()
        },
    );
}

#[test]
fn default_strong_uses_double_asterisk() {
    compare(
        indoc! {"
        **bold**
        "},
        "**bold**",
        FormattingOptions::default(),
    );
}

#[test]
fn underscore_strong() {
    compare(
        indoc! {"
        __bold__
        "},
        "**bold**",
        FormattingOptions {
            strong_token: Some("__".into()),
            ..Default::default()
        },
    );
}

#[test]
fn default_list_uses_dash() {
    compare(
        indoc! {"
        - item1
        - item2
        "},
        indoc! {"
        - item1
        - item2
        "},
        FormattingOptions::default(),
    );
}

#[test]
fn asterisk_list() {
    compare(
        indoc! {"
        * item1
        * item2
        "},
        indoc! {"
        - item1
        - item2
        "},
        FormattingOptions {
            list_token: Some("*".into()),
            ..Default::default()
        },
    );
}

#[test]
fn plus_list() {
    compare(
        indoc! {"
        + item1
        + item2
        "},
        indoc! {"
        - item1
        - item2
        "},
        FormattingOptions {
            list_token: Some("+".into()),
            ..Default::default()
        },
    );
}

#[test]
fn custom_rule_token() {
    compare(
        indoc! {"
        *****
        "},
        "---",
        FormattingOptions {
            rule_token: Some("*".into()),
            rule_token_count: Some(5),
            ..Default::default()
        },
    );
}

#[test]
fn default_rule_token() {
    compare(
        &format!("{}\n", "-".repeat(72)),
        "---",
        FormattingOptions::default(),
    );
}

#[test]
fn ordered_list_with_dot() {
    compare(
        indoc! {"
        1.  item1
        2.  item2
        "},
        indoc! {"
        1. item1
        2. item2
        "},
        FormattingOptions::default(),
    );
}

#[test]
fn ordered_list_with_paren() {
    compare(
        indoc! {"
        1)  item1
        2)  item2
        "},
        indoc! {"
        1. item1
        2. item2
        "},
        FormattingOptions {
            ordered_list_token: Some(")".into()),
            ..Default::default()
        },
    );
}

#[test]
fn increment_ordered_list_bullets_true() {
    compare(
        indoc! {"
        1.  item1
        2.  item2
        3.  item3
        "},
        indoc! {"
        1. item1
        1. item2
        1. item3
        "},
        FormattingOptions::default(),
    );
}

#[test]
fn increment_ordered_list_bullets_false() {
    compare(
        indoc! {"
        1.  item1
        1.  item2
        1.  item3
        "},
        indoc! {"
        1. item1
        2. item2
        3. item3
        "},
        FormattingOptions {
            increment_ordered_list_bullets: Some(false),
            ..Default::default()
        },
    );
}

#[test]
fn code_block_with_backtick() {
    compare(
        indoc! {"
        ``` rust
        fn main() {}
        ```
        "},
        indoc! {"
        ```rust
        fn main() {}
        ```
        "},
        FormattingOptions::default(),
    );
}

#[test]
fn code_block_with_tilde() {
    compare(
        indoc! {"
        ~~~ rust
        fn main() {}
        ~~~
        "},
        indoc! {"
        ```rust
        fn main() {}
        ```
        "},
        FormattingOptions {
            code_block_token: Some("~".into()),
            ..Default::default()
        },
    );
}

#[test]
fn code_block_with_custom_count() {
    compare(
        indoc! {"
        ```` rust
        fn main() {}
        ````
        "},
        indoc! {"
        ```rust
        fn main() {}
        ```
        "},
        FormattingOptions {
            code_block_token_count: Some(4),
            ..Default::default()
        },
    );
}

#[test]
fn rule_with_underscore() {
    compare(
        indoc! {"
        ___
        "},
        "---",
        FormattingOptions {
            rule_token: Some("_".into()),
            rule_token_count: Some(3),
            ..Default::default()
        },
    );
}

#[test]
fn all_custom_tokens() {
    let input = indoc! {"
        # Header

        *italic* and **bold**

        + item1
        + item2

        ---
    "};

    let formatting = FormattingOptions {
        emphasis_token: Some("_".into()),
        strong_token: Some("__".into()),
        list_token: Some("+".into()),
        rule_token: Some("*".into()),
        rule_token_count: Some(3),
        ..Default::default()
    };

    let result = normalize_with(input, formatting);

    assert!(result.contains("_italic_"), "expected underscore emphasis");
    assert!(result.contains("__bold__"), "expected underscore strong");
    assert!(result.contains("+ item1"), "expected plus list token");
    assert!(result.contains("***"), "expected asterisk rule");
}

#[test]
fn invalid_emphasis_token_falls_back_to_default() {
    let formatting = FormattingOptions {
        emphasis_token: Some("~~".into()),
        ..Default::default()
    }
    .validated();

    assert_eq!(formatting.emphasis_token(), "*");
}

#[test]
fn invalid_strong_token_falls_back_to_default() {
    let formatting = FormattingOptions {
        strong_token: Some("***".into()),
        ..Default::default()
    }
    .validated();

    assert_eq!(formatting.strong_token(), "**");
}

#[test]
fn invalid_list_token_falls_back_to_default() {
    let formatting = FormattingOptions {
        list_token: Some(">".into()),
        ..Default::default()
    }
    .validated();

    assert_eq!(formatting.list_token(), "-");
}

#[test]
fn invalid_code_block_token_count_falls_back_to_default() {
    let formatting = FormattingOptions {
        code_block_token_count: Some(1),
        ..Default::default()
    }
    .validated();

    assert_eq!(formatting.code_block_token_count(), 3);
}

#[test]
fn invalid_rule_token_count_falls_back_to_default() {
    let formatting = FormattingOptions {
        rule_token_count: Some(2),
        ..Default::default()
    }
    .validated();

    assert_eq!(formatting.rule_token_count(), 72);
}

#[test]
fn valid_values_preserved_after_validation() {
    let formatting = FormattingOptions {
        emphasis_token: Some("_".into()),
        strong_token: Some("__".into()),
        list_token: Some("+".into()),
        ordered_list_token: Some(")".into()),
        code_block_token: Some("~".into()),
        code_block_token_count: Some(4),
        increment_ordered_list_bullets: Some(true),
        rule_token: Some("*".into()),
        rule_token_count: Some(5),
    }
    .validated();

    assert_eq!(formatting.emphasis_token(), "_");
    assert_eq!(formatting.strong_token(), "__");
    assert_eq!(formatting.list_token(), "+");
    assert_eq!(formatting.ordered_list_token_char(), ')');
    assert_eq!(formatting.code_block_token_char(), '~');
    assert_eq!(formatting.code_block_token_count(), 4);
    assert_eq!(formatting.increment_ordered_list_bullets(), true);
    assert_eq!(formatting.rule_token(), "*");
    assert_eq!(formatting.rule_token_count(), 5);
}

fn compare(expected: &str, input: &str, formatting: FormattingOptions) {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        formatting,
        ..Default::default()
    });

    graph.from_markdown("key".into(), input, MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    println!("{:#?}", graph);
    println!("{}", expected);
    println!("normalized:");
    println!("{}", normalized);

    assert_str_eq!(expected, normalized);
}

fn normalize_with(input: &str, formatting: FormattingOptions) -> String {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        formatting,
        ..Default::default()
    });

    graph.from_markdown("key".into(), input, MarkdownReader::new());
    graph.to_markdown(&"key".into())
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().try_init();
    });
}
