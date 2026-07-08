use serde_yaml::Value;

use crate::query::block::BlockPredicate;
use crate::query::wire::RawProjection;
use crate::query::{
    build_projection, FieldPath, Projection, ProjectionBase, ProjectionField, ProjectionSource,
    PseudoField,
};

pub fn parse_projection(s: &str, base: ProjectionBase) -> Result<Projection, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("projection argument cannot be empty".to_string());
    }

    if trimmed.contains(':') || trimmed.contains('{') || trimmed.contains('}') {
        return parse_mapping(trimmed, base);
    }

    let mut fields: Vec<ProjectionField> = Vec::new();
    for item in trimmed.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        fields.push(parse_item(item)?);
    }
    if fields.is_empty() {
        return Err("projection argument cannot be empty".to_string());
    }
    Ok(Projection { fields, base })
}

fn parse_mapping(trimmed: &str, base: ProjectionBase) -> Result<Projection, String> {
    let value: Value = serde_yaml::from_str(trimmed).map_err(|_| invalid_mapping(trimmed))?;
    let Value::Mapping(map) = value else {
        return Err(invalid_mapping(trimmed));
    };
    build_projection(RawProjection(map), base).map_err(|e| format!("invalid projection: {}", e))
}

fn invalid_mapping(input: &str) -> String {
    format!(
        "projection '{}' is not a valid YAML mapping; to rename a field or select blocks, \
         wrap the whole projection in braces, e.g. '{{ body: $content }}'",
        input
    )
}

fn parse_item(item: &str) -> Result<ProjectionField, String> {
    if let Some((name, src)) = item.split_once('=') {
        let name = name.trim();
        let src = src.trim();
        check_output_name(name)?;
        let source = parse_source(src)?;
        return Ok(ProjectionField {
            output: name.to_string(),
            source,
        });
    }

    if item == "$blocks" {
        return Ok(ProjectionField {
            output: "blocks".to_string(),
            source: ProjectionSource::Blocks(BlockPredicate::empty()),
        });
    }

    if let Some(stripped) = item.strip_prefix('$') {
        let selector = format!("${}", stripped);
        let pf = PseudoField::from_selector(&selector)
            .ok_or_else(|| format!("unknown projection source '{}'", selector))?;
        return Ok(ProjectionField {
            output: pf.default_output_name().to_string(),
            source: ProjectionSource::Pseudo(pf),
        });
    }

    check_output_name(item)?;
    Ok(ProjectionField {
        output: item.to_string(),
        source: ProjectionSource::Frontmatter(FieldPath(vec![item.to_string()])),
    })
}

fn parse_source(src: &str) -> Result<ProjectionSource, String> {
    if src == "$blocks" {
        return Ok(ProjectionSource::Blocks(BlockPredicate::empty()));
    }
    if let Some(stripped) = src.strip_prefix('$') {
        let selector = format!("${}", stripped);
        let pf = PseudoField::from_selector(&selector)
            .ok_or_else(|| format!("unknown projection source '{}'", selector))?;
        Ok(ProjectionSource::Pseudo(pf))
    } else if !src.is_empty() {
        let segments: Vec<String> = src.split('.').map(|s| s.to_string()).collect();
        Ok(ProjectionSource::Frontmatter(FieldPath(segments)))
    } else {
        Err("projection source cannot be empty".to_string())
    }
}

fn check_output_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("projection output name cannot be empty".to_string());
    }
    if name.starts_with('$') {
        return Err(format!(
            "projection output name '{}' must not start with '$'",
            name
        ));
    }
    if name.contains('.') {
        let leaf = name.rsplit('.').next().unwrap_or(name);
        return Err(format!(
            "projection output name '{}' must not contain '.'\n  hint: use '{}={}' to project a nested field",
            name, leaf, name
        ));
    }
    for bad in [':', '=', ',', '{', '}'] {
        if name.contains(bad) {
            return Err(format!(
                "projection output name '{}' must not contain '{}'",
                name, bad
            ));
        }
    }
    if name.chars().any(|c| c.is_whitespace()) {
        return Err(format!(
            "projection output name '{}' must not contain whitespace",
            name
        ));
    }
    if matches!(name.chars().next(), Some('_' | '#' | '@')) {
        return Err(format!(
            "projection output name '{}' starts with reserved character",
            name
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_projection_replace(s: &str) -> Result<Projection, String> {
        parse_projection(s, ProjectionBase::Empty)
    }

    #[test]
    fn comma_list_simple_fields() {
        let p = parse_projection_replace("title,author").unwrap();
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[0].output, "title");
        match &p.fields[0].source {
            ProjectionSource::Frontmatter(fp) => assert_eq!(fp.0, vec!["title".to_string()]),
            _ => panic!("expected frontmatter"),
        }
    }

    #[test]
    fn comma_list_bare_pseudos() {
        let p = parse_projection_replace("$content,$includedBy").unwrap();
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[0].output, "content");
        assert!(matches!(
            p.fields[0].source,
            ProjectionSource::Pseudo(PseudoField::Content)
        ));
        assert_eq!(p.fields[1].output, "includedBy");
        assert!(matches!(
            p.fields[1].source,
            ProjectionSource::Pseudo(PseudoField::IncludedBy)
        ));
    }

    #[test]
    fn comma_list_bare_blocks() {
        let p = parse_projection_replace("$blocks").unwrap();
        assert_eq!(p.fields[0].output, "blocks");
        assert!(matches!(p.fields[0].source, ProjectionSource::Blocks(_)));
    }

    #[test]
    fn bare_pseudo_uses_default_name() {
        let p = parse_projection_replace("$content").unwrap();
        assert_eq!(p.fields[0].output, "content");
        assert!(matches!(
            p.fields[0].source,
            ProjectionSource::Pseudo(PseudoField::Content)
        ));
    }

    #[test]
    fn yaml_mapping_form() {
        let p = parse_projection_replace("body: $content").unwrap();
        assert_eq!(p.fields[0].output, "body");
        assert!(matches!(
            p.fields[0].source,
            ProjectionSource::Pseudo(PseudoField::Content)
        ));
    }

    #[test]
    fn yaml_flow_form() {
        let p = parse_projection_replace("{body: $content, parents: $includedBy}").unwrap();
        assert_eq!(p.fields.len(), 2);
    }

    #[test]
    fn braced_multi_pair_renames() {
        let p = parse_projection_replace("{ test: $key, test2: $key }").unwrap();
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[0].output, "test");
        assert!(matches!(
            p.fields[0].source,
            ProjectionSource::Pseudo(PseudoField::Key)
        ));
        assert_eq!(p.fields[1].output, "test2");
    }

    #[test]
    fn mapping_dotted_path_source() {
        let p = parse_projection_replace("{ priority: meta.priority }").unwrap();
        match &p.fields[0].source {
            ProjectionSource::Frontmatter(fp) => {
                assert_eq!(fp.0, vec!["meta".to_string(), "priority".to_string()]);
            }
            _ => panic!("expected frontmatter"),
        }
    }

    #[test]
    fn comma_list_aliased_pseudo() {
        let p = parse_projection_replace("body=$content,parents=$includedBy").unwrap();
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[0].output, "body");
        assert!(matches!(
            p.fields[0].source,
            ProjectionSource::Pseudo(PseudoField::Content)
        ));
        assert_eq!(p.fields[1].output, "parents");
        assert!(matches!(
            p.fields[1].source,
            ProjectionSource::Pseudo(PseudoField::IncludedBy)
        ));
    }

    #[test]
    fn comma_list_dotted_path_source() {
        let p = parse_projection_replace("priority=meta.priority").unwrap();
        match &p.fields[0].source {
            ProjectionSource::Frontmatter(fp) => {
                assert_eq!(fp.0, vec!["meta".to_string(), "priority".to_string()]);
            }
            _ => panic!("expected frontmatter"),
        }
    }

    #[test]
    fn unbraced_multi_pair_is_invalid_mapping() {
        let err = parse_projection_replace("test: $key, test2: $key").unwrap_err();
        assert_eq!(
            err,
            "projection 'test: $key, test2: $key' is not a valid YAML mapping; to rename a field \
             or select blocks, wrap the whole projection in braces, e.g. '{ body: $content }'"
        );
    }

    #[test]
    fn multi_field_mapping_without_braces_errors() {
        let err =
            parse_projection_replace("goals: { $content: x }, usage: { $content: y }").unwrap_err();
        assert_eq!(
            err,
            "projection 'goals: { $content: x }, usage: { $content: y }' is not a valid YAML \
             mapping; to rename a field or select blocks, wrap the whole projection in braces, \
             e.g. '{ body: $content }'"
        );
    }

    #[test]
    fn nested_colon_source_without_braces_errors() {
        let err = parse_projection_replace("a: b: c").unwrap_err();
        assert_eq!(
            err,
            "projection 'a: b: c' is not a valid YAML mapping; to rename a field or select \
             blocks, wrap the whole projection in braces, e.g. '{ body: $content }'"
        );
    }

    #[test]
    fn output_name_with_whitespace_rejected() {
        let err = parse_projection_replace("bad name=$key").unwrap_err();
        assert_eq!(
            err,
            "projection output name 'bad name' must not contain whitespace"
        );
    }

    #[test]
    fn unknown_pseudo_rejected() {
        let bare = parse_projection_replace("$bogus").unwrap_err();
        assert_eq!(bare, "unknown projection source '$bogus'");
        let aliased = parse_projection_replace("x=$bogus").unwrap_err();
        assert_eq!(aliased, "unknown projection source '$bogus'");
    }

    #[test]
    fn reserved_output_rejected() {
        let err = parse_projection_replace("$key=$key").unwrap_err();
        assert_eq!(err, "projection output name '$key' must not start with '$'");
    }
}
