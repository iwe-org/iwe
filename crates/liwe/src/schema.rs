mod document;
mod infer;

pub use schematter_validator::{
    compile_schema, Block, BlockKind, CompiledSchema, Crumb, Document, Item, SchemaError, Section,
    Violation,
};

pub use document::build_document;
pub use infer::{infer_schema, Coverage, FieldSchema, TypeCount, ValueCount};
