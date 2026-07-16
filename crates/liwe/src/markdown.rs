use crate::graph::Reader;
use crate::model::config::MarkdownOptions;
use crate::model::document::Document;
use crate::model::writer::{
    blocks_to_markdown_sparce, blocks_to_markdown_sparce_skip_frontmatter, Blocks,
};

pub mod reader;

pub use reader::MarkdownEventsReader;
pub struct MarkdownReader {}

pub mod writer;

impl Default for MarkdownReader {
    fn default() -> Self {
        Self::new()
    }
}

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
            frontmatter: reader.frontmatter(),
        }
    }
}

pub struct MarkdownWriter {}

impl Default for MarkdownWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownWriter {
    pub fn new() -> MarkdownWriter {
        MarkdownWriter {}
    }
}

impl MarkdownWriter {
    pub fn write(&self, blocks: &Blocks, markdown_options: &MarkdownOptions) -> String {
        blocks_to_markdown_sparce(blocks, markdown_options)
    }

    pub fn write_skip_frontmatter(
        &self,
        blocks: &Blocks,
        markdown_options: &MarkdownOptions,
    ) -> String {
        blocks_to_markdown_sparce_skip_frontmatter(blocks, markdown_options)
    }
}
