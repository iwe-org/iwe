use serde_yaml::{Mapping, Value};

use crate::graph::{Graph, GraphContext};
use crate::model::config::MarkdownOptions;
use crate::model::inline::{inlines_to_markdown, to_plain_text};
use crate::model::node::Node;
use crate::model::projector::Projector;
use crate::model::reference::ReferenceType;
use crate::model::tree::{Tree, TreeIter};
use crate::model::writer::{blocks_to_markdown_sparce, Block};
use crate::model::{Key, NodeId};
use crate::query::block::{BlockOp, BlockPredicate, BlockType, MatchesSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Header,
    Paragraph,
    List,
    Item,
    Code,
    Quote,
    Table,
    Ref,
    Hr,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Header => "header",
            Kind::Paragraph => "paragraph",
            Kind::List => "list",
            Kind::Item => "item",
            Kind::Code => "code",
            Kind::Quote => "quote",
            Kind::Table => "table",
            Kind::Ref => "ref",
            Kind::Hr => "hr",
        }
    }

    fn of(t: BlockType) -> Kind {
        match t {
            BlockType::Header => Kind::Header,
            BlockType::Paragraph => Kind::Paragraph,
            BlockType::Item => Kind::Item,
            BlockType::Code => Kind::Code,
            BlockType::Table => Kind::Table,
            BlockType::Ref => Kind::Ref,
            BlockType::Hr => Kind::Hr,
        }
    }

    fn is_container(self) -> bool {
        matches!(self, Kind::Header | Kind::Item | Kind::List | Kind::Quote)
    }
}

struct BlockInfo {
    id: Option<NodeId>,
    node: Node,
    parent: Option<usize>,
    children: Vec<usize>,
    kind: Kind,
    own_text: Option<String>,
    header_level: u8,
    path: Vec<String>,
    ref_targets: Vec<Key>,
}

pub struct BlockIndex {
    blocks: Vec<BlockInfo>,
    roots: Vec<usize>,
    options: MarkdownOptions,
    parent_dir: String,
}

struct ForestNode {
    idx: usize,
    children: Vec<ForestNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub idx: usize,
    pub tree: bool,
}

impl BlockIndex {
    pub fn build(graph: &Graph, key: &Key) -> BlockIndex {
        let tree = graph.collect(key);
        BlockIndex::from_tree(
            &tree,
            graph.format_options().markdown_options(),
            key.parent(),
        )
    }

    pub fn from_tree(tree: &Tree, options: MarkdownOptions, parent_dir: String) -> BlockIndex {
        let mut index = BlockIndex {
            blocks: Vec::new(),
            roots: Vec::new(),
            options,
            parent_dir,
        };
        for child in &tree.children {
            let i = index.add(child, None, 1, &[]);
            index.roots.push(i);
        }
        index
    }

    pub fn options(&self) -> &MarkdownOptions {
        &self.options
    }

    pub fn parent_dir(&self) -> &str {
        &self.parent_dir
    }

    pub fn select(&self, pred: &BlockPredicate) -> Vec<usize> {
        let mask = self.eval(pred);
        (0..self.blocks.len()).filter(|i| mask[*i]).collect()
    }

    pub fn has_match(&self, pred: &BlockPredicate) -> bool {
        self.eval(pred).into_iter().any(|selected| selected)
    }

    pub fn coalesced_targets(&self, pred: &BlockPredicate) -> Vec<Target> {
        let mask = self.eval(pred);
        let mut sub_full = vec![false; self.blocks.len()];
        for &root in &self.roots {
            self.compute_sub_full(root, &mask, &mut sub_full);
        }
        let whole: Vec<bool> = (0..self.blocks.len())
            .map(|i| {
                mask[i]
                    && !self.blocks[i].children.is_empty()
                    && (self.blocks[i].kind != Kind::Header || sub_full[i])
            })
            .collect();
        (0..self.blocks.len())
            .filter(|&i| mask[i] && !self.has_ancestor_in(i, &whole))
            .map(|i| Target {
                idx: i,
                tree: whole[i],
            })
            .collect()
    }

    fn compute_sub_full(&self, i: usize, mask: &[bool], out: &mut [bool]) -> bool {
        let children_full = self.blocks[i]
            .children
            .iter()
            .all(|&c| self.compute_sub_full(c, mask, out));
        let full = mask[i] && children_full;
        out[i] = full;
        full
    }

    pub fn node_id(&self, i: usize) -> Option<NodeId> {
        self.blocks[i].id
    }

    pub fn parent(&self, i: usize) -> Option<usize> {
        self.blocks[i].parent
    }

    pub fn is_ancestor(&self, ancestor: usize, mut descendant: usize) -> bool {
        while let Some(p) = self.blocks[descendant].parent {
            if p == ancestor {
                return true;
            }
            descendant = p;
        }
        false
    }

    pub fn is_container(&self, i: usize) -> bool {
        self.blocks[i].kind.is_container()
    }

    pub fn is_header(&self, i: usize) -> bool {
        self.blocks[i].kind == Kind::Header
    }

    pub fn has_children(&self, i: usize) -> bool {
        !self.blocks[i].children.is_empty()
    }

    pub fn is_table(&self, i: usize) -> bool {
        self.blocks[i].kind == Kind::Table
    }

    pub fn is_item(&self, i: usize) -> bool {
        self.blocks[i].kind == Kind::Item
    }

    pub fn is_list(&self, i: usize) -> bool {
        self.blocks[i].kind == Kind::List
    }

    pub fn own_text(&self, i: usize) -> Option<&str> {
        self.blocks[i].own_text.as_deref()
    }

    pub fn kind_name(&self, i: usize) -> &'static str {
        self.blocks[i].kind.name()
    }

    pub fn path(&self, i: usize) -> &[String] {
        &self.blocks[i].path
    }

    pub fn display_line(&self, i: usize) -> String {
        if let Some(text) = &self.blocks[i].own_text {
            if let Some(line) = text.lines().next() {
                return line.to_string();
            }
        }
        if let Node::Reference(reference) = &self.blocks[i].node {
            return reference.key.to_string();
        }
        let mut mask = vec![false; self.blocks.len()];
        self.mark_subtree(i, &mut mask);
        let forest = self.forest(&[i], &mask);
        let blocks = self.render_forest(&forest);
        blocks_to_markdown_sparce(&blocks, &self.options)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    fn add(&mut self, tree: &Tree, parent: Option<usize>, level: u8, path: &[String]) -> usize {
        let i = self.blocks.len();
        self.blocks.push(BlockInfo {
            id: tree.id,
            node: tree.node.clone(),
            parent,
            children: Vec::new(),
            kind: kind_of(&tree.node),
            own_text: self.compute_own_text(&tree.node),
            header_level: level,
            path: path.to_vec(),
            ref_targets: ref_targets(&tree.node),
        });
        let (child_level, child_path) = match &tree.node {
            Node::Section(inlines) => {
                let mut p = path.to_vec();
                if parent.is_some() {
                    p.push(to_plain_text(inlines).trim().to_string());
                }
                (level + 1, p)
            }
            Node::Quote() | Node::BulletList() | Node::OrderedList() => (1, path.to_vec()),
            _ => (level, path.to_vec()),
        };
        for child in &tree.children {
            let c = self.add(child, Some(i), child_level, &child_path);
            self.blocks[i].children.push(c);
        }
        i
    }

    fn compute_own_text(&self, node: &Node) -> Option<String> {
        match node {
            Node::Section(inlines) | Node::Leaf(inlines) | Node::Item(_, inlines) => {
                Some(inlines_to_markdown(
                    &Projector::resolve(&self.parent_dir, self.options.refs_path, inlines.clone()),
                    &self.options,
                ))
            }
            Node::Raw(_, content) => Some(content.trim_matches('\n').to_string()),
            Node::Table(table) => {
                let block = Block::Table(
                    table
                        .header
                        .iter()
                        .map(|cell| {
                            Projector::resolve(
                                &self.parent_dir,
                                self.options.refs_path,
                                cell.clone(),
                            )
                        })
                        .collect(),
                    table.alignment.clone(),
                    table
                        .rows
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|cell| {
                                    Projector::resolve(
                                        &self.parent_dir,
                                        self.options.refs_path,
                                        cell.clone(),
                                    )
                                })
                                .collect()
                        })
                        .collect(),
                );
                let rendered = block.to_markdown(&self.options);
                let lines: Vec<&str> = rendered
                    .lines()
                    .enumerate()
                    .filter(|(n, _)| *n != 1)
                    .map(|(_, line)| line)
                    .collect();
                Some(lines.join("\n"))
            }
            Node::Reference(reference)
                if reference.reference_type == ReferenceType::WikiLinkPiped =>
            {
                Some(reference.text.clone())
            }
            _ => None,
        }
    }

    fn eval(&self, pred: &BlockPredicate) -> Vec<bool> {
        let mut acc = vec![true; self.blocks.len()];
        for op in &pred.0 {
            let set = self.eval_op(op);
            for (a, b) in acc.iter_mut().zip(set.iter()) {
                *a = *a && *b;
            }
        }
        acc
    }

    fn eval_op(&self, op: &BlockOp) -> Vec<bool> {
        match op {
            BlockOp::Text(m) => self.map_own_text(|text| m.matches(text)),
            BlockOp::Matches(r) => self.map_own_text(|text| r.is_match(text)),
            BlockOp::Within(p) => {
                let set = self.eval(p);
                self.blocks
                    .iter()
                    .enumerate()
                    .map(|(i, _)| set[i] && self.has_ancestor_in(i, &set))
                    .collect()
            }
            BlockOp::Contains(p) => {
                let set = self.eval(p);
                let mut out = vec![false; self.blocks.len()];
                for (i, selected) in set.iter().enumerate() {
                    if *selected {
                        let mut current = self.blocks[i].parent;
                        while let Some(a) = current {
                            out[a] = true;
                            current = self.blocks[a].parent;
                        }
                    }
                }
                out
            }
            BlockOp::Section(p) => self.subtrees(p, Kind::Header),
            BlockOp::Quote(p) => self.subtrees(p, Kind::Quote),
            BlockOp::List(p) => self.subtrees(p, Kind::List),
            BlockOp::Type(t, p) => {
                let set = self.eval(p);
                self.blocks
                    .iter()
                    .enumerate()
                    .map(|(i, b)| set[i] && b.kind == Kind::of(*t))
                    .collect()
            }
            BlockOp::References(key) => self
                .blocks
                .iter()
                .map(|b| b.ref_targets.contains(key))
                .collect(),
            BlockOp::And(preds) => {
                let mut out = vec![true; self.blocks.len()];
                for p in preds {
                    let set = self.eval(p);
                    for (a, b) in out.iter_mut().zip(set.iter()) {
                        *a = *a && *b;
                    }
                }
                out
            }
            BlockOp::Or(preds) => {
                let mut out = vec![false; self.blocks.len()];
                for p in preds {
                    let set = self.eval(p);
                    for (a, b) in out.iter_mut().zip(set.iter()) {
                        *a = *a || *b;
                    }
                }
                out
            }
            BlockOp::Nor(preds) => {
                let mut out = vec![false; self.blocks.len()];
                for p in preds {
                    let set = self.eval(p);
                    for (a, b) in out.iter_mut().zip(set.iter()) {
                        *a = *a || *b;
                    }
                }
                out.iter().map(|b| !b).collect()
            }
        }
    }

    fn map_own_text(&self, f: impl Fn(&str) -> bool) -> Vec<bool> {
        self.blocks
            .iter()
            .map(|b| b.own_text.as_deref().map(&f).unwrap_or(false))
            .collect()
    }

    fn has_ancestor_in(&self, i: usize, set: &[bool]) -> bool {
        let mut current = self.blocks[i].parent;
        while let Some(a) = current {
            if set[a] {
                return true;
            }
            current = self.blocks[a].parent;
        }
        false
    }

    fn subtrees(&self, root_pred: &BlockPredicate, kind: Kind) -> Vec<bool> {
        let roots = self.eval(root_pred);
        let mut out = vec![false; self.blocks.len()];
        for (i, selected) in roots.iter().enumerate() {
            if *selected && self.blocks[i].kind == kind {
                self.mark_subtree(i, &mut out);
            }
        }
        out
    }

    fn mark_subtree(&self, i: usize, out: &mut [bool]) {
        out[i] = true;
        for &c in &self.blocks[i].children {
            self.mark_subtree(c, out);
        }
    }

    fn forest(&self, ids: &[usize], selected: &[bool]) -> Vec<ForestNode> {
        let mut out = Vec::new();
        for &i in ids {
            if selected[i] {
                out.push(ForestNode {
                    idx: i,
                    children: self.forest(&self.blocks[i].children, selected),
                });
            } else {
                out.extend(self.forest(&self.blocks[i].children, selected));
            }
        }
        out
    }

    pub fn render_content(&self, pred: &BlockPredicate) -> String {
        let selected = self.eval(pred);
        let forest = self.forest(&self.roots, &selected);
        if forest.is_empty() {
            return String::new();
        }
        let blocks = self.render_forest(&forest);
        blocks_to_markdown_sparce(&blocks, &self.options)
    }

    fn render_forest(&self, nodes: &[ForestNode]) -> Vec<Block> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            let info = &self.blocks[nodes[i].idx];
            if info.kind == Kind::Header {
                if let Node::Section(inlines) = &info.node {
                    out.push(Block::Header(
                        info.header_level,
                        Projector::resolve(
                            &self.parent_dir,
                            self.options.refs_path,
                            inlines.clone(),
                        ),
                    ));
                }
                out.extend(self.render_forest(&nodes[i].children));
                i += 1;
            } else {
                let mut j = i;
                while j < nodes.len() && self.blocks[nodes[j].idx].kind != Kind::Header {
                    j += 1;
                }
                for tree in self.prune_children(&nodes[i..j], usize::MAX) {
                    out.extend(Projector::project(
                        TreeIter::new(&tree),
                        &self.parent_dir,
                        self.options.refs_path,
                    ));
                }
                i = j;
            }
        }
        out
    }

    fn prune_children(&self, nodes: &[ForestNode], container: usize) -> Vec<Tree> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            let info = &self.blocks[nodes[i].idx];
            if info.kind == Kind::Item && info.parent != Some(container) {
                let list = info.parent.expect("item has a parent list");
                let mut items = Vec::new();
                while i < nodes.len() && self.blocks[nodes[i].idx].parent == Some(list) {
                    items.push(self.prune_tree(&nodes[i]));
                    i += 1;
                }
                out.push(Tree {
                    id: None,
                    node: self.blocks[list].node.clone(),
                    children: items,
                });
            } else {
                out.push(self.prune_tree(&nodes[i]));
                i += 1;
            }
        }
        out
    }

    fn prune_tree(&self, node: &ForestNode) -> Tree {
        Tree {
            id: None,
            node: self.blocks[node.idx].node.clone(),
            children: self.prune_children(&node.children, node.idx),
        }
    }

    pub fn blocks_entries(&self, pred: &BlockPredicate) -> Vec<Mapping> {
        let selected = self.eval(pred);
        self.blocks
            .iter()
            .enumerate()
            .filter(|(i, _)| selected[*i])
            .map(|(_, b)| {
                let mut entry = Mapping::new();
                entry.insert(
                    Value::String("type".to_string()),
                    Value::String(b.kind.name().to_string()),
                );
                entry.insert(Value::String("path".to_string()), path_value(&b.path));
                if let Node::Reference(reference) = &b.node {
                    entry.insert(
                        Value::String("target".to_string()),
                        Value::String(reference.key.to_string()),
                    );
                }
                entry.insert(
                    Value::String("text".to_string()),
                    Value::String(b.own_text.clone().unwrap_or_default()),
                );
                entry
            })
            .collect()
    }

    pub fn matches_entries(&self, source: &MatchesSource) -> Vec<Mapping> {
        let scope = self.eval(&source.scope);
        let mut out = Vec::new();
        for (i, b) in self.blocks.iter().enumerate() {
            if !scope[i] {
                continue;
            }
            let Some(text) = &b.own_text else {
                continue;
            };
            for line in text.lines() {
                if source.pattern.is_match(line) {
                    let mut entry = Mapping::new();
                    entry.insert(Value::String("path".to_string()), path_value(&b.path));
                    entry.insert(
                        Value::String("text".to_string()),
                        Value::String(line.to_string()),
                    );
                    out.push(entry);
                }
            }
        }
        out
    }
}

fn kind_of(node: &Node) -> Kind {
    match node {
        Node::Section(_) => Kind::Header,
        Node::Item(_, _) => Kind::Item,
        Node::Leaf(_) => Kind::Paragraph,
        Node::Raw(_, _) => Kind::Code,
        Node::Quote() => Kind::Quote,
        Node::BulletList() | Node::OrderedList() => Kind::List,
        Node::Table(_) => Kind::Table,
        Node::Reference(_) => Kind::Ref,
        Node::HorizontalRule() => Kind::Hr,
        Node::Document(_, _) => Kind::Paragraph,
    }
}

fn ref_targets(node: &Node) -> Vec<Key> {
    match node {
        Node::Reference(reference) => vec![reference.key.clone()],
        Node::Section(inlines) | Node::Leaf(inlines) | Node::Item(_, inlines) => {
            inlines.iter().flat_map(|i| i.ref_keys()).collect()
        }
        Node::Table(table) => table
            .header
            .iter()
            .chain(table.rows.iter().flatten())
            .flatten()
            .flat_map(|i| i.ref_keys())
            .collect(),
        _ => Vec::new(),
    }
}

fn path_value(path: &[String]) -> Value {
    Value::Sequence(path.iter().map(|p| Value::String(p.clone())).collect())
}
