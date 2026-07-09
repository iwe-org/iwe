pub mod block;
pub mod block_eval;
pub mod block_update;
pub mod builder;
pub mod cli;
pub mod document;
pub mod edges;
mod eval;
pub mod execute;
pub mod filter;
pub mod frontmatter;
mod graph_match;
pub mod project;
pub mod sort;
pub mod update;
pub mod wire;

pub use builder::{
    build_projection, build_update_doc, parse_expect, parse_filter_expression, parse_operation,
    ParseError,
};
pub use document::{
    BlockUpdate, BlockUpdateOp, CountOp, DeleteOp, Expect, FieldOp, FieldPath, Filter, FindOp,
    InclusionAnchor, KeyOp, Limit, Operation, OperationKind, Projection, ProjectionBase,
    ProjectionField, ProjectionSource, PseudoField, ReferenceAnchor, Sort, SortDir, Update,
    UpdateOp, UpdateOperator, YamlType,
};
pub use eval::evaluate;
pub use execute::{execute, strict_guard_violations, FindMatch, Outcome};
