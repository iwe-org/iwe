use super::node::Node;
use super::node_iter::NodeIter;
use super::tree::Tree;

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
        let mut node = self.tree_node;

        for n in self.path.iter() {
            if let Some(n) = &node.children.get(*n) {
                node = n;
            } else {
                return None;
            }
        }

        Some(node.node.clone())
    }
}
