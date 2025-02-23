use crate::model::InlinesContext;

use crate::model::graph::{to_plain_text, GraphInlines};
use crate::model::{Key, LineId};

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    id: LineId,
    inlines: GraphInlines,
}

impl Line {
    pub fn new(id: LineId, inlines: GraphInlines) -> Line {
        Line { id, inlines }
    }

    pub fn id(&self) -> LineId {
        self.id
    }

    pub fn to_plain_text(&self) -> String {
        to_plain_text(&self.inlines)
    }

    pub fn inlines(&self) -> &GraphInlines {
        &self.inlines
    }

    pub fn ref_keys(&self) -> Vec<Key> {
        self.inlines.iter().flat_map(|i| i.ref_keys()).collect()
    }

    pub fn normalize(&self, context: impl InlinesContext) -> GraphInlines {
        self.inlines.iter().map(|i| i.normalize(context)).collect()
    }
}
