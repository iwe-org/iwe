use jsonschema::{Draft, Validator};
use regex::Regex;
use serde_json::Value;
use serde_yaml::Mapping;

use crate::schema::dialect::{
    parse_dialect, AdditionalBlocks, AdditionalSections, BlockSchema, DocumentSchema, HeaderSchema,
    ItemSchema, ReducedBlock, ReducedSection, SectionSchema,
};
use crate::schema::document::BlockKind;

mod eval;

pub const DIALECT_V1: &str = "https://iwe.md/document-schema/v1";

const REDUCED_FORBIDDEN: &[&str] = &[
    "sections",
    "additionalSections",
    "allSections",
    "minContains",
    "maxContains",
];

const REDUCED_BLOCK_FORBIDDEN: &[&str] = &[
    "type",
    "lang",
    "items",
    "target",
    "blocks",
    "additionalBlocks",
    "allBlocks",
    "minContains",
    "maxContains",
    "minItems",
    "maxItems",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub pointer: String,
    pub message: String,
}

pub struct CompiledSchema {
    description: Option<String>,
    max_tokens: Option<usize>,
    max_depth: Option<usize>,
    all_sections: Option<CompiledReduced>,
    sections: Vec<CompiledSection>,
    additional_sections: CompiledAdditional,
    blocks: Vec<CompiledBlock>,
    additional_blocks: CompiledBlockAdditional,
    all_blocks: Option<CompiledReducedBlock>,
    frontmatter: Option<Validator>,
    frontmatter_source: Option<Value>,
}

struct CompiledSection {
    header: Option<CompiledHeader>,
    max_tokens: Option<usize>,
    max_depth: Option<usize>,
    min_contains: usize,
    max_contains: Option<usize>,
    description: Option<String>,
    all_sections: Option<CompiledReduced>,
    sections: Vec<CompiledSection>,
    additional_sections: CompiledAdditional,
    blocks: Vec<CompiledBlock>,
    additional_blocks: CompiledBlockAdditional,
    all_blocks: Option<CompiledReducedBlock>,
    pointer: String,
}

struct CompiledBlock {
    kind: Option<BlockKind>,
    text: Option<CompiledHeader>,
    max_tokens: Option<usize>,
    min_contains: usize,
    max_contains: Option<usize>,
    description: Option<String>,
    lang: Option<CompiledHeader>,
    target: Option<CompiledHeader>,
    items: Option<Box<CompiledItem>>,
    min_items: Option<usize>,
    max_items: Option<usize>,
    blocks: Vec<CompiledBlock>,
    additional_blocks: CompiledBlockAdditional,
    all_blocks: Option<CompiledReducedBlock>,
    pointer: String,
}

struct CompiledItem {
    text: Option<CompiledHeader>,
    max_tokens: Option<usize>,
    description: Option<String>,
    blocks: Vec<CompiledBlock>,
    additional_blocks: CompiledBlockAdditional,
    all_blocks: Option<CompiledReducedBlock>,
    pointer: String,
}

struct CompiledReducedBlock {
    text: Option<CompiledHeader>,
    max_tokens: Option<usize>,
    description: Option<String>,
    pointer: String,
}

enum CompiledBlockAdditional {
    Allow,
    Deny { pointer: String },
    Schema(Box<CompiledReducedBlock>),
}

struct CompiledHeader {
    pattern: Option<Regex>,
    konst: Option<String>,
    choices: Option<Vec<String>>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    max_tokens: Option<usize>,
    description: Option<String>,
    pointer: String,
}

struct CompiledReduced {
    header: Option<CompiledHeader>,
    max_tokens: Option<usize>,
    max_depth: Option<usize>,
    description: Option<String>,
    pointer: String,
}

enum CompiledAdditional {
    Allow,
    Deny { pointer: String },
    Schema(Box<CompiledReduced>),
}

#[derive(Clone, Copy, PartialEq)]
enum Context {
    Full,
    ReducedSection,
    ReducedBlock,
}

pub fn compile_schema(source: &str) -> Result<CompiledSchema, Vec<SchemaError>> {
    let document = match parse_dialect(source) {
        Ok(document) => document,
        Err(error) => {
            return Err(vec![SchemaError {
                pointer: String::new(),
                message: error.to_string(),
            }])
        }
    };

    let mut errors = Vec::new();
    let compiled = compile_document(&document, &mut errors);

    if errors.is_empty() {
        Ok(compiled)
    } else {
        Err(errors)
    }
}

fn compile_document(document: &DocumentSchema, errors: &mut Vec<SchemaError>) -> CompiledSchema {
    if let Some(dialect) = &document.dialect {
        if dialect != DIALECT_V1 {
            errors.push(SchemaError {
                pointer: "/$schema".to_string(),
                message: format!("unknown schema dialect '{dialect}'; expected {DIALECT_V1}"),
            });
        }
    }

    check_extra(&document.extra, "", Context::Full, errors);

    let frontmatter = document
        .frontmatter
        .as_ref()
        .and_then(|value| compile_frontmatter(value, errors));

    let all_sections = document
        .all_sections
        .as_ref()
        .map(|reduced| compile_reduced(reduced, "/allSections", errors));

    let sections = compile_sections(&document.sections, "", errors);

    let additional_sections = compile_additional(document.additional_sections.as_ref(), "", errors);

    let blocks = compile_blocks(&document.blocks, "", errors);
    let additional_blocks =
        compile_block_additional(document.additional_blocks.as_ref(), "", errors);
    let all_blocks = document
        .all_blocks
        .as_ref()
        .map(|reduced| compile_reduced_block(reduced, "/allBlocks", errors));

    CompiledSchema {
        description: document.description.clone(),
        max_tokens: document.max_tokens,
        max_depth: document.max_depth,
        all_sections,
        sections,
        additional_sections,
        blocks,
        additional_blocks,
        all_blocks,
        frontmatter,
        frontmatter_source: document.frontmatter.clone(),
    }
}

fn compile_sections(
    sections: &[SectionSchema],
    parent: &str,
    errors: &mut Vec<SchemaError>,
) -> Vec<CompiledSection> {
    sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let pointer = format!("{parent}/sections/{index}");
            compile_section(section, pointer, errors)
        })
        .collect()
}

fn compile_section(
    section: &SectionSchema,
    pointer: String,
    errors: &mut Vec<SchemaError>,
) -> CompiledSection {
    check_extra(&section.extra, &pointer, Context::Full, errors);

    let header = section
        .header
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/header"), errors));

    let (min_contains, max_contains) =
        compile_counts(section.min_contains, section.max_contains, &pointer, errors);

    let all_sections = section
        .all_sections
        .as_ref()
        .map(|reduced| compile_reduced(reduced, &format!("{pointer}/allSections"), errors));

    let sections = compile_sections(&section.sections, &pointer, errors);

    let additional_sections =
        compile_additional(section.additional_sections.as_ref(), &pointer, errors);

    let blocks = compile_blocks(&section.blocks, &pointer, errors);
    let additional_blocks =
        compile_block_additional(section.additional_blocks.as_ref(), &pointer, errors);
    let all_blocks = section
        .all_blocks
        .as_ref()
        .map(|reduced| compile_reduced_block(reduced, &format!("{pointer}/allBlocks"), errors));

    CompiledSection {
        header,
        max_tokens: section.max_tokens,
        max_depth: section.max_depth,
        min_contains,
        max_contains,
        description: section.description.clone(),
        all_sections,
        sections,
        additional_sections,
        blocks,
        additional_blocks,
        all_blocks,
        pointer,
    }
}

fn compile_blocks(
    blocks: &[BlockSchema],
    parent: &str,
    errors: &mut Vec<SchemaError>,
) -> Vec<CompiledBlock> {
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| compile_block(block, format!("{parent}/blocks/{index}"), errors))
        .collect()
}

fn compile_block(
    block: &BlockSchema,
    pointer: String,
    errors: &mut Vec<SchemaError>,
) -> CompiledBlock {
    check_extra(&block.extra, &pointer, Context::Full, errors);

    let kind = compile_block_kind(block.r#type.as_deref(), &pointer, errors);

    let is_list = matches!(
        kind,
        Some(BlockKind::BulletList) | Some(BlockKind::OrderedList)
    );
    let is_quote = kind == Some(BlockKind::Quote);

    if block.lang.is_some() && kind != Some(BlockKind::Code) {
        errors.push(applicability_error(&pointer, "lang", "type: code"));
    }
    if block.items.is_some() && !is_list {
        errors.push(applicability_error(&pointer, "items", "a list type"));
    }
    if block.min_items.is_some() && !is_list {
        errors.push(applicability_error(&pointer, "minItems", "a list type"));
    }
    if block.max_items.is_some() && !is_list {
        errors.push(applicability_error(&pointer, "maxItems", "a list type"));
    }
    if block.target.is_some() && kind != Some(BlockKind::Ref) {
        errors.push(applicability_error(&pointer, "target", "type: ref"));
    }
    if !block.blocks.is_empty() && !is_quote {
        errors.push(applicability_error(&pointer, "blocks", "type: quote"));
    }
    if block.additional_blocks.is_some() && !is_quote {
        errors.push(applicability_error(
            &pointer,
            "additionalBlocks",
            "type: quote",
        ));
    }
    if block.all_blocks.is_some() && !is_quote {
        errors.push(applicability_error(&pointer, "allBlocks", "type: quote"));
    }

    let text = block
        .text
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/text"), errors));
    let lang = block
        .lang
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/lang"), errors));
    let target = block
        .target
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/target"), errors));

    let (min_contains, max_contains) =
        compile_counts(block.min_contains, block.max_contains, &pointer, errors);
    let (min_items, max_items) =
        compile_item_counts(block.min_items, block.max_items, &pointer, errors);

    let items = block
        .items
        .as_ref()
        .map(|item| Box::new(compile_item(item, format!("{pointer}/items"), errors)));

    let blocks = compile_blocks(&block.blocks, &pointer, errors);
    let additional_blocks =
        compile_block_additional(block.additional_blocks.as_ref(), &pointer, errors);
    let all_blocks = block
        .all_blocks
        .as_ref()
        .map(|reduced| compile_reduced_block(reduced, &format!("{pointer}/allBlocks"), errors));

    CompiledBlock {
        kind,
        text,
        max_tokens: block.max_tokens,
        min_contains,
        max_contains,
        description: block.description.clone(),
        lang,
        target,
        items,
        min_items,
        max_items,
        blocks,
        additional_blocks,
        all_blocks,
        pointer,
    }
}

fn compile_item(item: &ItemSchema, pointer: String, errors: &mut Vec<SchemaError>) -> CompiledItem {
    check_extra(&item.extra, &pointer, Context::Full, errors);

    let text = item
        .text
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/text"), errors));

    let blocks = compile_blocks(&item.blocks, &pointer, errors);
    let additional_blocks =
        compile_block_additional(item.additional_blocks.as_ref(), &pointer, errors);
    let all_blocks = item
        .all_blocks
        .as_ref()
        .map(|reduced| compile_reduced_block(reduced, &format!("{pointer}/allBlocks"), errors));

    CompiledItem {
        text,
        max_tokens: item.max_tokens,
        description: item.description.clone(),
        blocks,
        additional_blocks,
        all_blocks,
        pointer,
    }
}

fn compile_reduced_block(
    reduced: &ReducedBlock,
    pointer: &str,
    errors: &mut Vec<SchemaError>,
) -> CompiledReducedBlock {
    check_extra(&reduced.extra, pointer, Context::ReducedBlock, errors);

    let text = reduced
        .text
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/text"), errors));

    CompiledReducedBlock {
        text,
        max_tokens: reduced.max_tokens,
        description: reduced.description.clone(),
        pointer: pointer.to_string(),
    }
}

fn compile_block_additional(
    additional: Option<&AdditionalBlocks>,
    parent: &str,
    errors: &mut Vec<SchemaError>,
) -> CompiledBlockAdditional {
    let pointer = format!("{parent}/additionalBlocks");
    match additional {
        None | Some(AdditionalBlocks::Bool(true)) => CompiledBlockAdditional::Allow,
        Some(AdditionalBlocks::Bool(false)) => CompiledBlockAdditional::Deny { pointer },
        Some(AdditionalBlocks::Schema(reduced)) => CompiledBlockAdditional::Schema(Box::new(
            compile_reduced_block(reduced, &pointer, errors),
        )),
    }
}

fn compile_block_kind(
    name: Option<&str>,
    pointer: &str,
    errors: &mut Vec<SchemaError>,
) -> Option<BlockKind> {
    let name = name?;
    match name {
        "paragraph" => Some(BlockKind::Paragraph),
        "bullet-list" => Some(BlockKind::BulletList),
        "ordered-list" => Some(BlockKind::OrderedList),
        "code" => Some(BlockKind::Code),
        "quote" => Some(BlockKind::Quote),
        "table" => Some(BlockKind::Table),
        "ref" => Some(BlockKind::Ref),
        "rule" => Some(BlockKind::Rule),
        other => {
            errors.push(SchemaError {
                pointer: format!("{pointer}/type"),
                message: format!("unknown block type '{other}'"),
            });
            None
        }
    }
}

fn applicability_error(pointer: &str, keyword: &str, requirement: &str) -> SchemaError {
    SchemaError {
        pointer: format!("{pointer}/{keyword}"),
        message: format!("{keyword} requires {requirement}"),
    }
}

fn compile_header(
    header: &HeaderSchema,
    pointer: String,
    errors: &mut Vec<SchemaError>,
) -> CompiledHeader {
    check_extra(&header.extra, &pointer, Context::Full, errors);

    if header.konst.is_some() && header.choices.is_some() {
        errors.push(SchemaError {
            pointer: pointer.clone(),
            message: "const and enum are mutually exclusive".to_string(),
        });
    }

    let pattern = header
        .pattern
        .as_ref()
        .and_then(|source| match Regex::new(source) {
            Ok(regex) => Some(regex),
            Err(error) => {
                errors.push(SchemaError {
                    pointer: format!("{pointer}/pattern"),
                    message: format!("invalid pattern: {error}"),
                });
                None
            }
        });

    CompiledHeader {
        pattern,
        konst: header.konst.clone(),
        choices: header.choices.clone(),
        min_length: header.min_length,
        max_length: header.max_length,
        max_tokens: header.max_tokens,
        description: header.description.clone(),
        pointer,
    }
}

fn compile_reduced(
    reduced: &ReducedSection,
    pointer: &str,
    errors: &mut Vec<SchemaError>,
) -> CompiledReduced {
    check_extra(&reduced.extra, pointer, Context::ReducedSection, errors);

    let header = reduced
        .header
        .as_ref()
        .map(|header| compile_header(header, format!("{pointer}/header"), errors));

    CompiledReduced {
        header,
        max_tokens: reduced.max_tokens,
        max_depth: reduced.max_depth,
        description: reduced.description.clone(),
        pointer: pointer.to_string(),
    }
}

fn compile_additional(
    additional: Option<&AdditionalSections>,
    parent: &str,
    errors: &mut Vec<SchemaError>,
) -> CompiledAdditional {
    let pointer = format!("{parent}/additionalSections");
    match additional {
        None | Some(AdditionalSections::Bool(true)) => CompiledAdditional::Allow,
        Some(AdditionalSections::Bool(false)) => CompiledAdditional::Deny { pointer },
        Some(AdditionalSections::Schema(reduced)) => {
            CompiledAdditional::Schema(Box::new(compile_reduced(reduced, &pointer, errors)))
        }
    }
}

fn compile_counts(
    min_contains: Option<i64>,
    max_contains: Option<i64>,
    pointer: &str,
    errors: &mut Vec<SchemaError>,
) -> (usize, Option<usize>) {
    check_count_pair(
        min_contains,
        max_contains,
        pointer,
        "minContains",
        "maxContains",
        errors,
    );

    let min = min_contains.filter(|value| *value >= 0).unwrap_or(1) as usize;
    let max = max_contains
        .filter(|value| *value >= 0)
        .map(|value| value as usize);
    (min, max)
}

fn compile_item_counts(
    min_items: Option<i64>,
    max_items: Option<i64>,
    pointer: &str,
    errors: &mut Vec<SchemaError>,
) -> (Option<usize>, Option<usize>) {
    check_count_pair(
        min_items, max_items, pointer, "minItems", "maxItems", errors,
    );

    let min = min_items
        .filter(|value| *value >= 0)
        .map(|value| value as usize);
    let max = max_items
        .filter(|value| *value >= 0)
        .map(|value| value as usize);
    (min, max)
}

fn check_count_pair(
    min: Option<i64>,
    max: Option<i64>,
    pointer: &str,
    min_key: &str,
    max_key: &str,
    errors: &mut Vec<SchemaError>,
) {
    if let Some(value) = min {
        if value < 0 {
            errors.push(SchemaError {
                pointer: format!("{pointer}/{min_key}"),
                message: format!("{min_key} must not be negative"),
            });
        }
    }
    if let Some(value) = max {
        if value < 0 {
            errors.push(SchemaError {
                pointer: format!("{pointer}/{max_key}"),
                message: format!("{max_key} must not be negative"),
            });
        }
    }
    if let (Some(min), Some(max)) = (min, max) {
        if min >= 0 && max >= 0 && min > max {
            errors.push(SchemaError {
                pointer: format!("{pointer}/{min_key}"),
                message: format!("{min_key} exceeds {max_key}"),
            });
        }
    }
}

fn compile_frontmatter(value: &Value, errors: &mut Vec<SchemaError>) -> Option<Validator> {
    if has_external_ref(value) {
        errors.push(SchemaError {
            pointer: "/frontmatter".to_string(),
            message: "external references are not allowed".to_string(),
        });
        return None;
    }

    match jsonschema::options()
        .with_draft(Draft::Draft202012)
        .should_validate_formats(true)
        .build(value)
    {
        Ok(validator) => Some(validator),
        Err(error) => {
            errors.push(SchemaError {
                pointer: "/frontmatter".to_string(),
                message: error.to_string(),
            });
            None
        }
    }
}

fn has_external_ref(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(target)) = map.get("$ref") {
                if !target.starts_with('#') {
                    return true;
                }
            }
            map.values().any(has_external_ref)
        }
        Value::Array(items) => items.iter().any(has_external_ref),
        _ => false,
    }
}

fn check_extra(extra: &Mapping, pointer: &str, context: Context, errors: &mut Vec<SchemaError>) {
    for key in extra.keys().filter_map(|key| key.as_str()) {
        let message = if context == Context::ReducedSection && REDUCED_FORBIDDEN.contains(&key) {
            format!("'{key}' is not allowed in allSections/additionalSections")
        } else if context == Context::ReducedBlock && REDUCED_BLOCK_FORBIDDEN.contains(&key) {
            format!("'{key}' is not allowed in allBlocks/additionalBlocks")
        } else {
            format!("unknown keyword '{key}'")
        };
        errors.push(SchemaError {
            pointer: format!("{pointer}/{key}"),
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors(source: &str) -> Vec<SchemaError> {
        compile_schema(source).err().unwrap_or_default()
    }

    fn error(source: &str) -> SchemaError {
        let mut all = errors(source);
        assert_eq!(all.len(), 1, "expected exactly one error, got {all:?}");
        all.remove(0)
    }

    #[test]
    fn empty_schema_compiles() {
        assert!(compile_schema("{}").is_ok());
    }

    #[test]
    fn unknown_keyword_is_rejected() {
        assert_eq!(
            error("width: 3\n"),
            SchemaError {
                pointer: "/width".to_string(),
                message: "unknown keyword 'width'".to_string(),
            }
        );
    }

    #[test]
    fn structural_keyword_in_all_sections_is_rejected() {
        assert_eq!(
            error("allSections:\n  sections: []\n"),
            SchemaError {
                pointer: "/allSections/sections".to_string(),
                message: "'sections' is not allowed in allSections/additionalSections".to_string(),
            }
        );
    }

    #[test]
    fn negative_count_is_rejected() {
        assert_eq!(
            error("sections:\n  - minContains: -1\n"),
            SchemaError {
                pointer: "/sections/0/minContains".to_string(),
                message: "minContains must not be negative".to_string(),
            }
        );
    }

    #[test]
    fn min_greater_than_max_is_rejected() {
        assert_eq!(
            error("sections:\n  - minContains: 3\n    maxContains: 1\n"),
            SchemaError {
                pointer: "/sections/0/minContains".to_string(),
                message: "minContains exceeds maxContains".to_string(),
            }
        );
    }

    #[test]
    fn const_and_enum_together_are_rejected() {
        assert_eq!(
            error("sections:\n  - header: { const: A, enum: [A, B] }\n"),
            SchemaError {
                pointer: "/sections/0/header".to_string(),
                message: "const and enum are mutually exclusive".to_string(),
            }
        );
    }

    #[test]
    fn invalid_pattern_is_rejected() {
        assert_eq!(
            error("sections:\n  - header: { pattern: \"[\" }\n"),
            SchemaError {
                pointer: "/sections/0/header/pattern".to_string(),
                message: "invalid pattern: regex parse error:\n    [\n    ^\nerror: unclosed character class".to_string(),
            }
        );
    }

    #[test]
    fn external_ref_is_rejected() {
        assert_eq!(
            error("frontmatter:\n  $ref: https://example.com/schema.json\n"),
            SchemaError {
                pointer: "/frontmatter".to_string(),
                message: "external references are not allowed".to_string(),
            }
        );
    }

    #[test]
    fn unknown_dialect_is_rejected() {
        assert_eq!(
            error("$schema: https://iwe.md/document-schema/v2\n"),
            SchemaError {
                pointer: "/$schema".to_string(),
                message: "unknown schema dialect 'https://iwe.md/document-schema/v2'; expected https://iwe.md/document-schema/v1".to_string(),
            }
        );
    }

    #[test]
    fn unknown_block_type_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: heading\n"),
            SchemaError {
                pointer: "/blocks/0/type".to_string(),
                message: "unknown block type 'heading'".to_string(),
            }
        );
    }

    #[test]
    fn lang_on_non_code_block_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: paragraph\n    lang: { const: rust }\n"),
            SchemaError {
                pointer: "/blocks/0/lang".to_string(),
                message: "lang requires type: code".to_string(),
            }
        );
    }

    #[test]
    fn items_on_non_list_block_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: code\n    items:\n      text: { maxTokens: 5 }\n"),
            SchemaError {
                pointer: "/blocks/0/items".to_string(),
                message: "items requires a list type".to_string(),
            }
        );
    }

    #[test]
    fn target_on_non_ref_block_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: table\n    target: { const: other }\n"),
            SchemaError {
                pointer: "/blocks/0/target".to_string(),
                message: "target requires type: ref".to_string(),
            }
        );
    }

    #[test]
    fn quote_keyword_on_non_quote_block_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: paragraph\n    blocks:\n      - type: paragraph\n"),
            SchemaError {
                pointer: "/blocks/0/blocks".to_string(),
                message: "blocks requires type: quote".to_string(),
            }
        );
    }

    #[test]
    fn type_specific_keyword_without_type_is_rejected() {
        assert_eq!(
            error("blocks:\n  - lang: { const: rust }\n"),
            SchemaError {
                pointer: "/blocks/0/lang".to_string(),
                message: "lang requires type: code".to_string(),
            }
        );
    }

    #[test]
    fn reduced_block_forbidden_key_is_rejected() {
        assert_eq!(
            error("allBlocks:\n  items:\n    text: { maxTokens: 5 }\n"),
            SchemaError {
                pointer: "/allBlocks/items".to_string(),
                message: "'items' is not allowed in allBlocks/additionalBlocks".to_string(),
            }
        );
    }

    #[test]
    fn negative_min_items_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: bullet-list\n    minItems: -1\n"),
            SchemaError {
                pointer: "/blocks/0/minItems".to_string(),
                message: "minItems must not be negative".to_string(),
            }
        );
    }

    #[test]
    fn min_items_greater_than_max_items_is_rejected() {
        assert_eq!(
            error("blocks:\n  - type: bullet-list\n    minItems: 4\n    maxItems: 2\n"),
            SchemaError {
                pointer: "/blocks/0/minItems".to_string(),
                message: "minItems exceeds maxItems".to_string(),
            }
        );
    }

    #[test]
    fn block_schema_compiles() {
        let source = "\
$schema: https://iwe.md/document-schema/v1
allBlocks:
  text: { maxTokens: 40 }
sections:
  - header: { pattern: \".+\" }
    blocks:
      - type: paragraph
        maxContains: 1
      - type: bullet-list
        minItems: 1
        items:
          text: { maxTokens: 40 }
          blocks:
            - type: quote
              blocks:
                - type: paragraph
      - type: code
        lang: { enum: [rust, toml] }
    additionalBlocks: false
";
        assert!(compile_schema(source).is_ok());
    }

    #[test]
    fn broken_frontmatter_meta_schema_is_rejected() {
        let source = "frontmatter:\n  type: 5\n";
        let all = errors(source);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].pointer, "/frontmatter");
    }

    #[test]
    fn valid_schema_compiles() {
        let source = "\
$schema: https://iwe.md/document-schema/v1
frontmatter:
  type: object
  required: [status]
  properties:
    status: { enum: [draft, published] }
maxTokens: 1200
sections:
  - header: { pattern: \"^[A-Z]\", maxTokens: 12 }
    maxContains: 1
    sections:
      - header: { const: Summary }
        maxContains: 1
additionalSections: false
";
        assert!(compile_schema(source).is_ok());
    }
}
