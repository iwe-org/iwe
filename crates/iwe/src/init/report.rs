use std::collections::BTreeMap;

use serde::Serialize;

use crate::init::evidence::{Evidence, SCAN_CAP};
use crate::init::fit::Churn;
use crate::init::probe::Probes;
use crate::init::settings::{Confidence, SettingId, Settings, Value, ALL_SETTINGS};

#[derive(Debug, Serialize)]
pub struct Report {
    pub written: bool,
    pub config_path: String,
    pub settings: BTreeMap<String, Value>,
    pub confidence: BTreeMap<String, Confidence>,
    pub evidence: EvidenceSummary,
    pub normalize_churn: Churn,
    pub default_churn: Churn,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceSummary {
    pub files: usize,
    pub scanned_files: usize,
    pub capped: bool,
    pub library_path: String,
    pub markdown_files: usize,
    pub djot_files: usize,
    pub wiki_links: usize,
    pub markdown_links: usize,
    pub unresolved_links: usize,
    pub refs_with_extension: usize,
    pub refs_bare: usize,
    pub refs_relative_votes: usize,
    pub refs_absolute_votes: usize,
    pub frontmatter_keys: BTreeMap<String, usize>,
}

pub fn summarize(evidence: &Evidence) -> EvidenceSummary {
    EvidenceSummary {
        files: evidence.files,
        scanned_files: evidence.scanned_files,
        capped: evidence.capped,
        library_path: evidence.library_path.clone(),
        markdown_files: evidence.markdown_files,
        djot_files: evidence.djot_files,
        wiki_links: evidence.wiki_links,
        markdown_links: evidence.markdown_links,
        unresolved_links: evidence.unresolved_links,
        refs_with_extension: evidence.refs_with_extension,
        refs_bare: evidence.refs_bare,
        refs_relative_votes: evidence.refs_relative_votes,
        refs_absolute_votes: evidence.refs_absolute_votes,
        frontmatter_keys: evidence.frontmatter_keys.entries().into_iter().collect(),
    }
}

fn counted(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{} {}", count, singular)
    } else {
        format!("{} {}", count, plural)
    }
}

pub fn summary_line(evidence: &Evidence) -> String {
    let mut parts = Vec::new();

    let location = if evidence.library_path.is_empty() {
        "here".to_string()
    } else {
        format!("in {}/", evidence.library_path)
    };
    parts.push(format!(
        "{} {}",
        counted(evidence.scanned_files, "file", "files"),
        location
    ));

    if evidence.wiki_links > 0 {
        parts.push(format!("{} wiki links", evidence.wiki_links));
    }
    if evidence.markdown_links > 0 {
        parts.push(format!("{} markdown links", evidence.markdown_links));
    }
    if evidence.unresolved_links > 0 {
        parts.push(format!("{} unresolved", evidence.unresolved_links));
    }

    parts.join(" · ")
}

pub fn warnings(evidence: &Evidence, settings: &Settings, probes: &Probes) -> Vec<String> {
    let mut warnings = Vec::new();

    for id in ALL_SETTINGS {
        if settings.is_mixed(id) {
            warnings.push(format!(
                "{} is mixed across the corpus — {}",
                id.label(),
                settings.note(id)
            ));
        }
    }

    if evidence.root_relative_links > 0 {
        warnings.push(format!(
            "{} from the library root but carry no leading slash — normalize rewrites to /path form",
            counted(evidence.root_relative_links, "link resolves", "links resolve")
        ));
    }

    if evidence.crlf_files > 0 {
        warnings.push(format!(
            "{} CRLF line endings — normalize writes LF",
            counted(evidence.crlf_files, "file uses", "files use")
        ));
    }

    if evidence.bom_files > 0 {
        warnings.push(format!(
            "{} with a byte order mark — normalize strips it",
            counted(evidence.bom_files, "file starts", "files start")
        ));
    }

    if evidence.setext_headers > 0 {
        warnings.push(format!(
            "{} will be rewritten as ATX headers",
            counted(evidence.setext_headers, "setext header", "setext headers")
        ));
    }

    if !evidence.case_collisions.is_empty() {
        warnings.push(format!(
            "keys differing only by case will collide on case-insensitive filesystems: {}",
            evidence.case_collisions.join("; ")
        ));
    }

    if !evidence.unreadable_files.is_empty() {
        warnings.push(format!(
            "{} could not be read as UTF-8 — skipped",
            counted(evidence.unreadable_files.len(), "file", "files")
        ));
    }

    if evidence.capped {
        warnings.push(format!(
            "corpus is larger than {} files — detection used the first {} found",
            SCAN_CAP, SCAN_CAP
        ));
    }

    if let Some(parent) = &probes.nested_library {
        warnings.push(format!("nested library — {} already contains .iwe", parent));
    }

    warnings
}

pub fn notes(evidence: &Evidence) -> Vec<String> {
    let mut notes = Vec::new();

    if evidence.unresolved_links > 0 {
        let mut note = format!(
            "{} to nothing",
            counted(evidence.unresolved_links, "link resolves", "links resolve")
        );
        if !evidence.broken_link_samples.is_empty() {
            note.push_str(&format!(" ({})", evidence.broken_link_samples.join(", ")));
        }
        notes.push(note);
    }

    if evidence.embeds > 0 {
        notes.push(format!(
            "{} (![[…]]) — read as links, not transclusions",
            counted(evidence.embeds, "embed", "embeds")
        ));
    }
    if evidence.callouts > 0 {
        notes.push(format!(
            "{} (> [!note]) — kept as block quotes",
            counted(evidence.callouts, "callout", "callouts")
        ));
    }
    if evidence.comment_lines > 0 {
        notes.push(format!(
            "{} with %%comments%% — kept as plain text",
            counted(evidence.comment_lines, "line", "lines")
        ));
    }

    if evidence.inline_tags > 0 || evidence.frontmatter_tag_files > 0 {
        notes.push(format!(
            "tags: {} inline #hashtags, {} files with frontmatter tags",
            evidence.inline_tags, evidence.frontmatter_tag_files
        ));
    }

    let frontmatter = evidence.frontmatter_keys.entries();
    if !frontmatter.is_empty() && evidence.scanned_files > 0 {
        let mut fields: Vec<(String, usize)> = frontmatter;
        fields.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        fields.truncate(6);
        let rendered: Vec<String> = fields
            .iter()
            .map(|(field, count)| format!("{} {}%", field, count * 100 / evidence.scanned_files))
            .collect();
        notes.push(format!("frontmatter fields: {}", rendered.join(", ")));
    }

    if evidence.keys_with_spaces > 0 {
        notes.push(format!(
            "{} spaces",
            counted(
                evidence.keys_with_spaces,
                "filename contains",
                "filenames contain"
            )
        ));
    }

    if !evidence.duplicate_titles.is_empty() {
        notes.push(format!(
            "duplicate titles: {}",
            evidence.duplicate_titles.join("; ")
        ));
    }

    notes
}

pub fn epilogue(probes: &Probes) -> Vec<String> {
    let mut lines = vec!["try: iwe normalize --dry-run · iwe find --fuzzy <query>".to_string()];

    for editor in &probes.editors {
        lines.push(format!("editor: {}", editor.hint()));
    }

    if probes.git_repository {
        if probes.git_clean {
            lines.push(
                "your notes are under version control — iwe normalize is safe to try".to_string(),
            );
        } else {
            lines.push("working tree is dirty — commit before running iwe normalize".to_string());
        }
    }

    lines
}

pub fn render_settings(settings: &Settings) -> String {
    let mut output = String::new();
    for id in ALL_SETTINGS {
        if id == SettingId::Agents {
            continue;
        }
        output.push_str(&format!(
            "{:<52} {:<16} {}\n",
            id.key(),
            settings.get(id).to_string(),
            settings.confidence(id)
        ));
    }
    output
}
