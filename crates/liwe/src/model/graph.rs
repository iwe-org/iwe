use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::model;
use crate::model::document::DocumentInlines;
use crate::model::{InlinesContext, Key, Lang, Level, Title, Url};

use super::document::LinkType;
use super::NodeId;

pub type Blocks = Vec<Block>;
pub type Inlines = Vec<Inline>;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Document(Key),
    Section(Inlines),
    Quote(),
    BulletList(),
    OrderedList(),
    Leaf(Inlines),
    Raw(Option<String>, String),
    HorizontalRule(),
    Reference(Reference),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub key: Key,
    pub text: String,
    pub reference_type: ReferenceType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReferenceType {
    Regular,
    WikiLink,
    WikiLinkPiped,
}

impl ReferenceType {
    pub fn to_link_type(&self) -> LinkType {
        match self {
            ReferenceType::Regular => LinkType::Regular,
            ReferenceType::WikiLink => LinkType::WikiLink,
            ReferenceType::WikiLinkPiped => LinkType::WikiLinkPiped,
        }
    }
}

impl Node {
    pub fn is_doucment(&self) -> bool {
        match self {
            Node::Document(_) => true,
            _ => false,
        }
    }
    pub fn is_section(&self) -> bool {
        match self {
            Node::Section(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Code(Option<Lang>, String),
    Emph(Inlines),
    Image(Url, Title, Inlines),
    LineBreak,
    Link(Url, Title, LinkType, Inlines),
    Math(String),
    RawInline(Lang, String),
    SmallCaps(Inlines),
    SoftBreak,
    Space,
    Str(String),
    Strikeout(Inlines),
    Strong(Inlines),
    Subscript(Inlines),
    Superscript(Inlines),
    Underline(Inlines),
}

pub trait NodeIter<'a>: Sized {
    fn next(&self) -> Option<Self>;
    fn child(&self) -> Option<Self>;
    fn node(&self) -> Option<Node>;
}

pub trait GraphNodeIter<'a>: NodeIter<'a> {
    fn id(&self) -> Option<NodeId>;

    fn collect_tree(self) -> TreeNode {
        TreeNode::from_iter(self).expect("to have node")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub id: Option<NodeId>,
    pub payload: Node,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn iter(&self) -> TreeIter {
        TreeIter::new(self)
    }

    pub fn pre_sub_header_position(&self) -> usize {
        self.children
            .iter()
            .take_while(|child| !child.payload.is_section())
            .count()
    }

    pub fn remove_node(&self, target_id: NodeId) -> TreeNode {
        TreeNode {
            id: self.id,
            payload: self.payload.clone(),
            children: self
                .clone()
                .children
                .iter()
                .filter(|child| !child.id_eq(target_id))
                .map(|child| child.remove_node(target_id))
                .collect(),
        }
    }

    pub fn append_pre_header(&self, target_id: NodeId, new: TreeNode) -> TreeNode {
        let mut children = self.children.clone();

        if self.id_eq(target_id) {
            children.insert(self.pre_sub_header_position(), new.clone());
        }

        TreeNode {
            id: self.id,
            payload: self.payload.clone(),
            children: children
                .into_iter()
                .map(|child| child.append_pre_header(target_id, new.clone()))
                .collect(),
        }
    }

    pub fn id_eq(&self, id: NodeId) -> bool {
        self.id == Some(id)
    }

    pub fn from_iter<'a>(iter: impl GraphNodeIter<'a>) -> Option<TreeNode> {
        let id = iter.id();
        let payload = iter.node()?;
        let mut children = Vec::new();

        iter.child().map(|child| children.push(child));

        if let Some(child) = iter.child() {
            let mut i = child;
            while let Some(next) = i.next() {
                children.push(next);
                i = i.next().unwrap();
            }
        }

        Some(TreeNode {
            id,
            payload,
            children: children
                .into_iter()
                .map(|c| TreeNode::from_iter(c))
                .flatten()
                .collect_vec(),
        })
    }
}

pub struct TreeIter<'a> {
    tree_node: &'a TreeNode,
    path: Vec<usize>,
}

impl<'a> TreeIter<'a> {
    pub fn new(tree_node: &'a TreeNode) -> TreeIter<'a> {
        TreeIter {
            tree_node: &tree_node,
            path: vec![],
        }
    }
}

impl<'a, 'b> NodeIter<'a> for TreeIter<'b> {
    fn next(&self) -> Option<Self> {
        self.path.last().map(|n| {
            let mut path = self.path.clone();
            path.pop();
            path.push(n + 1);

            TreeIter {
                tree_node: self.tree_node,
                path,
            }
        })
    }

    fn child(&self) -> Option<Self> {
        let mut path = self.path.clone();
        path.push(0);

        Some(TreeIter {
            tree_node: self.tree_node,
            path,
        })
    }

    fn node(&self) -> Option<Node> {
        let mut node = self.tree_node;

        for n in self.path.iter() {
            if let Some(n) = &node.children.get(*n) {
                node = n;
            } else {
                return None;
            }
        }

        Some(node.payload.clone())
    }
}

impl<'a, 'b> GraphNodeIter<'a> for TreeIter<'b> {
    fn id(&self) -> Option<NodeId> {
        self.tree_node.id
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct MarkdownOptions {
    pub refs_extension: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct LibraryOptions {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Configuration {
    pub markdown: MarkdownOptions,
    pub library: LibraryOptions,
}

#[allow(dead_code)]
impl Block {
    fn is_sparce_list(&self) -> bool {
        match self {
            Block::BulletList(items) => items
                .iter()
                .any(|item| item.iter().filter(|block| block.is_paragraph()).count() > 1),
            Block::OrderedList(items) => items
                .iter()
                .any(|item| item.iter().filter(|block| block.is_paragraph()).count() > 1),
            _ => false,
        }
    }

    fn is_list(&self) -> bool {
        match self {
            Block::BulletList(_) => true,
            Block::OrderedList(_) => true,
            _ => false,
        }
    }

    fn is_paragraph(&self) -> bool {
        match self {
            Block::Plain(_) => true,
            Block::Para(_) => true,
            _ => false,
        }
    }

    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        match self {
            Block::Plain(inlines) => format!("{}\n", inlines_to_markdown(inlines, options)),
            Block::Para(inlines) => format!("{}\n", inlines_to_markdown(inlines, options)),
            Block::LineBlock(lines) => lines
                .iter()
                .map(|line| inlines_to_markdown(line, options))
                .collect::<Vec<String>>()
                .join("\n"),
            Block::CodeBlock(lang, text) => lang
                .clone()
                .filter(|lang| !lang.trim().is_empty())
                .map(|lang| format!("``` {}\n{}\n```\n", lang, text.trim_matches('\n')))
                .unwrap_or_else(|| format!("```\n{}\n```\n", text.trim_matches('\n'))),
            Block::RawBlock(_, text) => text.clone(),
            Block::BlockQuote(blocks) => {
                blocks_to_markdown_sparce(blocks, options)
                    .lines()
                    .map(|line| format!("> {}", line))
                    .map(|line| line.trim().to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
                    + "\n"
            }
            Block::OrderedList(items) => items
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
            Block::BulletList(items) => items
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
            Block::Header(level, inlines) => {
                format!(
                    "{} {}\n",
                    "#".repeat(*level as usize),
                    inlines_to_markdown(inlines, options)
                )
            }
            Block::HorizontalRule => format!("{}\n", "-".repeat(72)),
        }
    }
}

impl Inline {
    pub fn from_string(str: &str) -> Inlines {
        vec![Inline::Str(str.to_string())]
    }
    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        match self {
            Inline::Str(text) => text.clone(),
            Inline::Emph(emph) => format!("*{}*", inlines_to_markdown(emph, options)),
            Inline::Underline(underline) => inlines_to_markdown(underline, options),
            Inline::Strong(strong) => format!("**{}**", inlines_to_markdown(strong, options)),
            Inline::Strikeout(strikeout) => {
                format!("~~{}~~", inlines_to_markdown(strikeout, options))
            }
            Inline::Superscript(superscript) => {
                format!("^{}^", inlines_to_markdown(superscript, options))
            }
            Inline::Subscript(subscript) => {
                format!("~{}~", inlines_to_markdown(subscript, options))
            }
            Inline::SmallCaps(small_caps) => inlines_to_markdown(small_caps, options),
            Inline::Code(_, text) => format!("`{}`", text),
            Inline::Space => " ".into(),
            Inline::SoftBreak => "\n".into(),
            Inline::LineBreak => "\n".into(),
            Inline::Link(url, _, link_type, inlines) => {
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
            Inline::Image(url, _, inlines) => {
                format!("![{}]({})", inlines_to_markdown(inlines, options), url)
            }
            Inline::RawInline(_, content) => format!("`{}`", content),
            Inline::Math(math) => format!("${}$", math),
        }
    }
    pub fn to_plain_text(&self) -> String {
        match self {
            Inline::Str(text) => text.clone(),
            Inline::Emph(emph) => to_plain_text(emph),
            Inline::Underline(underline) => to_plain_text(underline),
            Inline::Strong(strong) => to_plain_text(strong),
            Inline::Strikeout(strikeout) => to_plain_text(strikeout),
            Inline::Superscript(superscript) => to_plain_text(superscript),
            Inline::Subscript(subscript) => to_plain_text(subscript),
            Inline::SmallCaps(small_caps) => to_plain_text(small_caps),
            Inline::Code(_, text) => text.clone(),
            Inline::Space => " ".into(),
            Inline::SoftBreak => "\n".into(),
            Inline::LineBreak => "\n".into(),
            Inline::Link(_, _, _, inlines) => to_plain_text(inlines),
            Inline::Image(_, _, inlines) => to_plain_text(inlines),
            Inline::RawInline(_, content) => content.clone(),
            _ => "".into(),
        }
    }

    pub fn ref_keys(&self) -> Vec<Key> {
        match self {
            Inline::Emph(emph) => emph.iter().flat_map(|inline| inline.ref_keys()).collect(),
            Inline::Underline(underline) => underline
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Strong(strong) => strong.iter().flat_map(|inline| inline.ref_keys()).collect(),
            Inline::Strikeout(strikeout) => strikeout
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Superscript(superscript) => superscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Subscript(subscript) => subscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::SmallCaps(small_caps) => small_caps
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Link(_, _, _, _) => self.ref_key().map(|key| vec![key]).unwrap_or_default(),
            Inline::Image(_, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            _ => vec![],
        }
    }

    pub fn normalize(&self, context: impl InlinesContext) -> Inline {
        match self {
            Inline::Emph(emph) => Inline::Emph(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            Inline::Strong(emph) => Inline::Strong(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Underline(emph) => Inline::Underline(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            Inline::Strikeout(emph) => Inline::Strikeout(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Superscript(emph) => Inline::Superscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Subscript(emph) => Inline::Subscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::SmallCaps(emph) => Inline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Link(url, title, link_type, inlines) => {
                if self.is_ref() {
                    let new_inlines = match *link_type {
                        LinkType::Regular => context
                            .get_ref_title(&Key::from_rel_link_url(url))
                            .map(|title| vec![Inline::Str(title)])
                            .unwrap_or(inlines.clone()),
                        LinkType::WikiLink => vec![],
                        LinkType::WikiLinkPiped => inlines.clone(),
                    };

                    return Inline::Link(url.clone(), title.clone(), *link_type, new_inlines);
                }

                return self.clone();
            }
            _ => self.clone(),
        }
    }

    pub fn change_key(
        &self,
        target_key: &Key,
        updated_key: &Key,
        context: impl InlinesContext,
    ) -> Inline {
        match self {
            Inline::Emph(emph) => Inline::Emph(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),

            Inline::Strong(emph) => Inline::Strong(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),
            Inline::Underline(emph) => Inline::Underline(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),

            Inline::Strikeout(emph) => Inline::Strikeout(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),
            Inline::Superscript(emph) => Inline::Superscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),
            Inline::Subscript(emph) => Inline::Subscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),
            Inline::SmallCaps(emph) => Inline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key, context))
                    .collect(),
            ),
            Inline::Link(_, title, link_type, _) => {
                if self.is_ref() && self.ref_key().map_or(false, |key| key.eq(target_key)) {
                    return Inline::Link(
                        updated_key.to_string(),
                        context.get_ref_title(target_key).unwrap_or(title.clone()),
                        *link_type,
                        vec![],
                    );
                }

                return self.clone();
            }
            _ => self.clone(),
        }
    }

    fn is_ref(&self) -> bool {
        match self {
            Inline::Link(url, _, _, _) => model::is_ref_url(url),
            _ => false,
        }
    }

    fn ref_key(&self) -> Option<Key> {
        match self {
            Inline::Link(url, _, _, _) => Some(Key::from_rel_link_url(url)),
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
    use crate::model::graph::{blocks_to_markdown, MarkdownOptions};
    use crate::model::graph::{Block, Inline};
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
        let list = vec![Block::OrderedList(vec![vec![
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

pub fn to_plain_text(content: &Inlines) -> String {
    content
        .iter()
        .map(|i| i.to_plain_text())
        .collect::<Vec<String>>()
        .join("")
}

pub fn inlines_to_markdown(content: &Inlines, options: &MarkdownOptions) -> String {
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

pub fn to_node_inlines(content: &DocumentInlines) -> Vec<Inline> {
    content.iter().map(|i| i.to_node_inline()).collect()
}
