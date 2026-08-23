use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use chrono::Local;
use minijinja::{context, Environment};
use serde::Serialize;
use serde_yaml::Value as YamlValue;

use serde_json::Value as JsonValue;

use crate::internal::claude::digest::{
    count_user_turns, digest_claude_chunks, value_occurred, Chunk,
};
use crate::internal::claude::hook::store::{
    ensure_state_directory, field_int, field_text, inflight_cutoff, is_safe_id, non_empty_var,
    project_slug, ChunkRecord, MemoryStore, DEFAULT_INFLIGHT_TTL_MINUTES,
};

const DEFAULT_SWEEP_THRESHOLD_LINES: usize = 120;
const DEFAULT_CHUNK_CHARS: usize = 10000;
const DEFAULT_MAX_CHUNKS_PER_SWEEP: usize = 30;
const DEFAULT_MAX_ITEMS_PER_CHUNK: usize = 3;

const CAPTURE_REASON_TEMPLATE: &str =
    include_str!("../../../../templates/claude/capture_reason.txt.jinja");
const DISTILL_AGENT: &str = "distill";
const PLUGIN_ROOT_VARIABLE: &str = "CLAUDE_PLUGIN_ROOT";

#[derive(Serialize)]
struct BlockDecision<'a> {
    decision: &'a str,
    reason: &'a str,
}

pub enum SweepMode {
    Claim,
    Survey,
    Adopt,
}

pub struct SweepOptions {
    pub mode: SweepMode,
    pub max_chunks: Option<usize>,
    pub transcripts: Option<PathBuf>,
    pub transcript_path: Option<String>,
    pub capture_reason: Option<String>,
}

struct SweepKnobs {
    threshold: usize,
    max_chunks: usize,
    chunk_chars: usize,
    max_items: usize,
    ttl: usize,
}

struct Transcript {
    session: String,
    path: PathBuf,
    lines: usize,
}

pub fn run_memory_sweep(
    store: &mut MemoryStore,
    options: &SweepOptions,
) -> Result<Option<String>, String> {
    let Some(directory) = transcripts_directory(
        options.transcripts.as_deref(),
        options.transcript_path.as_deref(),
    ) else {
        return Ok(None);
    };

    let knobs = SweepKnobs {
        threshold: store.knob_int("sweep_threshold_lines", DEFAULT_SWEEP_THRESHOLD_LINES),
        max_chunks: options.max_chunks.unwrap_or_else(|| {
            store.knob_int("max_chunks_per_sweep", DEFAULT_MAX_CHUNKS_PER_SWEEP)
        }),
        chunk_chars: store.knob_int("chunk_chars", DEFAULT_CHUNK_CHARS),
        max_items: store.knob_int("max_items_per_chunk", DEFAULT_MAX_ITEMS_PER_CHUNK),
        ttl: store.knob_int("inflight_ttl_minutes", DEFAULT_INFLIGHT_TTL_MINUTES),
    };

    let current = current_session_id(options.transcript_path.as_deref());

    if !matches!(options.mode, SweepMode::Survey) {
        ensure_state_directory(store.state_directory())?;
        store.relocate_store_chunks();
    }

    let transcripts = ordered_transcripts(&directory, current.as_deref());

    Ok(match options.mode {
        SweepMode::Survey => Some(render_survey(
            store,
            &directory,
            &knobs,
            &transcripts,
            current.as_deref(),
        )),
        SweepMode::Adopt => Some(adopt_transcripts(store, &transcripts)),
        SweepMode::Claim => import_and_spawn(store, &transcripts, &knobs, options),
    })
}

fn current_session_id(transcript_path: Option<&str>) -> Option<String> {
    let path = transcript_path?;
    if !path.ends_with(".jsonl") {
        return None;
    }
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
}

fn transcripts_directory(explicit: Option<&Path>, transcript: Option<&str>) -> Option<PathBuf> {
    if let Some(directory) = explicit {
        return directory.is_dir().then(|| directory.to_path_buf());
    }

    if let Some(directory) = non_empty_var("IWE_MEMORY_TRANSCRIPTS") {
        let directory = PathBuf::from(directory);
        return directory.is_dir().then_some(directory);
    }

    if let Some(transcript) = transcript {
        if let Some(parent) = Path::new(transcript).parent() {
            if parent.is_dir() {
                return Some(parent.to_path_buf());
            }
        }
    }

    let project = std::env::current_dir().ok()?;
    let config = match non_empty_var("CLAUDE_CONFIG_DIR") {
        Some(directory) => PathBuf::from(directory),
        None => PathBuf::from(non_empty_var("HOME")?).join(".claude"),
    };
    let directory = config
        .join("projects")
        .join(project_slug(&project.to_string_lossy()));
    directory.is_dir().then_some(directory)
}

fn transcript_time_range(path: &Path) -> (Option<String>, Option<String>) {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return (None, None),
    };

    let mut first = None;
    let mut last = None;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match std::io::BufRead::read_until(&mut reader, b'\n', &mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let line = String::from_utf8_lossy(&buffer);
        let stamp = serde_json::from_str::<JsonValue>(line.trim_end())
            .ok()
            .as_ref()
            .and_then(value_occurred);
        if let Some(stamp) = stamp {
            if first.is_none() {
                first = Some(stamp.clone());
            }
            last = Some(stamp);
        }
    }
    (first, last)
}

fn transcript_lines(path: &Path) -> usize {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return 0,
    };

    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 64 * 1024];
    let mut lines = 0;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => lines += buffer[..read].iter().filter(|byte| **byte == b'\n').count(),
            Err(_) => return 0,
        }
    }
    lines
}

fn ordered_transcripts(directory: &Path, current: Option<&str>) -> Vec<Transcript> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut found: Vec<(std::time::SystemTime, String, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jsonl") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        found.push((modified, name, path));
    }

    found.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));

    let mut transcripts: Vec<Transcript> = found
        .into_iter()
        .filter_map(|(_, name, path)| {
            let session = name.trim_end_matches(".jsonl").to_string();
            is_safe_id(&session).then(|| Transcript {
                lines: transcript_lines(&path),
                session,
                path,
            })
        })
        .collect();

    if let Some(current) = current {
        if let Some(position) = transcripts
            .iter()
            .position(|transcript| transcript.session == current)
        {
            let found = transcripts.remove(position);
            transcripts.insert(0, found);
        }
    }

    transcripts
}

fn stamp_now() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

fn render_survey(
    store: &MemoryStore,
    directory: &Path,
    knobs: &SweepKnobs,
    transcripts: &[Transcript],
    current: Option<&str>,
) -> String {
    let cutoff = inflight_cutoff(knobs.ttl);
    let pending: Vec<ChunkRecord> = store
        .chunks()
        .into_iter()
        .filter(ChunkRecord::is_pending)
        .collect();

    let mut report = String::new();
    report.push_str(&format!("transcripts: {}\n", directory.display()));
    report.push_str(&format!("chunks: {}\n", store.state_directory().display()));
    report.push_str(&format!("threshold: {} lines\n\n", knobs.threshold));
    report.push_str(&format!(
        "{:<38} {:>8} {:>10} {:>9} {:>7} {:>7}  {}\n",
        "session", "lines", "captured", "pending", "chunks", "signal", ""
    ));

    let mut total = 0;
    let mut pending_total = 0;
    let mut pending_lines = 0;
    let mut pending_signal = 0;

    for transcript in transcripts {
        total += 1;
        let watermark = store.session_watermark(&transcript.session);
        let behind = transcript.lines.saturating_sub(watermark);
        let chunks: Vec<&ChunkRecord> = pending
            .iter()
            .filter(|chunk| chunk.session == transcript.session)
            .collect();

        let signal = if behind >= knobs.threshold {
            count_user_turns(&transcript.path, watermark).unwrap_or(0)
        } else {
            0
        };

        let mut state = Vec::new();
        if behind >= knobs.threshold {
            state.push("pending".to_string());
            pending_total += 1;
            pending_lines += behind;
            pending_signal += signal;
        } else if behind > 0 {
            state.push("under the threshold".to_string());
        }
        if !chunks.is_empty() {
            state.push(
                if chunks.iter().all(|chunk| chunk.is_claimed(&cutoff)) {
                    "claimed"
                } else {
                    "unclaimed"
                }
                .to_string(),
            );
            if chunks.iter().any(|chunk| chunk.is_skipped(&cutoff)) {
                state.push("skipped".to_string());
            }
        }
        if Some(transcript.session.as_str()) == current {
            state.push("this session".to_string());
        }

        report.push_str(&format!(
            "{:<38} {:>8} {:>10} {:>9} {:>7} {:>7}  {}\n",
            transcript.session,
            transcript.lines,
            watermark,
            behind,
            chunks.len(),
            signal,
            state.join(", ")
        ));
    }

    let claimed = pending
        .iter()
        .filter(|chunk| chunk.is_claimed(&cutoff))
        .count();
    let skipped = pending
        .iter()
        .filter(|chunk| chunk.is_skipped(&cutoff))
        .count();
    report.push_str(&format!(
        "\n{} transcript(s), {} pending over {} uncaptured line(s) \
         carrying {} user turn(s), \
         {} pending chunk(s) of which {} claimed, {} skipped\n",
        total,
        pending_total,
        pending_lines,
        pending_signal,
        pending.len(),
        claimed,
        skipped
    ));

    report
}

fn adopt_transcripts(store: &MemoryStore, transcripts: &[Transcript]) -> String {
    let now = stamp_now();
    let mut report = String::new();
    let mut adopted = 0;

    for transcript in transcripts {
        if transcript.lines == 0 {
            continue;
        }
        if !store.ensure_session_document(&transcript.session, &now) {
            continue;
        }
        let mut fields = vec![
            (
                "distilled_lines",
                YamlValue::Number(transcript.lines.into()),
            ),
            (
                "transcript",
                YamlValue::String(transcript.path.to_string_lossy().to_string()),
            ),
        ];
        fields.extend(session_time_fields(&transcript.path));
        let updated = store.set_fields(&store.session_key(&transcript.session), &fields);
        if !updated {
            continue;
        }
        report.push_str(&format!(
            "adopted {} at line {}\n",
            transcript.session, transcript.lines
        ));
        adopted += 1;
    }

    reconcile_watermarks(store);

    report.push_str(&format!(
        "\n{} session(s) adopted without capture; memory starts from here\n",
        adopted
    ));

    report
}

fn import_and_spawn(
    store: &MemoryStore,
    transcripts: &[Transcript],
    knobs: &SweepKnobs,
    options: &SweepOptions,
) -> Option<String> {
    reconcile_watermarks(store);

    let now = stamp_now();
    let mut budget = knobs.max_chunks;

    for transcript in transcripts {
        if budget == 0 {
            break;
        }
        if transcript.lines == 0 {
            continue;
        }
        let watermark = store.session_watermark(&transcript.session);
        if transcript.lines.saturating_sub(watermark) < knobs.threshold {
            continue;
        }

        let chunks =
            match digest_claude_chunks(&transcript.path, watermark, knobs.chunk_chars, budget) {
                Ok(chunks) => chunks,
                Err(_) => continue,
            };
        if chunks.is_empty() {
            continue;
        }
        budget -= chunks.len();

        if !store.ensure_session_document(&transcript.session, &now) {
            continue;
        }
        let mut fields = vec![(
            "transcript",
            YamlValue::String(transcript.path.to_string_lossy().to_string()),
        )];
        fields.extend(session_time_fields(&transcript.path));
        store.set_fields(&store.session_key(&transcript.session), &fields);

        for chunk in &chunks {
            import_chunk(store, &transcript.session, chunk, knobs.max_items, &now);
        }
    }

    spawn_agent(store, knobs.ttl, options)
}

fn reconcile_watermarks(store: &MemoryStore) {
    let chunks = store.chunks();
    let sessions: std::collections::BTreeSet<&str> =
        chunks.iter().map(|chunk| chunk.session.as_str()).collect();

    for session in sessions {
        loop {
            let watermark = store.session_watermark(session);
            let healed = chunks.iter().find(|chunk| {
                chunk.session == session
                    && chunk.from == watermark
                    && !chunk.is_pending()
                    && chunk.covers > watermark
            });
            let Some(chunk) = healed else { break };
            let advanced = store.set_fields(
                &store.session_key(session),
                &[("distilled_lines", YamlValue::Number(chunk.covers.into()))],
            );
            if !advanced {
                break;
            }
        }

        let watermark = store.session_watermark(session);
        let now = stamp_now();
        let stranded = chunks.iter().filter(|chunk| {
            chunk.session == session && chunk.is_pending() && chunk.covers <= watermark
        });
        for chunk in stranded {
            store.stamp_chunk(
                &chunk.path,
                &[("captured_at", YamlValue::String(now.clone()))],
            );
        }
    }
}

fn session_time_fields(path: &Path) -> Vec<(&'static str, YamlValue)> {
    let (started, ended) = transcript_time_range(path);
    let mut fields = Vec::new();
    if let Some(started) = started {
        fields.push(("started", YamlValue::String(started)));
    }
    if let Some(ended) = ended {
        fields.push(("ended", YamlValue::String(ended)));
    }
    fields
}

fn import_chunk(store: &MemoryStore, session: &str, chunk: &Chunk, max_items: usize, now: &str) {
    let path = store.chunk_path(session, chunk.from);
    let mut created = now.to_string();
    let mut claimed = None;
    let mut skipped = None;

    if let Some(existing) = store.chunk_fields(&path) {
        if field_text(&existing, "captured_at").is_some() {
            return;
        }
        if field_int(&existing, "covers_lines").unwrap_or(0) >= chunk.covered {
            return;
        }
        created = field_text(&existing, "created").unwrap_or(created);
        claimed = field_text(&existing, "claimed");
        skipped = field_text(&existing, "skipped");
    }

    let mut stamps = match claimed {
        Some(claimed) => format!("claimed: \"{}\"\n", claimed),
        None => String::new(),
    };
    if let Some(skipped) = skipped {
        stamps.push_str(&format!("skipped: \"{}\"\n", skipped));
    }
    if chunk.text.is_empty() {
        stamps.push_str(&format!("captured_at: \"{}\"\n", now));
    }
    let occurred = match &chunk.occurred {
        Some(occurred) => format!("occurred: \"{}\"\n", occurred),
        None => String::new(),
    };
    let body = if chunk.text.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", chunk.text)
    };

    store.write_chunk(
        &path,
        &format!(
            "---\nsession: \"{}\"\ncreated: \"{}\"\n{}covers_from: {}\ncovers_lines: {}\nmax_items: {}\n{}---\n\n# Capture chunk {} lines {}-{}\n{}",
            session,
            created,
            occurred,
            chunk.from,
            chunk.covered,
            max_items,
            stamps,
            session,
            chunk.from,
            chunk.covered,
            body
        ),
    );
}

fn spawn_agent(store: &MemoryStore, ttl: usize, options: &SweepOptions) -> Option<String> {
    let pending: Vec<ChunkRecord> = store
        .chunks()
        .into_iter()
        .filter(ChunkRecord::is_pending)
        .collect();

    let cutoff = inflight_cutoff(ttl);
    if pending.iter().all(|chunk| chunk.is_skipped(&cutoff)) {
        return None;
    }
    if pending.iter().any(|chunk| chunk.is_claimed(&cutoff)) {
        return None;
    }

    let now = stamp_now();
    for chunk in &pending {
        store.stamp_chunk(&chunk.path, &[("claimed", YamlValue::String(now.clone()))]);
    }

    let reason = options
        .capture_reason
        .clone()
        .unwrap_or_else(default_capture_reason);
    Some(block_decision(&reason))
}

fn default_capture_reason() -> String {
    let agent = capture_agent();
    let mut env = Environment::new();
    env.add_template("capture-reason", CAPTURE_REASON_TEMPLATE)
        .expect("Failed to add template");
    env.get_template("capture-reason")
        .expect("Failed to get template")
        .render(context! { agent })
        .expect("Failed to render template")
        .trim()
        .to_string()
}

fn capture_agent() -> String {
    match plugin_name() {
        Some(plugin) => format!("{}:{}", plugin, DISTILL_AGENT),
        None => DISTILL_AGENT.to_string(),
    }
}

fn plugin_name() -> Option<String> {
    let root = std::env::var_os(PLUGIN_ROOT_VARIABLE)?;
    let manifest = Path::new(&root).join(".claude-plugin").join("plugin.json");
    let text = std::fs::read_to_string(manifest).ok()?;
    let manifest: JsonValue = serde_json::from_str(&text).ok()?;
    let name = manifest.get("name")?.as_str()?.trim();
    is_safe_id(name).then(|| name.to_string())
}

fn block_decision(reason: &str) -> String {
    let decision = BlockDecision {
        decision: "block",
        reason,
    };
    match serde_json::to_string_pretty(&decision) {
        Ok(json) => format!("{}\n", json),
        Err(_) => String::new(),
    }
}
