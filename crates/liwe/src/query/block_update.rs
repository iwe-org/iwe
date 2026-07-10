use std::collections::HashMap;

use crate::graph::{Graph, GraphContext};
use crate::markdown::MarkdownReader;
use crate::model::config::MarkdownOptions;
use crate::model::inline::Inline;
use crate::model::node::Node;
use crate::model::tree::Tree;
use crate::model::{Key, NodeId};
use crate::query::block::BlockPredicate;
use crate::query::block_eval::{BlockIndex, Target};
use crate::query::document::{BlockUpdate, BlockUpdateOp, Expect};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockRef {
    pub key: String,
    pub path: Vec<String>,
    pub line: String,
}

impl std::fmt::Display for BlockRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  {}", self.key)?;
        for element in &self.path {
            write!(f, " › {}", element)?;
        }
        write!(f, " › \"{}\"", self.line)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocRef {
    pub key: String,
    pub title: String,
}

impl std::fmt::Display for DocRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  {} › {}", self.key, self.title)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    AppendNonContainer {
        blocks: Vec<BlockRef>,
    },
    ReplaceTextNoText {
        blocks: Vec<BlockRef>,
    },
    FragmentNotList {
        op: &'static str,
        blocks: Vec<BlockRef>,
    },
    ReplaceTextAnchor {
        blocks: Vec<BlockRef>,
    },
    Overlap {
        op_a: &'static str,
        op_b: &'static str,
        block: BlockRef,
    },
    Expect {
        op: &'static str,
        expected: Expect,
        actual: usize,
        blocks: Vec<BlockRef>,
    },
    ExpectDocuments {
        op: &'static str,
        expected: Expect,
        actual: usize,
        documents: Vec<DocRef>,
    },
    SearchIndexMissing,
}

fn blocks_str(blocks: &[BlockRef]) -> String {
    blocks
        .iter()
        .map(|block| block.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn docs_str(documents: &[DocRef]) -> String {
    documents
        .iter()
        .map(|doc| doc.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn fmt_expect(expected: &Expect, unit: &str) -> String {
    let count = |n: u64| {
        if n == 1 {
            format!("1 {}", unit)
        } else {
            format!("{} {}s", n, unit)
        }
    };
    match expected {
        Expect::Exactly(n) => count(*n),
        Expect::Range {
            min: Some(min),
            max: Some(max),
        } => format!("between {} and {} {}s", min, max, unit),
        Expect::Range {
            min: Some(min),
            max: None,
        } => format!("at least {}", count(*min)),
        Expect::Range {
            min: None,
            max: Some(max),
        } => format!("at most {}", count(*max)),
        Expect::Range {
            min: None,
            max: None,
        } => format!("any number of {}s", unit),
    }
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::AppendNonContainer { blocks } => write!(
                f,
                "$append target is not a container (header, item, list, or quote)\n{}",
                blocks_str(blocks)
            ),
            EvalError::ReplaceTextNoText { blocks } => write!(
                f,
                "$replaceText target has no editable own text\n{}",
                blocks_str(blocks)
            ),
            EvalError::FragmentNotList { op, blocks } => write!(
                f,
                "{} content must be a single list to attach here\n{}",
                op,
                blocks_str(blocks)
            ),
            EvalError::ReplaceTextAnchor { blocks } => write!(
                f,
                "$replaceText 'from' must occur exactly once in the selected block\n{}",
                blocks_str(blocks)
            ),
            EvalError::Overlap { op_a, op_b, block } => write!(
                f,
                "overlapping selections: {} and {} both touch\n{}\n\
                 hint: block operator extents must be disjoint",
                op_a, op_b, block
            ),
            EvalError::Expect {
                op,
                expected,
                actual,
                blocks,
            } => write!(
                f,
                "{} expects {}, selected {}\n{}\n\
                 hint: narrow with $within or $matches, or raise expect",
                op,
                fmt_expect(expected, "block"),
                actual,
                blocks_str(blocks)
            ),
            EvalError::ExpectDocuments {
                op,
                expected,
                actual,
                documents,
            } => write!(
                f,
                "{} expects {}, matched {}\n{}\n\
                 hint: adjust the filter or raise expect",
                op,
                fmt_expect(expected, "document"),
                actual,
                docs_str(documents)
            ),
            EvalError::SearchIndexMissing => write!(
                f,
                "'search' requires the search-indexed graph, which is not built for this command"
            ),
        }
    }
}

impl std::error::Error for EvalError {}

enum Action {
    Delete,
    Dissolve,
    Replace(Vec<Tree>),
    RetitleHeader {
        inlines: Vec<Inline>,
        prepend: Vec<Tree>,
    },
    DissolveReplace(Vec<Tree>),
    InsertBefore(Vec<Tree>),
    InsertAfter(Vec<Tree>),
    InsertFirstChild(Vec<Tree>),
    Append(Vec<Tree>),
    SetInlines(Vec<Inline>),
    SetRefText(String),
    SetRawContent(String),
}

struct DocCtx {
    key: Key,
    tree: Tree,
    index: BlockIndex,
}

impl DocCtx {
    fn new(graph: &Graph, key: &Key) -> DocCtx {
        let tree = graph.collect(key);
        let index = BlockIndex::from_tree(
            &tree,
            graph.format_options().markdown_options(),
            key.parent(),
        );
        DocCtx {
            key: key.clone(),
            tree,
            index,
        }
    }

    fn block_ref(&self, i: usize) -> BlockRef {
        BlockRef {
            key: self.key.to_string(),
            path: self.index.path(i).to_vec(),
            line: self.index.display_line(i),
        }
    }
}

fn acted_targets(index: &BlockIndex, op: &BlockUpdateOp, selector: &BlockPredicate) -> Vec<Target> {
    match op {
        BlockUpdateOp::ReplaceText { .. } => index
            .select(selector)
            .into_iter()
            .map(|idx| Target { idx, tree: false })
            .collect(),
        _ => index.coalesced_targets(selector),
    }
}

pub fn plan_and_apply(
    graph: &Graph,
    keys: &[Key],
    ops: &[BlockUpdate],
) -> Result<HashMap<Key, Tree>, EvalError> {
    let docs: Vec<DocCtx> = keys.iter().map(|key| DocCtx::new(graph, key)).collect();

    let mut edits: Vec<HashMap<NodeId, Action>> = docs.iter().map(|_| HashMap::new()).collect();
    let mut acted: Vec<Vec<Vec<Target>>> = docs
        .iter()
        .map(|_| ops.iter().map(|_| Vec::new()).collect())
        .collect();

    let mut append_non_container: Vec<BlockRef> = Vec::new();
    let mut replacetext_no_text: Vec<BlockRef> = Vec::new();
    let mut fragment_not_list: Vec<(&'static str, BlockRef)> = Vec::new();
    let mut anchor_fail: Vec<BlockRef> = Vec::new();

    for (d, doc) in docs.iter().enumerate() {
        for (o, bu) in ops.iter().enumerate() {
            let targets = acted_targets(&doc.index, &bu.op, &bu.selector);
            acted[d][o] = targets.clone();

            for target in &targets {
                let i = target.idx;
                let Some(id) = doc.index.node_id(i) else {
                    continue;
                };
                let node_header = doc.index.is_header(i) && !target.tree;
                let has_children = doc.index.has_children(i);
                match &bu.op {
                    BlockUpdateOp::Delete => {
                        if node_header && has_children {
                            edits[d].insert(id, Action::Dissolve);
                        } else {
                            edits[d].insert(id, Action::Delete);
                        }
                    }
                    BlockUpdateOp::ReplaceText { from, to } => {
                        let Some(text) = doc.index.own_text(i) else {
                            replacetext_no_text.push(doc.block_ref(i));
                            continue;
                        };
                        if doc.index.is_table(i) {
                            replacetext_no_text.push(doc.block_ref(i));
                            continue;
                        }
                        let replaced = match from {
                            Some(from) => {
                                if text.matches(from.as_str()).count() != 1 {
                                    anchor_fail.push(doc.block_ref(i));
                                    continue;
                                }
                                text.replacen(from.as_str(), to, 1)
                            }
                            None => to.clone(),
                        };
                        let action = match doc.index.kind_name(i) {
                            "ref" => Action::SetRefText(replaced),
                            "code" => Action::SetRawContent(replaced),
                            _ => Action::SetInlines(parse_inlines(
                                doc.index.options(),
                                doc.index.parent_dir(),
                                &replaced,
                            )),
                        };
                        edits[d].insert(id, action);
                    }
                    BlockUpdateOp::Replace { content } => {
                        if node_header && has_children {
                            let frag = parse_fragment(
                                doc.index.options(),
                                doc.index.parent_dir(),
                                content,
                            );
                            let action = match section_retitle(&frag) {
                                Some((inlines, prepend)) => {
                                    Action::RetitleHeader { inlines, prepend }
                                }
                                None => Action::DissolveReplace(frag),
                            };
                            edits[d].insert(id, action);
                        } else {
                            let list_mode = doc.index.is_item(i);
                            match fragment_for(doc, content, list_mode) {
                                Some(frag) => {
                                    edits[d].insert(id, Action::Replace(frag));
                                }
                                None => fragment_not_list.push(("$replace", doc.block_ref(i))),
                            }
                        }
                    }
                    BlockUpdateOp::InsertBefore { content } => {
                        let list_mode = doc.index.is_item(i);
                        match fragment_for(doc, content, list_mode) {
                            Some(frag) => {
                                edits[d].insert(id, Action::InsertBefore(frag));
                            }
                            None => fragment_not_list.push(("$insertBefore", doc.block_ref(i))),
                        }
                    }
                    BlockUpdateOp::InsertAfter { content } => {
                        if node_header {
                            let frag = parse_fragment(
                                doc.index.options(),
                                doc.index.parent_dir(),
                                content,
                            );
                            edits[d].insert(id, Action::InsertFirstChild(frag));
                        } else {
                            let list_mode = doc.index.is_item(i);
                            match fragment_for(doc, content, list_mode) {
                                Some(frag) => {
                                    edits[d].insert(id, Action::InsertAfter(frag));
                                }
                                None => fragment_not_list.push(("$insertAfter", doc.block_ref(i))),
                            }
                        }
                    }
                    BlockUpdateOp::Append { content } => {
                        if !doc.index.is_container(i) {
                            append_non_container.push(doc.block_ref(i));
                            continue;
                        }
                        let list_mode = doc.index.is_list(i);
                        match fragment_for(doc, content, list_mode) {
                            Some(frag) => {
                                edits[d].insert(id, Action::Append(frag));
                            }
                            None => fragment_not_list.push(("$append", doc.block_ref(i))),
                        }
                    }
                }
            }
        }
    }

    if !append_non_container.is_empty() {
        return Err(EvalError::AppendNonContainer {
            blocks: append_non_container,
        });
    }
    if !replacetext_no_text.is_empty() {
        return Err(EvalError::ReplaceTextNoText {
            blocks: replacetext_no_text,
        });
    }
    if !fragment_not_list.is_empty() {
        let op = fragment_not_list[0].0;
        let blocks = fragment_not_list.into_iter().map(|(_, b)| b).collect();
        return Err(EvalError::FragmentNotList { op, blocks });
    }
    if !anchor_fail.is_empty() {
        return Err(EvalError::ReplaceTextAnchor {
            blocks: anchor_fail,
        });
    }

    check_disjoint(&docs, &acted, ops)?;
    check_expect(&docs, &acted, ops)?;

    let mut out = HashMap::new();
    for (d, doc) in docs.into_iter().enumerate() {
        let tree = if edits[d].is_empty() {
            doc.tree
        } else {
            apply_edits(&doc.tree, &edits[d])
        };
        out.insert(doc.key, tree);
    }
    Ok(out)
}

fn check_disjoint(
    docs: &[DocCtx],
    acted: &[Vec<Vec<Target>>],
    ops: &[BlockUpdate],
) -> Result<(), EvalError> {
    for (d, doc) in docs.iter().enumerate() {
        for a in 0..ops.len() {
            for b in (a + 1)..ops.len() {
                for ta in &acted[d][a] {
                    for tb in &acted[d][b] {
                        if extents_overlap(doc, ta, tb) {
                            return Err(EvalError::Overlap {
                                op_a: ops[a].op.name(),
                                op_b: ops[b].op.name(),
                                block: doc.block_ref(ta.idx),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn extents_overlap(doc: &DocCtx, a: &Target, b: &Target) -> bool {
    if a.idx == b.idx {
        return true;
    }
    if a.tree && doc.index.is_ancestor(a.idx, b.idx) {
        return true;
    }
    if b.tree && doc.index.is_ancestor(b.idx, a.idx) {
        return true;
    }
    false
}

fn check_expect(
    docs: &[DocCtx],
    acted: &[Vec<Vec<Target>>],
    ops: &[BlockUpdate],
) -> Result<(), EvalError> {
    for (o, bu) in ops.iter().enumerate() {
        let Some(expected) = bu.expect else {
            continue;
        };
        let actual: usize = (0..docs.len()).map(|d| acted[d][o].len()).sum();
        if !expected.satisfied_by(actual as u64) {
            let blocks = docs
                .iter()
                .enumerate()
                .flat_map(|(d, doc)| acted[d][o].iter().map(move |t| doc.block_ref(t.idx)))
                .collect();
            return Err(EvalError::Expect {
                op: bu.op.name(),
                expected,
                actual,
                blocks,
            });
        }
    }
    Ok(())
}

pub fn check_document_expect(
    op: &'static str,
    expected: Option<Expect>,
    documents: &[DocRef],
) -> Result<(), EvalError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual = documents.len();
    if !expected.satisfied_by(actual as u64) {
        return Err(EvalError::ExpectDocuments {
            op,
            expected,
            actual,
            documents: documents.to_vec(),
        });
    }
    Ok(())
}

fn fragment_for(doc: &DocCtx, content: &str, list_mode: bool) -> Option<Vec<Tree>> {
    let blocks = parse_fragment(doc.index.options(), doc.index.parent_dir(), content);
    if list_mode {
        as_single_list(&blocks)
    } else {
        Some(blocks)
    }
}

fn as_single_list(blocks: &[Tree]) -> Option<Vec<Tree>> {
    match blocks {
        [only] if matches!(only.node, Node::BulletList() | Node::OrderedList()) => {
            Some(only.children.clone())
        }
        _ => None,
    }
}

fn section_retitle(blocks: &[Tree]) -> Option<(Vec<Inline>, Vec<Tree>)> {
    match blocks {
        [only] => match &only.node {
            Node::Section(inlines) => Some((inlines.clone(), only.children.clone())),
            _ => None,
        },
        _ => None,
    }
}

fn parse_fragment(options: &MarkdownOptions, parent_dir: &str, content: &str) -> Vec<Tree> {
    let mut graph = Graph::new_with_options(options.clone());
    let temp = if parent_dir.is_empty() {
        Key::name("__iwe_fragment__")
    } else {
        Key::combine(parent_dir, "__iwe_fragment__")
    };
    graph.from_markdown(temp.clone(), content, MarkdownReader::new());
    let graph_ref: &Graph = &graph;
    let tree = graph_ref.collect(&temp);
    tree.children.into_iter().map(strip_ids).collect()
}

fn parse_inlines(options: &MarkdownOptions, parent_dir: &str, text: &str) -> Vec<Inline> {
    for block in parse_fragment(options, parent_dir, text) {
        match block.node {
            Node::Leaf(inlines) | Node::Section(inlines) | Node::Item(_, inlines) => {
                return inlines
            }
            _ => {}
        }
    }
    vec![Inline::Str(text.to_string())]
}

fn strip_ids(tree: Tree) -> Tree {
    Tree {
        id: None,
        node: tree.node,
        children: tree.children.into_iter().map(strip_ids).collect(),
    }
}

fn apply_edits(tree: &Tree, edits: &HashMap<NodeId, Action>) -> Tree {
    let mut children = Vec::new();
    for child in &tree.children {
        let action = child.id.and_then(|id| edits.get(&id));
        match action {
            Some(Action::Delete) => {}
            Some(Action::Dissolve) => {
                let updated = apply_edits(child, edits);
                children.extend(updated.children);
            }
            Some(Action::Replace(frag)) => children.extend(frag.iter().cloned()),
            Some(Action::RetitleHeader { inlines, prepend }) => {
                let mut updated = apply_edits(child, edits);
                updated.node = with_inlines(&updated.node, inlines.clone());
                let mut new_children = prepend.clone();
                new_children.extend(updated.children);
                updated.children = new_children;
                children.push(updated);
            }
            Some(Action::DissolveReplace(frag)) => {
                let updated = apply_edits(child, edits);
                children.extend(frag.iter().cloned());
                children.extend(updated.children);
            }
            Some(Action::InsertBefore(frag)) => {
                children.extend(frag.iter().cloned());
                children.push(apply_edits(child, edits));
            }
            Some(Action::InsertAfter(frag)) => {
                children.push(apply_edits(child, edits));
                children.extend(frag.iter().cloned());
            }
            Some(Action::InsertFirstChild(frag)) => {
                let mut updated = apply_edits(child, edits);
                let mut new_children = frag.clone();
                new_children.extend(updated.children);
                updated.children = new_children;
                children.push(updated);
            }
            Some(Action::Append(frag)) => {
                let mut updated = apply_edits(child, edits);
                updated.children.extend(frag.iter().cloned());
                children.push(updated);
            }
            Some(Action::SetInlines(inlines)) => {
                let mut updated = apply_edits(child, edits);
                updated.node = with_inlines(&updated.node, inlines.clone());
                children.push(updated);
            }
            Some(Action::SetRefText(text)) => {
                let mut updated = apply_edits(child, edits);
                updated.node = with_ref_text(&updated.node, text.clone());
                children.push(updated);
            }
            Some(Action::SetRawContent(content)) => {
                let mut updated = apply_edits(child, edits);
                updated.node = with_raw_content(&updated.node, content.clone());
                children.push(updated);
            }
            None => children.push(apply_edits(child, edits)),
        }
    }
    Tree {
        id: tree.id,
        node: tree.node.clone(),
        children,
    }
}

fn with_inlines(node: &Node, inlines: Vec<Inline>) -> Node {
    match node {
        Node::Section(_) => Node::Section(inlines),
        Node::Leaf(_) => Node::Leaf(inlines),
        Node::Item(checked, _) => Node::Item(*checked, inlines),
        other => other.clone(),
    }
}

fn with_ref_text(node: &Node, text: String) -> Node {
    match node {
        Node::Reference(reference) => {
            let mut updated = reference.clone();
            updated.text = text;
            Node::Reference(updated)
        }
        other => other.clone(),
    }
}

fn with_raw_content(node: &Node, content: String) -> Node {
    match node {
        Node::Raw(lang, _) => Node::Raw(lang.clone(), content),
        other => other.clone(),
    }
}
