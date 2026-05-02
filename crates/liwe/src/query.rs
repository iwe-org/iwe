pub mod builder;
pub mod document;
pub mod edges;
mod eval;
pub mod execute;
pub mod filter;
pub mod frontmatter;
mod graph_match;
pub mod prelude;
pub mod project;
pub mod sort;
pub mod update;
pub mod wire;

pub use builder::{
    build_projection, build_update_doc, parse_filter_expression, parse_operation, ParseError,
};
pub use document::{
    CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, InclusionAnchor, KeyOp, Limit,
    Operation, OperationKind, Projection, ProjectionField, ProjectionMode, ProjectionSource,
    PseudoField, ReferenceAnchor, Sort, SortDir, Update, UpdateOp, UpdateOperator, YamlType,
};
pub use eval::evaluate;
pub use execute::{execute, FindMatch, Outcome};
