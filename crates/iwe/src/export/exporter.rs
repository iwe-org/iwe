use anyhow::Result;

use crate::export::graph_data::GraphData;

pub trait GraphExporter {
    fn export(&self, graph_data: &GraphData) -> Result<String>;
}
