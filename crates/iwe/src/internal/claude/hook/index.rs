use std::collections::HashSet;

use diwe::find::{DocumentFinder, FindOptions};
use diwe::tokens::count_tokens;
use liwe::model::Key;
use liwe::query::{FieldPath, Filter, KeyOp, Sort as QuerySort};
use minijinja::{context, Environment};
use serde::Serialize;

use crate::internal::claude::hook::store::{
    machinery_exclusion, MemoryStore, Slice, SESSION_START_SECTION,
};
use crate::internal::claude::record::mark_reminded;
use crate::internal::claude::session::{backlog_of, reminder_due, Backlog, SessionOptions};
use crate::projection_args::parse_projection_replace;
use crate::render::FindBlockRenderer;

const DEFAULT_INJECTION_MAX_TOKENS: usize = 2000;

const MEMORY_INDEX_TEMPLATE: &str =
    include_str!("../../../../templates/claude/memory_index.md.jinja");

#[derive(Serialize)]
pub struct Group {
    pub heading: String,
    pub entries: String,
}

struct Listed {
    text: String,
    keys: Vec<String>,
    tokens: usize,
}

pub fn injection_groups(store: &MemoryStore) -> Vec<Group> {
    let budget = store.knob_int("injection_max_tokens", DEFAULT_INJECTION_MAX_TOKENS);
    let slices = store
        .injection_slices()
        .unwrap_or_else(|_| Slice::default_slices());

    let mut remaining = budget;
    let mut seen: HashSet<String> = HashSet::new();
    let mut groups: Vec<Group> = Vec::new();
    for slice in &slices {
        if remaining == 0 {
            break;
        }
        let Some(listed) = list_slice(store, slice, &seen, remaining) else {
            continue;
        };
        remaining = remaining.saturating_sub(listed.tokens + count_tokens(&slice.heading));
        seen.extend(listed.keys);
        groups.push(Group {
            heading: slice.heading.clone(),
            entries: listed.text,
        });
    }
    groups
}

pub fn render_memory_index(store: &MemoryStore, current: Option<String>) -> Option<String> {
    let groups = injection_groups(store);
    if groups.is_empty() {
        return None;
    }

    let backlog = backlog_of(&SessionOptions {
        transcripts: None,
        current,
    });
    let reminder = backlog.is_some() && reminder_due(store);
    if reminder {
        mark_reminded(&chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    }

    Some(render_index_block(
        &groups,
        store.has_document("queries"),
        backlog.as_ref().map(backlog_line),
        reminder,
        store.policy_section(SESSION_START_SECTION),
    ))
}

fn list_slice(
    store: &MemoryStore,
    slice: &Slice,
    seen: &HashSet<String>,
    budget: usize,
) -> Option<Listed> {
    let unseen = (!seen.is_empty()).then(|| {
        let mut keys: Vec<&String> = seen.iter().collect();
        keys.sort();
        Filter::Key(KeyOp::Nin(
            keys.into_iter().map(|key| Key::name(key)).collect(),
        ))
    });
    let sort = slice.sort.as_ref().map(|(name, dir)| QuerySort {
        key: FieldPath::from_dotted(name),
        dir: *dir,
    });
    let projection = match &slice.sort {
        Some((name, _)) => format!("title=$title,key=$key,{0}={0}", name),
        None => "title=$title,key=$key".to_string(),
    };

    find_entries(
        store,
        conjoin([
            Some(machinery_exclusion()),
            slice.filter.clone().map(|(filter, _)| filter),
            unseen,
        ]),
        sort,
        &projection,
        slice.limit,
        budget,
    )
}

fn conjoin<const N: usize>(parts: [Option<Filter>; N]) -> Filter {
    let mut parts: Vec<Filter> = parts.into_iter().flatten().collect();
    if parts.len() == 1 {
        parts.pop().expect("one part")
    } else {
        Filter::And(parts)
    }
}

fn backlog_line(backlog: &Backlog) -> String {
    let plural = if backlog.sessions == 1 { "" } else { "s" };
    let since = match &backlog.since {
        Some(since) => format!(" since {}", since),
        None => String::new(),
    };
    format!(
        "{} session{} ({} user turn{}){} are not distilled — `/iwe:distill` reads them with you.",
        backlog.sessions,
        plural,
        backlog.turns,
        if backlog.turns == 1 { "" } else { "s" },
        since
    )
}

fn render_index_block(
    groups: &[Group],
    queries: bool,
    backlog: Option<String>,
    reminder: bool,
    policy: Option<String>,
) -> String {
    let mut env = Environment::new();
    env.set_keep_trailing_newline(true);
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    env.add_template("memory-index", MEMORY_INDEX_TEMPLATE)
        .expect("Failed to add template");

    let template = env
        .get_template("memory-index")
        .expect("Failed to get template");
    template
        .render(context! { groups, queries, backlog, reminder, policy })
        .expect("Failed to render template")
}

fn clip_to_budget(entries: &str, budget: usize) -> (String, usize, usize) {
    let mut used = 0;
    let mut kept: Vec<&str> = Vec::new();
    for line in entries.lines() {
        let cost = count_tokens(line);
        if !kept.is_empty() && used + cost > budget {
            break;
        }
        used += cost;
        kept.push(line);
    }
    (kept.join("\n"), kept.len(), used)
}

fn find_entries(
    store: &MemoryStore,
    filter: Filter,
    sort: Option<QuerySort>,
    projection: &str,
    limit: Option<usize>,
    budget: usize,
) -> Option<Listed> {
    let graph = store.graph();
    let options = FindOptions {
        fuzzy: None,
        lexical: None,
        refs_to: None,
        refs_from: None,
        filter: Some(filter),
        limit,
        sort,
        project: parse_projection_replace(projection).ok(),
        max_tokens: Some(budget),
        max_document_tokens: None,
    };

    let output = DocumentFinder::new(graph).find(&options);
    if output.keys.is_empty() {
        return None;
    }

    let markdown_options = graph.format_options().markdown_options();
    let renderer =
        FindBlockRenderer::new(&markdown_options, graph, None, &output.truncation.clipped);
    let rendered = renderer.render(&output.keys, &output.results, &[], false, &[]);

    let trimmed = rendered.trim_end_matches('\n');
    if trimmed.is_empty() {
        return None;
    }
    let (text, lines, tokens) = clip_to_budget(trimmed, budget);
    let keys = output
        .keys
        .iter()
        .take(lines)
        .map(|key| key.to_string())
        .collect();
    Some(Listed { text, keys, tokens })
}
