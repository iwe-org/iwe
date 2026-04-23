use super::document::LinkType;
use crate::markdown::writer::MarkdownWriter;
use crate::model;
use crate::model::config::MarkdownOptions;
use crate::model::document::DocumentInlines;
use crate::model::node::ColumnAlignment;
use crate::model::reference::{Reference, ReferenceType};
use crate::model::{InlinesContext, Key, Lang, Level, LibraryUrl, Title};

pub type Blocks = Vec<GraphBlock>;
pub type GraphInlines = Vec<GraphInline>;

#[derive(Debug, Clone, PartialEq)]
pub enum GraphBlock {
    Frontmatter(String),
    Plain(GraphInlines),
    Para(GraphInlines),
    LineBlock(Vec<GraphInlines>),
    CodeBlock(Option<Lang>, String),
    RawBlock(String, String),
    BlockQuote(Blocks),
    OrderedList(Vec<Blocks>),
    BulletList(Vec<Blocks>),
    Header(Level, GraphInlines),
    HorizontalRule,
    Table(
        Vec<GraphInlines>,
        Vec<ColumnAlignment>,
        Vec<Vec<GraphInlines>>,
    ),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphInline {
    Code(Option<Lang>, String),
    Emph(GraphInlines),
    Image(LibraryUrl, Title, GraphInlines),
    LineBreak,
    Link(LibraryUrl, Title, LinkType, GraphInlines),
    Reference(Reference),
    Math(String),
    RawInline(Lang, String),
    SmallCaps(GraphInlines),
    SoftBreak,
    Space,
    Str(String),
    Strikeout(GraphInlines),
    Strong(GraphInlines),
    Subscript(GraphInlines),
    Superscript(GraphInlines),
    Underline(GraphInlines),
}

impl From<&str> for GraphInline {
    fn from(s: &str) -> Self {
        GraphInline::Str(s.to_string())
    }
}

impl From<String> for GraphInline {
    fn from(s: String) -> Self {
        GraphInline::Str(s)
    }
}

#[allow(dead_code)]
impl GraphBlock {
    fn is_sparce_list(&self) -> bool {
        match self {
            GraphBlock::BulletList(items) => items
                .iter()
                .any(|item| item.iter().filter(|block| block.is_paragraph()).count() > 1),
            GraphBlock::OrderedList(items) => items
                .iter()
                .any(|item| item.iter().filter(|block| block.is_paragraph()).count() > 1),
            _ => false,
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, GraphBlock::BulletList(_) | GraphBlock::OrderedList(_))
    }

    fn is_paragraph(&self) -> bool {
        matches!(self, GraphBlock::Plain(_) | GraphBlock::Para(_))
    }

    fn is_frontmatter(&self) -> bool {
        matches!(self, GraphBlock::Frontmatter(_))
    }

    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        match self {
            GraphBlock::Frontmatter(content) => format!("---\n{}---\n", content),
            GraphBlock::Plain(inlines) => format!("{}\n", inlines_to_markdown(inlines, options)),
            GraphBlock::Para(inlines) => format!("{}\n", inlines_to_markdown(inlines, options)),
            GraphBlock::LineBlock(lines) => lines
                .iter()
                .map(|line| inlines_to_markdown(line, options))
                .collect::<Vec<String>>()
                .join("\n"),
            GraphBlock::CodeBlock(lang, text) => {
                let fence = options
                    .formatting
                    .code_block_token()
                    .repeat(options.formatting.code_block_token_count());
                lang.clone()
                    .filter(|lang| !lang.trim().is_empty())
                    .map(|lang| {
                        format!("{} {}\n{}\n{}\n", fence, lang, text.trim_matches('\n'), fence)
                    })
                    .unwrap_or_else(|| {
                        format!("{}\n{}\n{}\n", fence, text.trim_matches('\n'), fence)
                    })
            }
            GraphBlock::RawBlock(_, text) => text.clone(),
            GraphBlock::BlockQuote(blocks) => {
                blocks_to_markdown_sparce(blocks, options)
                    .lines()
                    .map(|line| format!("> {}", line))
                    .map(|line| line.trim().to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
                    + "\n"
            }
            GraphBlock::OrderedList(items) => items
                .iter()
                .enumerate()
                .map(|(n, item)| {
                    let num = if options.formatting.increment_ordered_list_bullets() {
                        n + 1
                    } else {
                        1
                    };
                    left_pad_and_prefix_num(
                        &blocks_to_markdown_and(item, self.is_sparce_list(), options),
                        num,
                        options.formatting.ordered_list_token_char(),
                    )
                })
                .collect::<Vec<String>>()
                .join(if self.is_sparce_list() { "\n" } else { "" }),
            GraphBlock::BulletList(items) => items
                .iter()
                .map(|item| {
                    left_pad_and_prefix(
                        &blocks_to_markdown_and(item, self.is_sparce_list(), options),
                        options.formatting.list_token(),
                    )
                })
                .collect::<Vec<String>>()
                .join(if self.is_sparce_list() { "\n" } else { "" }),
            GraphBlock::Header(level, inlines) => {
                format!(
                    "{} {}\n",
                    "#".repeat(*level as usize),
                    inlines_to_markdown(inlines, options)
                )
            }
            GraphBlock::HorizontalRule => {
                let fmt = &options.formatting;
                format!("{}\n", fmt.rule_token().repeat(fmt.rule_token_count()))
            }
            GraphBlock::Table(_, _, _) => {
                let writer = MarkdownWriter::new(options.clone());
                format!("{}\n", writer.write(vec![self.clone()]))
            }
        }
    }
}

impl GraphInline {
    pub fn from_string(str: &str) -> GraphInlines {
        vec![GraphInline::Str(str.to_string())]
    }
    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        match self {
            GraphInline::Str(text) => text.clone(),
            GraphInline::Emph(emph) => {
                let t = options.formatting.emphasis_token();
                format!("{t}{}{t}", inlines_to_markdown(emph, options))
            }
            GraphInline::Underline(underline) => inlines_to_markdown(underline, options),
            GraphInline::Strong(strong) => {
                let t = options.formatting.strong_token();
                format!("{t}{}{t}", inlines_to_markdown(strong, options))
            }
            GraphInline::Strikeout(strikeout) => {
                format!("~~{}~~", inlines_to_markdown(strikeout, options))
            }
            GraphInline::Superscript(superscript) => {
                format!("^{}^", inlines_to_markdown(superscript, options))
            }
            GraphInline::Subscript(subscript) => {
                format!("~{}~", inlines_to_markdown(subscript, options))
            }
            GraphInline::SmallCaps(small_caps) => inlines_to_markdown(small_caps, options),
            GraphInline::Code(_, text) => format!("`{}`", text),
            GraphInline::Space => " ".into(),
            GraphInline::SoftBreak => "\n".into(),
            GraphInline::LineBreak => "\n".into(),
            GraphInline::Link(url, _, link_type, inlines) => {
                let text = inlines_to_markdown(inlines, options);
                if *link_type == LinkType::WikiLinkPiped {
                    return format!("[[{}|{}]]", url, text);
                }
                if *link_type == LinkType::WikiLink {
                    return format!("[[{}]]", url);
                }
                if model::is_ref_url(url) {
                    format!("[{}]({}{})", text, url, options.refs_extension)
                } else if text.eq_ignore_ascii_case(url) {
                    format!("<{}>", url)
                } else {
                    format!("[{}]({})", text, url)
                }
            }
            GraphInline::Reference(reference) => {
                let url = reference.key.to_library_url();
                match reference.reference_type {
                    ReferenceType::WikiLinkPiped => {
                        format!("[[{}|{}]]", url, reference.text)
                    }
                    ReferenceType::WikiLink => format!("[[{}]]", url),
                    ReferenceType::Regular => {
                        format!(
                            "[{}]({}{})",
                            reference.text, url, options.refs_extension
                        )
                    }
                }
            }
            GraphInline::Image(url, _, inlines) => {
                format!("![{}]({})", inlines_to_markdown(inlines, options), url)
            }
            GraphInline::RawInline(_, content) => format!("`{}`", content),
            GraphInline::Math(math) => format!("${}$", math),
        }
    }
    pub fn plain_text(&self) -> String {
        match self {
            GraphInline::Str(text) => text.clone(),
            GraphInline::Emph(emph) => to_plain_text(emph),
            GraphInline::Underline(underline) => to_plain_text(underline),
            GraphInline::Strong(strong) => to_plain_text(strong),
            GraphInline::Strikeout(strikeout) => to_plain_text(strikeout),
            GraphInline::Superscript(superscript) => to_plain_text(superscript),
            GraphInline::Subscript(subscript) => to_plain_text(subscript),
            GraphInline::SmallCaps(small_caps) => to_plain_text(small_caps),
            GraphInline::Code(_, text) => text.clone(),
            GraphInline::Space => " ".into(),
            GraphInline::SoftBreak => "\n".into(),
            GraphInline::LineBreak => "\n".into(),
            GraphInline::Link(_, _, _, inlines) => to_plain_text(inlines),
            GraphInline::Reference(reference) => reference.text.clone(),
            GraphInline::Image(_, _, inlines) => to_plain_text(inlines),
            GraphInline::RawInline(_, content) => content.clone(),
            _ => "".into(),
        }
    }

    pub fn ref_keys(&self) -> Vec<Key> {
        match self {
            GraphInline::Emph(emph) => emph.iter().flat_map(|inline| inline.ref_keys()).collect(),
            GraphInline::Underline(underline) => underline
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Strong(strong) => {
                strong.iter().flat_map(|inline| inline.ref_keys()).collect()
            }
            GraphInline::Strikeout(strikeout) => strikeout
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Superscript(superscript) => superscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Subscript(subscript) => subscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::SmallCaps(small_caps) => small_caps
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Link(_, _, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Reference(reference) => vec![reference.key.clone()],
            GraphInline::Image(_, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            _ => vec![],
        }
    }

    pub fn normalize(&self, context: impl InlinesContext) -> GraphInline {
        match self {
            GraphInline::Emph(emph) => GraphInline::Emph(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            GraphInline::Strong(emph) => GraphInline::Strong(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Underline(emph) => GraphInline::Underline(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            GraphInline::Strikeout(emph) => GraphInline::Strikeout(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Superscript(emph) => GraphInline::Superscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Subscript(emph) => GraphInline::Subscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::SmallCaps(emph) => GraphInline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Reference(reference) => {
                let new_text = match reference.reference_type {
                    ReferenceType::Regular => context
                        .get_ref_title(&reference.key)
                        .unwrap_or_else(|| reference.text.clone()),
                    ReferenceType::WikiLink => String::new(),
                    ReferenceType::WikiLinkPiped => reference.text.clone(),
                };

                GraphInline::Reference(Reference {
                    key: reference.key.clone(),
                    text: new_text,
                    reference_type: reference.reference_type,
                })
            }
            _ => self.clone(),
        }
    }

    pub fn change_key(&self, target_key: &Key, updated_key: &Key) -> GraphInline {
        match self {
            GraphInline::Emph(emph) => GraphInline::Emph(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),

            GraphInline::Strong(emph) => GraphInline::Strong(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Underline(emph) => GraphInline::Underline(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),

            GraphInline::Strikeout(emph) => GraphInline::Strikeout(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Superscript(emph) => GraphInline::Superscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Subscript(emph) => GraphInline::Subscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::SmallCaps(emph) => GraphInline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Reference(reference) => {
                if reference.key.eq(target_key) {
                    return GraphInline::Reference(Reference {
                        key: updated_key.clone(),
                        text: reference.text.clone(),
                        reference_type: reference.reference_type,
                    });
                }
                self.clone()
            }
            _ => self.clone(),
        }
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, GraphInline::Reference(_))
    }
}

fn left_pad_and_prefix(text: &str, list_token: &str) -> String {
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push('\n');
        } else if n == 0 {
            result.push_str(&format!("{} {}\n", list_token, line));
        } else {
            result.push_str(&format!("{} {}\n", " ".repeat(list_token.len()), line));
        }
    }

    result
}

fn left_pad_and_prefix_num(text: &str, num: usize, ordered_list_token: char) -> String {
    let prefix = format!("{}{}{}", num, ordered_list_token, if num > 9 { "" } else { " " });
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push('\n');
        } else if n == 0 {
            result.push_str(&format!("{} {}\n", prefix, line));
        } else {
            result.push_str(&format!("{} {}\n", " ".repeat(prefix.len()), line));
        }
    }

    result
}

pub fn to_plain_text(content: &GraphInlines) -> String {
    content
        .iter()
        .map(|i| i.plain_text())
        .collect::<Vec<String>>()
        .join("")
}

pub fn inlines_to_markdown(content: &GraphInlines, options: &MarkdownOptions) -> String {
    content
        .iter()
        .map(|i| i.to_markdown(options))
        .collect::<Vec<String>>()
        .join("")
}

fn ensure_trailing_newline(s: String) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s
    } else {
        s + "\n"
    }
}

pub fn blocks_to_markdown_and(blocks: &Blocks, sparce: bool, options: &MarkdownOptions) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown(options))
            .collect::<Vec<String>>()
            .join(if sparce { "\n" } else { "" }),
    )
}

pub fn blocks_to_markdown(blocks: &Blocks, options: &MarkdownOptions) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown(options))
            .collect::<Vec<String>>()
            .join(""),
    )
}

pub fn blocks_to_markdown_sparce(blocks: &Blocks, options: &MarkdownOptions) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown(options))
            .collect::<Vec<String>>()
            .join("\n"),
    )
}

pub fn blocks_to_markdown_sparce_skip_frontmatter(
    blocks: &Blocks,
    options: &MarkdownOptions,
) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .filter_map(|block| (!block.is_frontmatter()).then_some(block.to_markdown(options)))
            .collect::<Vec<String>>()
            .join("\n"),
    )
}

pub fn to_graph_inlines(content: &DocumentInlines, relative_to: &str) -> Vec<GraphInline> {
    content
        .iter()
        .map(|i| i.to_graph_inline(relative_to))
        .collect()
}

#[cfg(test)]
pub mod tests {
    use crate::model::config::MarkdownOptions;
    use crate::model::graph::blocks_to_markdown;
    use crate::model::graph::{GraphBlock, GraphInline};
    use indoc::indoc;

    fn plain(text: &str) -> GraphBlock {
        GraphBlock::Plain(vec![GraphInline::Str(text.into())])
    }

    #[test]
    fn test_ordered_list_to_markdown() {
        let list = vec![GraphBlock::OrderedList(vec![
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
        ])];
        assert_eq!(
            indoc! {"
                1.  item
                2.  item
                3.  item
                4.  item
                5.  item
                6.  item
                7.  item
                8.  item
                9.  item
                10. item
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_ordered_list_with_para() {
        let list = vec![GraphBlock::OrderedList(vec![vec![
            plain("item1"),
            plain("para"),
        ]])];
        assert_eq!(
            indoc! {"
                1.  item1

                    para
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_to_markdown() {
        let list = vec![GraphBlock::BulletList(vec![
            vec![plain("item1")],
            vec![plain("item2")],
        ])];
        assert_eq!(
            indoc! {"
                - item1
                - item2
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_sub_list() {
        let list = vec![GraphBlock::BulletList(vec![vec![
            plain("item1"),
            GraphBlock::BulletList(vec![vec![plain("item2")]]),
        ]])];
        assert_eq!(
            indoc! {"
                - item1
                  - item2
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_with_para() {
        let list = vec![GraphBlock::BulletList(vec![vec![
            plain("item1"),
            plain("para"),
        ]])];
        assert_eq!(
            indoc! {"
                - item1

                  para
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_with_para2() {
        let list = vec![GraphBlock::BulletList(vec![
            vec![plain("item1"), plain("para1")],
            vec![plain("item2"), plain("para2")],
        ])];
        assert_eq!(
            indoc! {"
                - item1

                  para1

                - item2

                  para2
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_sub_sub_list() {
        let list = vec![GraphBlock::BulletList(vec![vec![
            plain("item1"),
            GraphBlock::BulletList(vec![vec![
                plain("item2"),
                GraphBlock::BulletList(vec![vec![plain("item3")]]),
            ]]),
        ]])];
        assert_eq!(
            indoc! {"
                - item1
                  - item2
                    - item3
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }
}
