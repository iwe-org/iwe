use liwe::graph::{Graph, GraphContext};
use liwe::model::config::MarkdownOptions;
use liwe::model::graph::{blocks_to_markdown_sparce, GraphBlock};
use liwe::model::node::NodePointer;
use liwe::model::projector::Projector;
use liwe::model::tree::TreeIter;
use liwe::model::Key;
use liwe::retrieve::{DocumentOutput, RetrieveOutput};
use serde::Serialize;

#[derive(Serialize)]
struct DocumentFrontmatter {
    document: DocumentMeta,
}

#[derive(Serialize)]
struct DocumentMeta {
    key: String,
    title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parents: Vec<LinkMeta>,
    #[serde(rename = "back-links", skip_serializing_if = "Vec::is_empty")]
    back_links: Vec<LinkMeta>,
}

#[derive(Serialize)]
struct LinkMeta {
    key: String,
    title: String,
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
            .join("")
    }

    fn render_document(&self, doc: &DocumentOutput) -> String {
        let mut output = String::new();

        let frontmatter = DocumentFrontmatter {
            document: DocumentMeta {
                key: doc.key.clone(),
                title: doc.title.clone(),
                parents: doc
                    .parent_documents
                    .iter()
                    .map(|p| LinkMeta {
                        key: p.key.clone(),
                        title: p.title.clone(),
                    })
                    .collect(),
                back_links: doc
                    .backlinks
                    .iter()
                    .map(|b| LinkMeta {
                        key: b.key.clone(),
                        title: b.title.clone(),
                    })
                    .collect(),
            },
        };

        if let Ok(yaml) = serde_yaml::to_string(&frontmatter) {
            output.push_str("---\n");
            output.push_str(&yaml);
            output.push_str("---\n\n");
        }

        if !doc.content.is_empty() {
            let key = Key::name(&doc.key);
            let content = self.render_content_to_string(&key);
            output.push_str(&content);
        }

        output.push_str("\n\n");

        output
    }

    fn render_content_to_string(&self, key: &Key) -> String {
        let blocks = self.render_content(key);
        blocks_to_markdown_sparce(&blocks, self.options)
    }

    fn render_content(&self, key: &Key) -> Vec<GraphBlock> {
        let tree = self.graph.collect(key);

        let parent_lookup = |ref_key: &Key| -> Vec<(Key, String)> {
            let refs = self.graph.get_block_references_to(ref_key);
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
