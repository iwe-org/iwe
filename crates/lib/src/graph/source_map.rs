use std::collections::HashMap;

use crate::model::{Key, LineId, LineNumber, LineRange, NodeId};

type LineRanges = Vec<LineRange>;
type NodeRange = (LineRanges, NodeId);

#[derive(Clone, Default)]
pub struct SourceMap {
    // Node id to first line number
    pub line_nubers: HashMap<NodeId, LineNumber>,
}
impl SourceMap {
    fn new() -> SourceMap {
        SourceMap {
            ..Default::default()
        }
    }

    pub fn set_node_line_number(&mut self, node_id: NodeId, line_number: LineNumber) {
        self.line_nubers.insert(node_id, line_number);
    }

    pub fn get_node_line_number(&self, node_id: NodeId) -> Option<LineNumber> {
        self.line_nubers.get(&node_id).cloned()
    }
}
