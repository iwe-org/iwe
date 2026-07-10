mod attach;
mod changes;
mod config;
mod delete;
mod extract;
mod inline;
mod rename;
mod select;
mod util;

pub use attach::{attach_reference, AttachTarget};
pub use changes::{Changes, OperationError};
pub use config::{ExtractConfig, InlineConfig};
pub use delete::delete;
pub use extract::{extract, extract_all};
pub use inline::inline;
pub use rename::rename;
pub use select::{
    references, sections, select_reference, select_section, InclusionRef, SectionRef, SelectError,
};
pub use util::{format_target_key, string_to_slug, KeyFormatContext};
