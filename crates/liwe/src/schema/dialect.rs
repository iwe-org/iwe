use serde::Deserialize;
use serde_yaml::Mapping;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSchema {
    #[serde(rename = "$schema")]
    pub dialect: Option<String>,
    pub description: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
    pub max_tokens: Option<usize>,
    pub max_depth: Option<usize>,
    pub all_sections: Option<ReducedSection>,
    #[serde(default)]
    pub sections: Vec<SectionSchema>,
    pub additional_sections: Option<AdditionalSections>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionSchema {
    pub header: Option<HeaderSchema>,
    pub max_tokens: Option<usize>,
    pub max_depth: Option<usize>,
    pub min_contains: Option<i64>,
    pub max_contains: Option<i64>,
    pub description: Option<String>,
    pub all_sections: Option<ReducedSection>,
    #[serde(default)]
    pub sections: Vec<SectionSchema>,
    pub additional_sections: Option<AdditionalSections>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderSchema {
    pub pattern: Option<String>,
    #[serde(rename = "const")]
    pub konst: Option<String>,
    #[serde(rename = "enum")]
    pub choices: Option<Vec<String>>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub max_tokens: Option<usize>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReducedSection {
    pub header: Option<HeaderSchema>,
    pub max_tokens: Option<usize>,
    pub max_depth: Option<usize>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AdditionalSections {
    Bool(bool),
    Schema(Box<ReducedSection>),
}

pub fn parse_dialect(source: &str) -> Result<DocumentSchema, serde_yaml::Error> {
    serde_yaml::from_str(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_schema_parses() {
        let schema = parse_dialect("{}").unwrap();
        assert_eq!(schema.dialect, None);
        assert_eq!(schema.max_tokens, None);
        assert_eq!(schema.sections.len(), 0);
        assert_eq!(schema.extra, Mapping::new());
    }

    #[test]
    fn known_keywords_bind_to_fields() {
        let source = "\
$schema: https://iwe.md/document-schema/v1
maxTokens: 1200
maxDepth: 3
sections:
  - header: { const: Summary }
    minContains: 1
    maxContains: 1
additionalSections: false
";
        let schema = parse_dialect(source).unwrap();
        assert_eq!(
            schema.dialect,
            Some("https://iwe.md/document-schema/v1".to_string())
        );
        assert_eq!(schema.max_tokens, Some(1200));
        assert_eq!(schema.max_depth, Some(3));
        assert_eq!(schema.sections.len(), 1);
        assert_eq!(schema.extra, Mapping::new());

        let section = &schema.sections[0];
        assert_eq!(section.min_contains, Some(1));
        assert_eq!(section.max_contains, Some(1));
        assert_eq!(
            section.header.as_ref().unwrap().konst,
            Some("Summary".to_string())
        );

        match schema.additional_sections {
            Some(AdditionalSections::Bool(false)) => {}
            other => panic!("expected additionalSections false, got {:?}", other),
        }
    }

    #[test]
    fn additional_sections_accepts_reduced_schema() {
        let schema = parse_dialect("additionalSections:\n  maxTokens: 300\n").unwrap();
        match schema.additional_sections {
            Some(AdditionalSections::Schema(reduced)) => {
                assert_eq!(reduced.max_tokens, Some(300));
            }
            other => panic!("expected reduced schema, got {:?}", other),
        }
    }

    #[test]
    fn unknown_keywords_land_in_extra() {
        let schema = parse_dialect("blocks: []\nwidth: 3\n").unwrap();
        let keys: Vec<&str> = schema.extra.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["blocks", "width"]);
    }

    #[test]
    fn header_enum_parses_as_choices() {
        let schema =
            parse_dialect("sections:\n  - header:\n      enum: [Draft, Published]\n").unwrap();
        assert_eq!(
            schema.sections[0].header.as_ref().unwrap().choices,
            Some(vec!["Draft".to_string(), "Published".to_string()])
        );
    }
}
