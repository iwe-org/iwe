use std::future::Future;
use std::io::{Read, Write};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::graph::Reader;
use crate::model::document::DocumentBlocks;
pub mod reader;

use reader::MarkdownEventsReader;
pub struct MarkdownReader {}

impl MarkdownReader {
    pub fn new() -> MarkdownReader {
        MarkdownReader {}
    }
}

impl Reader for MarkdownReader {
    fn blocks(&self, content: &str) -> DocumentBlocks {
        let mut reader = MarkdownEventsReader::new();
        reader.read(content)
    }
}
