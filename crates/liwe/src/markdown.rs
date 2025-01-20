use crate::graph::Reader;
use crate::model::document::Document;

pub mod reader;

use reader::MarkdownEventsReader;
pub struct MarkdownReader {}

impl MarkdownReader {
    pub fn new() -> MarkdownReader {
        MarkdownReader {}
    }
}

impl Reader for MarkdownReader {
    fn document(&self, content: &str) -> Document {
        let mut reader = MarkdownEventsReader::new();
        reader.read(content);

        Document {
            blocks: reader.blocks(),
            metadata: reader.metadata(),
        }
    }
}
