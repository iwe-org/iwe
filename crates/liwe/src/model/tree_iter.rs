use super::ids::alloc_node_id;
use super::node::Node;
use super::node_iter::NodeIter;
use super::tree::Tree;
use super::{LineRange, NodeId};

pub struct TreeIter<'a> {
    tree_node: &'a Tree,
    path: Vec<usize>,
}

impl<'a> TreeIter<'a> {
    pub fn new(tree_node: &'a Tree) -> TreeIter<'a> {
        TreeIter {
            tree_node,
            path: vec![],
        }
    }

    fn current(&self) -> Option<&Tree> {
        let mut node = self.tree_node;

        for n in self.path.iter() {
            node = node.children.get(*n)?;
        }

        Some(node)
    }
}

impl<'a, 'b> NodeIter<'a> for TreeIter<'b> {
    fn next(&self) -> Option<Self> {
        self.path
            .last()
            .filter(|_| self.node().is_some())
            .map(|last_index| {
                let mut path = self.path.clone();
                path.pop();
                path.push(last_index + 1);

                TreeIter {
                    tree_node: self.tree_node,
                    path,
                }
            })
    }

    fn child(&self) -> Option<Self> {
        let mut path = self.path.clone();
        path.push(0);

        Some(TreeIter {
            tree_node: self.tree_node,
            path,
        })
        .filter(|_| self.node().is_some())
    }

    fn node(&self) -> Option<Node> {
        self.current().map(|node| node.node.clone())
    }

    fn iter_id(&self) -> NodeId {
        self.current()
            .map(|node| node.id)
            .unwrap_or_else(alloc_node_id)
    }

    fn line_range(&self) -> Option<LineRange> {
        self.current().and_then(|node| node.line_range.clone())
    }
}
