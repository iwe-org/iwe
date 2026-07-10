use crate::graph::Reader;
use crate::markdown::{MarkdownReader, MarkdownWriter};
use crate::model::config::{FormatOptions, MarkdownOptions};
use crate::model::document::Document;
use crate::model::writer::Blocks;

#[cfg(feature = "djot")]
use crate::model::config::DjotOptions;

/// The format boundary: a reader/writer pair for one document format, constructed with its own
/// options. Markdown is always built in; djot lives behind the `djot` feature. A consumer can
/// implement this trait to inject another format.
pub trait DocumentFormat {
    fn read(&self, content: &str) -> Document;
    fn write(&self, blocks: &Blocks) -> String;
    fn write_skip_frontmatter(&self, blocks: &Blocks) -> String;
}

pub struct MarkdownFormat {
    options: MarkdownOptions,
}

impl MarkdownFormat {
    pub fn new(options: MarkdownOptions) -> Self {
        Self { options }
    }
}

impl DocumentFormat for MarkdownFormat {
    fn read(&self, content: &str) -> Document {
        MarkdownReader::new().document(content, &self.options)
    }

    fn write(&self, blocks: &Blocks) -> String {
        MarkdownWriter::new().write(blocks, &self.options)
    }

    fn write_skip_frontmatter(&self, blocks: &Blocks) -> String {
        MarkdownWriter::new().write_skip_frontmatter(blocks, &self.options)
    }
}

#[cfg(feature = "djot")]
pub struct DjotFormat {
    options: DjotOptions,
}

#[cfg(feature = "djot")]
impl DjotFormat {
    pub fn new(options: DjotOptions) -> Self {
        Self { options }
    }
}

#[cfg(feature = "djot")]
impl DocumentFormat for DjotFormat {
    fn read(&self, content: &str) -> Document {
        crate::djot::DjotReader::new().document(content, &self.options)
    }

    fn write(&self, blocks: &Blocks) -> String {
        crate::djot::DjotWriter::new().write(blocks, &self.options)
    }

    fn write_skip_frontmatter(&self, blocks: &Blocks) -> String {
        crate::djot::DjotWriter::new().write_skip_frontmatter(blocks, &self.options)
    }
}

/// Build the built-in [`DocumentFormat`] for the given options. Djot resolves only when the
/// `djot` feature is enabled; otherwise it falls back to markdown.
pub fn format_for(format: &FormatOptions) -> Box<dyn DocumentFormat> {
    match format {
        FormatOptions::Markdown(options) => Box::new(MarkdownFormat::new(options.clone())),
        #[cfg(feature = "djot")]
        FormatOptions::Djot(options) => Box::new(DjotFormat::new(options.clone())),
        #[cfg(not(feature = "djot"))]
        FormatOptions::Djot(_) => Box::new(MarkdownFormat::new(MarkdownOptions::default())),
    }
}

pub fn read_document(content: &str, format: &FormatOptions) -> Document {
    match format {
        FormatOptions::Markdown(options) => MarkdownReader::new().document(content, options),
        #[cfg(feature = "djot")]
        FormatOptions::Djot(options) => crate::djot::DjotReader::new().document(content, options),
        #[cfg(not(feature = "djot"))]
        FormatOptions::Djot(_) => {
            MarkdownReader::new().document(content, &MarkdownOptions::default())
        }
    }
}

pub fn write_document(blocks: &Blocks, format: &FormatOptions) -> String {
    match format {
        FormatOptions::Markdown(options) => MarkdownWriter::new().write(blocks, options),
        #[cfg(feature = "djot")]
        FormatOptions::Djot(options) => crate::djot::DjotWriter::new().write(blocks, options),
        #[cfg(not(feature = "djot"))]
        FormatOptions::Djot(_) => MarkdownWriter::new().write(blocks, &MarkdownOptions::default()),
    }
}

pub fn write_document_skip_frontmatter(blocks: &Blocks, format: &FormatOptions) -> String {
    match format {
        FormatOptions::Markdown(options) => {
            MarkdownWriter::new().write_skip_frontmatter(blocks, options)
        }
        #[cfg(feature = "djot")]
        FormatOptions::Djot(options) => {
            crate::djot::DjotWriter::new().write_skip_frontmatter(blocks, options)
        }
        #[cfg(not(feature = "djot"))]
        FormatOptions::Djot(_) => {
            MarkdownWriter::new().write_skip_frontmatter(blocks, &MarkdownOptions::default())
        }
    }
}
