use indoc::indoc;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::graph::GraphContext;

use super::{node::NodeIter, NodeId};

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct MarkdownOptions {
    pub refs_extension: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct LibraryOptions {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Configuration {
    pub markdown: MarkdownOptions,
    pub library: LibraryOptions,
    pub models: HashMap<String, Model>,
    pub actions: HashMap<String, BlockAction>,
    pub prompt_key_prefix: Option<String>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            markdown: Default::default(),
            library: Default::default(),
            models: Default::default(),
            actions: Default::default(),
            prompt_key_prefix: Some("prompt".to_string()),
        }
    }
}

impl Configuration {
    pub fn template() -> Self {
        Self {
            markdown: Default::default(),
            library: Default::default(),
            prompt_key_prefix: Some("prompt".to_string()),
            models: vec![
                (
                "default".to_string(),
                Model {
                    api_key_env: "OPENAI_API_KEY".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                    name: "gpt-4o".to_string(),
                    max_tokens: None,
                    max_completion_tokens: None,
                    temperature: None,
                },
                ),
                (
                "fast".to_string(),
                Model {
                    api_key_env: "OPENAI_API_KEY".to_string(),
                    base_url: "https://api.openai.com".to_string(),
                    name: "gpt-4o-mini".to_string(),
                    max_tokens: None,
                    max_completion_tokens: None,
                    temperature: None,
                },
            )
            ]
            .into_iter()
            .collect(),
            actions: vec![
                (
                "rewrite".to_string(),
                BlockAction {
                    title: "Rewrite".to_string(),
                    model: "default".to_string(),
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
            "expand".to_string(),
            BlockAction {
                title: "Expand".to_string(),
                model: "default".to_string(),
                prompt_template: indoc! {r##"
                    Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}}{{context_end}} tag.

                    The part you'll need to update is marked with {{update_start}}{{update_end}}.

                    {{context_start}}

                    {{context}}

                    {{context_end}}

                    - You can't replace entire text, your answer will be inserted in place of the {{update_start}}{{update_end}}. Don't include the {{context_start}}{{context_end}} and {{context_start}}{{context_end}} tags in your output.
                    - Preserve the links in the text. Do not return list item "-" or header "#" prefix

                    Expand the text you need to update, generate a couple paragraphs.
                    "##}.to_string(),
                context: Context::Document
            },
        ),
            (
            "keywords".to_string(),
            BlockAction {
                title: "Keywords".to_string(),
                model: "default".to_string(),
                prompt_template: indoc! {r##"
                    Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}}{{context_end}} tag.

                    The part you'll need to update is marked with {{update_start}}{{update_end}}.

                    {{context_start}}

                    {{context}}

                    {{context_end}}

                    - You can't replace entire text, your answer will be inserted in place of the {{update_start}}{{update_end}}. Don't include the {{context_start}}{{context_end}} and {{context_start}}{{context_end}} tags in your output.

                    Mark most important keywords with bold using ** markdown syntax. Keep the text unchanged!
                    "##}.to_string(),
                context: Context::Document
            },
        ),
            (
            "emoji".to_string(),
            BlockAction {
                title: "Emojify".to_string(),
                model: "default".to_string(),
                prompt_template: indoc! {r##"
                    Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}} {{context_end}} tags.

                    - The part you'll need to update is marked with {{update_start}} {{update_end}} tags.
                    - You can't replace entire text, your answer will be inserted in between {{update_start}} {{update_end}} tags.
                    - Add a relevant emoji one per list item (prior to list item text), header (prior to header text) or paragraph. Keep the text otherwise unchanged.
                    - Don't include the {{update_start}} {{update_end}} tags in your answer.

                    {{context_start}}

                    {{context}}

                    {{context_end}}
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
    Block,
    Paragraph,
    List,
}

impl TargetType {
    pub fn acceptable_target(&self, id: NodeId, context: impl GraphContext) -> Option<NodeId> {
        match self {
            TargetType::Block => Some(id).filter(|id| !context.node(*id).is_section()),
            TargetType::Paragraph => Some(id).filter(|id| context.node(*id).is_leaf()),
            TargetType::List => Some(id).filter(|id| context.node(*id).is_leaf()),
        }
    }
}

// just create set of model configs with names
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Model {
    pub api_key_env: String,
    pub base_url: String,

    pub name: String,
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
