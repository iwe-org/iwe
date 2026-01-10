use indoc::indoc;
use log::debug;
use std::{collections::HashMap, env, fs::read_to_string};
use toml_edit::{value, DocumentMut, Item};

use serde::{Deserialize, Serialize};

use crate::graph::GraphContext;

use super::{node::NodeIter, NodeId};

const CONFIG_FILE_NAME: &str = "config.toml";
const IWE_MARKER: &str = ".iwe";

pub const DEFAULT_KEY_DATE_FORMAT: &str = "%Y-%m-%d";

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
    pub default_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct CompletionOptions {
    pub link_format: Option<LinkType>,
}

impl Default for LibraryOptions {
    fn default() -> Self {
        Self {
            path: String::new(),
            date_format: Some(DEFAULT_KEY_DATE_FORMAT.into()),
            prompt_key_prefix: None,
            default_template: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Configuration {
    pub version: Option<u32>,
    pub markdown: MarkdownOptions,
    pub library: LibraryOptions,
    #[serde(default)]
    pub completion: CompletionOptions,
    pub models: HashMap<String, Model>,
    pub actions: HashMap<String, ActionDefinition>,
    #[serde(default)]
    pub templates: HashMap<String, NoteTemplate>,
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
pub enum ActionDefinition {
    #[serde(rename = "transform")]
    Transform(Transform),
    #[serde(rename = "attach")]
    Attach(Attach),
    #[serde(rename = "sort")]
    Sort(Sort),
    #[serde(rename = "inline")]
    Inline(Inline),
    #[serde(rename = "extract")]
    Extract(Extract),
    #[serde(rename = "extract_all")]
    ExtractAll(ExtractAll),
    #[serde(rename = "link")]
    Link(Link),
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Sort {
    pub title: String,
    pub reverse: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Inline {
    pub title: String,
    pub inline_type: InlineType,
    pub keep_target: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum InlineType {
    #[serde(rename = "section")]
    Section,
    #[serde(rename = "quote")]
    Quote,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Extract {
    pub title: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ExtractAll {
    pub title: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Link {
    pub title: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum LinkType {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "wiki")]
    WikiLink,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NoteTemplate {
    pub key_template: String,
    pub document_template: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            version: Some(1),
            markdown: Default::default(),
            library: Default::default(),
            completion: Default::default(),
            models: Default::default(),
            actions: Default::default(),
            templates: Default::default(),
        }
    }
}

impl Configuration {
    pub fn template() -> Self {
        let mut template = Self {
            version: Some(2),
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
            ActionDefinition::Attach(Attach {
                title: "Add Date".into(),
                key_template: "{{today}}".into(),
                document_template: "# {{today}}\n\n{{content}}\n".into(),
            }),
        );

        template.actions.insert(
            "rewrite".into(),
            ActionDefinition::Transform(
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
            ActionDefinition::Transform(
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
            ActionDefinition::Transform(
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
            ActionDefinition::Transform(
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

        template.actions.insert(
            "sort".into(),
            ActionDefinition::Sort(Sort {
                title: "Sort A-Z".into(),
                reverse: Some(false),
            }),
        );

        template.actions.insert(
            "sort_desc".into(),
            ActionDefinition::Sort(Sort {
                title: "Sort Z-A".into(),
                reverse: Some(true),
            }),
        );

        template.actions.insert(
            "inline_section".into(),
            ActionDefinition::Inline(Inline {
                title: "Inline section".into(),
                inline_type: InlineType::Section,
                keep_target: Some(false),
            }),
        );

        template.actions.insert(
            "inline_quote".into(),
            ActionDefinition::Inline(Inline {
                title: "Inline quote".into(),
                inline_type: InlineType::Quote,
                keep_target: Some(false),
            }),
        );

        template.actions.insert(
            "extract".into(),
            ActionDefinition::Extract(Extract {
                title: "Extract".into(),
                link_type: Some(LinkType::Markdown),
                key_template: "{{id}}".into(),
            }),
        );

        template.actions.insert(
            "extract_all".into(),
            ActionDefinition::ExtractAll(ExtractAll {
                title: "Extract all subsections".into(),
                link_type: Some(LinkType::Markdown),
                key_template: "{{id}}".into(),
            }),
        );

        template.actions.insert(
            "link".into(),
            ActionDefinition::Link(Link {
                title: "Link".into(),
                link_type: Some(LinkType::Markdown),
                key_template: "{{id}}".into(),
            }),
        );

        template.templates.insert(
            "default".into(),
            NoteTemplate {
                key_template: "{{slug}}".into(),
                document_template: "# {{title}}\n\n{{content}}".into(),
            },
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

        let configuration = migrate(&read_to_string(config_path).expect("to read config file"));

        toml::from_str::<Configuration>(&configuration).expect("to parse config file")
    } else {
        debug!("using default configuration");
        Configuration::template()
    }
}

fn migrate(config: &str) -> String {
    let doc = config.parse::<DocumentMut>().expect("valid TOML");
    let current_version = doc
        .get("version")
        .and_then(|v| v.as_value())
        .and_then(|v| v.as_integer())
        .unwrap_or(0);

    let mut updated = config.to_string();
    let mut needs_update = false;

    // Migrate from version 0 to version 1
    if current_version < 1 {
        debug!("applying migrations from version 0 to 1");
        updated = add_default_type_to_actions(&updated);
        updated = add_default_code_actions(&updated);
        updated = set_config_version(&updated, 1);
        needs_update = true;
    }

    // Migrate from version 1 to version 2
    if current_version < 2 {
        debug!("applying migrations from version 1 to 2");
        updated = add_refs_extension_field(&updated);
        updated = add_link_action(&updated);
        updated = set_config_version(&updated, 2);
        needs_update = true;
    }

    if needs_update {
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

fn add_refs_extension_field(input: &str) -> String {
    let mut doc = input.parse::<DocumentMut>().expect("valid TOML");

    if doc.get("markdown").is_none() {
        doc["markdown"] = Item::Table(toml_edit::Table::new());
    }

    if let Some(Item::Table(markdown)) = doc.get_mut("markdown") {
        if markdown.get("refs_extension").is_none() {
            markdown.insert("refs_extension", value(""));
        }
    }

    doc.to_string()
}

fn add_link_action(input: &str) -> String {
    let mut doc = input.parse::<DocumentMut>().expect("valid TOML");

    if doc.get("actions").is_none() {
        doc["actions"] = Item::Table(toml_edit::Table::new());
    }

    if let Some(Item::Table(actions)) = doc.get_mut("actions") {
        // Check if link action already exists
        let has_link = actions.iter().any(|(_, action)| {
            if let Item::Table(action_table) = action {
                if let Some(Item::Value(action_type)) = action_table.get("type") {
                    if let Some(type_str) = action_type.as_str() {
                        return type_str == "link";
                    }
                }
            }
            false
        });

        if !has_link {
            let mut link_table = toml_edit::Table::new();
            link_table.insert("type", value("link"));
            link_table.insert("title", value("Link word"));
            link_table.insert("link_type", value("markdown"));
            link_table.insert("key_template", value("{{id}}"));
            actions.insert("link", Item::Table(link_table));
        }
    }

    doc.to_string()
}

fn set_config_version(input: &str, version: i64) -> String {
    let mut doc = input.parse::<DocumentMut>().expect("valid TOML");

    doc.insert("version", value(version));

    doc.to_string()
}

fn add_default_code_actions(input: &str) -> String {
    let mut doc = input.parse::<DocumentMut>().expect("valid TOML");

    if doc.get("actions").is_none() {
        doc["actions"] = Item::Table(toml_edit::Table::new());
    }

    if let Some(Item::Table(actions)) = doc.get_mut("actions") {
        let mut has_extract = false;
        let mut has_extract_all = false;
        let mut has_inline = false;

        for (_, action) in actions.iter() {
            if let Item::Table(action_table) = action {
                if let Some(Item::Value(action_type)) = action_table.get("type") {
                    if let Some(type_str) = action_type.as_str() {
                        match type_str {
                            "extract" => has_extract = true,
                            "extract_all" => has_extract_all = true,
                            "inline" => has_inline = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        if !has_extract {
            let mut extract_table = toml_edit::Table::new();
            extract_table.insert("type", value("extract"));
            extract_table.insert("title", value("Extract"));
            extract_table.insert("link_type", value("markdown"));
            extract_table.insert("key_template", value("{{id}}"));
            actions.insert("extract", Item::Table(extract_table));
        }

        if !has_extract_all {
            let mut extract_all_table = toml_edit::Table::new();
            extract_all_table.insert("type", value("extract_all"));
            extract_all_table.insert("title", value("Extract all subsections"));
            extract_all_table.insert("link_type", value("markdown"));
            extract_all_table.insert("key_template", value("{{id}}"));
            actions.insert("extract_all", Item::Table(extract_all_table));
        }

        if !has_inline {
            let mut inline_section_table = toml_edit::Table::new();
            inline_section_table.insert("type", value("inline"));
            inline_section_table.insert("title", value("Inline section"));
            inline_section_table.insert("inline_type", value("section"));
            inline_section_table.insert("keep_target", value(false));
            actions.insert("inline_section", Item::Table(inline_section_table));

            let mut inline_quote_table = toml_edit::Table::new();
            inline_quote_table.insert("type", value("inline"));
            inline_quote_table.insert("title", value("Inline quote"));
            inline_quote_table.insert("inline_type", value("quote"));
            inline_quote_table.insert("keep_target", value(false));
            actions.insert("inline_quote", Item::Table(inline_quote_table));
        }
    }

    doc.to_string()
}
