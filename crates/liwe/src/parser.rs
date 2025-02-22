use crate::{
    graph::Reader,
    model::{
        document::{Document, DocumentInline},
        Key, Position,
    },
};
pub struct Parser {
    document: Document,
}

impl Parser {
    pub fn new(content: &str, reader: impl Reader) -> Parser {
        let document = reader.document(content);
        Parser { document }
    }

    pub fn link_at(&self, position: Position) -> Option<DocumentInline> {
        self.document.link_at(position)
    }

    pub fn key_at(&self, position: Position) -> Option<Key> {
        self.document
            .link_at(position)
            .and_then(|link| link.ref_key())
    }
}

#[test]
pub fn link_in_paragraph() {
    let parser = Parser::new(
        indoc::indoc! {"
            # test

            test [test](link1) test

            test
            "},
        crate::markdown::MarkdownReader::new(),
    );

    assert_eq!(
        Key::from_file_name("link1"),
        parser.key_at((2, 8).into()).unwrap()
    );
    assert_eq!(None, parser.key_at((1, 8).into()));
    assert_eq!(None, parser.key_at((3, 8).into()));
    assert_eq!(None, parser.key_at((2, 2).into()));
    assert_eq!(None, parser.key_at((2, 21).into()));
}
