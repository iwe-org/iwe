use liwe::graph::{Graph, GraphContext};
use liwe::model::config::MarkdownOptions;
use liwe::model::graph::{blocks_to_markdown_sparce_skip_frontmatter, GraphBlock};
use liwe::model::node::NodePointer;
use liwe::model::projector::Projector;
use liwe::model::tree::TreeIter;
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
}

impl<'a> RetrieveRenderer<'a> {
    pub fn new(output: &'a RetrieveOutput, options: &'a MarkdownOptions, graph: &'a Graph) -> Self {
        Self {
            output,
            options,
            graph,
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
        let body = if doc.content.is_empty() {
            String::new()
        } else {
            render_body(self.graph, self.options, &doc.key)
        };
        render_block(&doc.key, &frontmatter, &[], &body)
    }
}

pub struct FindBlockRenderer<'a> {
    options: &'a MarkdownOptions,
    graph: &'a Graph,
}

impl<'a> FindBlockRenderer<'a> {
    pub fn new(options: &'a MarkdownOptions, graph: &'a Graph) -> Self {
        Self { options, graph }
    }

    pub fn render(
        &self,
        keys: &[Key],
        results: &[Mapping],
        content_output_names: &[String],
    ) -> String {
        keys.iter()
            .zip(results.iter())
            .map(|(key, fm)| {
                let body = render_body(self.graph, self.options, &key.to_string());
                render_block(&key.to_string(), fm, content_output_names, &body)
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

fn build_retrieve_frontmatter(doc: &DocumentOutput) -> Mapping {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("title".to_string()),
        Value::String(doc.title.clone()),
    );
    if !doc.included_by.is_empty() {
        fm.insert(
            Value::String("includedBy".to_string()),
            edges_to_value(&doc.included_by),
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

fn render_content(graph: &Graph, key: &Key) -> Vec<GraphBlock> {
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
