use liwe::graph::{Graph, GraphContext};
use liwe::model::config::MarkdownOptions;
use liwe::model::node::NodePointer;
use liwe::model::projector::Projector;
use liwe::model::tree::TreeIter;
use liwe::model::writer::{blocks_to_markdown_sparce_skip_frontmatter, Block};
use liwe::model::Key;
use liwe::retrieve::{DocumentOutput, EdgeRef, RetrieveOutput};
use serde::Serialize;
use serde_yaml::{Mapping, Value};

#[derive(Serialize)]
struct EdgeRefMeta {
    key: String,
    title: String,
    #[serde(rename = "sectionPath", skip_serializing_if = "Vec::is_empty")]
    section_path: Vec<String>,
}

impl From<&EdgeRef> for EdgeRefMeta {
    fn from(e: &EdgeRef) -> Self {
        EdgeRefMeta {
            key: e.key.clone(),
            title: e.title.clone(),
            section_path: e.section_path.clone(),
        }
    }
}

pub struct RetrieveRenderer<'a> {
    output: &'a RetrieveOutput,
    options: &'a MarkdownOptions,
    graph: &'a Graph,
    max_document_tokens: Option<usize>,
}

impl<'a> RetrieveRenderer<'a> {
    pub fn new(
        output: &'a RetrieveOutput,
        options: &'a MarkdownOptions,
        graph: &'a Graph,
        max_document_tokens: Option<usize>,
    ) -> Self {
        Self {
            output,
            options,
            graph,
            max_document_tokens,
        }
    }

    pub fn render(&self) -> String {
        self.output
            .documents
            .iter()
            .map(|doc| self.render_document(doc))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn render_document(&self, doc: &DocumentOutput) -> String {
        let frontmatter = build_retrieve_frontmatter(doc);
        let clipped = self.output.truncation.clipped.iter().any(|k| k == &doc.key);
        let body = if doc.content.is_empty() {
            String::new()
        } else {
            let rendered = render_body(self.graph, self.options, &doc.key);
            truncate_rendered_body(rendered, self.max_document_tokens, clipped)
        };
        render_block(&doc.key, &frontmatter, &[], &body)
    }
}

pub struct FindBlockRenderer<'a> {
    options: &'a MarkdownOptions,
    graph: &'a Graph,
    max_document_tokens: Option<usize>,
    clipped: &'a [String],
}

impl<'a> FindBlockRenderer<'a> {
    pub fn new(
        options: &'a MarkdownOptions,
        graph: &'a Graph,
        max_document_tokens: Option<usize>,
        clipped: &'a [String],
    ) -> Self {
        Self {
            options,
            graph,
            max_document_tokens,
            clipped,
        }
    }

    pub fn render(
        &self,
        keys: &[Key],
        results: &[Mapping],
        content_output_names: &[String],
    ) -> String {
        if content_output_names.is_empty() {
            keys.iter()
                .zip(results.iter())
                .map(|(key, fm)| render_index_line(key, fm))
                .collect::<String>()
        } else {
            keys.iter()
                .zip(results.iter())
                .map(|(key, fm)| {
                    let key_str = key.to_string();
                    let clipped = self.clipped.iter().any(|k| k == &key_str);
                    let rendered = render_body(self.graph, self.options, &key_str);
                    let body = truncate_rendered_body(rendered, self.max_document_tokens, clipped);
                    render_block(&key_str, fm, content_output_names, &body)
                })
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
}

fn truncate_rendered_body(
    body: String,
    max_document_tokens: Option<usize>,
    clipped: bool,
) -> String {
    if !clipped {
        return body;
    }
    match max_document_tokens.filter(|&m| m > 0) {
        Some(max) => {
            let (head, omitted) = liwe::tokens::truncate_to_tokens(&body, max);
            if omitted > 0 {
                format!(
                    "{}{}",
                    head.trim_end_matches('\n'),
                    liwe::tokens::truncation_marker(omitted)
                )
            } else {
                body
            }
        }
        None => body,
    }
}

fn render_index_line(key: &Key, fm: &Mapping) -> String {
    let title = fm
        .get(Value::String("title".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| key.to_string());

    let mut edges = String::new();
    let mut annotations = String::new();

    for (k, v) in fm {
        let name = match k.as_str() {
            Some(s) => s,
            None => continue,
        };
        if name == "key" || name == "title" {
            continue;
        }
        match edge_direction(name) {
            Some(dir) => {
                if let Some(rendered) = render_edge_field(dir, v) {
                    edges.push(' ');
                    edges.push_str(&rendered);
                }
            }
            None => {
                annotations.push_str(" · ");
                annotations.push_str(name);
                annotations.push_str(": ");
                annotations.push_str(&render_annotation_value(v));
            }
        }
    }

    format!("- [{}]({}){}{}\n", title, key, edges, annotations)
}

fn edge_direction(name: &str) -> Option<&'static str> {
    match name {
        "includedBy" | "referencedBy" => Some("<-"),
        "includes" | "references" => Some("->"),
        _ => None,
    }
}

fn render_edge_field(dir: &str, v: &Value) -> Option<String> {
    let seq = v.as_sequence()?;
    let targets: Vec<String> = seq
        .iter()
        .filter_map(|item| {
            let m = item.as_mapping()?;
            let key = m.get(Value::String("key".to_string()))?.as_str()?;
            let title = m
                .get(Value::String("title".to_string()))
                .and_then(|t| t.as_str())
                .unwrap_or(key);
            Some(format!("[{}]({})", title, key))
        })
        .collect();
    if targets.is_empty() {
        return None;
    }
    Some(format!("{} {}", dir, targets.join(", ")))
}

fn is_scalar(v: &Value) -> bool {
    matches!(
        v,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn render_annotation_value(v: &Value) -> String {
    match v {
        Value::Sequence(items) if items.iter().all(is_scalar) => items
            .iter()
            .map(inline_value)
            .collect::<Vec<String>>()
            .join(", "),
        _ => inline_value(v),
    }
}

fn inline_value(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Sequence(items) => {
            let inner = items
                .iter()
                .map(inline_value)
                .collect::<Vec<String>>()
                .join(", ");
            format!("[{}]", inner)
        }
        Value::Mapping(m) => {
            let inner = m
                .iter()
                .filter_map(|(k, val)| {
                    k.as_str()
                        .map(|name| format!("{}: {}", name, inline_value(val)))
                })
                .collect::<Vec<String>>()
                .join(", ");
            format!("{{{}}}", inner)
        }
        Value::Tagged(t) => inline_value(&t.value),
    }
}

fn build_retrieve_frontmatter(doc: &DocumentOutput) -> Mapping {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("title".to_string()),
        Value::String(doc.title.clone()),
    );
    if !doc.references.is_empty() {
        fm.insert(
            Value::String("references".to_string()),
            edges_to_value(&doc.references),
        );
    }
    if !doc.includes.is_empty() {
        fm.insert(
            Value::String("includes".to_string()),
            edges_to_value(&doc.includes),
        );
    }
    if !doc.referenced_by.is_empty() {
        fm.insert(
            Value::String("referencedBy".to_string()),
            edges_to_value(&doc.referenced_by),
        );
    }
    if !doc.included_by.is_empty() {
        fm.insert(
            Value::String("includedBy".to_string()),
            edges_to_value(&doc.included_by),
        );
    }
    fm
}

fn edges_to_value(edges: &[EdgeRef]) -> Value {
    let metas: Vec<EdgeRefMeta> = edges.iter().map(EdgeRefMeta::from).collect();
    serde_yaml::to_value(metas).expect("edges serialize")
}

fn render_block(
    key: &str,
    frontmatter: &Mapping,
    omit_in_frontmatter: &[String],
    body: &str,
) -> String {
    let trimmed = trim_frontmatter(frontmatter, omit_in_frontmatter);
    let fence = "`".repeat(outer_fence_len(body));
    let has_frontmatter = !trimmed.is_empty();

    let mut block = String::new();
    block.push_str(&fence);
    block.push_str("markdown #");
    block.push_str(key);
    block.push('\n');

    if has_frontmatter {
        let yaml = serde_yaml::to_string(&Value::Mapping(trimmed)).expect("frontmatter serializes");
        block.push_str("---\n");
        block.push_str(&yaml);
        block.push_str("---\n");
    }

    if !body.is_empty() {
        if has_frontmatter {
            block.push('\n');
        }
        block.push_str(body.trim_end_matches('\n'));
        block.push('\n');
    }
    block.push_str(&fence);
    block.push('\n');
    block
}

fn trim_frontmatter(fm: &Mapping, omit: &[String]) -> Mapping {
    let mut out = Mapping::new();
    for (k, v) in fm {
        let name = match k.as_str() {
            Some(s) => s,
            None => continue,
        };
        if name == "key" {
            continue;
        }
        if omit.iter().any(|o| o == name) {
            continue;
        }
        let trimmed = strip_empty(v.clone());
        if is_empty_collection(&trimmed) {
            continue;
        }
        out.insert(k.clone(), trimmed);
    }
    out
}

fn strip_empty(v: Value) -> Value {
    match v {
        Value::Sequence(items) => Value::Sequence(items.into_iter().map(strip_empty).collect()),
        Value::Mapping(m) => {
            let mut out = Mapping::new();
            for (k, v) in m {
                let v = strip_empty(v);
                if is_empty_collection(&v) {
                    continue;
                }
                out.insert(k, v);
            }
            Value::Mapping(out)
        }
        other => other,
    }
}

fn is_empty_collection(v: &Value) -> bool {
    match v {
        Value::Sequence(s) => s.is_empty(),
        Value::Mapping(m) => m.is_empty(),
        _ => false,
    }
}

fn render_body(graph: &Graph, options: &MarkdownOptions, key: &str) -> String {
    let key = Key::name(key);
    let blocks = render_content(graph, &key);
    blocks_to_markdown_sparce_skip_frontmatter(&blocks, options)
}

fn render_content(graph: &Graph, key: &Key) -> Vec<Block> {
    let tree = graph.collect(key);

    let parent_lookup = |ref_key: &Key| -> Vec<(Key, String)> {
        let refs = graph.get_inclusion_edges_to(ref_key);
        let mut parents = Vec::new();

        for ref_id in refs {
            let node = graph.node(ref_id);
            if let Some(doc_node) = node.to_document() {
                if let Some(doc_key) = doc_node.document_key() {
                    if doc_key == *key {
                        continue;
                    }
                    let title = graph
                        .get_key_title(&doc_key)
                        .unwrap_or_else(|| doc_key.to_string());
                    if !parents.iter().any(|(k, _)| k == &doc_key) {
                        parents.push((doc_key, title));
                    }
                }
            }
        }

        parents
    };

    let annotated = tree.annotate_references(&parent_lookup, &key.parent());

    Projector::project(TreeIter::new(&annotated), &key.parent())
}

fn outer_fence_len(body: &str) -> usize {
    let mut max_run = 0usize;
    for line in body.lines() {
        let trimmed = line.trim_start_matches(' ');
        let leading = line.len() - trimmed.len();
        if leading > 3 {
            continue;
        }
        let run = trimmed.chars().take_while(|&c| c == '`').count();
        if run > max_run {
            max_run = run;
        }
    }
    std::cmp::max(4, max_run + 1)
}

#[cfg(test)]
mod tests {
    use super::outer_fence_len;

    #[test]
    fn fence_default_is_four() {
        assert_eq!(outer_fence_len(""), 4);
        assert_eq!(outer_fence_len("plain text\nno backticks"), 4);
    }

    #[test]
    fn fence_three_backticks_does_not_escalate() {
        assert_eq!(outer_fence_len("```\ncode\n```"), 4);
    }

    #[test]
    fn fence_escalates_past_inner_four_backticks() {
        assert_eq!(outer_fence_len("````\ninner\n````"), 5);
    }

    #[test]
    fn fence_escalates_past_inner_six_backticks() {
        assert_eq!(outer_fence_len("``````\ninner\n``````"), 7);
    }

    #[test]
    fn fence_ignores_deeply_indented_runs() {
        assert_eq!(outer_fence_len("    ````not a fence"), 4);
    }
}
