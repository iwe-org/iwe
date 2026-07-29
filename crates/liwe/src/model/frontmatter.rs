use pulldown_cmark::{Event, Parser, Tag};

use crate::markdown::reader::PARSER_OPTIONS;
use crate::model::{frontmatter_to_string, Frontmatter};
use crate::query::frontmatter::strip_reserved;

pub fn split_raw_frontmatter(content: &str) -> (Option<&str>, &str) {
    match leading_metadata_block_end(content) {
        Some(end) => (Some(&content[..end]), &content[end..]),
        None => (None, content),
    }
}

pub fn prepend_frontmatter(
    frontmatter: Option<Frontmatter>,
    rendered: &str,
) -> Result<String, String> {
    let mut mapping = match frontmatter {
        Some(mapping) => mapping,
        None => return Ok(rendered.to_string()),
    };

    strip_reserved(&mut mapping);
    if mapping.is_empty() {
        return Ok(rendered.to_string());
    }

    if leading_metadata_block_end(rendered).is_some() {
        return Err(
            "the document already begins with a frontmatter block, it would be written twice; \
             drop the frontmatter fields, or pass the complete document as content"
                .to_string(),
        );
    }

    Ok(format!(
        "---\n{}\n---\n\n{}",
        frontmatter_to_string(&mapping),
        rendered
    ))
}

fn leading_metadata_block_end(content: &str) -> Option<usize> {
    if let Some(end) = metadata_block_end(content) {
        return Some(end);
    }

    if content.is_empty() || content.ends_with('\n') {
        return None;
    }

    let terminated = format!("{}\n", content);
    metadata_block_end(&terminated).map(|end| end.min(content.len()))
}

fn metadata_block_end(content: &str) -> Option<usize> {
    match Parser::new_ext(content, PARSER_OPTIONS)
        .into_offset_iter()
        .next()
    {
        Some((Event::Start(Tag::MetadataBlock(_)), range)) if range.start == 0 => {
            Some(range.end + line_ending_len(&content[range.end..]))
        }
        _ => None,
    }
}

fn line_ending_len(rest: &str) -> usize {
    if rest.starts_with("\r\n") {
        2
    } else if rest.starts_with('\n') {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    fn mapping(yaml: &str) -> Frontmatter {
        match serde_yaml::from_str::<Value>(yaml).unwrap() {
            Value::Mapping(m) => m,
            _ => panic!("expected a mapping"),
        }
    }

    #[test]
    fn splits_dash_closed_block() {
        assert_eq!(
            split_raw_frontmatter("---\ntype: note\n---\n\nBody\n"),
            (Some("---\ntype: note\n---\n"), "\nBody\n")
        );
    }

    #[test]
    fn splits_dot_closed_block() {
        assert_eq!(
            split_raw_frontmatter("---\ntype: note\n...\n\nBody\n"),
            (Some("---\ntype: note\n...\n"), "\nBody\n")
        );
    }

    #[test]
    fn splits_crlf_block() {
        assert_eq!(
            split_raw_frontmatter("---\r\ntype: note\r\n---\r\n\r\nBody\r\n"),
            (Some("---\r\ntype: note\r\n---\r\n"), "\r\nBody\r\n")
        );
    }

    #[test]
    fn splits_block_without_trailing_newline() {
        assert_eq!(
            split_raw_frontmatter("---\ntype: note\n---"),
            (Some("---\ntype: note\n---"), "")
        );
    }

    #[test]
    fn keeps_lone_thematic_break() {
        assert_eq!(
            split_raw_frontmatter("---\n\nBody\n"),
            (None, "---\n\nBody\n")
        );
    }

    #[test]
    fn keeps_two_thematic_breaks() {
        assert_eq!(
            split_raw_frontmatter("---\n\n---\n\nBody\n"),
            (None, "---\n\n---\n\nBody\n")
        );
    }

    #[test]
    fn keeps_unterminated_block() {
        assert_eq!(
            split_raw_frontmatter("---\ntype: note\n\nBody\n"),
            (None, "---\ntype: note\n\nBody\n")
        );
    }

    #[test]
    fn keeps_block_below_the_first_line() {
        assert_eq!(
            split_raw_frontmatter("# Title\n\n---\ntype: note\n---\n"),
            (None, "# Title\n\n---\ntype: note\n---\n")
        );
    }

    #[test]
    fn keeps_empty_input() {
        assert_eq!(split_raw_frontmatter(""), (None, ""));
    }

    #[test]
    fn prepends_nothing_for_absent_mapping() {
        assert_eq!(
            prepend_frontmatter(None, "# Title\n"),
            Ok("# Title\n".to_string())
        );
    }

    #[test]
    fn prepends_nothing_for_empty_mapping() {
        assert_eq!(
            prepend_frontmatter(Some(Frontmatter::new()), "# Title\n"),
            Ok("# Title\n".to_string())
        );
    }

    #[test]
    fn prepends_nothing_when_all_keys_are_reserved() {
        assert_eq!(
            prepend_frontmatter(Some(mapping("_internal: 1\n$x: 2\n")), "# Title\n"),
            Ok("# Title\n".to_string())
        );
    }

    #[test]
    fn prepends_fenced_mapping() {
        assert_eq!(
            prepend_frontmatter(Some(mapping("type: note\ntags:\n- demo\n")), "# Title\n"),
            Ok("---\ntype: note\ntags:\n- demo\n---\n\n# Title\n".to_string())
        );
    }

    #[test]
    fn prepends_mapping_without_reserved_keys() {
        assert_eq!(
            prepend_frontmatter(Some(mapping("_internal: 1\ntype: note\n")), "# Title\n"),
            Ok("---\ntype: note\n---\n\n# Title\n".to_string())
        );
    }

    #[test]
    fn rejects_document_with_leading_block() {
        assert_eq!(
            prepend_frontmatter(
                Some(mapping("type: note\n")),
                "---\nother: 1\n---\n\n# Title\n"
            ),
            Err(
                "the document already begins with a frontmatter block, it would be written twice; \
                 drop the frontmatter fields, or pass the complete document as content"
                    .to_string()
            )
        );
    }
}
