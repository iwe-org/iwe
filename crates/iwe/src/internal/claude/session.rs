use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Duration, Local};
use clap::Command;
use diwe::schema::SchemaBindings;
use liwe::model::Key;
use liwe::query::evaluate as evaluate_filter;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::internal::claude::digest::{count_user_turns, digest_claude_chunks, value_time};
use crate::internal::claude::hook::index::injection_groups;
use crate::internal::claude::hook::store::{
    flow_yaml, is_safe_id, non_empty_var, project_slug, MemoryStore,
};
use crate::internal::claude::prompt::unknown_invocations;
use crate::internal::claude::record::{
    all_session_records, load_session_record, mark_reminded, reminded_at, save_session_record,
    Proposal, ProposalStatus, SessionRecord,
};
use crate::new::normalize_content;
use crate::schema::render_schema;

pub const DEFAULT_CHUNK_CHARS: usize = 10000;
pub const DEFAULT_MAX_PROPOSALS_PER_READ: usize = 5;
pub const DEFAULT_REMIND_EVERY_DAYS: usize = 7;

const SESSION_VARIABLE: &str = "CLAUDE_CODE_SESSION_ID";
const ACTIVE_MINUTES: i64 = 30;
const REJECTION_SAMPLE: usize = 20;
const HEAD_SCAN_LINES: usize = 200;
const TAIL_SCAN_BYTES: u64 = 64 * 1024;
const TAIL_SCAN_LIMIT: u64 = 4 * 1024 * 1024;

pub const POLICY_SECTIONS: [&str; 3] = ["What to capture", "How to write it", "Dedup and updates"];

pub struct SessionOptions {
    pub transcripts: Option<PathBuf>,
    pub current: Option<String>,
}

impl SessionOptions {
    pub fn current_session(&self) -> Option<String> {
        self.current
            .clone()
            .filter(|id| is_safe_id(id))
            .or_else(current_session)
    }
}

pub struct StageOptions {
    pub session: Option<String>,
    pub content: String,
}

pub struct CompleteOptions {
    pub session: Option<String>,
    pub lines: Option<String>,
    pub wrote: Vec<String>,
    pub offered: Option<usize>,
    pub rejected: Vec<String>,
    pub drop_pending: bool,
    pub title: Option<String>,
    pub summary: Option<String>,
}

pub struct Backlog {
    pub sessions: usize,
    pub turns: usize,
    pub since: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Current,
    Active,
    Pending,
    Done,
    Adopted,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            State::Current => "current",
            State::Active => "active",
            State::Pending => "pending",
            State::Done => "done",
            State::Adopted => "adopted",
        }
    }

    fn is_settled(self) -> bool {
        matches!(self, State::Done | State::Adopted)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scan {
    Full,
    PendingOnly,
}

struct Transcript {
    session: String,
    path: PathBuf,
    lines: usize,
}

struct Entry {
    session: String,
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

struct Row {
    session: String,
    path: PathBuf,
    size: u64,
    lines: usize,
    remembered: bool,
    distilled: usize,
    behind: usize,
    turns: usize,
    started: Option<String>,
    ended: Option<String>,
    state: State,
    title: Option<String>,
}

pub fn current_session() -> Option<String> {
    non_empty_var(SESSION_VARIABLE).filter(|id| is_safe_id(id))
}

pub fn policy_problems(store: &MemoryStore, app: &Command) -> Vec<String> {
    let mut problems = Vec::new();
    for section in POLICY_SECTIONS {
        if store.policy_section(section).is_none() {
            problems.push(format!("missing section: ## {}", section));
        }
    }
    if let Err(problem) = store.injection_slices() {
        problems.push(problem);
    }
    problems.extend(unknown_invocations(app, &store.policy_body()));
    problems
}

pub fn session_brief(store: &MemoryStore, app: &Command) -> String {
    let mut brief = String::from("=== policy: MEMORY ===\n");
    brief.push_str(store.policy_body().trim());
    brief.push('\n');

    brief.push_str("\n=== policy check ===\n");
    let problems = policy_problems(store, app);
    if problems.is_empty() {
        brief.push_str("ok\n");
    } else {
        for problem in &problems {
            brief.push_str(&format!("{}\n", problem));
        }
    }

    let keys = injection_keys(store);
    brief.push_str(&format!(
        "\n=== schema: {} document(s) the injection selects ===\n",
        keys.len()
    ));
    let fields = liwe::schema::infer_schema(store.graph(), &keys);
    if fields.is_empty() {
        brief.push_str("no frontmatter yet: the policy is your only guide\n");
    } else {
        brief.push_str(&render_schema(&fields));
    }

    brief.push_str("\n=== schemas: what `--strict` enforces on those documents ===\n");
    brief.push_str(&schema_coverage(store, &keys));

    brief.push_str("\n=== hubs: area documents and what they include ===\n");
    brief.push_str(&hub_census_text(store, &keys));

    brief.push_str("\n=== injection: what session start lists ===\n");
    let groups = injection_groups(store);
    if groups.is_empty() {
        brief.push_str("nothing listed yet\n");
    } else {
        for (at, group) in groups.iter().enumerate() {
            if at > 0 {
                brief.push('\n');
            }
            brief.push_str(&format!("{}\n\n{}\n", group.heading, group.entries));
        }
    }

    let rejected = recent_rejections();
    brief.push_str(&format!(
        "\n=== rejected: {} recent proposal(s) the user turned down ===\n",
        rejected.len()
    ));
    if rejected.is_empty() {
        brief.push_str("nothing rejected yet\n");
    } else {
        for title in &rejected {
            brief.push_str(&format!("{}\n", title));
        }
    }

    brief
}

pub fn session_list(options: &SessionOptions, all: bool) -> Result<String, String> {
    let directory = directory_or_error(options)?;
    let rows = rows_of(&directory, options, Scan::Full);

    let mut report = format!("transcripts: {}\n", directory.display());
    match options.current_session() {
        Some(current) => report.push_str(&format!("current session: {}\n\n", current)),
        None => report.push_str(
            "current session: unknown — CLAUDE_CODE_SESSION_ID is not set; \
             confirm with the user before treating the newest row as this session\n\n",
        ),
    }

    report.push_str(&format!(
        "{:<38} {:>16} {:>16} {:>7} {:>6} {:>10} {:>8}  {:<8} {}\n",
        "session", "started", "last", "lines", "turns", "distilled", "pending", "state", "subject"
    ));

    let mut listed = 0;
    let mut hidden = 0;
    let mut pending_sessions = 0;
    let mut pending_lines = 0;
    let mut pending_turns = 0;

    for row in &rows {
        if row.state == State::Pending {
            pending_sessions += 1;
            pending_lines += row.behind;
            pending_turns += row.turns;
        }
        if row.state.is_settled() && !all {
            hidden += 1;
            continue;
        }
        listed += 1;
        report.push_str(&format!(
            "{:<38} {:>16} {:>16} {:>7} {:>6} {:>10} {:>8}  {:<8} {}\n",
            row.session,
            row.started.as_deref().unwrap_or("-"),
            row.ended.as_deref().unwrap_or("-"),
            row.lines,
            row.turns,
            row.distilled,
            row.behind,
            row.state.as_str(),
            row.title.as_deref().unwrap_or("")
        ));
    }

    report.push_str(&format!(
        "\n{} session(s) listed, {} settled and hidden, \
         {} pending over {} undistilled line(s) carrying {} user turn(s)\n",
        listed, hidden, pending_sessions, pending_lines, pending_turns
    ));
    if hidden > 0 && !all {
        report.push_str("--all lists the distilled and adopted sessions too\n");
    }
    let (staged, staged_sessions) = staged_census();
    if staged > 0 {
        report.push_str(&format!(
            "{} staged proposal(s) wait in {} session record(s) — `session inbox` prints them\n",
            staged, staged_sessions
        ));
    }
    Ok(report)
}

pub fn session_read(
    store: &MemoryStore,
    options: &SessionOptions,
    session: Option<&str>,
    from: Option<usize>,
    max_chars: Option<usize>,
) -> Result<String, String> {
    let directory = directory_or_error(options)?;
    let transcript = transcript_of(&directory, session, options)?;

    let distilled = distilled_lines_of(&transcript.session);
    let from = from.unwrap_or(distilled);
    let max_chars = max_chars.unwrap_or_else(|| store.knob_int("chunk_chars", DEFAULT_CHUNK_CHARS));
    let max_proposals = store.knob_int("max_proposals_per_read", DEFAULT_MAX_PROPOSALS_PER_READ);

    let chunks = digest_claude_chunks(&transcript.path, from, max_chars, 1)
        .map_err(|error| format!("cannot read {}: {}", transcript.path.display(), error))?;
    let chunk = chunks.into_iter().next();
    let covered = chunk.as_ref().map(|chunk| chunk.covered).unwrap_or(from);
    let occurred = chunk.as_ref().and_then(|chunk| chunk.occurred.clone());
    let text = chunk.map(|chunk| chunk.text).unwrap_or_default();

    let mut report = format!(
        "session: {}\ncovers_from: {}\ncovers_lines: {}\n",
        transcript.session, from, covered
    );
    if let Some(occurred) = occurred {
        report.push_str(&format!("occurred: {}\n", occurred));
    }
    report.push_str(&format!("max_proposals: {}\n", max_proposals));
    report.push_str(&format!("transcript_lines: {}\n\n", transcript.lines));

    if text.trim().is_empty() {
        report.push_str(
            "nothing left to read in this span: already distilled to the end of the transcript\n",
        );
    } else {
        report.push_str(text.trim());
        report.push('\n');
    }

    Ok(report)
}

pub fn session_stage(
    store: &MemoryStore,
    options: &SessionOptions,
    stage: &StageOptions,
) -> Result<String, String> {
    let session = session_id(
        options.current_session().as_deref(),
        stage.session.as_deref(),
    )?;
    let transcript = transcript_path(options, &session);
    if transcript.is_none() && load_session_record(&session).is_none() {
        return Err(format!("{}: no transcript and no session record", session));
    }

    let proposal = parse_proposal(store, &stage.content)?;
    let title = proposal.title.clone();
    let target = match &proposal.updates {
        Some(key) => format!("an update of {}", key),
        None => format!("the new document {}", proposal.key),
    };

    let mut record = load_session_record(&session).unwrap_or_else(|| SessionRecord::new(&session));
    if let Some(path) = &transcript {
        record.set_transcript(path);
    }
    record.stage_proposal(proposal);
    let pending = record.pending_proposals().len();

    if !save_session_record(&record) {
        return Err(format!(
            "could not write the session record for {}",
            session
        ));
    }

    Ok(format!(
        "staged \"{}\" as {} in {}: {} proposal(s) pending\n",
        title, target, session, pending
    ))
}

pub fn session_inbox(session: Option<&str>) -> Result<String, String> {
    match session {
        Some(session) => one_session_inbox(session),
        None => Ok(grouped_inbox()),
    }
}

fn one_session_inbox(session: &str) -> Result<String, String> {
    if !is_safe_id(session) {
        return Err(format!("'{}' is not a session id", session));
    }
    let record = load_session_record(session);
    let pending = record
        .as_ref()
        .map(|record| record.pending_proposals())
        .unwrap_or_default();
    if pending.is_empty() {
        return Ok(format!("inbox: {} — nothing staged\n", session));
    }

    let mut report = format!(
        "inbox: {} — {} pending proposal(s)\n",
        session,
        pending.len()
    );
    for proposal in pending {
        report.push('\n');
        report.push_str(&entry_field("title", &proposal.title));
        report.push_str(&entry_field("key", &proposal.key));
        if let Some(classification) = &proposal.classification {
            report.push_str(&entry_field("classification", classification));
        }
        if let Some(updates) = &proposal.updates {
            report.push_str(&entry_field("updates", updates));
        }
        report.push_str(&entry_field("body", &proposal.body));
        report.push_str(&entry_field("evidence", &proposal.evidence));
    }
    Ok(report)
}

fn grouped_inbox() -> String {
    let records = all_session_records();
    let mut groups: BTreeMap<String, Vec<(&str, &Proposal)>> = BTreeMap::new();
    let mut sessions = 0;
    let mut staged = 0;

    for record in &records {
        let pending = record.pending_proposals();
        if pending.is_empty() {
            continue;
        }
        sessions += 1;
        staged += pending.len();
        for proposal in pending {
            groups
                .entry(proposal.target().to_string())
                .or_default()
                .push((record.session.as_str(), proposal));
        }
    }

    if staged == 0 {
        return "inbox: nothing staged\n".to_string();
    }

    let mut ordered: Vec<(&String, &Vec<(&str, &Proposal)>)> = groups.iter().collect();
    ordered.sort_by(|left, right| right.1.len().cmp(&left.1.len()).then(left.0.cmp(right.0)));

    let mut report = format!(
        "inbox: {} pending proposal(s) staged in {} session record(s), \
         over {} candidate key(s)\n",
        staged,
        sessions,
        ordered.len()
    );
    for (number, (key, entries)) in ordered.into_iter().enumerate() {
        let contributors: HashSet<&str> = entries.iter().map(|(session, _)| *session).collect();
        report.push_str(&format!(
            "\n=== {}. {} — {} proposal(s) from {} session(s) ===\n",
            number + 1,
            key,
            entries.len(),
            contributors.len()
        ));
        for (session, proposal) in entries {
            let classification = match &proposal.classification {
                Some(classification) => format!(" [{}]", classification),
                None => String::new(),
            };
            let shape = match &proposal.updates {
                Some(updates) => format!("an update of {}", updates),
                None => format!("new, as {}", proposal.key),
            };
            report.push_str(&format!(
                "{}{} — {} — {}\n",
                proposal.title, classification, session, shape
            ));
            let summary = proposal.summary();
            if !summary.is_empty() {
                report.push_str(&format!("  {}\n", summary));
            }
        }
    }
    report.push_str(
        "\n`session inbox <id>` prints one session's staged proposals with their evidence\n",
    );
    report
}

fn entry_field(name: &str, value: &str) -> String {
    if value.contains('\n') {
        let indented: Vec<String> = value.lines().map(|line| format!("  {}", line)).collect();
        format!("{}:\n{}\n", name, indented.join("\n"))
    } else {
        format!("{}: {}\n", name, value)
    }
}

fn parse_proposal(store: &MemoryStore, content: &str) -> Result<Proposal, String> {
    if content.trim().is_empty() {
        return Err(
            "--content: no proposal — pass the entry as YAML on stdin with `--content -`"
                .to_string(),
        );
    }

    let mut proposal: Proposal = serde_yaml::from_str(content).map_err(|error| {
        format!(
            "--content: {} — a proposal is a YAML mapping of title, key, body and evidence, \
             with classification and updates when they apply",
            error
        )
    })?;

    proposal.status = ProposalStatus::Pending;
    proposal.title = proposal.title.trim().to_string();
    proposal.key = proposal.key.trim().trim_start_matches('/').to_string();
    proposal.body = proposal.body.trim().to_string();
    proposal.evidence = proposal.evidence.trim().to_string();
    proposal.classification = proposal
        .classification
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    proposal.updates = proposal
        .updates
        .map(|value| value.trim().trim_start_matches('/').to_string())
        .filter(|value| !value.is_empty());

    for (name, value) in [
        ("title", &proposal.title),
        ("key", &proposal.key),
        ("body", &proposal.body),
        ("evidence", &proposal.evidence),
    ] {
        if value.is_empty() {
            return Err(format!("--content: {} is empty", name));
        }
    }

    proposal.key = Key::name(&proposal.key).to_string();
    if proposal.key == "MEMORY" {
        return Err(
            "--content: key MEMORY: the machinery's own documents are not capture output"
                .to_string(),
        );
    }

    if let Some(updates) = &proposal.updates {
        let key = Key::name(updates).to_string();
        if !store.has_document(&key) {
            return Err(format!(
                "--content: updates {}: no such document in this store",
                key
            ));
        }
        proposal.updates = Some(key);
    }

    Ok(proposal)
}

pub fn session_complete(
    store: &MemoryStore,
    options: &SessionOptions,
    complete: &CompleteOptions,
) -> Result<String, String> {
    let current = options.current_session();
    let session = session_id(current.as_deref(), complete.session.as_deref())?;

    let transcript = transcript_path(options, &session);

    if transcript.is_none() && load_session_record(&session).is_none() {
        return Err(format!("{}: no transcript and no session record", session));
    }

    let measured = transcript
        .as_ref()
        .map(|path| (file_size(path), transcript_lines(path)));

    let advance = match complete.lines.as_deref() {
        None => None,
        Some("now") => match (&transcript, measured) {
            (Some(path), Some((_, total))) => {
                if current.as_deref() != Some(session.as_str()) && is_live(path) {
                    return Err(format!(
                        "--lines now on {}: it is another live conversation, and its transcript \
                         is still growing — leave it to its own session, or name the line count \
                         you read to",
                        session
                    ));
                }
                Some(total)
            }
            _ => {
                return Err(format!(
                    "--lines now: no transcript on disk for {}, so its length is unknown",
                    session
                ))
            }
        },
        Some(text) => match text.parse::<usize>() {
            Ok(lines) => {
                if let Some((_, total)) = measured {
                    if lines > total {
                        return Err(format!(
                            "--lines {}: {} is {} line(s) long",
                            lines, session, total
                        ));
                    }
                }
                Some(lines)
            }
            Err(_) => return Err(format!("--lines {}: expected a line count or `now`", text)),
        },
    };

    let keys = resolve_links(store, &complete.wrote)?;
    let linked = link_into_hubs(store, &keys);

    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let mut record = load_session_record(&session).unwrap_or_else(|| SessionRecord::new(&session));

    let distilled = record.distilled_lines;
    let advanced = advance.filter(|lines| *lines > distilled);
    if let Some(lines) = advanced {
        record.distilled_lines = lines;
        record.distilled_at = Some(now.clone());
    }
    if let Some(path) = &transcript {
        record.set_transcript(path);
    }
    if let (Some(_), Some((size, total))) = (advance, measured) {
        record.set_size(size, total);
    }
    record.add_ledger(complete.offered.unwrap_or(0), &complete.rejected);
    let settled = record.settle_proposals(&keys, &complete.rejected);
    let swept = match complete.drop_pending {
        true => record.reject_pending_proposals(),
        false => Vec::new(),
    };
    record.add_ledger(0, &swept);
    let kept = keys.len();
    if kept > 0 {
        record.add_capture(&now, advanced, keys);
    }
    record.set_title_once(complete.title.as_deref());
    record.set_summary_once(complete.summary.as_deref());
    let pending = record.pending_proposals().len();

    if !save_session_record(&record) {
        return Err(format!(
            "could not write the session record for {}",
            session
        ));
    }

    if advance.is_some() {
        mark_reminded(&now);
    }

    let mut report = match advanced {
        Some(lines) => format!(
            "completed {}: distilled through line {}, {} link(s)\n",
            session, lines, kept
        ),
        None => format!(
            "recorded {}: distilled line unchanged at {}, {} link(s)\n",
            session, distilled, kept
        ),
    };
    for item in &settled {
        match item.status {
            ProposalStatus::Written => report.push_str(&format!(
                "settled the staged proposal \"{}\": written to {}\n",
                item.title, item.key
            )),
            ProposalStatus::Rejected => report.push_str(&format!(
                "settled the staged proposal \"{}\": rejected\n",
                item.title
            )),
            ProposalStatus::Pending => {}
        }
    }
    if !swept.is_empty() {
        report.push_str(&format!(
            "dropped {} staged proposal(s) nobody kept: {}\n",
            swept.len(),
            swept.join(", ")
        ));
    }
    if pending > 0 {
        report.push_str(&format!(
            "{} staged proposal(s) still pending in {}\n",
            pending, session
        ));
    }
    for (key, area) in &linked {
        report.push_str(&format!("linked {} into its area hub {}\n", key, area));
    }
    Ok(report)
}

fn schema_coverage(store: &MemoryStore, keys: &[Key]) -> String {
    let config = store.config();
    let none = format!(
        "no schema binds any of the {} document(s) the filter selects — nothing enforces \"how to write it\" (`iwe docs schema`; a reflect session writes one)\n",
        keys.len()
    );
    if config.schemas.is_empty() {
        return none;
    }
    let bindings = match SchemaBindings::compile(&config.schemas) {
        Ok(bindings) => bindings,
        Err(errors) => return format!("{}\n", errors.join("\n")),
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut unbound = 0;
    for key in keys {
        let names = bindings.schemas_for(&key.to_string());
        if names.is_empty() {
            unbound += 1;
        }
        for name in names {
            *counts.entry(name.to_string()).or_default() += 1;
        }
    }
    if counts.is_empty() {
        return none;
    }
    let mut text = String::new();
    for (name, count) in &counts {
        text.push_str(&format!(
            "{} — .iwe/schemas/{}.yaml binds {} document(s)\n",
            name, name, count
        ));
    }
    if unbound > 0 {
        text.push_str(&format!("{} document(s) bind no schema\n", unbound));
    }
    text
}

struct HubCensus {
    area: String,
    members: usize,
    strays: Vec<String>,
}

fn area_of(key: &str) -> Option<&str> {
    key.split_once('/').map(|(area, _)| area)
}

fn included_by(store: &MemoryStore, hub: &str) -> HashSet<String> {
    let expression = format!(
        "{{ $includedBy: {} }}",
        flow_yaml(&YamlValue::String(hub.to_string()))
    );
    store
        .filter_of(&expression)
        .map(|filter| evaluate_filter(&filter, store.graph()))
        .unwrap_or_default()
        .into_iter()
        .map(|key| key.to_string())
        .collect()
}

fn hub_census(store: &MemoryStore, keys: &[Key]) -> (Vec<HubCensus>, Vec<String>) {
    let mut areas: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut flat: Vec<String> = Vec::new();
    for key in keys {
        let text = key.to_string();
        match area_of(&text) {
            Some(area) if store.has_document(area) => areas
                .entry(area.to_string())
                .or_default()
                .push(text.clone()),
            Some(_) => {}
            None => flat.push(text.clone()),
        }
    }
    let census = areas
        .into_iter()
        .map(|(area, members)| {
            let included = included_by(store, &area);
            let strays: Vec<String> = members
                .iter()
                .filter(|member| !included.contains(*member))
                .cloned()
                .collect();
            HubCensus {
                area,
                members: members.len(),
                strays,
            }
        })
        .collect();
    flat.retain(|key| key != "MEMORY");
    (census, flat)
}

fn hub_census_text(store: &MemoryStore, keys: &[Key]) -> String {
    let (census, flat) = hub_census(store, keys);
    if census.is_empty() {
        return "no area hubs: no top-level document shares its key with a directory of these documents (a reflect session groups them when the policy allows)\n".to_string();
    }
    let hubs: HashSet<&str> = census.iter().map(|hub| hub.area.as_str()).collect();
    let mut text = String::new();
    for hub in &census {
        if hub.strays.is_empty() {
            text.push_str(&format!(
                "{} — includes all {} document(s) under {}/\n",
                hub.area, hub.members, hub.area
            ));
        } else {
            text.push_str(&format!(
                "{} — includes {} of {} document(s) under {}/; not included: {}\n",
                hub.area,
                hub.members - hub.strays.len(),
                hub.members,
                hub.area,
                hub.strays.join(", ")
            ));
        }
    }
    let outside: Vec<&String> = flat
        .iter()
        .filter(|key| !hubs.contains(key.as_str()))
        .collect();
    if !outside.is_empty() {
        let named: Vec<&str> = outside.iter().map(|key| key.as_str()).collect();
        text.push_str(&format!(
            "{} document(s) at the top level, outside every area: {}\n",
            outside.len(),
            named.join(", ")
        ));
    }
    text
}

fn link_into_hubs(store: &MemoryStore, keys: &[String]) -> Vec<(String, String)> {
    let mut linked = Vec::new();
    for key in keys {
        let Some(area) = area_of(key) else {
            continue;
        };
        if !store.has_document(area) || included_by(store, area).contains(key) {
            continue;
        }
        let title = store
            .graph()
            .get_key_title(&Key::name(key))
            .unwrap_or_else(|| key.clone());
        let path = store.document_path(area);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let content = format!("{}\n\n[{}]({})\n", raw.trim_end(), title, key);
        let content = normalize_content(store.config(), &Key::name(area), &content);
        if std::fs::write(&path, content).is_ok() {
            linked.push((key.clone(), area.to_string()));
        }
    }
    linked
}

pub fn session_adopt(options: &SessionOptions, sessions: &[String]) -> Result<String, String> {
    let directory = directory_or_error(options)?;
    for session in sessions {
        if !is_safe_id(session) {
            return Err(format!("'{}' is not a session id", session));
        }
    }

    let rows = rows_of(&directory, options, Scan::Full);
    for session in sessions {
        if !rows.iter().any(|row| &row.session == session) {
            return Err(format!(
                "no transcript for {} under {}",
                session,
                directory.display()
            ));
        }
    }

    let mut report = String::new();
    let mut adopted = 0;
    let mut refused = 0;
    let mut skipped = 0;

    for row in &rows {
        let named = sessions.contains(&row.session);
        if !sessions.is_empty() && !named {
            continue;
        }
        if matches!(row.state, State::Current | State::Active) {
            if named {
                report.push_str(&format!(
                    "refused {}: it is the {} conversation — adopting it would mark a live \
                     session read\n",
                    row.session,
                    row.state.as_str()
                ));
                refused += 1;
            } else {
                skipped += 1;
            }
            continue;
        }
        if row.behind == 0 || row.lines == 0 {
            if named {
                report.push_str(&format!(
                    "nothing to adopt in {}: it is distilled through its end already\n",
                    row.session
                ));
            }
            if !row.remembered && row.lines > 0 {
                let mut record = load_session_record(&row.session)
                    .unwrap_or_else(|| SessionRecord::new(&row.session));
                record.set_size(row.size, row.lines);
                save_session_record(&record);
            }
            continue;
        }
        if !named && row.state != State::Pending {
            continue;
        }
        let mut record =
            load_session_record(&row.session).unwrap_or_else(|| SessionRecord::new(&row.session));
        record.distilled_lines = row.lines;
        record.set_transcript(&row.path);
        record.set_size(row.size, row.lines);
        let dropped = record.drop_pending_proposals();
        if !save_session_record(&record) {
            continue;
        }
        match dropped {
            0 => report.push_str(&format!("adopted {} at line {}\n", row.session, row.lines)),
            dropped => report.push_str(&format!(
                "adopted {} at line {}, dropping {} staged proposal(s)\n",
                row.session, row.lines, dropped
            )),
        }
        adopted += 1;
    }

    report.push_str(&format!(
        "\n{} session(s) adopted without reading, {} refused",
        adopted, refused
    ));
    if skipped > 0 {
        report.push_str(&format!(
            ", {} left alone as the current or a live conversation",
            skipped
        ));
    }
    report.push('\n');
    Ok(report)
}

pub fn backlog_of(options: &SessionOptions) -> Option<Backlog> {
    let directory = transcripts_directory(options.transcripts.as_deref())?;
    let rows = rows_of(&directory, options, Scan::PendingOnly);

    let pending: Vec<&Row> = rows
        .iter()
        .filter(|row| row.state == State::Pending)
        .collect();
    if pending.is_empty() {
        return None;
    }

    let since = pending
        .iter()
        .filter_map(|row| row.started.clone().or_else(|| row.ended.clone()))
        .min()
        .map(|stamp| stamp.split_whitespace().next().unwrap_or("").to_string())
        .filter(|day| !day.is_empty());

    Some(Backlog {
        sessions: pending.len(),
        turns: pending.iter().map(|row| row.turns).sum(),
        since,
    })
}

pub fn reminder_due(store: &MemoryStore) -> bool {
    let days = store.knob_int("remind_every_days", DEFAULT_REMIND_EVERY_DAYS);
    if days == 0 {
        return false;
    }
    match reminded_at() {
        Some(stamp) => {
            let cutoff = (Local::now() - Duration::days(days as i64))
                .format("%Y-%m-%d %H:%M")
                .to_string();
            stamp.as_str() < cutoff.as_str()
        }
        None => true,
    }
}

fn session_id(current: Option<&str>, explicit: Option<&str>) -> Result<String, String> {
    let session = match explicit.or(current) {
        Some(session) => session.to_string(),
        None => {
            return Err(
                "no session id: pass one, or run where CLAUDE_CODE_SESSION_ID is set".to_string(),
            )
        }
    };
    if !is_safe_id(&session) {
        return Err(format!("'{}' is not a session id", session));
    }
    Ok(session)
}

fn transcript_path(options: &SessionOptions, session: &str) -> Option<PathBuf> {
    transcripts_directory(options.transcripts.as_deref())
        .map(|directory| directory.join(format!("{}.jsonl", session)))
        .filter(|path| path.is_file())
}

fn staged_census() -> (usize, usize) {
    let mut staged = 0;
    let mut sessions = 0;
    for record in all_session_records() {
        let pending = record.pending_proposals().len();
        if pending > 0 {
            sessions += 1;
            staged += pending;
        }
    }
    (staged, sessions)
}

fn directory_or_error(options: &SessionOptions) -> Result<PathBuf, String> {
    transcripts_directory(options.transcripts.as_deref())
        .ok_or_else(|| "no transcript directory found for this project".to_string())
}

fn transcript_of(
    directory: &Path,
    session: Option<&str>,
    options: &SessionOptions,
) -> Result<Transcript, String> {
    let session = match session
        .map(str::to_string)
        .or_else(|| options.current_session())
    {
        Some(session) => session,
        None => {
            return Err(
                "no session id: pass one, or run where CLAUDE_CODE_SESSION_ID is set".to_string(),
            )
        }
    };
    if !is_safe_id(&session) {
        return Err(format!("'{}' is not a session id", session));
    }

    let path = directory.join(format!("{}.jsonl", session));
    if !path.is_file() {
        return Err(format!(
            "no transcript for {} under {}",
            session,
            directory.display()
        ));
    }
    Ok(Transcript {
        lines: transcript_lines(&path),
        session,
        path,
    })
}

fn rows_of(directory: &Path, options: &SessionOptions, scan: Scan) -> Vec<Row> {
    let current = options.current_session();
    let full = scan == Scan::Full;

    ordered_transcripts(directory)
        .into_iter()
        .map(|transcript| {
            let record = load_session_record(&transcript.session);
            let distilled = record
                .as_ref()
                .map(|record| record.distilled_lines)
                .unwrap_or(0);
            let distilled_at = record
                .as_ref()
                .and_then(|record| record.distilled_at.clone());
            let remembered = record
                .as_ref()
                .and_then(|record| record.remembered_lines(transcript.size));
            let lines = remembered.unwrap_or_else(|| transcript_lines(&transcript.path));
            let behind = lines.saturating_sub(distilled);
            let is_current = Some(transcript.session.as_str()) == current.as_deref();

            let turns = if behind > 0 && (full || !is_current) {
                count_user_turns(&transcript.path, distilled).unwrap_or(0)
            } else {
                0
            };
            let stamps = if full || turns > 0 {
                transcript_stamps(&transcript.path)
            } else {
                Stamps {
                    started: None,
                    ended: None,
                    last: None,
                }
            };

            let state = if is_current {
                State::Current
            } else if still_live(&stamps, transcript.modified) {
                State::Active
            } else if behind > 0 && turns > 0 {
                State::Pending
            } else if distilled_at.is_some() {
                State::Done
            } else if distilled > 0 {
                State::Adopted
            } else {
                State::Done
            };

            let (started, ended) = (stamps.started.clone(), stamps.ended.clone());
            let title = record.and_then(|record| record.title);

            Row {
                session: transcript.session,
                path: transcript.path,
                size: transcript.size,
                lines,
                remembered: remembered.is_some(),
                distilled,
                behind,
                turns,
                started,
                ended,
                state,
                title,
            }
        })
        .collect()
}

pub fn transcripts_directory(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(directory) = explicit {
        return directory.is_dir().then(|| directory.to_path_buf());
    }

    if let Some(directory) = non_empty_var("IWE_MEMORY_TRANSCRIPTS") {
        let directory = PathBuf::from(directory);
        return directory.is_dir().then_some(directory);
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

fn ordered_transcripts(directory: &Path) -> Vec<Entry> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut found: Vec<(SystemTime, String, PathBuf, u64)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jsonl") {
            continue;
        }
        let (size, modified) = entry
            .metadata()
            .map(|meta| {
                (
                    meta.len(),
                    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                )
            })
            .unwrap_or((0, SystemTime::UNIX_EPOCH));
        found.push((modified, name, path, size));
    }

    found.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));

    found
        .into_iter()
        .filter_map(|(modified, name, path, size)| {
            let session = name.trim_end_matches(".jsonl").to_string();
            is_safe_id(&session).then_some(Entry {
                session,
                path,
                size,
                modified,
            })
        })
        .collect()
}

fn distilled_lines_of(session: &str) -> usize {
    load_session_record(session)
        .map(|record| record.distilled_lines)
        .unwrap_or(0)
}

fn is_live(path: &Path) -> bool {
    still_live(&transcript_stamps(path), modified_at(path))
}

fn modified_at(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
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

pub struct Stamps {
    pub started: Option<String>,
    pub ended: Option<String>,
    last: Option<DateTime<Local>>,
}

pub fn transcript_stamps(path: &Path) -> Stamps {
    let head = head_stamp(path);
    let tail = tail_stamp(path).or_else(|| head.clone());
    Stamps {
        started: head.map(|(text, _)| text),
        ended: tail.as_ref().map(|(text, _)| text.clone()),
        last: tail.map(|(_, time)| time),
    }
}

fn line_stamp(line: &[u8]) -> Option<(String, DateTime<Local>)> {
    let text = String::from_utf8_lossy(line);
    let value = serde_json::from_str::<JsonValue>(text.trim()).ok()?;
    let time = value_time(&value)?;
    Some((time.format("%Y-%m-%d %H:%M").to_string(), time))
}

fn head_stamp(path: &Path) -> Option<(String, DateTime<Local>)> {
    let mut reader = BufReader::new(File::open(path).ok()?);
    let mut buffer = Vec::new();
    for _ in 0..HEAD_SCAN_LINES {
        buffer.clear();
        match std::io::BufRead::read_until(&mut reader, b'\n', &mut buffer) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {}
        }
        if let Some(stamp) = line_stamp(&buffer) {
            return Some(stamp);
        }
    }
    None
}

fn tail_stamp(path: &Path) -> Option<(String, DateTime<Local>)> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let mut window = TAIL_SCAN_BYTES;

    loop {
        let start = length.saturating_sub(window);
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let mut lines = buffer.split(|byte| *byte == b'\n');
        if start > 0 {
            lines.next();
        }
        let found = lines.filter_map(line_stamp).next_back();
        if found.is_some() || start == 0 || window >= TAIL_SCAN_LIMIT {
            return found;
        }
        window *= 8;
    }
}

fn still_live(stamps: &Stamps, modified: SystemTime) -> bool {
    match stamps.last {
        Some(last) => Local::now().signed_duration_since(last) < Duration::minutes(ACTIVE_MINUTES),
        None => {
            modified
                >= SystemTime::now() - std::time::Duration::from_secs((ACTIVE_MINUTES * 60) as u64)
        }
    }
}

fn recent_rejections() -> Vec<String> {
    let mut records: Vec<SessionRecord> = all_session_records()
        .into_iter()
        .filter(|record| !record.rejected.is_empty())
        .collect();

    records.sort_by(|left, right| {
        right
            .distilled_at
            .cmp(&left.distilled_at)
            .then_with(|| right.ended.cmp(&left.ended))
            .then_with(|| right.session.cmp(&left.session))
    });

    records
        .into_iter()
        .flat_map(|record| record.rejected.into_iter().rev())
        .take(REJECTION_SAMPLE)
        .collect()
}

fn injection_keys(store: &MemoryStore) -> Vec<Key> {
    evaluate_filter(&store.injection_scope(), store.graph())
}

fn resolve_links(store: &MemoryStore, wrote: &[String]) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut keys = Vec::new();

    for raw in wrote {
        let trimmed = raw.trim().trim_start_matches('/');
        if trimmed.is_empty() {
            return Err("--wrote: empty key".to_string());
        }
        let key = Key::name(trimmed).to_string();
        if !seen.insert(key.clone()) {
            continue;
        }
        if key == "MEMORY" {
            return Err(format!(
                "--wrote {}: the machinery's own documents are not capture output",
                key
            ));
        }
        if !store.has_document(&key) {
            return Err(format!("--wrote {}: no such document in this store", key));
        }
        keys.push(key);
    }

    Ok(keys)
}
