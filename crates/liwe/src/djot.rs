use crate::model::config::DjotOptions;
use crate::model::document::Document;

pub mod reader;
pub mod writer;

pub use writer::DjotWriter;

use reader::DjotEventsReader;

pub struct DjotReader {}

impl Default for DjotReader {
    fn default() -> Self {
        Self::new()
    }
}

impl DjotReader {
    pub fn new() -> DjotReader {
        DjotReader {}
    }

    pub fn document(&self, content: &str, options: &DjotOptions) -> Document {
        let mut reader = DjotEventsReader::new_with_options(options);
        reader.read(content);

        Document {
            blocks: reader.blocks(),
            frontmatter: reader.frontmatter(),
        }
    }
}
