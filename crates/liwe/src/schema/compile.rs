use jsonschema::{Draft, Validator};
use regex::Regex;
use serde_json::Value;
use serde_yaml::Mapping;

use crate::schema::dialect::{
    parse_dialect, AdditionalSections, DocumentSchema, HeaderSchema, ReducedSection, SectionSchema,
};

pub const DIALECT_V1: &str = "https://iwe.md/document-schema/v1";

const RESERVED_KEYWORDS: &[&str] = &[
    "blocks",
    "additionalBlocks",
    "type",
    "items",
    "minItems",
    "maxItems",
    "ordered",
    "lang",
    "text",
    "target",
];

const REDUCED_FORBIDDEN: &[&str] = &[
    "sections",
    "additionalSections",
    "allSections",
    "minContains",
    "maxContains",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub pointer: String,
    pub message: String,
}

pub struct CompiledSchema {
    pub(crate) description: Option<String>,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) all_sections: Option<CompiledReduced>,
    pub(crate) sections: Vec<CompiledSection>,
    pub(crate) additional_sections: CompiledAdditional,
    pub(crate) frontmatter: Option<Validator>,
    pub(crate) frontmatter_source: Option<Value>,
}

pub(crate) struct CompiledSection {
    pub(crate) header: Option<CompiledHeader>,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) min_contains: usize,
    pub(crate) max_contains: Option<usize>,
    pub(crate) description: Option<String>,
    pub(crate) all_sections: Option<CompiledReduced>,
    pub(crate) sections: Vec<CompiledSection>,
    pub(crate) additional_sections: CompiledAdditional,
    pub(crate) pointer: String,
}

pub(crate) struct CompiledHeader {
    pub(crate) pattern: Option<Regex>,
    pub(crate) konst: Option<String>,
    pub(crate) choices: Option<Vec<String>>,
    pub(crate) min_length: Option<usize>,
    pub(crate) max_length: Option<usize>,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) description: Option<String>,
    pub(crate) pointer: String,
}

pub(crate) struct CompiledReduced {
    pub(crate) header: Option<CompiledHeader>,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) description: Option<String>,
    pub(crate) pointer: String,
}

pub(crate) enum CompiledAdditional {
    Allow,
    Deny { pointer: String },
    Schema(Box<CompiledReduced>),
}

#[derive(Clone, Copy, PartialEq)]
enum Context {
    Full,
    Reduced,
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

    CompiledSchema {
        description: document.description.clone(),
        max_tokens: document.max_tokens,
        max_depth: document.max_depth,
        all_sections,
        sections,
        additional_sections,
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
        pointer,
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
    check_extra(&reduced.extra, pointer, Context::Reduced, errors);

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
    if let Some(min) = min_contains {
        if min < 0 {
            errors.push(SchemaError {
                pointer: format!("{pointer}/minContains"),
                message: "minContains must not be negative".to_string(),
            });
        }
    }
    if let Some(max) = max_contains {
        if max < 0 {
            errors.push(SchemaError {
                pointer: format!("{pointer}/maxContains"),
                message: "maxContains must not be negative".to_string(),
            });
        }
    }
    if let (Some(min), Some(max)) = (min_contains, max_contains) {
        if min >= 0 && max >= 0 && min > max {
            errors.push(SchemaError {
                pointer: format!("{pointer}/minContains"),
                message: "minContains exceeds maxContains".to_string(),
            });
        }
    }

    let min = min_contains.filter(|value| *value >= 0).unwrap_or(1) as usize;
    let max = max_contains
        .filter(|value| *value >= 0)
        .map(|value| value as usize);
    (min, max)
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
        let message = if context == Context::Reduced && REDUCED_FORBIDDEN.contains(&key) {
            format!("'{key}' is not allowed in allSections/additionalSections")
        } else if RESERVED_KEYWORDS.contains(&key) {
            format!("keyword '{key}' is reserved for a future version")
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
    fn reserved_keyword_is_rejected() {
        assert_eq!(
            error("blocks: []\n"),
            SchemaError {
                pointer: "/blocks".to_string(),
                message: "keyword 'blocks' is reserved for a future version".to_string(),
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
