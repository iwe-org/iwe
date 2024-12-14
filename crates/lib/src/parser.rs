use crate::model::Key;
use indoc::indoc;
use markdown::{
    self,
    mdast::{Link, Node, Text},
    unist::{Point, Position},
};

pub struct Parser {
    root: Node,
}

#[derive(Copy, Clone)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
}

impl Pos {
    pub fn new(line: usize, column: usize) -> Self {
        Pos { line, column }
    }
}

impl From<(usize, usize)> for Pos {
    fn from(value: (usize, usize)) -> Self {
        Pos {
            line: value.0,
            column: value.1,
        }
    }
}

impl Parser {
    pub fn new(content: &str) -> Parser {
        let root = markdown::to_mdast(content, &markdown::ParseOptions::gfm()).unwrap();
        Parser { root }
    }

    pub fn link_at(&self, pos: Pos) -> Option<Key> {
        if let Some(node) = self.in_position(pos) {
            if let Node::Link(link) = node {
                return Some(link.url.clone());
            }
        }
        None
    }

    fn is_in_position(node: &Node, pos: Pos) -> bool {
        match node.position() {
            Some(position) => {
                (pos.line > position.start.line
                    || (pos.line == position.start.line && pos.column >= position.start.column))
                    && (pos.line < position.end.line
                        || (pos.line == position.end.line && pos.column <= position.end.column))
            }
            _ => false,
        }
    }

    fn in_position(&self, pos: Pos) -> Option<&Node> {
        Self::get_link_in_position(&self.root, pos)
    }

    fn get_link_in_position(node: &Node, pos: Pos) -> Option<&Node> {
        if Parser::is_in_position(node, pos) {
            if let Node::Link(_) = node {
                return Some(node);
            }

            for child in node.children().map(|v| v.as_slice()).unwrap_or(&[]) {
                if let Some(n) = Parser::get_link_in_position(child, pos) {
                    return Some(n);
                }
            }
            None
        } else {
            None
        }
    }
}

#[test]
pub fn ast_test() {
    let map = Parser::new(indoc! {"
            # test
            [test](link)
            "});

    assert_eq!(
        &Node::Link(Link {
            children: vec![Node::Text(Text {
                value: "test".to_string(),
                position: Some(Position::new(2, 2, 8, 2, 6, 12))
            })],
            position: Some(Position::new(2, 1, 7, 2, 13, 19)),
            url: "link".to_string(),
            title: None
        }),
        map.in_position((2, 1).into()).unwrap()
    );
}

#[test]
pub fn ast_test2() {
    let map = Parser::new(indoc! {"
            # test
            [test](link1)
            [test](link2)
            [test](link3)
            "});

    assert_eq!(
        "link2",
        match map.in_position((3, 1).into()).unwrap() {
            Node::Link(link) => &link.url,
            _ => panic!(),
        }
    );
}

#[test]
pub fn link_in_paragraph() {
    let map = Parser::new(indoc! {"
            # test
            test [test](link1) test
            test
            "});

    assert_eq!(
        &Node::Link(Link {
            children: vec![Node::Text(Text {
                value: "test".to_string(),
                position: Some(Position::new(2, 7, 13, 2, 11, 17))
            })],
            position: Some(Position::new(2, 6, 12, 2, 19, 25)),
            url: "link1".to_string(),
            title: None
        }),
        map.in_position((2, 8).into()).unwrap()
    );

    assert_eq!(None, map.in_position((3, 2).into()));
}
