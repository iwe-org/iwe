use crate::model::InlinesContext;

use crate::model::{Key, LineId};
use crate::model::graph::{to_plain_text, Inline, Inlines};

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    id: LineId,
    inlines: Inlines,
}

impl Line {
    pub fn new(id: LineId, inlines: Inlines) -> Line {
        Line { id, inlines }
    }

    pub fn id(&self) -> LineId {
        self.id
    }

    pub fn to_plain_text(&self) -> String {
        to_plain_text(&self.inlines)
    }

    pub fn inlines(&self) -> &Inlines {
        &self.inlines
    }

    pub fn ref_keys(&self) -> Vec<Key> {
        self.inlines.iter().flat_map(|i| i.ref_keys()).collect()
    }

    pub fn normalize(&self, context: impl InlinesContext) -> Inlines {
        self.inlines.iter().map(|i| i.normalize(context)).collect()
    }
}

pub fn from_inlines(inlines: &[Inline]) -> Inlines {
    inlines.to_vec()
}
