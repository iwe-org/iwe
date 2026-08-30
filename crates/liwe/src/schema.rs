mod document;
mod infer;

pub use schematter_validator::{
    Block, BlockKind, CompileOptions, CompiledSchema, Crumb, Document, Item, SchemaError, Section,
    Violation,
};

use schematter_validator::compile_schema_with;

use crate::query::{query_schema_uri, QUERY_SCHEMA_DRAFTS};

pub fn compile_schema(source: &str) -> Result<CompiledSchema, Vec<SchemaError>> {
    compile_schema_with(source, &iwe_compile_options()?)
}

pub fn iwe_compile_options() -> Result<CompileOptions, Vec<SchemaError>> {
    let mut options = CompileOptions::new();
    for (draft, body) in QUERY_SCHEMA_DRAFTS {
        options = options
            .with_schema_source(query_schema_uri(draft), body)
            .map_err(|error| vec![error])?;
    }
    Ok(options)
}

pub use document::build_document;
pub use infer::{infer_schema, Coverage, FieldSchema, TypeCount, ValueCount};
