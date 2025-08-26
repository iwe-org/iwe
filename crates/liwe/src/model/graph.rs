use super::document::LinkType;
use crate::markdown::writer::MarkdownWriter;
use crate::model;
use crate::model::config::MarkdownOptions;
use crate::model::document::DocumentInlines;
use crate::model::node::ColumnAlignment;
use crate::model::{InlinesContext, Key, Lang, Level, LibraryUrl, Title};

pub type Blocks = Vec<GraphBlock>;
pub type GraphInlines = Vec<GraphInline>;

#[derive(Debug, Clone, PartialEq)]
pub enum GraphBlock {
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
        match self {
            GraphBlock::BulletList(_) => true,
            GraphBlock::OrderedList(_) => true,
            _ => false,
        }
    }

    fn is_paragraph(&self) -> bool {
        match self {
            GraphBlock::Plain(_) => true,
            GraphBlock::Para(_) => true,
            _ => false,
        }
    }

    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        match self {
            GraphBlock::Plain(inlines) => format!("{}\n", inlines_to_markdown(inlines, options)),
            GraphBlock::Para(inlines) => format!("{}\n", inlines_to_markdown(inlines, options)),
            GraphBlock::LineBlock(lines) => lines
                .iter()
                .map(|line| inlines_to_markdown(line, options))
                .collect::<Vec<String>>()
                .join("\n"),
            GraphBlock::CodeBlock(lang, text) => lang
                .clone()
                .filter(|lang| !lang.trim().is_empty())
                .map(|lang| format!("``` {}\n{}\n```\n", lang, text.trim_matches('\n')))
                .unwrap_or_else(|| format!("```\n{}\n```\n", text.trim_matches('\n'))),
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
                    left_pad_and_prefix_num(
                        &blocks_to_markdown_and(item, self.is_sparce_list(), options),
                        n + 1,
                    )
                })
                .collect::<Vec<String>>()
                .join(if self.is_sparce_list() { "\n" } else { "" }),
            GraphBlock::BulletList(items) => items
                .iter()
                .map(|item| {
                    left_pad_and_prefix(&blocks_to_markdown_and(
                        item,
                        self.is_sparce_list(),
                        options,
                    ))
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
            GraphBlock::HorizontalRule => format!("{}\n", "-".repeat(72)),
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
            GraphInline::Emph(emph) => format!("*{}*", inlines_to_markdown(emph, options)),
            GraphInline::Underline(underline) => inlines_to_markdown(underline, options),
            GraphInline::Strong(strong) => format!("**{}**", inlines_to_markdown(strong, options)),
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
                if !self.is_ref() && text.eq_ignore_ascii_case(url) {
                    format!("<{}>", url)
                } else if self.is_ref() {
                    format!("[{}]({}{})", text, url, options.refs_extension)
                } else {
                    format!("[{}]({})", text, url)
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
            GraphInline::Link(_, _, _, _) => {
                self.ref_key().map(|key| vec![key]).unwrap_or_default()
            }
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
            GraphInline::Link(url, title, link_type, inlines) => {
                if self.is_ref() {
                    let new_inlines = match *link_type {
                        LinkType::Regular => context
                            .get_ref_title(&Key::name(url))
                            .map(|title| vec![GraphInline::Str(title)])
                            .unwrap_or(inlines.clone()),
                        LinkType::WikiLink => vec![],
                        LinkType::WikiLinkPiped => inlines.clone(),
                    };

                    return GraphInline::Link(url.clone(), title.clone(), *link_type, new_inlines);
                }

                return self.clone();
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
            GraphInline::Link(_, title, link_type, _) => {
                if self.is_ref() && self.ref_key().map_or(false, |key| key.eq(target_key)) {
                    return GraphInline::Link(
                        updated_key.to_string(),
                        title.clone(),
                        *link_type,
                        vec![],
                    );
                }

                return self.clone();
            }
            _ => self.clone(),
        }
    }

    pub fn is_ref(&self) -> bool {
        match self {
            GraphInline::Link(url, _, _, _) => model::is_ref_url(url),
            _ => false,
        }
    }

    fn ref_key(&self) -> Option<Key> {
        match self {
            GraphInline::Link(url, _, _, _) => Some(Key::name(url)),
            _ => None,
        }
    }
}

fn left_pad_and_prefix(text: &str) -> String {
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push_str("\n");
        } else if n == 0 {
            result.push_str(&format!("- {}\n", line));
        } else {
            result.push_str(&format!("  {}\n", line));
        }
    }

    result
}

fn left_pad_and_prefix_num(text: &str, num: usize) -> String {
    let prefix = format!("{}.{}", num, if num > 9 { "" } else { " " });
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push_str("\n");
        } else if n == 0 {
            result.push_str(&format!("{} {}\n", prefix, line));
        } else {
            result.push_str(&format!("{} {}\n", " ".repeat(prefix.len()), line));
        }
    }

    result
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

pub fn blocks_to_markdown_and(blocks: &Blocks, sparce: bool, options: &MarkdownOptions) -> String {
    blocks
        .iter()
        .map(|block| block.to_markdown(options))
        .collect::<Vec<String>>()
        .join(if sparce { "\n" } else { "" })
}

pub fn blocks_to_markdown(blocks: &Blocks, options: &MarkdownOptions) -> String {
    blocks
        .iter()
        .map(|block| block.to_markdown(options))
        .collect::<Vec<String>>()
        .join("")
}

pub fn blocks_to_markdown_sparce(blocks: &Blocks, options: &MarkdownOptions) -> String {
    blocks
        .iter()
        .map(|block| block.to_markdown(options))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn to_graph_inlines(content: &DocumentInlines, relative_to: &str) -> Vec<GraphInline> {
    content
        .iter()
        .map(|i| i.to_graph_inline(relative_to))
        .collect()
}
