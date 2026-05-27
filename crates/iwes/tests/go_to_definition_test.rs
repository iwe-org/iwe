use indoc::indoc;
use liwe::model::config::MarkdownOptions;
use std::str::FromStr;

use crate::fixture::*;

fn utf16_offset_of(text: &str, needle: &str) -> u32 {
    let byte_offset = text.find(needle).expect("needle to exist");
    text[..byte_offset]
        .chars()
        .map(|ch| ch.len_utf16() as u32)
        .sum()
}

#[test]
fn no_definition() {
    Fixture::new().go_to_definition(
        uri(1).to_goto_definition_params(0, 0),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition() {
    Fixture::with(indoc! {"
            # test

            [test](link)

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 0),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    );
}

#[test]
fn definition_in_paragraph() {
    Fixture::with(indoc! {"
            # test

            text [test](link) text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_wiki_link() {
    Fixture::with(indoc! {"
            # test

            text [[link]] text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_wiki_link_after_cjk_text() {
    Fixture::with(indoc! {"
            # test

            新西兰旅行，四月最后一个周末。[[travel-2025-beijing]]

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 19),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/travel-2025-beijing.md").unwrap(),
        ),
    );
}

#[test]
fn definition_in_paragraph_wiki_link_after_emoji_text() {
    Fixture::with(indoc! {"
            # test

            Plan 🧭 [[travel-2025-beijing]]

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 8),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/travel-2025-beijing.md").unwrap(),
        ),
    );
}

#[test]
fn definition_for_wiki_links_inside_table_rows() {
    let line = "| 日 | [[2026-05-23]] | [[2026-05-25]] |";
    let state = std::collections::HashMap::from([
        ("source".to_string(), format!("# diary\n\n{}\n", line)),
        (
            "2026-05-23".to_string(),
            "# 2026-05-23\nPast day\n".to_string(),
        ),
        (
            "2026-05-25".to_string(),
            "# 2026-05-25\nFuture day\n".to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .go_to_definition(
            uri_from("source").to_goto_definition_params(2, 8),
            goto_definition_response_single(
                lsp_types::Uri::from_str("file:///basepath/2026-05-23.md").unwrap(),
            ),
        )
        .go_to_definition(
            uri_from("source").to_goto_definition_params(2, 25),
            goto_definition_response_single(
                lsp_types::Uri::from_str("file:///basepath/2026-05-25.md").unwrap(),
            ),
        );
}

#[test]
fn definition_in_paragraph_wiki_link_with_space() {
    Fixture::with(indoc! {"
            # test

            text [[link to something]] text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 9),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link%20to%20something.md").unwrap(),
        ),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 2),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_piped_wiki_link() {
    Fixture::with(indoc! {"
            # test

            text [[link|title]] text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 7),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 1),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_list() {
    Fixture::with(indoc! {"
            # test

            - [test](link)

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    );
}

#[test]
fn definition_in_nested_list() {
    Fixture::with(indoc! {"
            # test

            - list
              - item
              - [test](link)

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(4, 8),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    );
}

#[test]
fn definition_with_md_extension() {
    Fixture::with_options(
        indoc! {"
            # test

            [test](link.md)

            "},
        MarkdownOptions {
            refs_extension: ".md".to_string(),
            ..Default::default()
        },
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 0),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    );
}

#[test]
fn definition_with_relative_path() {
    Fixture::with_documents(vec![("d/1", "[](2)")]).go_to_definition(
        uri_from("d/1").to_goto_definition_params(0, 0),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/d/2.md").unwrap(),
        ),
    );
}

#[test]
fn definition_external_https_url() {
    Fixture::with(indoc! {"
            # test

            [example](https://example.com)

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 5),
        "https://example.com",
    );
}

#[test]
fn definition_external_http_url() {
    Fixture::with(indoc! {"
            # test

            [example](http://example.com)

            "})
    .go_to_definition_external(uri(1).to_goto_definition_params(2, 5), "http://example.com");
}

#[test]
fn definition_external_mailto_url() {
    Fixture::with(indoc! {"
            # test

            [email](mailto:test@example.com)

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 5),
        "mailto:test@example.com",
    );
}

#[test]
fn definition_bare_https_url() {
    Fixture::with(indoc! {"
            # test

            Check out https://example.com for more

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 15),
        "https://example.com",
    );
}

#[test]
fn definition_bare_http_url() {
    Fixture::with(indoc! {"
            # test

            Visit http://example.org today

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 10),
        "http://example.org",
    );
}

#[test]
fn definition_bare_mailto_url() {
    Fixture::with(indoc! {"
            # test

            Contact mailto:test@example.com

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 15),
        "mailto:test@example.com",
    );
}

#[test]
fn definition_with_complex_unicode_mixed_line_multiple_wiki_links() {
    let line = "\"新西兰旅行🗺️，四月最后一个周末（2025-04-26～2025-04-27）｜天气：12°C～18°C，风速≈7㎧；预算 NZ$2,888.50；同行者：张三／Alice／λ-user。备注：试试 Māori 美食、温泉♨️、观星🌌；关键词：CJK混排「漢字かなカナ한글」，Unicode：Ω≈ç√∫˜µ≤≥÷，数学：∀x∈ℝ,f(x)=x²→∞，Emoji：👨🏽‍💻🧋🐑🇳🇿，全角／半角：ＡBC123；引用：『人生は旅である』；路径：C:\\旅程\\NZ\\照片📷\\；标签：#旅行 #测试 [[travel-2025-beijing]] [[北京-旅行🧳]]\"";
    let state =
        std::collections::HashMap::from([("1".to_string(), format!("# test\n\n{}\n", line))]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .go_to_definition(
            uri(1)
                .to_goto_definition_params(2, utf16_offset_of(line, "[[travel-2025-beijing]]") + 2),
            goto_definition_response_single(
                lsp_types::Uri::from_str("file:///basepath/travel-2025-beijing.md").unwrap(),
            ),
        )
        .go_to_definition(
            uri(1).to_goto_definition_params(2, utf16_offset_of(line, "[[北京 - 旅行🧳]]") + 2),
            goto_definition_response_single(
                lsp_types::Uri::from_str(
                    "file:///basepath/%E5%8C%97%E4%BA%AC-%E6%97%85%E8%A1%8C%F0%9F%A7%B3.md",
                )
                .unwrap(),
            ),
        );
}
