use chrono::Locale;
use liwe::model::config::LinkType;
use liwe::operations::{extract, Changes, ExtractConfig};

use super::{Action, ActionContext, ActionProvider};

pub struct SectionExtract {
    pub title: String,
    pub identifier: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
    pub key_date_format: String,
    pub locale: Locale,
}

impl ActionProvider for SectionExtract {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier)
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                key: key.clone(),
                range: selection.clone(),
            })
    }

    fn changes(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Changes> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let tree = context.collect(&key);

        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .and_then(|_| {
                let graph = context.graph();
                let config = ExtractConfig {
                    key_template: self.key_template.clone(),
                    link_type: self.link_type.clone(),
                    key_date_format: self.key_date_format.clone(),
                    locale: self.locale,
                };

                extract(graph, &key, target_id, &config).ok()
            })
    }
}
