use liwe::graph::{Graph, GraphContext};
use liwe::model::config::MarkdownOptions;
use liwe::model::graph::{blocks_to_markdown_sparce, GraphBlock};
use liwe::model::node::NodePointer;
use liwe::model::projector::Projector;
use liwe::model::tree::TreeIter;
use liwe::model::Key;
use liwe::retrieve::{DocumentOutput, EdgeRef, RetrieveOutput};
use serde::Serialize;

#[derive(Serialize)]
struct Frontmatter {
    title: String,
    #[serde(rename = "includedBy", skip_serializing_if = "Vec::is_empty")]
    included_by: Vec<EdgeRefMeta>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    includes: Vec<EdgeRefMeta>,
    #[serde(rename = "referencedBy", skip_serializing_if = "Vec::is_empty")]
    referenced_by: Vec<EdgeRefMeta>,
}

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
        let frontmatter = Frontmatter {
            title: doc.title.clone(),
            included_by: doc.included_by.iter().map(EdgeRefMeta::from).collect(),
            includes: doc.includes.iter().map(EdgeRefMeta::from).collect(),
            referenced_by: doc.referenced_by.iter().map(EdgeRefMeta::from).collect(),
        };
        let yaml = serde_yaml::to_string(&frontmatter).expect("frontmatter serializes");

        let body = if doc.content.is_empty() {
            String::new()
        } else {
            let key = Key::name(&doc.key);
            self.render_content_to_string(&key)
        };

        let fence = "`".repeat(outer_fence_len(&body));

        let mut block = String::new();
        block.push_str(&fence);
        block.push_str("markdown #");
        block.push_str(&doc.key);
        block.push('\n');
        block.push_str("---\n");
        block.push_str(&yaml);
        block.push_str("---\n");
        if !body.is_empty() {
            block.push('\n');
            block.push_str(body.trim_end_matches('\n'));
            block.push('\n');
        }
        block.push_str(&fence);
        block.push('\n');
        block
    }

    fn render_content_to_string(&self, key: &Key) -> String {
        let blocks = self.render_content(key);
        blocks_to_markdown_sparce(&blocks, self.options)
    }

    fn render_content(&self, key: &Key) -> Vec<GraphBlock> {
        let tree = self.graph.collect(key);

        let parent_lookup = |ref_key: &Key| -> Vec<(Key, String)> {
            let refs = self.graph.get_inclusion_edges_to(ref_key);
            let mut parents = Vec::new();

            for ref_id in refs {
                let node = self.graph.node(ref_id);
                if let Some(doc_node) = node.to_document() {
                    if let Some(doc_key) = doc_node.document_key() {
                        if doc_key == *key {
                            continue;
                        }
                        let title = self
                            .graph
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
