pub mod builder;
pub mod document;
mod eval;
pub mod execute;
pub mod filter;
pub mod frontmatter;
mod graph_match;
pub mod project;
pub mod sort;
pub mod update;
pub mod wire;

pub use builder::{parse_filter_expression, parse_operation, ParseError};
pub use document::{
    CountArg, CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, InclusionAnchor,
    KeyOp, Limit, MaxDepth, NumExpr, NumOp, Operation, OperationKind, Projection,
    ReferenceAnchor, Sort, SortDir, Update, UpdateOp, UpdateOperator, YamlType,
};
pub use eval::evaluate;
pub use execute::{execute, FindMatch, Outcome};
