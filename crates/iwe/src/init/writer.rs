use diwe::config::Configuration;
use toml_edit::{DocumentMut, Item, Table};

use crate::init::settings::{to_configuration, Confidence, SettingId, Settings, ALL_SETTINGS};

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
    "library",
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
