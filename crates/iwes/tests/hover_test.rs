use indoc::indoc;
use lsp_types::{request::HoverRequest, *};
use std::collections::HashMap;

use crate::fixture::*;

fn utf16_offset_of(text: &str, needle: &str) -> u32 {
    let byte_offset = text.find(needle).expect("needle to exist");
    text[..byte_offset]
        .chars()
        .map(|ch| ch.len_utf16() as u32)
        .sum()
}

#[test]
fn hover_preview_for_wiki_link_strips_frontmatter() {
    Fixture::with_documents(vec![
        (
            "1",
            indoc! {
                "text [[2]] text"
            },
        ),
        (
            "2",
            indoc! {"
                ---
                title: Note Two
                ---
                # Heading
                Line 2
            "},
        ),
    ])
    .assert_response::<lsp_types::request::HoverRequest>(
        uri(1).to_hover_params(0, 7),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Heading\n\nLine 2\n".to_string(),
            }),
            range: None,
        }),
    );
}

#[test]
fn hover_preview_for_markdown_link() {
    let state = HashMap::from([
        (
            "1".to_string(),
            indoc! {
                "text [two](2) text"
            }
            .to_string(),
        ),
        (
            "2".to_string(),
            indoc! {"
                # Heading
                Line 2
            "}
            .to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .assert_response::<HoverRequest>(
            uri(1).to_hover_params(0, 7),
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "# Heading\n\nLine 2\n".to_string(),
                }),
                range: None,
            }),
        );
}

#[test]
fn hover_preview_for_wiki_link_after_cjk_text() {
    let state = HashMap::from([
        (
            "1".to_string(),
            "新西兰旅行，四月最后一个周末。[[travel-2025-beijing]]".to_string(),
        ),
        (
            "travel-2025-beijing".to_string(),
            indoc! {"
                # Beijing
                Trip plan
            "}
            .to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .assert_response::<HoverRequest>(
            uri(1).to_hover_params(0, 19),
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "# Beijing\n\nTrip plan\n".to_string(),
                }),
                range: None,
            }),
        );
}

#[test]
fn hover_preview_for_wiki_link_after_emoji_text() {
    let state = HashMap::from([
        (
            "1".to_string(),
            "Plan 🧭 [[travel-2025-beijing]]".to_string(),
        ),
        (
            "travel-2025-beijing".to_string(),
            indoc! {"
                # Beijing
                Trip plan
            "}
            .to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .assert_response::<HoverRequest>(
            uri(1).to_hover_params(0, 8),
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "# Beijing\n\nTrip plan\n".to_string(),
                }),
                range: None,
            }),
        );
}

#[test]
fn hover_preview_for_wiki_links_inside_table_rows() {
    let line = "| 日 | [[2026-05-23]] | [[2026-05-25]] |";
    let state = HashMap::from([
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

    let fixture = Fixture::with_options_and_client(state, Default::default(), "", None);

    fixture.assert_response::<HoverRequest>(
        uri_from("source").to_hover_params(2, utf16_offset_of(line, "[[2026-05-23]]") + 2),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# 2026-05-23\n\nPast day\n".to_string(),
            }),
            range: None,
        }),
    );
    fixture.assert_response::<HoverRequest>(
        uri_from("source").to_hover_params(2, utf16_offset_of(line, "[[2026-05-25]]") + 2),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# 2026-05-25\n\nFuture day\n".to_string(),
            }),
            range: None,
        }),
    );
}

#[test]
fn hover_preview_falls_back_to_unique_basename_key() {
    let state = HashMap::from([
        (
            "journal/day-1".to_string(),
            "text [[cxx陈小欣]] text".to_string(),
        ),
        (
            "01_Diary/01.01_People/cxx陈小欣".to_string(),
            "# Person\nKnown from notes\n".to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .assert_response::<HoverRequest>(
            uri_from("journal/day-1").to_hover_params(0, 7),
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "# Person\n\nKnown from notes\n".to_string(),
                }),
                range: None,
            }),
        );
}

#[test]
fn hover_outside_link_returns_none() {
    Fixture::with_documents(vec![("1", "no links here\n"), ("2", "# Heading\n")])
        .assert_response::<HoverRequest>(uri(1).to_hover_params(0, 0), None);
}

#[test]
fn hover_missing_target_returns_none() {
    Fixture::with_documents(vec![("1", "text [[missing]] text\n")])
        .assert_response::<HoverRequest>(uri(1).to_hover_params(0, 7), None);
}

#[test]
fn hover_preview_for_complex_unicode_mixed_line_multiple_wiki_links() {
    let line = "\"新西兰旅行🗺️，四月最后一个周末（2025-04-26～2025-04-27）｜天气：12°C～18°C，风速≈7㎧；预算 NZ$2,888.50；同行者：张三／Alice／λ-user。备注：试试 Māori 美食、温泉♨️、观星🌌；关键词：CJK混排「漢字かなカナ한글」，Unicode：Ω≈ç√∫˜µ≤≥÷，数学：∀x∈ℝ,f(x)=x²→∞，Emoji：👨🏽‍💻🧋🐑🇳🇿，全角／半角：ＡBC123；引用：『人生は旅である』；路径：C:\\旅程\\NZ\\照片📷\\；标签：#旅行 #测试 [[travel-2025-beijing]] [[北京-旅行🧳]] [[旅行/2025/新西兰🇳🇿]]\"";
    let state = HashMap::from([
        ("1".to_string(), line.to_string()),
        (
            "travel-2025-beijing".to_string(),
            "# Beijing\nTrip plan\n".to_string(),
        ),
        ("北京-旅行🧳".to_string(), "# 北京\n行程草案\n".to_string()),
        (
            "旅行/2025/新西兰🇳🇿".to_string(),
            "# 新西兰\n银河与温泉\n".to_string(),
        ),
    ]);

    let fixture = Fixture::with_options_and_client(state, Default::default(), "", None);

    fixture.assert_response::<HoverRequest>(
        uri(1).to_hover_params(0, utf16_offset_of(line, "[[travel-2025-beijing]]") + 2),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# Beijing\n\nTrip plan\n".to_string(),
            }),
            range: None,
        }),
    );
    fixture.assert_response::<HoverRequest>(
        uri(1).to_hover_params(0, utf16_offset_of(line, "[[北京-旅行🧳]]") + 2),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# 北京\n\n行程草案\n".to_string(),
            }),
            range: None,
        }),
    );
    fixture.assert_response::<HoverRequest>(
        uri(1).to_hover_params(0, utf16_offset_of(line, "[[旅行/2025/新西兰🇳🇿]]") + 2),
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "# 新西兰\n\n银河与温泉\n".to_string(),
            }),
            range: None,
        }),
    );
}
