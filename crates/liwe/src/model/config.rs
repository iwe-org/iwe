use indoc::indoc;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::graph::GraphContext;

use super::{
    node::{NodeIter, NodePointer},
    NodeId,
};

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct MarkdownOptions {
    pub refs_extension: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct LibraryOptions {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default, Serialize)]
pub struct Configuration {
    pub markdown: MarkdownOptions,
    pub library: LibraryOptions,
    pub models: HashMap<String, Model>,
    pub actions: HashMap<String, BlockAction>,
}

impl Configuration {
    pub fn template() -> Self {
        Self {
            markdown: Default::default(),
            library: Default::default(),
            models: vec![(
                "fast".to_string(),
                Model {
                    api_key_env: "OPENAI_API_KEY".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                    model: "gpt-4o".to_string(),
                    max_tokens: Some(1000),
                    max_completion_tokens: Some(1000),
                    temperature: Some(0.7),
                },
            )]
            .into_iter()
            .collect(),
            actions: vec![
                (
                "rewrite".to_string(),
                BlockAction {
                    title: "Rewrite".to_string(),
                    model: "fast".to_string(),
                    prompt_template: indoc! {r##"
                        Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}}{{context_end}} tag.

                        The part you'll need to update is marked with {{update_start}}{{update_end}}.

                        {{context_start}}

                        {{context}}

                        {{context_end}}

                        - You can't replace entire text, your answer will be inserted in place of the {{update_start}}{{update_end}}. Don't include the {{context_start}}{{context_end}} and {{context_start}}{{context_end}} tags in your output.
                        - Preserve the links in the text. Do not return list item "-" or header "#" prefix

                        Your goal is to rewrite a given text to improve its clarity and readability. Ensure the language remains personable and not overly formal. Focus on simplifying language, organizing sentences logically, and removing ambiguity while maintaining a conversational tone.
                        "##}.to_string(),
                    context: Context::Document
                },
            ),
            (
            "emoji".to_string(),
            BlockAction {
                title: "Emojify".to_string(),
                model: "fast".to_string(),
                prompt_template: indoc! {r##"
                    Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}}{{context_end}} tag.

                    The part you'll need to update is marked with {{update_start}}{{update_end}}.

                    {{context_start}}

                    {{context}}

                    {{context_end}}

                    - You can't replace entire text, your answer will be inserted in place of the {{update_start}}{{update_end}}. Don't include the {{context_start}}{{context_end}} and {{context_start}}{{context_end}} tags in your output.
                    - Preserve the links in the text. Do not return list item "-" or header "#" prefix

                    Repeat the text you need to update and add a relevant emoji to the beginning. Keep the text unchanged. You have to add an emoji!
                    "##}.to_string(),
                context: Context::Document
            },
        )
            ].into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Context {
    Document,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Operation {
    Replace,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TargetType {
    Header,
    Paragraph,
}

impl TargetType {
    pub fn acceptable_target(&self, id: NodeId, context: impl GraphContext) -> Option<NodeId> {
        match self {
            TargetType::Header => Some(id).filter(|id| context.node(*id).is_header()),
            TargetType::Paragraph => Some(id).filter(|id| context.node(*id).is_leaf()),
        }
    }
}

// just create set of model configs with names
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Model {
    pub api_key_env: String,
    pub base_url: String,

    pub model: String,
    pub max_tokens: Option<u64>,
    pub max_completion_tokens: Option<u64>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BlockAction {
    pub title: String,

    pub model: String,

    pub prompt_template: String,

    pub context: Context,
}

impl BlockAction {}
