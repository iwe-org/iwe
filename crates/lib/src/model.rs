use std::{collections::HashMap, ops::Range};

use serde::Serialize;

pub type Key = String;

pub type MaybeKey = Option<Key>;

pub type Content = String;
pub type Document = (Key, Content);
pub type State = HashMap<Key, Content>;

pub type NodeId = u64;
pub type MaybeNodeId = Option<NodeId>;

pub type LineId = usize;
pub type MaybeLineId = Option<LineId>;

pub type StrId = usize;
pub type MaybeStrId = Option<StrId>;

type MaybeString = Option<String>;

pub type LineNumber = usize;
pub type LineRange = Range<LineNumber>;
pub type NodesMap = Vec<(NodeId, LineRange)>;
pub type DocumentNodesMap = (Key, NodesMap);

pub type Lang = String;
pub type Url = String;

pub type Level = u8;
pub type Title = String;

pub mod document;
pub mod graph;
pub mod rank;

pub trait InlinesContext: Copy {
    fn get_ref_title(&self, key: Key) -> Option<String>;
}

pub fn is_ref_url(url: &str) -> bool {
    !(url.to_lowercase().starts_with("http://")
        || url.to_lowercase().starts_with("https://")
        || url.to_lowercase().starts_with("mailto:"))
}
