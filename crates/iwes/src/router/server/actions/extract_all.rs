use liwe::model::config::LinkType;
use liwe::operations::{extract_all, Changes, ExtractConfig};

use super::{Action, ActionContext, ActionProvider};

pub struct ExtractAll {
    pub title: String,
    pub identifier: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
    pub key_date_format: String,
}

impl ActionProvider for ExtractAll {
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

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
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

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
            .and_then(|_| {
                let graph = context.graph();
                let config = ExtractConfig {
                    key_template: self.key_template.clone(),
                    link_type: self.link_type.clone(),
                    key_date_format: self.key_date_format.clone(),
                };

                extract_all(graph, &key, target_id, &config).ok()
            })
    }
}
