use diwe::tokens::count_tokens;
use liwe::model::Key;
use liwe::schema::{build_document, compile_schema};

use crate::internal::claude::hook::store::MemoryStore;

pub const POLICY_SCHEMA: &str = include_str!("../../../templates/claude/policy.schema.yaml");

pub fn policy_violations(store: &MemoryStore) -> Result<Vec<String>, Vec<String>> {
    let compiled = compile_schema(POLICY_SCHEMA).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                if error.pointer.is_empty() {
                    error.message
                } else {
                    format!("{}: {}", error.pointer, error.message)
                }
            })
            .collect::<Vec<_>>()
    })?;

    let document = build_document(store.graph(), &Key::name("MEMORY"), count_tokens);
    Ok(compiled
        .validate(&document)
        .into_iter()
        .map(|violation| {
            let breadcrumb = violation.breadcrumb_text();
            let mut line = if breadcrumb.is_empty() {
                violation.message
            } else {
                format!("{} › {}", breadcrumb, violation.message)
            };
            if let Some(hint) = violation.hint {
                line.push_str(&format!("\n  hint: {}", hint));
            }
            line
        })
        .collect())
}

pub fn policy_report(store: &MemoryStore) -> (String, bool) {
    match policy_violations(store) {
        Err(errors) => (
            format!(
                "the embedded policy schema does not compile:\n{}\n",
                errors.join("\n")
            ),
            false,
        ),
        Ok(violations) if violations.is_empty() => (
            "MEMORY: ok — the policy carries the knobs and sections this binary reads\n"
                .to_string(),
            true,
        ),
        Ok(violations) => {
            let mut report = format!("MEMORY: {} problem(s)\n", violations.len());
            for violation in &violations {
                report.push_str(&format!("{}\n", violation));
            }
            (report, false)
        }
    }
}
