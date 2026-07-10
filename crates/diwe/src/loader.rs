use std::path::Path;

use liwe::graph::Graph;
use liwe::model::config::FormatOptions;

use crate::fs::new_for_path;

pub fn from_path(
    base_path: &Path,
    sequential_ids: bool,
    format_options: impl Into<FormatOptions>,
    frontmatter_document_title: Option<String>,
) -> Graph {
    let format_options = format_options.into();
    let state = new_for_path(&base_path.to_path_buf(), format_options.format());
    Graph::from_state(
        &state,
        sequential_ids,
        format_options,
        frontmatter_document_title,
    )
}
