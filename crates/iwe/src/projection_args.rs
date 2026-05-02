use liwe::query::wire::RawProjection;
use liwe::query::{
    build_projection, FieldPath, Projection, ProjectionField, ProjectionMode, ProjectionSource,
    PseudoField,
};
use serde_yaml::{Mapping, Value};

pub fn parse_projection(s: &str, mode: ProjectionMode) -> Result<Projection, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("projection argument cannot be empty".to_string());
    }

    if let Ok(value) = serde_yaml::from_str::<Value>(trimmed) {
        if let Value::Mapping(map) = value {
            let raw = RawProjection(map);
            return build_projection(raw, mode).map_err(|e| format!("invalid projection: {}", e));
        }
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
    Ok(Projection { fields, mode })
}

pub fn parse_projection_replace(s: &str) -> Result<Projection, String> {
    parse_projection(s, ProjectionMode::Replace)
}

pub fn parse_projection_extend(s: &str) -> Result<Projection, String> {
    parse_projection(s, ProjectionMode::Extend)
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
        return Err(format!(
            "projection output name '{}' must not contain '.'",
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

pub fn unused_warn() -> Mapping {
    Mapping::new()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn comma_list_aliased_pseudo() {
        let p = parse_projection_replace("body=$content,parents=$includedBy").unwrap();
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[0].output, "body");
        assert!(matches!(
            p.fields[0].source,
            ProjectionSource::Pseudo(PseudoField::Content)
        ));
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
    fn dotted_path_source() {
        let p = parse_projection_replace("priority=meta.priority").unwrap();
        match &p.fields[0].source {
            ProjectionSource::Frontmatter(fp) => {
                assert_eq!(fp.0, vec!["meta".to_string(), "priority".to_string()]);
            }
            _ => panic!("expected frontmatter"),
        }
    }

    #[test]
    fn unknown_pseudo_rejected() {
        assert!(parse_projection_replace("$bogus").is_err());
        assert!(parse_projection_replace("x=$bogus").is_err());
    }

    #[test]
    fn reserved_output_rejected() {
        assert!(parse_projection_replace("$key=$key").is_err());
    }
}
