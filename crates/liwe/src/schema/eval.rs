use crate::schema::compile::{
    CompiledAdditional, CompiledHeader, CompiledReduced, CompiledSchema, CompiledSection,
};
use crate::schema::document::{Document, Section};
use crate::schema::violation::{Crumb, Violation};

impl CompiledSchema {
    pub fn validate(&self, document: &Document) -> Vec<Violation> {
        let mut out = Vec::new();

        if let Some(validator) = &self.frontmatter {
            for error in validator.iter_errors(&document.frontmatter) {
                let schema_path = error.schema_path().as_str().to_string();
                out.push(Violation {
                    breadcrumb: frontmatter_breadcrumb(error.instance_path().as_str()),
                    message: error.to_string(),
                    hint: frontmatter_hint(
                        self.frontmatter_source.as_ref(),
                        &schema_path,
                        self.description.as_deref(),
                    ),
                    keyword: last_segment(&schema_path),
                    schema_pointer: format!("/frontmatter{schema_path}"),
                });
            }
        }

        if let Some(max) = self.max_tokens {
            if document.body_tokens > max {
                out.push(violation(
                    &[],
                    format!("body is {} tokens (limit {max})", document.body_tokens),
                    hint(&[], &[self.description.as_deref()]),
                    "/maxTokens".to_string(),
                    "maxTokens",
                ));
            }
        }

        let descs = vec![self.description.as_deref()];

        if let Some(limit) = self.max_depth {
            report_deep(
                &document.sections,
                limit,
                "/maxDepth",
                self.description.as_deref(),
                &descs,
                &[],
                &mut out,
            );
        }

        let scope: Vec<&CompiledReduced> = self.all_sections.iter().collect();
        walk(
            &self.sections,
            &self.additional_sections,
            &scope,
            &document.sections,
            &[],
            &descs,
            &mut out,
        );

        out
    }
}

fn walk<'a>(
    entries: &'a [CompiledSection],
    additional: &'a CompiledAdditional,
    all_scope: &[&'a CompiledReduced],
    sections: &[Section],
    crumbs: &[Crumb],
    descs: &[Option<&'a str>],
    out: &mut Vec<Violation>,
) {
    let mut pointer = 0;
    let mut binds: Vec<Option<usize>> = Vec::with_capacity(sections.len());
    let mut counts = vec![0usize; entries.len()];
    for section in sections {
        let mut matched = None;
        let mut candidate = pointer;
        while candidate < entries.len() {
            if entry_identity_matches(&entries[candidate], &section.header) {
                matched = Some(candidate);
                break;
            }
            candidate += 1;
        }
        if let Some(entry) = matched {
            pointer = entry;
            counts[entry] += 1;
            binds.push(Some(entry));
        } else {
            binds.push(None);
        }
    }

    for (index, entry) in entries.iter().enumerate() {
        let count = counts[index];
        if count < entry.min_contains {
            out.push(violation(
                crumbs,
                format!("required section {} missing", entry_name(entry, index)),
                hint(&[entry.description.as_deref()], descs),
                format!("{}/minContains", entry.pointer),
                "minContains",
            ));
        }
        if let Some(max) = entry.max_contains {
            if count > max {
                out.push(violation(
                    crumbs,
                    format!(
                        "section {} occurs {count} times (maximum {max})",
                        entry_name(entry, index)
                    ),
                    hint(&[entry.description.as_deref()], descs),
                    format!("{}/maxContains", entry.pointer),
                    "maxContains",
                ));
            }
        }
    }

    for (index, section) in sections.iter().enumerate() {
        let mut child_crumbs = crumbs.to_vec();
        child_crumbs.push(section_crumb(section, index));

        for reduced in all_scope {
            apply_reduced(reduced, section, &child_crumbs, descs, out);
        }

        match binds[index] {
            Some(entry_index) => {
                let entry = &entries[entry_index];
                if let Some(header) = &entry.header {
                    check_header(
                        header,
                        section,
                        false,
                        entry.description.as_deref(),
                        descs,
                        &child_crumbs,
                        out,
                    );
                }
                if let Some(max) = entry.max_tokens {
                    if section.subtree_tokens > max {
                        out.push(violation(
                            &child_crumbs,
                            format!("section is {} tokens (limit {max})", section.subtree_tokens),
                            hint(&[entry.description.as_deref()], descs),
                            format!("{}/maxTokens", entry.pointer),
                            "maxTokens",
                        ));
                    }
                }
                if let Some(depth) = entry.max_depth {
                    report_deep(
                        &section.sections,
                        section.level + depth,
                        &format!("{}/maxDepth", entry.pointer),
                        entry.description.as_deref(),
                        descs,
                        &child_crumbs,
                        out,
                    );
                }

                let mut child_scope = all_scope.to_vec();
                if let Some(reduced) = &entry.all_sections {
                    child_scope.push(reduced);
                }
                let mut child_descs = descs.to_vec();
                child_descs.push(entry.description.as_deref());
                walk(
                    &entry.sections,
                    &entry.additional_sections,
                    &child_scope,
                    &section.sections,
                    &child_crumbs,
                    &child_descs,
                    out,
                );
            }
            None => {
                match additional {
                    CompiledAdditional::Allow => {}
                    CompiledAdditional::Deny { pointer } => {
                        out.push(violation(
                            &child_crumbs,
                            "unexpected section".to_string(),
                            hint(&[], descs),
                            pointer.clone(),
                            "additionalSections",
                        ));
                    }
                    CompiledAdditional::Schema(reduced) => {
                        apply_reduced(reduced, section, &child_crumbs, descs, out);
                    }
                }
                walk(
                    &[],
                    &CompiledAdditional::Allow,
                    all_scope,
                    &section.sections,
                    &child_crumbs,
                    descs,
                    out,
                );
            }
        }
    }
}

fn apply_reduced<'a>(
    reduced: &'a CompiledReduced,
    section: &Section,
    crumbs: &[Crumb],
    descs: &[Option<&'a str>],
    out: &mut Vec<Violation>,
) {
    if let Some(header) = &reduced.header {
        check_header(
            header,
            section,
            true,
            reduced.description.as_deref(),
            descs,
            crumbs,
            out,
        );
    }
    if let Some(max) = reduced.max_tokens {
        if section.subtree_tokens > max {
            out.push(violation(
                crumbs,
                format!("section is {} tokens (limit {max})", section.subtree_tokens),
                hint(&[reduced.description.as_deref()], descs),
                format!("{}/maxTokens", reduced.pointer),
                "maxTokens",
            ));
        }
    }
    if let Some(depth) = reduced.max_depth {
        report_deep(
            &section.sections,
            section.level + depth,
            &format!("{}/maxDepth", reduced.pointer),
            reduced.description.as_deref(),
            descs,
            crumbs,
            out,
        );
    }
}

fn check_header<'a>(
    header: &'a CompiledHeader,
    section: &Section,
    include_identity: bool,
    outer_desc: Option<&'a str>,
    descs: &[Option<&'a str>],
    crumbs: &[Crumb],
    out: &mut Vec<Violation>,
) {
    let text = &section.header;
    let chain = [header.description.as_deref(), outer_desc];

    if include_identity {
        if let Some(pattern) = &header.pattern {
            if !pattern.is_match(text) {
                out.push(violation(
                    crumbs,
                    format!(
                        "header '{text}' does not match pattern '{}'",
                        pattern.as_str()
                    ),
                    hint(&chain, descs),
                    format!("{}/pattern", header.pointer),
                    "pattern",
                ));
            }
        }
        if let Some(expected) = &header.konst {
            if text != expected {
                out.push(violation(
                    crumbs,
                    format!("header is '{text}', expected '{expected}'"),
                    hint(&chain, descs),
                    format!("{}/const", header.pointer),
                    "const",
                ));
            }
        }
        if let Some(choices) = &header.choices {
            if !choices.iter().any(|choice| choice == text) {
                out.push(violation(
                    crumbs,
                    format!("header '{text}' is not one of {}", quote_join(choices)),
                    hint(&chain, descs),
                    format!("{}/enum", header.pointer),
                    "enum",
                ));
            }
        }
    }

    let length = text.chars().count();
    if let Some(min) = header.min_length {
        if length < min {
            out.push(violation(
                crumbs,
                format!("header is {length} characters (minimum {min})"),
                hint(&chain, descs),
                format!("{}/minLength", header.pointer),
                "minLength",
            ));
        }
    }
    if let Some(max) = header.max_length {
        if length > max {
            out.push(violation(
                crumbs,
                format!("header is {length} characters (maximum {max})"),
                hint(&chain, descs),
                format!("{}/maxLength", header.pointer),
                "maxLength",
            ));
        }
    }
    if let Some(max) = header.max_tokens {
        if section.header_tokens > max {
            out.push(violation(
                crumbs,
                format!("header is {} tokens (limit {max})", section.header_tokens),
                hint(&chain, descs),
                format!("{}/maxTokens", header.pointer),
                "maxTokens",
            ));
        }
    }
}

fn report_deep(
    sections: &[Section],
    limit: usize,
    pointer: &str,
    own_desc: Option<&str>,
    descs: &[Option<&str>],
    base_crumbs: &[Crumb],
    out: &mut Vec<Violation>,
) {
    for (index, section) in sections.iter().enumerate() {
        let mut crumbs = base_crumbs.to_vec();
        crumbs.push(section_crumb(section, index));
        if section.level > limit {
            out.push(violation(
                &crumbs,
                format!("heading depth {} exceeds maximum {limit}", section.level),
                hint(&[own_desc], descs),
                pointer.to_string(),
                "maxDepth",
            ));
        }
        report_deep(
            &section.sections,
            limit,
            pointer,
            own_desc,
            descs,
            &crumbs,
            out,
        );
    }
}

fn entry_identity_matches(entry: &CompiledSection, text: &str) -> bool {
    match &entry.header {
        None => true,
        Some(header) => header_identity_matches(header, text),
    }
}

fn header_identity_matches(header: &CompiledHeader, text: &str) -> bool {
    if let Some(pattern) = &header.pattern {
        if !pattern.is_match(text) {
            return false;
        }
    }
    if let Some(expected) = &header.konst {
        if text != expected {
            return false;
        }
    }
    if let Some(choices) = &header.choices {
        if !choices.iter().any(|choice| choice == text) {
            return false;
        }
    }
    true
}

fn entry_name(entry: &CompiledSection, index: usize) -> String {
    if let Some(header) = &entry.header {
        if let Some(expected) = &header.konst {
            return format!("'{expected}'");
        }
        if let Some(choices) = &header.choices {
            return format!("one of {}", quote_join(choices));
        }
        if let Some(pattern) = &header.pattern {
            return format!("matching '{}'", pattern.as_str());
        }
    }
    format!("at position {index}")
}

fn section_crumb(section: &Section, index: usize) -> Crumb {
    if section.header.is_empty() {
        Crumb::Position(index)
    } else {
        Crumb::Header(section.header.clone())
    }
}

fn quote_join(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("'{value}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn hint(chain: &[Option<&str>], descs: &[Option<&str>]) -> Option<String> {
    chain
        .iter()
        .flatten()
        .next()
        .or_else(|| descs.iter().rev().flatten().next())
        .map(|value| value.to_string())
}

fn violation(
    crumbs: &[Crumb],
    message: String,
    hint: Option<String>,
    schema_pointer: String,
    keyword: &str,
) -> Violation {
    Violation {
        breadcrumb: crumbs.to_vec(),
        message,
        hint,
        schema_pointer,
        keyword: keyword.to_string(),
    }
}

fn frontmatter_breadcrumb(instance_path: &str) -> Vec<Crumb> {
    let mut crumbs = vec![Crumb::Frontmatter];
    for segment in instance_path.split('/').filter(|part| !part.is_empty()) {
        crumbs.push(Crumb::Field(segment.to_string()));
    }
    crumbs
}

fn frontmatter_hint(
    source: Option<&serde_json::Value>,
    schema_path: &str,
    document_description: Option<&str>,
) -> Option<String> {
    if let Some(source) = source {
        let segments: Vec<&str> = schema_path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        for end in (0..segments.len()).rev() {
            let pointer = if end == 0 {
                String::new()
            } else {
                format!("/{}", segments[..end].join("/"))
            };
            if let Some(serde_json::Value::Object(map)) = source.pointer(&pointer) {
                if let Some(serde_json::Value::String(description)) = map.get("description") {
                    return Some(description.clone());
                }
            }
        }
    }
    document_description.map(|value| value.to_string())
}

fn last_segment(schema_path: &str) -> String {
    schema_path
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::schema::compile::compile_schema;
    use crate::schema::document::{Document, Section};
    use crate::schema::violation::{Crumb, Violation};

    fn node(
        header: &str,
        level: usize,
        header_tokens: usize,
        subtree_tokens: usize,
        sections: Vec<Section>,
    ) -> Section {
        Section {
            header: header.to_string(),
            level,
            header_tokens,
            subtree_tokens,
            sections,
        }
    }

    fn leaf(header: &str, level: usize) -> Section {
        node(header, level, 1, 1, vec![])
    }

    fn parent(header: &str, level: usize, sections: Vec<Section>) -> Section {
        node(header, level, 1, 1, sections)
    }

    fn document(sections: Vec<Section>) -> Document {
        Document {
            frontmatter: json!({}),
            body_tokens: 0,
            sections,
        }
    }

    fn header_crumb(text: &str) -> Crumb {
        Crumb::Header(text.to_string())
    }

    fn violation(
        breadcrumb: Vec<Crumb>,
        message: &str,
        hint: Option<&str>,
        schema_pointer: &str,
        keyword: &str,
    ) -> Violation {
        Violation {
            breadcrumb,
            message: message.to_string(),
            hint: hint.map(str::to_string),
            schema_pointer: schema_pointer.to_string(),
            keyword: keyword.to_string(),
        }
    }

    fn validate(schema: &str, document: &Document) -> Vec<Violation> {
        compile_schema(schema).unwrap().validate(document)
    }

    #[test]
    fn empty_schema_passes_any_document() {
        let document = document(vec![parent("A", 1, vec![leaf("B", 2)])]);
        assert_eq!(validate("{}", &document), vec![]);
    }

    #[test]
    fn missing_required_and_closed_template() {
        let schema = "\
sections:
  - header: { const: Summary }
  - header: { const: Tasks }
additionalSections: false
";
        let document = document(vec![leaf("Notes", 1)]);
        assert_eq!(
            validate(schema, &document),
            vec![
                violation(
                    vec![],
                    "required section 'Summary' missing",
                    None,
                    "/sections/0/minContains",
                    "minContains",
                ),
                violation(
                    vec![],
                    "required section 'Tasks' missing",
                    None,
                    "/sections/1/minContains",
                    "minContains",
                ),
                violation(
                    vec![header_crumb("Notes")],
                    "unexpected section",
                    None,
                    "/additionalSections",
                    "additionalSections",
                ),
            ]
        );
    }

    #[test]
    fn binding_by_header_validates_the_rest() {
        let schema = "\
sections:
  - header: { const: Tasks }
    sections:
      - header: { const: Done }
";
        let document = document(vec![parent("Tasks", 1, vec![leaf("Todo", 2)])]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![header_crumb("Tasks")],
                "required section 'Done' missing",
                None,
                "/sections/0/sections/0/minContains",
                "minContains",
            )]
        );
    }

    #[test]
    fn consecutive_run_below_min_contains() {
        let schema = "\
sections:
  - header: { pattern: '^\\d{4}-\\d{2}-\\d{2}$' }
    minContains: 3
";
        let document = document(vec![
            leaf("2026-01-01", 1),
            leaf("2026-01-02", 1),
            leaf("Notes", 1),
        ]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![],
                "required section matching '^\\d{4}-\\d{2}-\\d{2}$' missing",
                None,
                "/sections/0/minContains",
                "minContains",
            )]
        );
    }

    #[test]
    fn greedy_wildcard_absorbs_later_match() {
        let schema = "\
sections:
  - {}
  - header: { const: Last }
";
        let document = document(vec![leaf("A", 1), leaf("Last", 1)]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![],
                "required section 'Last' missing",
                None,
                "/sections/1/minContains",
                "minContains",
            )]
        );
    }

    #[test]
    fn out_of_order_section_is_additional() {
        let schema = "\
sections:
  - header: { const: A }
  - header: { const: B }
additionalSections: true
";
        let document = document(vec![leaf("B", 1), leaf("A", 1)]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![],
                "required section 'A' missing",
                None,
                "/sections/0/minContains",
                "minContains",
            )]
        );
    }

    #[test]
    fn all_sections_applies_at_every_depth() {
        let schema = "\
allSections:
  header: { pattern: \"^[A-Z]\" }
";
        let document = document(vec![parent("Intro", 1, vec![leaf("lower", 2)])]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![header_crumb("Intro"), header_crumb("lower")],
                "header 'lower' does not match pattern '^[A-Z]'",
                None,
                "/allSections/header/pattern",
                "pattern",
            )]
        );
    }

    #[test]
    fn document_max_depth_flags_deep_section() {
        let schema = "maxDepth: 1\n";
        let document = document(vec![parent("A", 1, vec![leaf("B", 2)])]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![header_crumb("A"), header_crumb("B")],
                "heading depth 2 exceeds maximum 1",
                None,
                "/maxDepth",
                "maxDepth",
            )]
        );
    }

    #[test]
    fn header_and_section_token_budgets() {
        let schema = "\
sections:
  - header: { const: Big, maxTokens: 1 }
    maxTokens: 5
";
        let document = document(vec![node("Big", 1, 3, 10, vec![])]);
        assert_eq!(
            validate(schema, &document),
            vec![
                violation(
                    vec![header_crumb("Big")],
                    "header is 3 tokens (limit 1)",
                    None,
                    "/sections/0/header/maxTokens",
                    "maxTokens",
                ),
                violation(
                    vec![header_crumb("Big")],
                    "section is 10 tokens (limit 5)",
                    None,
                    "/sections/0/maxTokens",
                    "maxTokens",
                ),
            ]
        );
    }

    #[test]
    fn additional_sections_schema_validates_leftover() {
        let schema = "\
sections:
  - header: { const: Intro }
additionalSections:
  maxTokens: 5
";
        let document = document(vec![
            node("Intro", 1, 1, 3, vec![]),
            node("Extra", 1, 1, 10, vec![]),
        ]);
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![header_crumb("Extra")],
                "section is 10 tokens (limit 5)",
                None,
                "/additionalSections/maxTokens",
                "maxTokens",
            )]
        );
    }

    #[test]
    fn frontmatter_enum_reports_with_description_hint() {
        let schema = "\
frontmatter:
  type: object
  required: [status]
  properties:
    status:
      enum: [draft, published]
      description: every note declares a status
";
        let mut document = document(vec![]);
        document.frontmatter = json!({ "status": "archived" });
        assert_eq!(
            validate(schema, &document),
            vec![violation(
                vec![Crumb::Frontmatter, Crumb::Field("status".to_string())],
                "\"archived\" is not one of \"draft\" or \"published\"",
                Some("every note declares a status"),
                "/frontmatter/properties/status/enum",
                "enum",
            )]
        );
    }
}
