use chrono::Local;
use minijinja::{context, Environment};
use sanitize_filename::sanitize;

use crate::graph::GraphContext;
use crate::model::Key;

pub fn string_to_slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub struct KeyFormatContext<'a> {
    pub id: &'a str,
    pub title: String,
    pub parent_title: Option<String>,
    pub parent_key: Option<String>,
    pub source_key: &'a Key,
    pub source_title: Option<String>,
}

pub fn format_target_key<C: GraphContext>(
    key_template: &str,
    key_date_format: &str,
    fmt_ctx: &KeyFormatContext,
    graph_ctx: C,
) -> Key {
    let date = Local::now().date_naive();
    let formatted_date = date.format(key_date_format).to_string();

    let slug = string_to_slug(&fmt_ctx.title);
    let source_title = fmt_ctx.source_title.clone().unwrap_or_default();

    let relative_key = Environment::new()
        .template_from_str(key_template)
        .expect("correct template")
        .render(context! {
            today => formatted_date,
            id => fmt_ctx.id.to_string(),
            title => sanitize(&fmt_ctx.title),
            slug => slug,
            parent => context! {
                title => fmt_ctx.parent_title.clone().map(|t| sanitize(&t)).unwrap_or_default(),
                slug => fmt_ctx.parent_title.clone().map(|t| string_to_slug(&t)).unwrap_or_default(),
                key => fmt_ctx.parent_key.clone().unwrap_or_default(),
            },
            source => context! {
                key => fmt_ctx.source_key.to_string(),
                file => fmt_ctx.source_key.source(),
                title => source_title.clone(),
                slug => string_to_slug(&source_title),
                path => fmt_ctx.source_key.path().unwrap_or_default(),
            }
        })
        .expect("template to work");

    let base_key = Key::combine(&fmt_ctx.source_key.parent(), &relative_key);
    ensure_unique_key(base_key, graph_ctx)
}

fn ensure_unique_key<C: GraphContext>(base_key: Key, ctx: C) -> Key {
    let mut candidate_key = base_key.clone();
    let mut counter = 1;

    while ctx.get_node_id(&candidate_key).is_some() {
        let suffixed_name = format!("{}-{}", base_key, counter);
        candidate_key = Key::name(&suffixed_name);
        counter += 1;
    }

    candidate_key
}
