use std::{collections::HashMap, fmt::Display, ops::Range, path::Path, sync::Arc};

use percent_encoding::percent_decode_str;
use relative_path::RelativePath;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use config::RefsPath;

pub mod config;
pub mod document;
pub mod ids;
pub mod inline;
pub mod key_index;
pub mod node;
pub mod node_iter;
pub mod node_pointer;
pub mod projector;
pub mod reference;
pub mod tree;
pub mod tree_iter;
pub mod writer;

pub use document::{
    Attributes, BlockQuote, BulletList, Code, CodeBlock, Div, Document, DocumentBlock,
    DocumentBlocks, DocumentInline, DocumentInlines, Emph, Format, Header, HorizontalRule, Image,
    LineBlock, LineBreak, Link, LinkType, Math, MathType, OrderedList, Para, Plain, RawBlock,
    RawInline, SmallCaps, SoftBreak, Space, Strikeout, Strong, Subscript, Superscript, Target,
    Underline,
};
pub use inline::{inlines_to_markdown, to_graph_inlines, to_plain_text, Inline, Inlines};
pub use node::{ColumnAlignment, Node, Reference, ReferenceType};
pub use node_iter::NodeIter;
pub use node_pointer::NodePointer;
pub use projector::Projector;
pub use tree::Tree;
pub use tree_iter::TreeIter;
pub use writer::{blocks_to_markdown, Block};

use writer::frontmatter_to_yaml;

pub type Frontmatter = serde_yaml::Mapping;

pub fn frontmatter_from_str(raw: &str) -> Option<Frontmatter> {
    if raw.trim().is_empty() {
        return Some(Frontmatter::new());
    }
    match serde_yaml::from_str::<serde_yaml::Value>(raw.trim()).ok()? {
        serde_yaml::Value::Mapping(mapping) => Some(mapping),
        _ => None,
    }
}

pub fn frontmatter_to_string(frontmatter: &Frontmatter) -> String {
    if frontmatter.is_empty() {
        return String::new();
    }
    frontmatter_to_yaml(frontmatter)
        .trim_start_matches("---\n")
        .trim_end()
        .to_string()
}

pub type Markdown = String;

pub type MaybeKey = Option<Key>;

pub type Content = String;
pub type State = HashMap<String, String>;

pub type NodeId = i64;
pub type MaybeNodeId = Option<NodeId>;

pub type LineId = i64;
pub type MaybeLineId = Option<LineId>;

pub type StrId = usize;
pub type MaybeStrId = Option<StrId>;

pub type LineNumber = usize;
pub type LineRange = Range<LineNumber>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    text: Arc<str>,
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Key {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn parent(&self) -> String {
        RelativePath::new(self.as_str())
            .parent()
            .map(|p| p.to_string())
            .unwrap_or_default()
    }

    pub fn source(&self) -> String {
        RelativePath::new(self.as_str())
            .file_name()
            .unwrap_or("")
            .to_string()
    }

    pub fn path(&self) -> Option<String> {
        RelativePath::new(self.as_str())
            .parent()
            .map(|p| p.to_string())
    }

    pub fn combine(parent: &str, id: &str) -> Key {
        let path = RelativePath::new(parent).join(id).to_string();
        Key {
            text: Arc::from(path),
        }
    }

    pub fn name(name: &str) -> Self {
        Key {
            text: Arc::from(strip_doc_extension(name)),
        }
    }

    pub fn from_stripped(key: &str) -> Self {
        Key {
            text: Arc::from(key),
        }
    }

    pub fn from_rel_link_url(url: &str, relative_to: &str) -> Self {
        let decoded = percent_decode_str(url).decode_utf8_lossy().into_owned();
        let without_fragment = match decoded.split_once('#') {
            Some((path, _)) => path,
            None => decoded.as_str(),
        };
        let key = strip_doc_extension(without_fragment);
        let base = if key.starts_with('/') {
            ""
        } else {
            relative_to
        };
        let path = RelativePath::new(base).join_normalized(key).to_string();
        Key {
            text: Arc::from(path),
        }
    }

    pub fn to_rel_link_url(&self, relative_to: &str) -> String {
        RelativePath::new(relative_to)
            .relative(self.as_str())
            .to_string()
    }

    pub fn to_library_url(&self) -> String {
        self.as_str().to_string()
    }

    pub fn link_url(&self, relative_to: &str, refs_path: RefsPath) -> String {
        match refs_path {
            RefsPath::Relative => self.to_rel_link_url(relative_to),
            RefsPath::Absolute => format!("/{}", self.to_library_url()),
        }
    }

    pub fn last_url_segment(&self) -> String {
        self.as_str().to_string()
    }

    pub fn to_path(&self, format: config::Format) -> String {
        format!("{}.{}", self.as_str(), format.extension())
    }

    pub fn from_path(path: &Path) -> Option<Key> {
        let name = path.file_name()?.to_string_lossy().to_string();
        Some(Key::name(&name))
    }
}

impl Default for Key {
    fn default() -> Self {
        Key::name("")
    }
}

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Key::name(&s))
    }
}

pub fn strip_doc_extension(name: &str) -> &str {
    name.strip_suffix(".md")
        .or_else(|| name.strip_suffix(".dj"))
        .unwrap_or(name)
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::name(value)
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::name(&value)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Default, Hash)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

impl From<(usize, usize)> for Position {
    fn from(value: (usize, usize)) -> Self {
        Position {
            line: value.0,
            character: value.1,
        }
    }
}

pub type InlineRange = Range<Position>;

pub type NodesMap = Vec<(NodeId, LineRange)>;
pub type DocumentNodesMap = (Key, NodesMap);

pub type Lang = String;
pub type LibraryUrl = String;

pub type Level = u8;
pub type Title = String;

pub trait InlinesContext: Copy {
    fn get_ref_title(&self, key: &Key) -> Option<String>;
    fn wiki_display(&self, key: &Key, original_url: &str) -> String;
    fn normalize_ref_text(&self) -> bool;
}

pub fn is_ref_url(url: &str) -> bool {
    !(url.to_lowercase().starts_with("http://")
        || url.to_lowercase().starts_with("https://")
        || url.to_lowercase().starts_with("mailto:"))
}

pub fn normalize_url(url: &str, extension: &str) -> String {
    if !is_ref_url(url) {
        return url.to_string();
    }
    match url.split_once('#') {
        Some((path, fragment)) => format!(
            "{}#{}",
            path.strip_suffix(extension).unwrap_or(path),
            fragment
        ),
        None => url.strip_suffix(extension).unwrap_or(url).to_string(),
    }
}
