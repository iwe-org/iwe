use crate::model::InlinesContext;

use crate::model::inline::{to_plain_text, Inlines};
use crate::model::reference::Target;
use crate::model::LineId;

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

    pub fn targets(&self) -> Vec<Target> {
        self.inlines.iter().flat_map(|i| i.targets()).collect()
    }

    pub fn normalize(&self, context: impl InlinesContext) -> Inlines {
        self.inlines.iter().map(|i| i.normalize(context)).collect()
    }
}
