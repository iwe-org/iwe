use std::collections::HashSet;

use chrono::Local;
use liwe::model::Key;
use serde_yaml::Value as YamlValue;

use crate::internal::claude::hook::store::{is_safe_id, ChunkRecord, MemoryStore, SESSIONS_PREFIX};
use crate::schema::render_schema;

const KNOWLEDGE_FILTER: &str =
    "{ distilled_lines: { $exists: false }, $key: { $nin: [MEMORY, queries] } }";

const BRIEF_SAMPLE: usize = 10;

pub const DEFAULT_FRONTIER_CHARS: usize = 24000;

pub fn next_capture_chunk(store: &MemoryStore) -> Result<Option<String>, String> {
    let cutoff = store.claim_cutoff();
    let Some(chunk) = store
        .chunks()
        .into_iter()
        .find(|chunk| is_servable(store, chunk, &cutoff))
    else {
        return Ok(None);
    };

    claim(store, &chunk)?;
    Ok(Some(render(store, &chunk)))
}

pub fn frontier_capture_chunks(
    store: &MemoryStore,
    limit: usize,
    max_chars: usize,
) -> Result<Option<String>, String> {
    let cutoff = store.claim_cutoff();
    let chunks = frontier(store, &cutoff);
    if chunks.is_empty() {
        return Ok(None);
    }

    let mut entries = Vec::new();
    let mut used = 0;
    for chunk in chunks.iter().take(limit) {
        let text = render(store, chunk);
        let length = text.chars().count();
        if !entries.is_empty() && used + length > max_chars {
            break;
        }
        used += length;
        entries.push((chunk, text));
    }

    for (chunk, _) in &entries {
        claim(store, chunk)?;
    }

    let mut report = format!(
        "frontier: {} of {} servable session(s)\n",
        entries.len(),
        chunks.len()
    );
    for (index, (_, text)) in entries.iter().enumerate() {
        report.push_str(&format!(
            "\n=== chunk {} of {} ===\n",
            index + 1,
            entries.len()
        ));
        report.push_str(text);
    }
    Ok(Some(report))
}

fn frontier(store: &MemoryStore, cutoff: &str) -> Vec<ChunkRecord> {
    store
        .chunks()
        .into_iter()
        .filter(|chunk| is_servable(store, chunk, cutoff))
        .collect()
}

fn is_servable(store: &MemoryStore, chunk: &ChunkRecord, cutoff: &str) -> bool {
    chunk.is_pending()
        && !chunk.is_skipped(cutoff)
        && chunk.from == store.session_watermark(&chunk.session)
}

fn claim(store: &MemoryStore, chunk: &ChunkRecord) -> Result<(), String> {
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    if store.stamp_chunk(&chunk.path, &[("claimed", YamlValue::String(now))]) {
        Ok(())
    } else {
        Err(format!(
            "cannot claim {}: the chunk is not writable, so the queue cannot serve it",
            chunk.path.display()
        ))
    }
}

fn render(store: &MemoryStore, chunk: &ChunkRecord) -> String {
    let body = store.chunk_body(&chunk.path).unwrap_or_default();
    let occurred = chunk
        .occurred
        .as_ref()
        .map(|occurred| format!("occurred: {}\n", occurred))
        .unwrap_or_default();
    format!(
        "session: {}\ncovers_from: {}\ncovers_lines: {}\nmax_items: {}\ncreated: {}\n{}\n{}\n",
        chunk.session,
        chunk.from,
        chunk.covers,
        chunk.max_items,
        chunk.created,
        occurred,
        body.trim()
    )
}

pub fn capture_brief(store: &MemoryStore) -> String {
    let mut brief = String::from("=== policy: MEMORY ===\n");
    brief.push_str(store.body_of("MEMORY").unwrap_or_default().trim());
    brief.push('\n');

    let keys = knowledge_keys(store);
    brief.push_str(&format!(
        "\n=== schema: {} document(s), machinery excluded ===\n",
        keys.len()
    ));
    let fields = liwe::schema::infer_schema(store.graph(), &keys);
    if fields.is_empty() {
        brief.push_str("no frontmatter yet: the policy is your only guide\n");
    } else {
        brief.push_str(&render_schema(&fields));
    }

    let recent = recent_keys(store, &keys);
    brief.push_str(&format!(
        "\n=== recent: {} of {} document(s) ===\n",
        recent.len(),
        keys.len()
    ));
    for (key, title) in &recent {
        brief.push_str(&format!("{} — {}\n", key, title));
    }

    brief
}

fn knowledge_keys(store: &MemoryStore) -> Vec<Key> {
    match store.filter_of(KNOWLEDGE_FILTER) {
        Some(filter) => liwe::query::evaluate(&filter, store.graph()),
        None => Vec::new(),
    }
}

fn recent_keys(store: &MemoryStore, keys: &[Key]) -> Vec<(String, String)> {
    let graph = store.graph();
    let mut dated: Vec<(String, String, String)> = keys
        .iter()
        .map(|key| {
            let created = graph
                .frontmatter(key)
                .and_then(|front| front.get(YamlValue::String("created".to_string())))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let title = graph.get_key_title(key).unwrap_or_else(|| key.to_string());
            (created, key.to_string(), title)
        })
        .collect();

    dated.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    dated
        .into_iter()
        .take(BRIEF_SAMPLE)
        .map(|(_, key, title)| (key, title))
        .collect()
}

pub fn complete_capture_chunk(
    store: &MemoryStore,
    session: &str,
    lines: usize,
    wrote: &[String],
    title: Option<&str>,
    summary: Option<&str>,
) -> Result<String, String> {
    if !is_safe_id(session) {
        return Err(format!("'{}' is not a session id", session));
    }

    let session_key = store.session_key(session);
    let chunks: Vec<ChunkRecord> = store
        .chunks()
        .into_iter()
        .filter(|chunk| chunk.session == session)
        .collect();

    if chunks.is_empty() && !store.document_exists(&session_key) {
        return Err(format!(
            "no capture chunk for {} and no session document {}",
            session, session_key
        ));
    }

    let watermark = store.session_watermark(session);
    if watermark >= lines {
        let stamped = stamp_captured(store, &chunks, watermark);
        return Ok(format!(
            "already complete: {} is at line {}; stamped {} leftover chunk(s)",
            session, watermark, stamped
        ));
    }

    let target = match chunks
        .iter()
        .find(|chunk| chunk.is_pending() && chunk.from == watermark)
    {
        Some(target) => target,
        None => {
            return Err(format!(
                "no pending capture chunk starting at line {} for {}; \
                 the next sweep imports it",
                watermark, session
            ))
        }
    };
    if target.covers != lines {
        return Err(format!(
            "--lines {}: the chunk at line {} covers through line {}",
            lines, target.from, target.covers
        ));
    }

    let links = resolve_links(store, &session_key, wrote)?;

    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    if !store.ensure_session_document(session, &now) {
        return Err(format!(
            "could not create the session document {}",
            session_key
        ));
    }
    store.retitle_session(session, title, summary);

    let note = capture_note(store, &session_key, &target.created, target.covers, &links);
    let completed = store.set_fields_and_append(
        &session_key,
        &[
            ("distilled_lines", YamlValue::Number(target.covers.into())),
            ("distilled_at", YamlValue::String(target.created.clone())),
        ],
        &note,
    );
    if !completed {
        return Err(format!("could not update {}", session_key));
    }

    if !store.stamp_chunk(&target.path, &[("captured_at", YamlValue::String(now))]) {
        return Err(format!(
            "watermark advanced to line {}, but {} could not be stamped captured; \
             run the same command again to finish",
            target.covers,
            target.path.display()
        ));
    }

    Ok(format!(
        "completed {}: watermark at line {}, {} link(s)",
        session,
        target.covers,
        links.len()
    ))
}

pub fn skip_capture_chunk(
    store: &MemoryStore,
    session: &str,
    lines: usize,
) -> Result<String, String> {
    if !is_safe_id(session) {
        return Err(format!("'{}' is not a session id", session));
    }

    let watermark = store.session_watermark(session);
    let chunks: Vec<ChunkRecord> = store
        .chunks()
        .into_iter()
        .filter(|chunk| chunk.session == session)
        .collect();
    let target = chunks
        .iter()
        .find(|chunk| chunk.is_pending() && chunk.from == watermark)
        .ok_or_else(|| {
            format!(
                "no pending capture chunk starting at line {} for {}",
                watermark, session
            )
        })?;
    if target.covers != lines {
        return Err(format!(
            "--lines {}: the chunk at line {} covers through line {}",
            lines, target.from, target.covers
        ));
    }

    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    if !store.stamp_chunk(&target.path, &[("skipped", YamlValue::String(now))]) {
        return Err(format!("could not stamp {}", target.path.display()));
    }

    Ok(format!(
        "skipped {} at line {}: the queue serves other sessions; \
         this chunk returns to a fresh agent after the in-flight TTL",
        session, target.covers
    ))
}

pub fn reset_capture_session(
    store: &MemoryStore,
    session: &str,
    to: usize,
) -> Result<String, String> {
    if !is_safe_id(session) {
        return Err(format!("'{}' is not a session id", session));
    }

    let session_key = store.session_key(session);
    if !store.document_exists(&session_key) {
        return Err(format!("no session document {}", session_key));
    }

    let mut removed = 0;
    for chunk in store.chunks() {
        if chunk.session == session && chunk.covers > to && store.remove_chunk(&chunk.path) {
            removed += 1;
        }
    }

    if !store.set_fields(
        &session_key,
        &[("distilled_lines", YamlValue::Number(to.into()))],
    ) {
        return Err(format!("could not update {}", session_key));
    }

    Ok(format!(
        "reset {} to line {}: removed {} chunk(s); the next sweep re-imports the span",
        session, to, removed
    ))
}

fn stamp_captured(store: &MemoryStore, chunks: &[ChunkRecord], watermark: usize) -> usize {
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let mut stamped = 0;
    for chunk in chunks {
        if !chunk.is_pending() || chunk.covers > watermark {
            continue;
        }
        if store.stamp_chunk(
            &chunk.path,
            &[("captured_at", YamlValue::String(now.clone()))],
        ) {
            stamped += 1;
        }
    }
    stamped
}

fn resolve_links(
    store: &MemoryStore,
    session_key: &str,
    wrote: &[String],
) -> Result<Vec<(String, String)>, String> {
    let graph = store.graph();
    let mut seen = HashSet::new();
    let mut links = Vec::new();

    for raw in wrote {
        let trimmed = raw.trim().trim_start_matches('/');
        if trimmed.is_empty() {
            return Err("--wrote: empty key".to_string());
        }
        let key = Key::name(trimmed).to_string();
        if !seen.insert(key.clone()) {
            continue;
        }
        let machinery = key == "MEMORY"
            || key == session_key
            || key.starts_with(&format!("{}/", SESSIONS_PREFIX))
            || graph
                .frontmatter(&Key::name(&key))
                .map(|front| front.contains_key(YamlValue::String("distilled_lines".to_string())))
                .unwrap_or(false);
        if machinery {
            return Err(format!(
                "--wrote {}: the machinery's own documents are not capture output",
                key
            ));
        }
        if !store.has_document(&key) {
            return Err(format!("--wrote {}: no such document in this store", key));
        }
        let title = graph
            .get_key_title(&Key::name(&key))
            .unwrap_or_else(|| key.clone());
        links.push((key, link_text(&title)));
    }

    Ok(links)
}

fn capture_note(
    store: &MemoryStore,
    session_key: &str,
    created: &str,
    covers: usize,
    links: &[(String, String)],
) -> String {
    let parent = Key::name(session_key).parent();
    let options = store.graph().format_options().markdown_options();

    let mut note = format!(
        "{} — captured {} item(s) through line {}",
        created,
        links.len(),
        covers
    );
    for (key, text) in links {
        let mut url = Key::name(key).link_url(&parent, options.refs_path);
        url.push_str(&options.refs_extension);
        note.push_str(&format!("\n\n[{}]({})", text, url));
    }
    note
}

fn link_text(title: &str) -> String {
    title
        .replace(['[', ']'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
