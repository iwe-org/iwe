use std::future::Future;
use std::io::{Read, Write};

use crate::graph::Reader;
use crate::model::document::{Document, DocumentBlocks};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
