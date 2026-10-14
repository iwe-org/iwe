use diwe::config::{ActionDefinition, Configuration, LinkType};
use liwe::model::config::{DjotOptions, Format, MarkdownOptions, RefsPath, RefsText, WikiLinkPath};
use toml_edit::{DocumentMut, Item, Table};

use liwe::init::settings::{
    formatting_options, optional_text, Confidence, SettingId, Settings, ALL_SETTINGS,
};

pub fn render(settings: &Settings) -> String {
    let config = to_configuration(settings);
    let sorted = toml::Value::try_from(&config).expect("configuration converts to a TOML value");
    let mut document: DocumentMut = toml::to_string(&sorted)
        .expect("configuration serializes to TOML")
        .parse()
        .expect("serialized configuration parses back");

    order_tables(&mut document);

    for id in ALL_SETTINGS {
        if id == SettingId::Agents {
            continue;
        }
        let confidence = settings.confidence(id);
        if confidence == Confidence::Assumed {
            continue;
        }
        let note = settings.note(id);
        if note.is_empty() {
            continue;
        }
        let comment = format!("# {}: {}\n", confidence, note);
        annotate(&mut document, id.key(), &comment);
    }

    document.to_string()
}

const TABLE_ORDER: [&str; 9] = [
    "workspace",
    "markdown",
    "djot",
    "completion",
    "search",
    "templates",
    "commands",
    "actions",
    "schemas",
];

fn order_tables(document: &mut DocumentMut) {
    let mut position: isize = 0;
    for name in TABLE_ORDER {
        if let Some(item) = document.get_mut(name) {
            assign_positions(item, &mut position);
        }
    }
}

fn assign_positions(item: &mut Item, position: &mut isize) {
    let table = match item.as_table_mut() {
        Some(table) => table,
        None => return,
    };

    table.set_position(Some(*position));
    *position += 1;

    let children: Vec<String> = table
        .iter()
        .filter(|(_, child)| child.is_table())
        .map(|(key, _)| key.to_string())
        .collect();

    for key in children {
        if let Some(child) = table.get_mut(&key) {
            assign_positions(child, position);
        }
    }
}

fn annotate(document: &mut DocumentMut, key: &str, comment: &str) {
    let mut segments: Vec<&str> = key.split('.').collect();
    let leaf = match segments.pop() {
        Some(leaf) => leaf,
        None => return,
    };

    let mut table: &mut Table = document.as_table_mut();
    for segment in segments {
        table = match table.get_mut(segment).and_then(Item::as_table_mut) {
            Some(nested) => nested,
            None => return,
        };
    }

    if let Some(mut leaf_key) = table.key_mut(leaf) {
        let decor = leaf_key.leaf_decor_mut();
        let existing = decor
            .prefix()
            .and_then(|prefix| prefix.as_str())
            .unwrap_or("")
            .to_string();
        decor.set_prefix(format!("{}{}", existing, comment));
    }
}

pub fn parse(rendered: &str) -> Configuration {
    toml::from_str(rendered).expect("rendered configuration parses")
}

pub fn to_configuration(settings: &Settings) -> Configuration {
    let mut config = Configuration::template();

    config.workspace.path = optional_text(settings, SettingId::WorkspacePath).unwrap_or_default();
    config.workspace.date_format = optional_text(settings, SettingId::KeyDateFormat);
    config.workspace.frontmatter_document_title =
        optional_text(settings, SettingId::FrontmatterTitle);

    config.format = match settings.get(SettingId::Format).as_text() {
        Some("djot") => Format::Djot,
        _ => Format::Markdown,
    };

    let link_format = match settings.get(SettingId::LinkFormat).as_text() {
        Some("wiki") => LinkType::WikiLink,
        _ => LinkType::Markdown,
    };
    config.completion.link_format = Some(link_format.clone());

    for action in config.actions.values_mut() {
        match action {
            ActionDefinition::Extract(extract) => extract.link_type = Some(link_format.clone()),
            ActionDefinition::ExtractAll(extract_all) => {
                extract_all.link_type = Some(link_format.clone())
            }
            ActionDefinition::Link(link) => link.link_type = Some(link_format.clone()),
            _ => {}
        }
    }

    let formatting = formatting_options(settings);

    config.markdown = MarkdownOptions {
        refs_extension: optional_text(settings, SettingId::RefsExtension).unwrap_or_default(),
        refs_path: match settings.get(SettingId::RefsPath).as_text() {
            Some("absolute") => RefsPath::Absolute,
            _ => RefsPath::Relative,
        },
        refs_text: match settings.get(SettingId::RefsText).as_text() {
            Some("normalize") => RefsText::Normalize,
            _ => RefsText::Preserve,
        },
        date_format: optional_text(settings, SettingId::DisplayDateFormat),
        time_format: None,
        locale: None,
        wiki_link_path: match settings.get(SettingId::WikiLinkPath).as_text() {
            Some("full") => WikiLinkPath::Full,
            Some("short") => WikiLinkPath::Short,
            _ => WikiLinkPath::Preserve,
        },
        formatting: formatting.clone(),
    };

    config.djot = DjotOptions {
        refs_extension: config.markdown.refs_extension.clone(),
        refs_path: config.markdown.refs_path,
        refs_text: config.markdown.refs_text,
        date_format: config.markdown.date_format.clone(),
        time_format: None,
        locale: None,
        formatting,
    };

    config.search.language =
        optional_text(settings, SettingId::SearchLanguage).unwrap_or_else(|| "english".to_string());

    if let Some(template) = config.templates.get_mut("default") {
        if let Some(key_template) = optional_text(settings, SettingId::KeyTemplate) {
            template.key_template = key_template;
        }
    }

    config
}
