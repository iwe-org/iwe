use serde_yaml::Value;

use liwe::query::block::{BlockPredicate, BlockRegex, MatchesSource};
use liwe::query::document::{
    CountOp, CountPred, DeleteOp, FieldOp, FieldPath, Filter, FindOp, InclusionAnchor, KeyOp,
    Operation, Projection, ProjectionField, ProjectionSource, PseudoField, ReferenceAnchor, Sort,
    Update, UpdateOp, YamlType,
};

pub fn filter(f: Filter) -> FindOp {
    FindOp::new().filter(f)
}

pub fn find(op: FindOp) -> Operation {
    Operation::Find(op)
}

pub fn count(op: impl Into<CountOp>) -> Operation {
    Operation::Count(op.into())
}

pub fn delete(op: impl Into<DeleteOp>) -> Operation {
    Operation::Delete(op.into())
}

pub fn update(op: UpdateOp) -> Operation {
    Operation::Update(op)
}

pub fn update_op(f: Filter, doc: Update) -> UpdateOp {
    UpdateOp::new(f, doc)
}

pub fn and(filters: Vec<Filter>) -> Filter {
    Filter::And(filters)
}

pub fn or(filters: Vec<Filter>) -> Filter {
    Filter::Or(filters)
}

pub fn nor(filters: Vec<Filter>) -> Filter {
    Filter::Nor(filters)
}

pub fn eq(path: &str, v: impl Into<Value>) -> Filter {
    Filter::eq(path, v)
}

pub fn ne(path: &str, v: impl Into<Value>) -> Filter {
    Filter::ne(path, v)
}

pub fn gt(path: &str, v: impl Into<Value>) -> Filter {
    Filter::gt(path, v)
}

pub fn gte(path: &str, v: impl Into<Value>) -> Filter {
    Filter::gte(path, v)
}

pub fn lt(path: &str, v: impl Into<Value>) -> Filter {
    Filter::lt(path, v)
}

pub fn lte(path: &str, v: impl Into<Value>) -> Filter {
    Filter::lte(path, v)
}

pub fn exists(path: &str, present: bool) -> Filter {
    Filter::exists(path, present)
}

fn field_op(path: &str, op: FieldOp) -> Filter {
    Filter::Field {
        path: FieldPath::from_dotted(path),
        op,
    }
}

pub fn in_(path: &str, values: impl IntoIterator<Item = impl Into<Value>>) -> Filter {
    field_op(
        path,
        FieldOp::In(values.into_iter().map(Into::into).collect()),
    )
}

pub fn nin(path: &str, values: impl IntoIterator<Item = impl Into<Value>>) -> Filter {
    field_op(
        path,
        FieldOp::Nin(values.into_iter().map(Into::into).collect()),
    )
}

pub fn all(path: &str, values: impl IntoIterator<Item = impl Into<Value>>) -> Filter {
    field_op(
        path,
        FieldOp::All(values.into_iter().map(Into::into).collect()),
    )
}

pub fn size(path: &str, n: u64) -> Filter {
    field_op(path, FieldOp::Size(CountPred::eq(n)))
}

pub fn type_of(path: &str, types: impl IntoIterator<Item = YamlType>) -> Filter {
    field_op(path, FieldOp::Type(types.into_iter().collect()))
}

pub fn key(op: KeyOp) -> Filter {
    Filter::Key(op)
}

pub fn content_filter(pred: BlockPredicate) -> Filter {
    Filter::Content(pred)
}

pub fn key_eq(k: impl Into<String>) -> Filter {
    Filter::Key(KeyOp::eq(k))
}

pub fn key_ne(k: impl Into<String>) -> Filter {
    Filter::Key(KeyOp::ne(k))
}

pub fn key_in(keys: &[&str]) -> Filter {
    Filter::Key(KeyOp::in_(keys))
}

pub fn key_nin(keys: &[&str]) -> Filter {
    Filter::Key(KeyOp::nin(keys))
}

pub fn includes(anchor: InclusionAnchor) -> Filter {
    Filter::Includes(Box::new(anchor))
}

pub fn included_by(anchor: InclusionAnchor) -> Filter {
    Filter::IncludedBy(Box::new(anchor))
}

pub fn references(anchor: ReferenceAnchor) -> Filter {
    Filter::References(Box::new(anchor))
}

pub fn referenced_by(anchor: ReferenceAnchor) -> Filter {
    Filter::ReferencedBy(Box::new(anchor))
}

pub fn inclusion(key: impl Into<String>, max_depth: u32) -> InclusionAnchor {
    InclusionAnchor::with_max(key, max_depth)
}

pub fn inclusion_range(key: impl Into<String>, min_depth: u32, max_depth: u32) -> InclusionAnchor {
    InclusionAnchor::new(key, min_depth, max_depth)
}

pub fn reference(key: impl Into<String>, max_distance: u32) -> ReferenceAnchor {
    ReferenceAnchor::with_max(key, max_distance)
}

pub fn reference_range(
    key: impl Into<String>,
    min_distance: u32,
    max_distance: u32,
) -> ReferenceAnchor {
    ReferenceAnchor::new(key, min_distance, max_distance)
}

pub fn any_document() -> Filter {
    Filter::all()
}

pub fn inclusion_count(match_filter: Filter, max_depth: u32, size: CountPred) -> InclusionAnchor {
    InclusionAnchor::with_match(match_filter, 1, max_depth).with_size(size)
}

pub fn reference_count(
    match_filter: Filter,
    max_distance: u32,
    size: CountPred,
) -> ReferenceAnchor {
    ReferenceAnchor::with_match(match_filter, 1, max_distance).with_size(size)
}

pub fn blocks(pred: BlockPredicate) -> ProjectionSource {
    ProjectionSource::Blocks(pred)
}

pub fn content(pred: BlockPredicate) -> ProjectionSource {
    if pred.is_empty() {
        ProjectionSource::Pseudo(PseudoField::Content)
    } else {
        ProjectionSource::ContentBlocks(pred)
    }
}

pub fn grep(pattern: &str, scope: BlockPredicate) -> ProjectionSource {
    ProjectionSource::Matches(MatchesSource {
        pattern: BlockRegex::compile(pattern).expect("valid regex"),
        scope,
    })
}

pub fn field(output: &str, source: ProjectionSource) -> ProjectionField {
    ProjectionField {
        output: output.to_string(),
        source,
    }
}

pub fn fields(names: &[&str]) -> Projection {
    Projection::fields(names)
}

pub fn asc(path: &str) -> Sort {
    Sort::asc(path)
}

pub fn desc(path: &str) -> Sort {
    Sort::desc(path)
}
