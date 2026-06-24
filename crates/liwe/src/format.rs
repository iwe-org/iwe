use crate::djot::{DjotReader, DjotWriter};
use crate::graph::Reader;
use crate::markdown::{MarkdownReader, MarkdownWriter};
use crate::model::config::FormatOptions;
use crate::model::document::Document;
use crate::model::writer::Blocks;

pub fn read_document(content: &str, format: &FormatOptions) -> Document {
    match format {
        FormatOptions::Markdown(options) => MarkdownReader::new().document(content, options),
        FormatOptions::Djot(options) => DjotReader::new().document(content, options),
    }
}

pub fn write_document(blocks: &Blocks, format: &FormatOptions) -> String {
    match format {
        FormatOptions::Markdown(options) => MarkdownWriter::new().write(blocks, options),
        FormatOptions::Djot(options) => DjotWriter::new().write(blocks, options),
    }
}

pub fn write_document_skip_frontmatter(blocks: &Blocks, format: &FormatOptions) -> String {
    match format {
        FormatOptions::Markdown(options) => {
            MarkdownWriter::new().write_skip_frontmatter(blocks, options)
        }
        FormatOptions::Djot(options) => DjotWriter::new().write_skip_frontmatter(blocks, options),
    }
}
