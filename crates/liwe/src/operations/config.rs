use crate::model::config::{InlineType, LinkType};

#[derive(Debug, Clone)]
pub struct ExtractConfig {
    pub key_template: String,
    pub link_type: Option<LinkType>,
    pub key_date_format: String,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            key_template: "{{slug}}".to_string(),
            link_type: Some(LinkType::Markdown),
            key_date_format: "%Y-%m-%d".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InlineConfig {
    pub inline_type: InlineType,
    pub keep_target: bool,
}

impl Default for InlineConfig {
    fn default() -> Self {
        Self {
            inline_type: InlineType::Section,
            keep_target: false,
        }
    }
}
