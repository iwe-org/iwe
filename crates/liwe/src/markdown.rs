use crate::graph::Reader;
use crate::model::config::MarkdownOptions;
use crate::model::document::Document;

pub mod reader;

use reader::MarkdownEventsReader;
pub struct MarkdownReader {}

pub mod writer;

impl MarkdownReader {
    pub fn new() -> MarkdownReader {
        MarkdownReader {}
    }
}

impl Reader for MarkdownReader {
    fn document(&self, content: &str, markdown_options: &MarkdownOptions) -> Document {
        let mut reader = MarkdownEventsReader::new_with_options(markdown_options);
        reader.read(content);

        Document {
            blocks: reader.blocks(),
            metadata: reader.metadata(),
        }
    }
}
