use indoc::indoc;
use log::debug;
use std::{collections::HashMap, env, fs::read_to_string};
use toml_edit::{value, DocumentMut, Item};

use serde::{Deserialize, Serialize};

use crate::graph::GraphContext;

use super::{node::NodeIter, NodeId};

const CONFIG_FILE_NAME: &str = "config.toml";
const IWE_MARKER: &str = ".iwe";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MarkdownOptions {
    pub refs_extension: String,
    pub date_format: Option<String>,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            refs_extension: String::new(),
            date_format: Some("%b %d, %Y".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LibraryOptions {
    pub path: String,
    pub date_format: Option<String>,
    pub prompt_key_prefix: Option<String>,
}

impl Default for LibraryOptions {
    fn default() -> Self {
        Self {
            path: String::new(),
            date_format: Some("%Y-%m-%d".into()),
            prompt_key_prefix: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Configuration {
    pub markdown: MarkdownOptions,
    pub library: LibraryOptions,
    pub models: HashMap<String, Model>,
    pub actions: HashMap<String, BlockAction>,
}

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
#[serde(tag = "type")]
pub enum BlockAction {
    #[serde(rename = "transform")]
    Transform(Transform),
    #[serde(rename = "attach")]
    Attach(Attach),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Transform {
    pub title: String,
    pub model: String,
    pub prompt_template: String,
    pub context: Context,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Attach {
    pub title: String,
    pub key_template: String,
    pub document_template: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            markdown: Default::default(),
            library: Default::default(),
            models: Default::default(),
            actions: Default::default(),
        }
    }
}

impl Configuration {
    pub fn template() -> Self {
        let mut template = Self {
            ..Default::default()
        };

        template.models.insert(
            "default".into(),
            Model {
                api_key_env: "OPENAI_API_KEY".to_string(),
                base_url: "https://api.openai.com".to_string(),
                name: "gpt-4o".into(),
                max_tokens: None,
                max_completion_tokens: None,
                temperature: None,
            },
        );

        template.models.insert(
            "fast".into(),
            Model {
                api_key_env: "OPENAI_API_KEY".into(),
                base_url: "https://api.openai.com".into(),
                name: "gpt-4o-mini".to_string(),
                max_tokens: None,
                max_completion_tokens: None,
                temperature: None,
            },
        );

        template.actions.insert(
            "today".into(),
            BlockAction::Attach(Attach {
                title: "Add Date".into(),
                key_template: "{{today}}".into(),
                document_template: "# {{today}}\n\n{{content}}\n".into(),
            }),
        );

        template.actions.insert(
            "rewrite".into(),
            BlockAction::Transform(
                Transform {
                    title: "Rewrite".into(),
                    model: "default".into(),
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
                }
            ),
        );

        template.actions.insert (
            "expand".to_string(),
            BlockAction::Transform(
                Transform {
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
                }
            ),
        );

        template.actions.insert (
            "keywords".into(),
            BlockAction::Transform(
                Transform {
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
                }
            ),
        );

        template.actions.insert(
            "emoji".into(),
            BlockAction::Transform(
                Transform {
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
                }
            )
        );

        template
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

pub fn load_config() -> Configuration {
    let current_dir = env::current_dir().expect("to get current dir");
    let mut config_path = current_dir.clone();
    config_path.push(IWE_MARKER);
    config_path.push(CONFIG_FILE_NAME);

    if config_path.exists() {
        debug!("reading config from path: {:?}", config_path);

        let configuration = migrate(
            &read_to_string(config_path)
                .ok()
                .expect("to read config file"),
        );

        toml::from_str::<Configuration>(&configuration).expect("to parse config file")
    } else {
        debug!("using default configuration");

        Configuration::default()
    }
}

fn migrate(config: &str) -> String {
    let updated = add_default_type_to_actions(config);

    if updated != config {
        debug!("configuration file migration applied");
        let current_dir = env::current_dir().expect("to get current dir");
        let mut config_path = current_dir.clone();
        config_path.push(IWE_MARKER);
        config_path.push(CONFIG_FILE_NAME);

        debug!("updating configuration file");
        std::fs::write(config_path, &updated).expect("to write updated config file");
    }
    updated
}

fn add_default_type_to_actions(input: &str) -> String {
    let mut doc = input.parse::<DocumentMut>().expect("valid TOML");

    if let Some(Item::Table(actions)) = doc.get_mut("actions") {
        for (_, action) in actions.iter_mut() {
            if let Item::Table(action_table) = action {
                action_table.entry("type").or_insert(value("transform"));
            }
        }
    }

    doc.to_string()
}
