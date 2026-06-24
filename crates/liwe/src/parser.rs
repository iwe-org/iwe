use crate::model::{
    config::FormatOptions,
    document::{Document, DocumentInline},
    Position,
};

pub struct Parser {
    document: Document,
    content: String,
}

impl Parser {
    pub fn new(content: &str, format: &FormatOptions) -> Parser {
        let document = crate::format::read_document(content, format);
        Parser {
            document,
            content: content.to_string(),
        }
    }

    pub fn link_at(&self, position: Position) -> Option<DocumentInline> {
        self.document.link_at(position)
    }

    pub fn url_at(&self, position: Position) -> Option<String> {
        if let Some(url) = self.document.link_at(position).and_then(|link| link.url()) {
            return Some(url);
        }

        self.bare_url_at(position)
    }

    fn bare_url_at(&self, position: Position) -> Option<String> {
        let line = self.content.lines().nth(position.line)?;
        let char_pos = position.character;

        for prefix in ["https://", "http://", "mailto:"] {
            if let Some(url) = Self::find_url_at_position(line, char_pos, prefix) {
                return Some(url);
            }
        }

        None
    }

    fn find_url_at_position(line: &str, char_pos: usize, prefix: &str) -> Option<String> {
        let mut search_start = 0;

        while let Some(start) = line[search_start..].find(prefix) {
            let absolute_start = search_start + start;
            let url_part = &line[absolute_start..];

            let end = url_part
                .find(|c: char| {
                    c.is_whitespace() || c == ')' || c == ']' || c == '>' || c == '"' || c == '\''
                })
                .unwrap_or(url_part.len());

            let url = &url_part[..end];
            let absolute_end = absolute_start + end;

            let char_start = line[..absolute_start].encode_utf16().count();
            let char_end = line[..absolute_end].encode_utf16().count();

            if char_pos >= char_start && char_pos < char_end {
                return Some(url.to_string());
            }

            search_start = absolute_start + 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn link_in_paragraph() {
        let parser = Parser::new(
            indoc! {"
                # test

                test [test](link1) test

                test
                "},
            &FormatOptions::default(),
        );

        assert_eq!("link1", parser.url_at((2, 8).into()).unwrap());
        assert_eq!(None, parser.url_at((1, 8).into()));
        assert_eq!(None, parser.url_at((3, 8).into()));
        assert_eq!(None, parser.url_at((2, 2).into()));
        assert_eq!(None, parser.url_at((2, 21).into()));
    }

    #[test]
    fn bare_https_url() {
        let parser = Parser::new(
            "Check out https://example.com for more info",
            &FormatOptions::default(),
        );

        assert_eq!(
            Some("https://example.com".to_string()),
            parser.url_at((0, 10).into())
        );
        assert_eq!(
            Some("https://example.com".to_string()),
            parser.url_at((0, 15).into())
        );
        assert_eq!(
            Some("https://example.com".to_string()),
            parser.url_at((0, 28).into())
        );
        assert_eq!(None, parser.url_at((0, 5).into()));
        assert_eq!(None, parser.url_at((0, 30).into()));
    }

    #[test]
    fn bare_http_url() {
        let parser = Parser::new("Visit http://example.org today", &FormatOptions::default());

        assert_eq!(
            Some("http://example.org".to_string()),
            parser.url_at((0, 10).into())
        );
    }

    #[test]
    fn bare_mailto_url() {
        let parser = Parser::new(
            "Contact mailto:test@example.com for help",
            &FormatOptions::default(),
        );

        assert_eq!(
            Some("mailto:test@example.com".to_string()),
            parser.url_at((0, 15).into())
        );
    }

    #[test]
    fn bare_url_with_path() {
        let parser = Parser::new(
            "See https://example.com/path/to/page?query=1#anchor for details",
            &FormatOptions::default(),
        );

        assert_eq!(
            Some("https://example.com/path/to/page?query=1#anchor".to_string()),
            parser.url_at((0, 10).into())
        );
    }

    #[test]
    fn multiple_bare_urls() {
        let parser = Parser::new(
            "First https://first.com then https://second.com",
            &FormatOptions::default(),
        );

        assert_eq!(
            Some("https://first.com".to_string()),
            parser.url_at((0, 10).into())
        );
        assert_eq!(
            Some("https://second.com".to_string()),
            parser.url_at((0, 35).into())
        );
    }

    #[test]
    fn markdown_link_preferred_over_bare_url() {
        let parser = Parser::new("[link](https://example.com)", &FormatOptions::default());

        assert_eq!(
            Some("https://example.com".to_string()),
            parser.url_at((0, 3).into())
        );
    }

    #[test]
    fn wiki_link_after_multibyte_text() {
        let parser = Parser::new(
            "- \u{03B1}\u{03B2}\u{03B3}[[target]]",
            &FormatOptions::default(),
        );

        assert_eq!(Some("target".to_string()), parser.url_at((0, 5).into()));
        assert_eq!(None, parser.url_at((0, 2).into()));
    }

    #[test]
    fn wiki_link_after_astral_text() {
        let parser = Parser::new("- \u{1F5FA}[[target]]", &FormatOptions::default());

        assert_eq!(Some("target".to_string()), parser.url_at((0, 13).into()));
        assert_eq!(None, parser.url_at((0, 3).into()));
    }

    #[test]
    fn bare_url_after_astral_text() {
        let parser = Parser::new("\u{1F5FA} https://example.com", &FormatOptions::default());

        assert_eq!(
            Some("https://example.com".to_string()),
            parser.url_at((0, 21).into())
        );
        assert_eq!(None, parser.url_at((0, 2).into()));
    }

    #[test]
    fn bare_url_after_multibyte_text() {
        let parser = Parser::new(
            "\u{03B1}\u{03B2} https://example.com",
            &FormatOptions::default(),
        );

        assert_eq!(
            Some("https://example.com".to_string()),
            parser.url_at((0, 5).into())
        );
        assert_eq!(None, parser.url_at((0, 1).into()));
    }
}
