use diwe::find::{DocumentFinder, FindOptions};
use diwe::tokens::count_tokens;
use liwe::query::{FieldPath, Sort as QuerySort, SortDir};
use minijinja::{context, Environment};

use crate::internal::claude::hook::store::MemoryStore;
use crate::projection_args::parse_projection_replace;
use crate::render::FindBlockRenderer;

const DEFAULT_INJECTION_MAX_TOKENS: usize = 2000;

const MEMORY_INDEX_TEMPLATE: &str =
    include_str!("../../../../templates/claude/memory_index.md.jinja");
const DEFAULT_FOOTER: &str = include_str!("../../../../templates/claude/footer.txt");

pub fn render_memory_index(store: &MemoryStore, footer: Option<&str>) -> Option<String> {
    let budget = store.knob_int("injection_max_tokens", DEFAULT_INJECTION_MAX_TOKENS);

    let base = "distilled_lines: { $exists: false }, $key: { $nin: [MEMORY, queries] }";

    let dated = find_entries(
        store,
        &format!("{{ {}, created: {{ $exists: true }} }}", base),
        Some(QuerySort {
            key: FieldPath::from_dotted("created"),
            dir: SortDir::Desc,
        }),
        "title=$title,key=$key,created=created",
        budget,
    );

    let entries = match dated {
        Some(entries) => entries,
        None => find_entries(
            store,
            &format!("{{ {} }}", base),
            None,
            "title=$title,key=$key",
            budget,
        )?,
    };

    Some(render_index_block(
        &entries,
        store.has_document("queries"),
        footer.unwrap_or(DEFAULT_FOOTER),
    ))
}

fn render_index_block(entries: &str, queries: bool, footer: &str) -> String {
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
        .render(context! { entries, queries, footer })
        .expect("Failed to render template")
}

fn clip_to_budget(entries: &str, budget: usize) -> String {
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
    kept.join("\n")
}

fn find_entries(
    store: &MemoryStore,
    expression: &str,
    sort: Option<QuerySort>,
    projection: &str,
    budget: usize,
) -> Option<String> {
    let graph = store.graph();
    let options = FindOptions {
        fuzzy: None,
        lexical: None,
        refs_to: None,
        refs_from: None,
        filter: store.filter_of(expression),
        limit: None,
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
        None
    } else {
        Some(clip_to_budget(trimmed, budget))
    }
}
