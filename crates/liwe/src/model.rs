use std::{collections::HashMap, fmt::Display, ops::Range, path::PathBuf, sync::Arc};

use relative_path::RelativePath;

pub type Markdown = String;

pub type MaybeKey = Option<Key>;

pub type Content = String;
pub type State = HashMap<String, String>;

pub type NodeId = u64;
pub type MaybeNodeId = Option<NodeId>;

pub type LineId = usize;
pub type MaybeLineId = Option<LineId>;

pub type StrId = usize;
pub type MaybeStrId = Option<StrId>;

pub type LineNumber = usize;
pub type LineRange = Range<LineNumber>;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Default, Hash)]
pub struct Key {
    pub relative_path: Arc<String>,
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.relative_path)
    }
}

impl Key {
    pub fn parent(&self) -> String {
        RelativePath::new(&self.relative_path.to_string())
            .parent()
            .map(|p| p.to_string())
            .unwrap_or_default()
    }

    pub fn from_file_name(name: &str) -> Self {
        let key = name.trim_end_matches(".md").to_string();

        Key {
            relative_path: Arc::new(key),
        }
    }

    pub fn from_rel_link_url(url: &str, relative_to: &str) -> Self {
        let key = url.trim_end_matches(".md").to_string();
        let path = RelativePath::new(relative_to).join(key).to_string();
        Key {
            relative_path: Arc::new(path),
        }
    }

    pub fn to_rel_link_url(&self, relative_to: &str) -> String {
        RelativePath::new(relative_to)
            .relative(self.relative_path.to_string())
            .to_string()
    }

    pub fn last_url_segment(&self) -> String {
        (&format!("{}", self.relative_path)).to_string()
    }

    pub fn to_path(&self) -> String {
        (&format!("{}.md", self.relative_path)).to_string()
    }

    pub fn from_path(path: &PathBuf) -> Key {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let key = name.trim_end_matches(".md").to_string();
        Key::from_file_name(&key)
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::from_file_name(value)
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::from_file_name(&value)
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

pub mod document;
pub mod graph;
pub mod rank;

pub trait InlinesContext: Copy {
    fn get_ref_title(&self, key: &Key) -> Option<String>;
}

pub fn is_ref_url(url: &str) -> bool {
    !(url.to_lowercase().starts_with("http://")
        || url.to_lowercase().starts_with("https://")
        || url.to_lowercase().starts_with("mailto:"))
}
