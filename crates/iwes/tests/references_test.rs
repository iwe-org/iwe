use indoc::indoc;

use crate::fixture::*;

fn utf16_offset_of(text: &str, needle: &str) -> u32 {
    let byte_offset = text.find(needle).expect("needle to exist");
    text[..byte_offset]
        .chars()
        .map(|ch| ch.len_utf16() as u32)
        .sum()
}

#[test]
fn single_reference() {
    Fixture::with(indoc! {"
        # doc1

        [target](3)
        _
        # doc2

        [target](3)
        _
        # target
        "})
    .references(
        uri(1).to_reference_params(2, 1, false),
        vec![uri(2).to_location(2, 3)],
    );
}

#[test]
fn two_references() {
    Fixture::with(indoc! {"
        # doc1

        [target](4)
        _
        # doc2

        [target](4)
        _
        # doc3

        [target](4)
        _
        # target
        "})
    .references(
        uri(1).to_reference_params(2, 1, false),
        vec![uri(2).to_location(2, 3), uri(3).to_location(2, 3)],
    );
}

#[test]
fn link() {
    Fixture::with(indoc! {"
        # header 1

        text and link [target](2)
        _
        # target
        "})
    .references(uri(1).to_reference_params(2, 15, false), vec![]);
}

#[test]
fn wiki_link_after_cjk_text() {
    Fixture::with(indoc! {"
        # doc1

        新西兰旅行，四月最后一个周末。[[3]]
        _
        # doc2

        [target](3)
        _
        # target
        "})
    .references(
        uri(1).to_reference_params(2, 19, false),
        vec![uri(2).to_location(2, 3)],
    );
}

#[test]
fn wiki_link_after_emoji_text() {
    Fixture::with(indoc! {"
        # doc1

        Plan 🧭 [[3]]
        _
        # doc2

        [target](3)
        _
        # target
        "})
    .references(
        uri(1).to_reference_params(2, 8, false),
        vec![uri(2).to_location(2, 3)],
    );
}

#[test]
fn wiki_links_inside_table_rows() {
    let line = "| 日 | [[2026-05-23]] | [[2026-05-25]] |";
    let state = std::collections::HashMap::from([
        ("source-1".to_string(), format!("# diary\n\n{}\n", line)),
        ("source-2".to_string(), format!("# diary\n\n{}\n", line)),
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
        .references(
            uri_from("source-1").to_reference_params(2, 8, false),
            vec![],
        )
        .references(
            uri_from("source-1").to_reference_params(2, 25, false),
            vec![],
        );
}

#[test]
fn wiki_links_inside_complex_unicode_mixed_line() {
    let line = "\"新西兰旅行🗺️，四月最后一个周末（2025-04-26～2025-04-27）｜天气：12°C～18°C，风速≈7㎧；预算 NZ$2,888.50；同行者：张三／Alice／λ-user。备注：试试 Māori 美食、温泉♨️、观星🌌；关键词：CJK混排「漢字かなカナ한글」，Unicode：Ω≈ç√∫˜µ≤≥÷，数学：∀x∈ℝ,f(x)=x²→∞，Emoji：👨🏽‍💻🧋🐑🇳🇿，全角／半角：ＡBC123；引用：『人生は旅である』；路径：C:\\旅程\\NZ\\照片📷\\；标签：#旅行 #测试 [[travel-2025-beijing]] [[北京-旅行🧳]] [[旅行/2025/新西兰🇳🇿]]\"";
    let state = std::collections::HashMap::from([
        ("1".to_string(), format!("# doc1\n\n{}\n", line)),
        (
            "2".to_string(),
            "[target](travel-2025-beijing)\n".to_string(),
        ),
        ("3".to_string(), "[target](北京-旅行🧳)\n".to_string()),
        (
            "4".to_string(),
            "[target](旅行/2025/新西兰🇳🇿)\n".to_string(),
        ),
    ]);

    Fixture::with_options_and_client(state, Default::default(), "", None)
        .references(
            uri(1).to_reference_params(
                2,
                utf16_offset_of(line, "[[travel-2025-beijing]]") + 2,
                false,
            ),
            vec![uri(2).to_location(0, 1)],
        )
        .references(
            uri(1).to_reference_params(2, utf16_offset_of(line, "[[北京-旅行🧳]]") + 2, false),
            vec![uri(3).to_location(0, 1)],
        )
        .references(
            uri(1).to_reference_params(
                2,
                utf16_offset_of(line, "[[旅行/2025/新西兰🇳🇿]]") + 2,
                false,
            ),
            vec![uri(4).to_location(0, 1)],
        );
}
