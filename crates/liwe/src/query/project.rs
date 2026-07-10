use std::cell::OnceCell;

use serde_yaml::{Mapping, Value};

use crate::graph::Graph;
use crate::model::Key;
use crate::query::block_eval::BlockIndex;
use crate::query::document::{
    FieldPath, Projection, ProjectionBase, ProjectionField, ProjectionSource, PseudoField,
};
use crate::query::edges::EdgeRef;
use crate::query::frontmatter::{is_reserved_segment, strip_reserved};

pub struct ProjectionContext<'a> {
    pub graph: &'a Graph,
    pub key: &'a Key,
    blocks: OnceCell<BlockIndex>,
}

impl<'a> ProjectionContext<'a> {
    pub fn new(graph: &'a Graph, key: &'a Key) -> Self {
        ProjectionContext {
            graph,
            key,
            blocks: OnceCell::new(),
        }
    }

    fn block_index(&self) -> &BlockIndex {
        self.blocks
            .get_or_init(|| BlockIndex::build(self.graph, self.key))
    }
}

pub fn apply_projection(ctx: &ProjectionContext<'_>, projection: &Projection) -> Mapping {
    let over_defaults;
    let fields: &[ProjectionField] = match projection.base {
        ProjectionBase::Document => {
            over_defaults = merge_over_defaults(&projection.fields);
            &over_defaults
        }
        _ => &projection.fields,
    };

    let mut out = Mapping::new();
    for field in fields {
        let v = resolve_field(ctx, field);
        out.insert(Value::String(field.output.clone()), v);
    }
    if projection.base != ProjectionBase::Empty {
        merge_user_frontmatter(ctx, &mut out);
    }
    out
}

fn merge_over_defaults(fields: &[ProjectionField]) -> Vec<ProjectionField> {
    let mut out = Projection::document_fields();
    for f in fields {
        match out.iter_mut().find(|d| d.output == f.output) {
            Some(existing) => *existing = f.clone(),
            None => out.push(f.clone()),
        }
    }
    out
}

fn merge_user_frontmatter(ctx: &ProjectionContext<'_>, out: &mut Mapping) {
    let Some(mut fm) = ctx.graph.frontmatter(ctx.key).cloned() else {
        return;
    };
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

fn resolve_field(ctx: &ProjectionContext<'_>, field: &ProjectionField) -> Value {
    match &field.source {
        ProjectionSource::Pseudo(p) => resolve_pseudo(ctx, *p),
        ProjectionSource::Frontmatter(path) => resolve_frontmatter(ctx, path),
        ProjectionSource::ContentBlocks(pred) => {
            Value::String(ctx.block_index().render_content(pred))
        }
        ProjectionSource::Blocks(pred) => Value::Sequence(
            ctx.block_index()
                .blocks_entries(pred)
                .into_iter()
                .map(Value::Mapping)
                .collect(),
        ),
        ProjectionSource::Matches(source) => Value::Sequence(
            ctx.block_index()
                .matches_entries(source)
                .into_iter()
                .map(Value::Mapping)
                .collect(),
        ),
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
        PseudoField::IncludedBy => {
            edges_to_value(crate::query::edges::included_by(ctx.graph, ctx.key))
        }
        PseudoField::Includes => edges_to_value(crate::query::edges::includes(ctx.graph, ctx.key)),
        PseudoField::ReferencedBy => {
            edges_to_value(crate::query::edges::referenced_by(ctx.graph, ctx.key))
        }
        PseudoField::References => {
            edges_to_value(crate::query::edges::references(ctx.graph, ctx.key))
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
