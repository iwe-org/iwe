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
    #[serde(default)]
    pub blocks: Vec<BlockSchema>,
    pub additional_blocks: Option<AdditionalBlocks>,
    pub all_blocks: Option<ReducedBlock>,
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
    #[serde(default)]
    pub blocks: Vec<BlockSchema>,
    pub additional_blocks: Option<AdditionalBlocks>,
    pub all_blocks: Option<ReducedBlock>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum TypeSpec {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockSchema {
    pub r#type: Option<TypeSpec>,
    pub text: Option<HeaderSchema>,
    pub max_tokens: Option<usize>,
    pub min_contains: Option<i64>,
    pub max_contains: Option<i64>,
    pub description: Option<String>,
    pub lang: Option<HeaderSchema>,
    pub items: Option<Box<ItemSchema>>,
    pub min_items: Option<i64>,
    pub max_items: Option<i64>,
    pub target: Option<HeaderSchema>,
    #[serde(default)]
    pub blocks: Vec<BlockSchema>,
    pub additional_blocks: Option<AdditionalBlocks>,
    pub all_blocks: Option<ReducedBlock>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSchema {
    pub text: Option<HeaderSchema>,
    pub max_tokens: Option<usize>,
    pub description: Option<String>,
    #[serde(default)]
    pub blocks: Vec<BlockSchema>,
    pub additional_blocks: Option<AdditionalBlocks>,
    pub all_blocks: Option<ReducedBlock>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReducedBlock {
    pub text: Option<HeaderSchema>,
    pub max_tokens: Option<usize>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: Mapping,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AdditionalBlocks {
    Bool(bool),
    Schema(Box<ReducedBlock>),
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
        let schema = parse_dialect("width: 3\nordered: true\n").unwrap();
        let keys: Vec<&str> = schema.extra.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["width", "ordered"]);
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

    #[test]
    fn block_keywords_bind_to_fields() {
        let source = "\
blocks:
  - type: code
    lang: { enum: [rust, toml] }
    maxContains: 1
  - type: bullet-list
    minItems: 1
    maxItems: 10
    items:
      text: { maxTokens: 40 }
additionalBlocks: false
allBlocks:
  text: { maxTokens: 40 }
";
        let schema = parse_dialect(source).unwrap();
        assert_eq!(schema.blocks.len(), 2);
        assert_eq!(schema.extra, Mapping::new());

        let code = &schema.blocks[0];
        assert_eq!(code.r#type, Some(TypeSpec::One("code".to_string())));
        assert_eq!(code.max_contains, Some(1));
        assert_eq!(
            code.lang.as_ref().unwrap().choices,
            Some(vec!["rust".to_string(), "toml".to_string()])
        );

        let list = &schema.blocks[1];
        assert_eq!(list.r#type, Some(TypeSpec::One("bullet-list".to_string())));
        assert_eq!(list.min_items, Some(1));
        assert_eq!(list.max_items, Some(10));
        assert_eq!(
            list.items
                .as_ref()
                .unwrap()
                .text
                .as_ref()
                .unwrap()
                .max_tokens,
            Some(40)
        );

        match schema.additional_blocks {
            Some(AdditionalBlocks::Bool(false)) => {}
            other => panic!("expected additionalBlocks false, got {:?}", other),
        }
        assert_eq!(
            schema.all_blocks.unwrap().text.unwrap().max_tokens,
            Some(40)
        );
    }

    #[test]
    fn additional_blocks_accepts_reduced_schema() {
        let schema = parse_dialect("additionalBlocks:\n  maxTokens: 20\n").unwrap();
        match schema.additional_blocks {
            Some(AdditionalBlocks::Schema(reduced)) => {
                assert_eq!(reduced.max_tokens, Some(20));
            }
            other => panic!("expected reduced schema, got {:?}", other),
        }
    }

    #[test]
    fn ordered_list_type_and_quote_recursion_parse() {
        let source = "\
blocks:
  - type: ordered-list
  - type: quote
    blocks:
      - type: paragraph
    allBlocks:
      maxTokens: 5
";
        let schema = parse_dialect(source).unwrap();
        assert_eq!(
            schema.blocks[0].r#type,
            Some(TypeSpec::One("ordered-list".to_string()))
        );
        let quote = &schema.blocks[1];
        assert_eq!(quote.r#type, Some(TypeSpec::One("quote".to_string())));
        assert_eq!(
            quote.blocks[0].r#type,
            Some(TypeSpec::One("paragraph".to_string()))
        );
        assert_eq!(quote.all_blocks.as_ref().unwrap().max_tokens, Some(5));
    }

    #[test]
    fn unknown_block_keyword_lands_in_block_extra() {
        let schema = parse_dialect("blocks:\n  - type: paragraph\n    width: 3\n").unwrap();
        let keys: Vec<&str> = schema.blocks[0]
            .extra
            .keys()
            .filter_map(|k| k.as_str())
            .collect();
        assert_eq!(keys, vec!["width"]);
    }
}
