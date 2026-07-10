use liwe::graph::{Graph, GraphContext};
use liwe::model::node::{Node, NodeIter, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::{Key, NodeId};

pub use liwe::operations::{
    delete, extract, extract_all, inline, rename, Changes, ExtractConfig, InlineConfig,
    OperationError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionRef {
    pub number: usize,
    pub title: String,
    pub id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionRef {
    pub number: usize,
    pub title: String,
    pub key: Key,
    pub id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectError<T> {
    NoSelector,
    NotFound(String),
    Ambiguous(String, Vec<T>),
    OutOfRange(usize, usize),
}

pub fn sections(tree: &Tree) -> Vec<SectionRef> {
    let mut out = Vec::new();
    collect_sections(tree, &mut out);
    out
}

fn collect_sections(tree: &Tree, out: &mut Vec<SectionRef>) {
    if let Node::Section(inlines) = &tree.node {
        out.push(SectionRef {
            number: out.len() + 1,
            title: inlines.iter().map(|i| i.plain_text()).collect(),
            id: tree.id.expect("section node has an id"),
        });
    }
    for child in &tree.children {
        collect_sections(child, out);
    }
}

pub fn select_section(
    tree: &Tree,
    title: Option<&str>,
    block: Option<usize>,
) -> Result<SectionRef, SelectError<SectionRef>> {
    let sections = sections(tree);
    if let Some(title) = title {
        let needle = title.to_lowercase();
        let matches: Vec<SectionRef> = sections
            .into_iter()
            .filter(|section| section.title.to_lowercase().contains(&needle))
            .collect();
        match matches.len() {
            0 => Err(SelectError::NotFound(title.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => Err(SelectError::Ambiguous(title.to_string(), matches)),
        }
    } else if let Some(block) = block {
        if block == 0 || block > sections.len() {
            Err(SelectError::OutOfRange(block, sections.len()))
        } else {
            Ok(sections.into_iter().nth(block - 1).unwrap())
        }
    } else {
        Err(SelectError::NoSelector)
    }
}

pub fn references(tree: &Tree) -> Vec<InclusionRef> {
    let mut out = Vec::new();
    collect_references(tree, &mut out);
    out
}

fn collect_references(tree: &Tree, out: &mut Vec<InclusionRef>) {
    if let Node::Reference(reference) = &tree.node {
        out.push(InclusionRef {
            number: out.len() + 1,
            title: reference.text.clone(),
            key: reference.key.clone(),
            id: tree.id.expect("reference node has an id"),
        });
    }
    for child in &tree.children {
        collect_references(child, out);
    }
}

pub fn select_reference(
    tree: &Tree,
    reference: Option<&str>,
    block: Option<usize>,
) -> Result<InclusionRef, SelectError<InclusionRef>> {
    let references = references(tree);
    if let Some(reference) = reference {
        let needle = reference.to_lowercase();
        let matches: Vec<InclusionRef> = references
            .into_iter()
            .filter(|inclusion| {
                inclusion.title.to_lowercase().contains(&needle)
                    || inclusion.key.to_string().to_lowercase().contains(&needle)
            })
            .collect();
        match matches.len() {
            0 => Err(SelectError::NotFound(reference.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => Err(SelectError::Ambiguous(reference.to_string(), matches)),
        }
    } else if let Some(block) = block {
        if block == 0 || block > references.len() {
            Err(SelectError::OutOfRange(block, references.len()))
        } else {
            Ok(references.into_iter().nth(block - 1).unwrap())
        }
    } else {
        Err(SelectError::NoSelector)
    }
}

pub enum AttachTarget {
    AlreadyAttached,
    Update(String),
    Create(String),
}

pub fn attach_reference(
    graph: &Graph,
    target_key: &Key,
    reference_key: &Key,
    reference_text: &str,
) -> AttachTarget {
    let format_options = graph.format_options();
    let reference = Tree {
        id: None,
        node: Node::Reference(Reference {
            key: reference_key.clone(),
            text: reference_text.to_string(),
            reference_type: ReferenceType::Regular,
            url: String::new(),
            display_url: None,
        }),
        children: vec![],
    };

    if graph.get_node_id(target_key).is_some() {
        let tree = graph.collect(target_key);
        if tree.get_all_inclusion_edge_keys().contains(reference_key) {
            return AttachTarget::AlreadyAttached;
        }
        AttachTarget::Update(
            tree.attach(reference)
                .iter()
                .to_text(&target_key.parent(), format_options),
        )
    } else {
        AttachTarget::Create(
            reference
                .iter()
                .to_text(&target_key.parent(), format_options),
        )
    }
}
