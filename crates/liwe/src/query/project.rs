use serde_yaml::{Mapping, Value};

use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::{
    FieldPath, Projection, ProjectionField, ProjectionMode, ProjectionSource, PseudoField,
};
use crate::query::frontmatter::{is_reserved_segment, strip_reserved};
use crate::retrieve::EdgeRef;

pub struct ProjectionContext<'a> {
    pub graph: &'a Graph,
    pub key: &'a Key,
}

pub fn apply_projection(ctx: &ProjectionContext<'_>, projection: &Projection) -> Mapping {
    let mut out = Mapping::new();
    let effective: Vec<&ProjectionField> = match projection.mode {
        ProjectionMode::Replace => projection.fields.iter().collect(),
        ProjectionMode::Extend => {
            let default = Projection::default_for_find();
            let mut by_name: Vec<ProjectionField> = default.fields.clone();
            for f in &projection.fields {
                if let Some(existing) = by_name.iter_mut().find(|d| d.output == f.output) {
                    *existing = f.clone();
                } else {
                    by_name.push(f.clone());
                }
            }
            return write_with_user_fm(ctx, &by_name);
        }
    };
    for field in &effective {
        let v = resolve_field(ctx, field);
        out.insert(Value::String(field.output.clone()), v);
    }
    if projection.mode == ProjectionMode::Replace
        && projection.fields.iter().any(|f| matches!(
            &f.source,
            ProjectionSource::Pseudo(PseudoField::Frontmatter)
        ))
    {
        return out;
    }
    if projection.mode == ProjectionMode::Replace {
        return out;
    }
    out
}

fn write_with_user_fm(ctx: &ProjectionContext<'_>, fields: &[ProjectionField]) -> Mapping {
    let mut out = Mapping::new();
    for field in fields {
        let v = resolve_field(ctx, field);
        out.insert(Value::String(field.output.clone()), v);
    }
    if let Some(mut fm) = ctx.graph.frontmatter(ctx.key).cloned() {
        strip_reserved(&mut fm);
        for (k, v) in fm {
            if !out.contains_key(&k) {
                out.insert(k, v);
            } else if let Some(s) = k.as_str() {
                if matches!(s, "key" | "title") {
                    out.insert(k, v);
                }
            }
        }
    }
    out
}

pub fn apply_projection_or_default(
    ctx: &ProjectionContext<'_>,
    projection: Option<&Projection>,
) -> Mapping {
    match projection {
        None => write_with_user_fm(ctx, &Projection::default_for_find().fields),
        Some(p) if matches!(p.mode, ProjectionMode::Extend) => apply_projection(ctx, p),
        Some(p) => apply_projection(ctx, p),
    }
}

fn resolve_field(ctx: &ProjectionContext<'_>, field: &ProjectionField) -> Value {
    match &field.source {
        ProjectionSource::Pseudo(p) => resolve_pseudo(ctx, *p),
        ProjectionSource::Frontmatter(path) => resolve_frontmatter(ctx, path),
    }
}

fn resolve_pseudo(ctx: &ProjectionContext<'_>, p: PseudoField) -> Value {
    match p {
        PseudoField::Key => Value::String(ctx.key.to_string()),
        PseudoField::Title => Value::String(
            ctx.graph
                .get_key_title(ctx.key)
                .unwrap_or_else(|| ctx.key.to_string()),
        ),
        PseudoField::TitleSlug => {
            let title = ctx
                .graph
                .get_key_title(ctx.key)
                .unwrap_or_else(|| ctx.key.to_string());
            Value::String(slugify(&title))
        }
        PseudoField::Content => Value::String(ctx.graph.to_markdown_skip_frontmatter(ctx.key)),
        PseudoField::Frontmatter => {
            let mut fm = ctx.graph.frontmatter(ctx.key).cloned().unwrap_or_default();
            strip_reserved(&mut fm);
            Value::Mapping(fm)
        }
        PseudoField::IncludedBy => edges_to_value(crate::query::edges::included_by(ctx.graph, ctx.key)),
        PseudoField::Includes => edges_to_value(crate::query::edges::includes(ctx.graph, ctx.key)),
        PseudoField::ReferencedBy => {
            edges_to_value(crate::query::edges::referenced_by(ctx.graph, ctx.key))
        }
        PseudoField::References => edges_to_value(crate::query::edges::references(ctx.graph, ctx.key)),
        PseudoField::IncludedByCount => {
            Value::Number((crate::query::edges::included_by(ctx.graph, ctx.key).len() as i64).into())
        }
        PseudoField::IncludesCount => {
            Value::Number((crate::query::edges::includes(ctx.graph, ctx.key).len() as i64).into())
        }
        PseudoField::ReferencedByCount => Value::Number(
            (crate::query::edges::referenced_by(ctx.graph, ctx.key).len() as i64).into(),
        ),
        PseudoField::ReferencesCount => {
            Value::Number((crate::query::edges::references(ctx.graph, ctx.key).len() as i64).into())
        }
    }
}

fn resolve_frontmatter(ctx: &ProjectionContext<'_>, path: &FieldPath) -> Value {
    let Some(mut fm) = ctx.graph.frontmatter(ctx.key).cloned() else {
        return Value::Null;
    };
    strip_reserved(&mut fm);
    let mut current = Value::Mapping(fm);
    for segment in &path.0 {
        if is_reserved_segment(segment) {
            return Value::Null;
        }
        match current {
            Value::Mapping(m) => match m.get(Value::String(segment.clone())) {
                Some(v) => current = v.clone(),
                None => return Value::Null,
            },
            _ => return Value::Null,
        }
    }
    current
}

fn edges_to_value(edges: Vec<EdgeRef>) -> Value {
    serde_yaml::to_value(&edges).unwrap_or(Value::Sequence(Vec::new()))
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = true;
    for c in s.chars() {
        let lc = c.to_ascii_lowercase();
        if lc.is_ascii_alphanumeric() {
            out.push(lc);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
