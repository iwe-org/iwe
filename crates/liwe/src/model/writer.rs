use serde_yaml::Mapping;

use crate::markdown::writer::MarkdownWriter;
use crate::model::config::{FormattingOptions, MarkdownOptions};
use crate::model::node::ColumnAlignment;
use crate::model::{Lang, Level};

use super::inline::{inlines_to_markdown, wrap_inlines, Inlines};

pub type Blocks = Vec<Block>;

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Frontmatter(Mapping),
    Plain(Inlines),
    Para(Inlines),
    LineBlock(Vec<Inlines>),
    CodeBlock(Option<Lang>, String),
    RawBlock(String, String),
    BlockQuote(Blocks),
    OrderedList(Vec<Blocks>),
    BulletList(Vec<Blocks>),
    Header(Level, Inlines),
    HorizontalRule,
    Table(Vec<Inlines>, Vec<ColumnAlignment>, Vec<Vec<Inlines>>),
}

#[allow(dead_code)]
impl Block {
    fn is_sparce_list(&self) -> bool {
        match self {
            Block::BulletList(items) => items.iter().any(item_requires_blank_lines),
            Block::OrderedList(items) => items.iter().any(item_requires_blank_lines),
            _ => false,
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, Block::BulletList(_) | Block::OrderedList(_))
    }

    fn is_paragraph(&self) -> bool {
        matches!(self, Block::Plain(_) | Block::Para(_))
    }

    fn requires_blank_line_separation(&self) -> bool {
        matches!(
            self,
            Block::CodeBlock(_, _)
                | Block::Table(_, _, _)
                | Block::BlockQuote(_)
                | Block::HorizontalRule
        )
    }

    fn is_frontmatter(&self) -> bool {
        matches!(self, Block::Frontmatter(_))
    }

    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        self.to_markdown_indented(options, 0)
    }

    pub fn to_markdown_indented(&self, options: &MarkdownOptions, indent: usize) -> String {
        match self {
            Block::Frontmatter(mapping) => {
                format!("---\n{}---\n", frontmatter_to_yaml(mapping))
            }
            Block::Plain(inlines) => format!("{}\n", wrap_inlines(inlines, options, indent)),
            Block::Para(inlines) => format!("{}\n", wrap_inlines(inlines, options, indent)),
            Block::LineBlock(lines) => lines
                .iter()
                .map(|line| inlines_to_markdown(line, options))
                .collect::<Vec<String>>()
                .join("\n"),
            Block::CodeBlock(lang, text) => {
                let fence = options
                    .formatting
                    .code_block_token()
                    .repeat(options.formatting.code_block_token_count());
                lang.clone()
                    .filter(|lang| !lang.trim().is_empty())
                    .map(|lang| {
                        format!(
                            "{} {}\n{}\n{}\n",
                            fence,
                            lang,
                            text.trim_matches('\n'),
                            fence
                        )
                    })
                    .unwrap_or_else(|| {
                        format!("{}\n{}\n{}\n", fence, text.trim_matches('\n'), fence)
                    })
            }
            Block::RawBlock(_, text) => text.clone(),
            Block::BlockQuote(blocks) => {
                blocks_to_markdown_sparce_indented(blocks, options, indent + 2)
                    .lines()
                    .map(|line| format!("> {}", line))
                    .map(|line| line.trim().to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
                    + "\n"
            }
            Block::OrderedList(items) => {
                let content_indent = options
                    .formatting
                    .ordered_list_content_indent()
                    .unwrap_or(0);
                let child_indent = indent
                    + ordered_prefix_indent(items.len(), &options.formatting).max(content_indent);
                items
                    .iter()
                    .enumerate()
                    .map(|(n, item)| {
                        let num = if options.formatting.increment_ordered_list_bullets() {
                            n + 1
                        } else {
                            1
                        };
                        left_pad_and_prefix_num(
                            &blocks_to_markdown_and_indented(
                                item,
                                self.is_sparce_list(),
                                options,
                                child_indent,
                            ),
                            num,
                            options.formatting.ordered_list_token_char(),
                            content_indent,
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(if self.is_sparce_list() { "\n" } else { "" })
            }
            Block::BulletList(items) => {
                let content_indent = options.formatting.bullet_list_content_indent().unwrap_or(0);
                let child_indent = indent
                    + (options.formatting.list_token().chars().count() + 1).max(content_indent);
                items
                    .iter()
                    .map(|item| {
                        left_pad_and_prefix(
                            &blocks_to_markdown_and_indented(
                                item,
                                self.is_sparce_list(),
                                options,
                                child_indent,
                            ),
                            options.formatting.list_token(),
                            content_indent,
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(if self.is_sparce_list() { "\n" } else { "" })
            }
            Block::Header(level, inlines) => {
                format!(
                    "{} {}\n",
                    "#".repeat(*level as usize),
                    inlines_to_markdown(inlines, options)
                )
            }
            Block::HorizontalRule => {
                let fmt = &options.formatting;
                format!("{}\n", fmt.rule_token().repeat(fmt.rule_token_count()))
            }
            Block::Table(_, _, _) => {
                let writer = MarkdownWriter::new(options.clone());
                writer.write(vec![self.clone()])
            }
        }
    }
}

fn ordered_prefix_indent(item_count: usize, formatting: &FormattingOptions) -> usize {
    let last = if formatting.increment_ordered_list_bullets() {
        item_count.max(1)
    } else {
        1
    };
    let prefix = format!("{}{}", last, formatting.ordered_list_token_char());
    prefix.len() + 1
}

fn left_pad_and_prefix(text: &str, list_token: &str, content_indent: usize) -> String {
    let token_width = list_token.chars().count();
    let pad = content_indent.max(token_width + 1);
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push('\n');
        } else if n == 0 {
            result.push_str(&format!(
                "{}{}{}\n",
                list_token,
                " ".repeat(pad - token_width),
                line
            ));
        } else {
            result.push_str(&format!("{}{}\n", " ".repeat(pad), line));
        }
    }

    result
}

fn left_pad_and_prefix_num(
    text: &str,
    num: usize,
    ordered_list_token: char,
    content_indent: usize,
) -> String {
    let prefix = format!("{}{}", num, ordered_list_token);
    let prefix_width = prefix.chars().count();
    let pad = content_indent.max(prefix_width + 1);
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push('\n');
        } else if n == 0 {
            result.push_str(&format!(
                "{}{}{}\n",
                prefix,
                " ".repeat(pad - prefix_width),
                line
            ));
        } else {
            result.push_str(&format!("{}{}\n", " ".repeat(pad), line));
        }
    }

    result
}

pub fn frontmatter_to_yaml(mapping: &Mapping) -> String {
    if mapping.is_empty() {
        return "{}\n".to_string();
    }
    serde_yaml::to_string(mapping).unwrap_or_default()
}

fn ensure_trailing_newline(s: String) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s
    } else {
        s + "\n"
    }
}

pub fn blocks_to_markdown_and(blocks: &Blocks, sparce: bool, options: &MarkdownOptions) -> String {
    blocks_to_markdown_and_indented(blocks, sparce, options, 0)
}

fn item_requires_blank_lines(item: &Blocks) -> bool {
    item.iter().filter(|block| block.is_paragraph()).count() > 1
        || item.windows(2).any(|pair| {
            pair[0].requires_blank_line_separation() || pair[1].requires_blank_line_separation()
        })
}

pub fn blocks_to_markdown_and_indented(
    blocks: &Blocks,
    sparce: bool,
    options: &MarkdownOptions,
    indent: usize,
) -> String {
    let rendered = blocks
        .iter()
        .map(|block| block.to_markdown_indented(options, indent))
        .collect::<Vec<String>>();

    let mut result = String::new();
    for (i, text) in rendered.iter().enumerate() {
        if i > 0
            && (sparce
                || blocks[i - 1].requires_blank_line_separation()
                || blocks[i].requires_blank_line_separation())
        {
            result.push('\n');
        }
        result.push_str(text);
    }

    ensure_trailing_newline(result)
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
    blocks_to_markdown_sparce_indented(blocks, options, 0)
}

pub fn blocks_to_markdown_sparce_indented(
    blocks: &Blocks,
    options: &MarkdownOptions,
    indent: usize,
) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown_indented(options, indent))
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

#[cfg(test)]
pub mod tests {
    use crate::model::config::MarkdownOptions;
    use crate::model::inline::Inline;
    use crate::model::writer::{blocks_to_markdown, Block};
    use indoc::indoc;

    fn plain(text: &str) -> Block {
        Block::Plain(vec![Inline::Str(text.into())])
    }

    #[test]
    fn test_ordered_list_to_markdown() {
        let list = vec![Block::OrderedList(vec![
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
                1. item
                2. item
                3. item
                4. item
                5. item
                6. item
                7. item
                8. item
                9. item
                10. item
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_ordered_list_with_para() {
        let list = vec![Block::OrderedList(vec![vec![
            plain("item1"),
            plain("para"),
        ]])];
        assert_eq!(
            indoc! {"
                1. item1

                   para
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_to_markdown() {
        let list = vec![Block::BulletList(vec![
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
        let list = vec![Block::BulletList(vec![vec![
            plain("item1"),
            Block::BulletList(vec![vec![plain("item2")]]),
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
        let list = vec![Block::BulletList(vec![vec![plain("item1"), plain("para")]])];
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
        let list = vec![Block::BulletList(vec![
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
        let list = vec![Block::BulletList(vec![vec![
            plain("item1"),
            Block::BulletList(vec![vec![
                plain("item2"),
                Block::BulletList(vec![vec![plain("item3")]]),
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
