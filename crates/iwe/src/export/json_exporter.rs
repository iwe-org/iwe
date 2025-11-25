use anyhow::Result;

use super::{exporter::GraphExporter, graph_data::GraphData};

#[derive(Default, Debug)]
pub struct JsonExporter {}

impl GraphExporter for JsonExporter {
    fn export(&self, graph_data: &GraphData) -> Result<String> {
        Ok(serde_json::to_string(graph_data)?)
    }
}
