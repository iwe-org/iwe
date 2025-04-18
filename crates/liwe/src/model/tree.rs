use std::collections::HashMap;

use itertools::Itertools;
use log::info;

use super::{
    graph::GraphInline,
    node::{Node, NodeIter, NodePointer, Reference, ReferenceType},
    Key, NodeId,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub id: Option<NodeId>,
    pub node: Node,
    pub children: Vec<Tree>,
}

impl Tree {
    pub fn iter(&self) -> TreeIter {
        TreeIter::new(self)
    }

    pub fn is_section(&self) -> bool {
        match self.node {
            Node::Section(_) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self.node {
            Node::BulletList() | Node::OrderedList() => true,
            _ => false,
        }
    }

    pub fn is_quote(&self) -> bool {
        match self.node {
            Node::Quote() => true,
            _ => false,
        }
    }

    pub fn extract_sections(&self, keys: HashMap<NodeId, (Key, String)>) -> Tree {
        self.id
            .filter(|id| keys.contains_key(&id))
            .map(|id| {
                let (key, text) = keys.get(&id).expect("to have key").clone();
                Tree {
                    id: None,
                    node: Node::Reference(Reference {
                        key,
                        text,
                        reference_type: ReferenceType::Regular,
                    }),
                    children: vec![],
                }
            })
            .unwrap_or_else(|| self.map_children(|child| child.extract_sections(keys.clone())))
    }

    pub fn replace(&self, node_id: NodeId, tree: &Tree) -> Tree {
        if self.id_eq(node_id) {
            tree.clone()
        } else {
            self.map_children(|child| child.replace(node_id, tree))
        }
    }

    pub fn change_list_type(&self, node_id: NodeId) -> Tree {
        if self.id_eq(node_id) {
            Tree {
                id: self.id,
                node: match &self.node {
                    Node::BulletList() => Node::OrderedList(),
                    Node::OrderedList() => Node::BulletList(),
                    _ => self.node.clone(),
                },
                children: self.children.clone(),
            }
        } else {
            self.map_children(|child| child.change_list_type(node_id))
        }
    }

    fn map_children(&self, f: impl Fn(&Tree) -> Tree) -> Tree {
        Tree {
            id: self.id,
            node: self.node.clone(),
            children: self.children.iter().map(f).collect(),
        }
    }

    pub fn mark_node(&self, node_id: NodeId, start: &str, end: &str) -> Tree {
        if self.parent_of(node_id) {
            let pos = self.position(node_id);

            let mut children = self.children.clone();

            children.insert(
                pos + 1,
                Tree {
                    id: None,
                    node: Node::Leaf(vec![GraphInline::Str(end.to_string())]),
                    children: vec![],
                },
            );
            children.insert(
                pos,
                Tree {
                    id: None,
                    node: Node::Leaf(vec![GraphInline::Str(start.to_string())]),
                    children: vec![],
                },
            );

            Tree {
                id: self.id,
                children,
                node: self.node.clone(),
            }
        } else {
            self.map_children(|child| child.mark_node(node_id, start, end))
        }
    }

    pub fn update_node(&self, target_id: NodeId, inlines: &Vec<GraphInline>) -> Tree {
        info!("Updating node with id: {:?}", self.id);

        if self.id_eq(target_id) {
            Tree {
                id: self.id,
                node: match &self.node {
                    Node::Section(_) => Node::Section(inlines.clone()),
                    Node::Leaf(_) => Node::Leaf(inlines.clone()),
                    _ => self.node.clone(),
                },
                children: self.children.clone(),
            }
        } else {
            self.map_children(|child| child.update_node(target_id, inlines))
        }
    }

    pub fn pre_sub_header_position(&self) -> usize {
        self.children
            .iter()
            .take_while(|child| !child.is_section())
            .count()
    }

    pub fn position(&self, id: NodeId) -> usize {
        self.children
            .iter()
            .take_while(|child| !child.id_eq(id))
            .count()
    }

    pub fn remove_node(&self, target_id: NodeId) -> Tree {
        Tree {
            id: self.id,
            node: self.node.clone(),
            children: self
                .clone()
                .children
                .iter()
                .filter(|child| !child.id_eq(target_id))
                .map(|child| child.remove_node(target_id))
                .collect(),
        }
    }

    pub fn append_pre_header(&self, target_id: NodeId, new: Tree) -> Tree {
        let mut children = self.children.clone();

        if self.id_eq(target_id) {
            children.insert(self.pre_sub_header_position(), new.clone());
        }

        Tree {
            id: self.id,
            node: self.node.clone(),
            children: children
                .into_iter()
                .map(|child| child.append_pre_header(target_id, new.clone()))
                .collect(),
        }
    }

    pub fn append_after(&self, target_id: NodeId, new: Tree) -> Tree {
        let mut children = self.children.clone();

        if self.parent_of(target_id) {
            children.insert(self.position(target_id), new.clone());
        }

        Tree {
            id: self.id,
            node: self.node.clone(),
            children: children
                .into_iter()
                .map(|child| child.append_pre_header(target_id, new.clone()))
                .collect(),
        }
    }

    pub fn id_eq(&self, id: NodeId) -> bool {
        self.id == Some(id)
    }

    pub fn from_pointer<'a>(pointer: impl NodePointer<'a>) -> Option<Tree> {
        let id = pointer.id();
        let payload = pointer.node()?;
        let mut children = Vec::new();

        pointer.child().map(|child| children.push(child));

        if let Some(child) = pointer.child() {
            let mut i = child;
            while let Some(next) = i.next() {
                children.push(next);
                i = i.next().unwrap();
            }
        }

        Some(Tree {
            id,
            node: payload,
            children: children
                .into_iter()
                .map(|child| Tree::from_pointer(child))
                .flatten()
                .collect_vec(),
        })
    }

    pub fn wrap_into_list(&self, node_id: NodeId) -> Tree {
        if self.id_eq(node_id) {
            Tree {
                id: self.id,
                node: Node::BulletList(),
                children: vec![self.clone()],
            }
        } else {
            self.map_children(|child| child.wrap_into_list(node_id))
        }
    }

    pub fn unwrap_list(&self, node_id: NodeId) -> Tree {
        if self.children.iter().any(|child| child.id_eq(node_id)) {
            let mut children = vec![];

            for child in self.children.iter() {
                if child.id_eq(node_id) {
                    children.extend(child.children.clone());
                } else {
                    children.push(child.unwrap_list(node_id));
                }
            }

            Tree {
                id: self.id,
                node: self.node.clone(),
                children,
            }
        } else {
            self.map_children(|child| child.unwrap_list(node_id))
        }
    }

    pub fn change_key(&self, target_key: &Key, updated_key: &Key) -> Tree {
        Tree {
            id: self.id,
            node: match &self.node {
                Node::Section(inlines) => Node::Section(
                    inlines
                        .iter()
                        .map(|inline| inline.change_key(target_key, updated_key))
                        .collect_vec(),
                ),
                Node::Leaf(inlines) => Node::Leaf(
                    inlines
                        .iter()
                        .map(|inline| inline.change_key(target_key, updated_key))
                        .collect_vec(),
                ),
                Node::Reference(reference) => Node::Reference(Reference {
                    key: if reference.key.eq(target_key) {
                        updated_key.clone()
                    } else {
                        reference.key.clone()
                    },
                    text: reference.text.clone(),
                    reference_type: reference.reference_type,
                }),
                _ => self.node.clone(),
            },
            children: self
                .map_children(|child| child.change_key(target_key, updated_key))
                .children,
        }
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.id_eq(id) || self.children.iter().any(|child| child.contains(id))
    }

    pub fn parent_of(&self, id: NodeId) -> bool {
        self.children.iter().any(|child| child.id_eq(id))
    }

    pub fn get_top_level_surrounding_list_id(&self, id: NodeId) -> Option<NodeId> {
        if self.contains(id) && self.is_list() {
            return self.id;
        }

        self.children
            .iter()
            .find(|child| child.contains(id))
            .and_then(|child| child.get_top_level_surrounding_list_id(id))
    }

    pub fn get_surrounding_top_level_block(&self, id: NodeId) -> Option<NodeId> {
        if self.contains(id) && (self.is_list() || self.is_list()) {
            return self.id;
        }

        self.children
            .iter()
            .find(|child| child.contains(id))
            .and_then(|child| child.get_surrounding_top_level_block(id))
    }

    pub fn get_surrounding_list_id(&self, id: NodeId) -> Option<NodeId> {
        if self.is_list() && self.parent_of(id) {
            return self.id;
        }

        self.children
            .iter()
            .find(|child| child.contains(id))
            .and_then(|child| child.get_surrounding_list_id(id))
    }

    pub fn get_surrounding_section_id(&self, id: NodeId) -> Option<NodeId> {
        if self.is_section() && self.parent_of(id) {
            return self.id;
        }

        self.children
            .iter()
            .find(|child| child.contains(id))
            .and_then(|child| child.get_surrounding_section_id(id))
    }

    pub fn squash_from_pointer<'a>(pointer: impl NodePointer<'a>, depth: u8) -> Vec<Tree> {
        let id = pointer.id();
        let node = pointer.node().unwrap();
        let mut children = Vec::new();

        pointer.child().map(|child| children.push(child));

        if let Some(child) = pointer.child() {
            let mut i = child;
            while let Some(next) = i.next() {
                if !next.is_reference() {
                    children.push(next);
                }
                i = i.next().unwrap();
            }
        }

        if let Some(child) = pointer.child() {
            let mut i = child;
            while let Some(next) = i.next() {
                if next.is_reference() {
                    children.push(next);
                }

                i = i.next().unwrap();
            }
        }

        vec![Tree {
            id,
            node,
            children: children
                .into_iter()
                .map(|child| {
                    if child.is_reference() {
                        child
                            .ref_key()
                            .filter(|_| depth > 0)
                            .and_then(|key| child.to_key(key))
                            .map(|pointer| Tree::squash_from_pointer(pointer, depth - 1))
                            .map(|r| r.first().unwrap().children.clone())
                            .unwrap_or(Tree::squash_from_pointer(child, 0))
                    } else {
                        Tree::squash_from_pointer(child, depth)
                    }
                })
                .flatten()
                .collect_vec(),
        }]
    }

    pub fn find(&self, id: NodeId) -> Option<Tree> {
        if self.id_eq(id) {
            return Some(self.clone());
        }

        self.children.iter().find_map(|child| child.find(id))
    }

    pub fn get(&self, id: NodeId) -> Tree {
        self.find(id).unwrap()
    }

    pub fn reference_key(&self, id: NodeId) -> Key {
        self.find(id)
            .and_then(|tree| tree.node.reference_key())
            .unwrap_or_default()
    }

    pub fn is_header(&self, id: NodeId) -> bool {
        if self.is_section() && self.id_eq(id) {
            return true;
        }

        if self.is_list() {
            return false;
        }

        self.children.iter().any(|child| child.is_header(id))
    }

    pub fn is_reference(&self) -> bool {
        self.node.is_reference()
    }

    pub fn is_bullet_list(&self) -> bool {
        match self.node {
            Node::BulletList() => true,
            _ => false,
        }
    }
}

pub struct TreeIter<'a> {
    tree_node: &'a Tree,
    path: Vec<usize>,
}

impl<'a> TreeIter<'a> {
    pub fn new(tree_node: &'a Tree) -> TreeIter<'a> {
        TreeIter {
            tree_node: &tree_node,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_section() {
        let section_node = Tree {
            id: None,
            node: Node::Section(vec![]),
            children: vec![],
        };
        let non_section_node = Tree {
            id: None,
            node: Node::Quote(),
            children: vec![],
        };

        assert!(section_node.is_section());
        assert!(!non_section_node.is_section());
    }

    #[test]
    fn test_is_list() {
        let bullet_list_node = Tree {
            id: None,
            node: Node::BulletList(),
            children: vec![],
        };
        let ordered_list_node = Tree {
            id: None,
            node: Node::OrderedList(),
            children: vec![],
        };
        let non_list_node = Tree {
            id: None,
            node: Node::Section(vec![]),
            children: vec![],
        };

        assert!(bullet_list_node.is_list());
        assert!(ordered_list_node.is_list());
        assert!(!non_list_node.is_list());
    }

    #[test]
    fn test_replace() {
        let replacer = Tree {
            id: Some(2),
            node: Node::Section(vec![]),
            children: vec![],
        };
        let root = Tree {
            id: Some(1),
            node: Node::Quote(),
            children: vec![Tree {
                id: Some(2),
                node: Node::BulletList(),
                children: vec![],
            }],
        };

        let result = root.replace(2, &replacer);

        assert_eq!(result.children[0], replacer);
    }

    #[test]
    fn test_update_node() {
        let inlines = vec![GraphInline::Str("Updated".to_string())];
        let root = Tree {
            id: Some(1),
            node: Node::Quote(),
            children: vec![Tree {
                id: Some(2),
                node: Node::Section(vec![GraphInline::Str("Old".to_string())]),
                children: vec![],
            }],
        };

        let result = root.update_node(2, &inlines);

        if let Node::Section(updated_inlines) = &result.children[0].node {
            assert_eq!(*updated_inlines, inlines);
        } else {
            panic!("The node was not updated properly");
        }
    }

    #[test]
    fn test_find() {
        let child_tree = Tree {
            id: Some(2),
            node: Node::Quote(),
            children: vec![],
        };
        let root = Tree {
            id: Some(1),
            node: Node::Section(vec![]),
            children: vec![child_tree.clone()],
        };

        let found_tree = root.find(2);
        assert_eq!(found_tree, Some(child_tree));
    }

    #[test]
    fn test_get() {
        let root = Tree {
            id: Some(1),
            node: Node::Section(vec![]),
            children: vec![Tree {
                id: Some(2),
                node: Node::Quote(),
                children: vec![],
            }],
        };

        let child = root.get(2);
        assert_eq!(child.id, Some(2));
        assert!(matches!(child.node, Node::Quote()));
    }

    #[cfg(test)]
    mod get_top_level_surrounding_list_id_tests {
        use super::*;

        #[test]
        fn test_with_list_containing_node() {
            let list_node = Tree {
                id: Some(1),
                node: Node::BulletList(),
                children: vec![Tree {
                    id: Some(2),
                    node: Node::Quote(),
                    children: vec![],
                }],
            };

            assert_eq!(list_node.get_top_level_surrounding_list_id(2), Some(1));
        }

        #[test]
        fn test_with_no_list_containing_node() {
            let root = Tree {
                id: Some(1),
                node: Node::Section(vec![]),
                children: vec![Tree {
                    id: Some(2),
                    node: Node::Quote(),
                    children: vec![Tree {
                        id: Some(3),
                        node: Node::Leaf(vec![GraphInline::Str("Test".to_string())]),
                        children: vec![],
                    }],
                }],
            };

            assert_eq!(root.get_top_level_surrounding_list_id(3), None);
        }
    }
}
