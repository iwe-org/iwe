use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use diwe::fs::walk_md_paths;
use liwe::model::config::Format;
use regex::Regex;
use serde::Serialize;

pub const SCAN_CAP: usize = 5000;

static WIKI_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(!?)\[\[([^\[\]]+)\]\]").expect("valid wiki link pattern"));

static MARKDOWN_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(!?)\[([^\]]*)\]\(([^)]*)\)").expect("valid markdown link pattern")
});

static CALLOUT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*>\s*\[!\w+\]").expect("valid callout pattern"));

static INLINE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|\s)#[A-Za-z][\w/-]*").expect("valid tag pattern"));

pub const KEY_DATE_PATTERNS: [&str; 6] = [
    "%Y-%m-%d", "%Y%m%d", "%Y_%m_%d", "%d-%m-%Y", "%m-%d-%Y", "%d.%m.%Y",
];

pub const DISPLAY_DATE_PATTERNS: [&str; 7] = [
    "%b %d, %Y",
    "%B %d, %Y",
    "%Y-%m-%d",
    "%d.%m.%Y",
    "%d %B %Y",
    "%m/%d/%Y",
    "%d/%m/%Y",
];

const ASSET_EXTENSIONS: [&str; 14] = [
    "png", "jpg", "jpeg", "gif", "svg", "webp", "pdf", "mp4", "mp3", "wav", "zip", "csv", "xlsx",
    "docx",
];

const STOPWORDS: [(&str, &[&str]); 10] = [
    (
        "english",
        &[
            "the", "and", "that", "with", "this", "from", "have", "for", "not", "are",
        ],
    ),
    (
        "german",
        &[
            "der", "die", "das", "und", "nicht", "mit", "ist", "auf", "für", "den",
        ],
    ),
    (
        "french",
        &[
            "les", "des", "est", "que", "pour", "dans", "une", "avec", "sur", "pas",
        ],
    ),
    (
        "spanish",
        &[
            "que", "los", "las", "por", "para", "con", "una", "del", "como", "más",
        ],
    ),
    (
        "italian",
        &[
            "che", "per", "non", "una", "con", "sono", "del", "nel", "come", "alla",
        ],
    ),
    (
        "portuguese",
        &[
            "que", "não", "uma", "com", "para", "por", "dos", "mais", "como", "está",
        ],
    ),
    (
        "dutch",
        &[
            "het", "een", "van", "dat", "niet", "voor", "met", "zijn", "aan", "worden",
        ],
    ),
    (
        "russian",
        &[
            "что", "это", "как", "для", "или", "все", "так", "его", "ный", "быть",
        ],
    ),
    (
        "swedish",
        &[
            "och", "att", "det", "som", "för", "inte", "med", "har", "den", "till",
        ],
    ),
    (
        "danish",
        &[
            "og", "det", "til", "som", "for", "ikke", "med", "har", "den", "kan",
        ],
    ),
];

#[derive(Debug, Clone, Default, Serialize)]
pub struct Tally<K: Ord + Clone + Serialize> {
    counts: BTreeMap<K, usize>,
}

impl<K: Ord + Clone + Serialize> Tally<K> {
    pub fn bump(&mut self, key: K) {
        *self.counts.entry(key).or_default() += 1;
    }

    pub fn add(&mut self, key: K, amount: usize) {
        *self.counts.entry(key).or_default() += amount;
    }

    pub fn count(&self, key: &K) -> usize {
        self.counts.get(key).copied().unwrap_or(0)
    }

    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn dominant(&self) -> Option<(K, usize)> {
        self.counts
            .iter()
            .max_by_key(|(key, count)| (**count, std::cmp::Reverse((*key).clone())))
            .map(|(key, count)| (key.clone(), *count))
    }

    pub fn runner_up(&self) -> Option<(K, usize)> {
        let dominant = self.dominant()?;
        self.counts
            .iter()
            .filter(|(key, _)| **key != dominant.0)
            .max_by_key(|(_, count)| **count)
            .map(|(key, count)| (key.clone(), *count))
    }

    pub fn entries(&self) -> Vec<(K, usize)> {
        self.counts
            .iter()
            .map(|(key, count)| (key.clone(), *count))
            .collect()
    }
}

impl<K: Ord + Clone + Serialize> PartialEq for Tally<K> {
    fn eq(&self, other: &Self) -> bool {
        self.counts == other.counts
    }
}

#[derive(Debug, Clone)]
pub struct LinkRef {
    pub from_key: String,
    pub target: String,
    pub text: String,
    pub is_wiki: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Evidence {
    pub files: usize,
    pub scanned_files: usize,
    pub capped: bool,
    pub markdown_files: usize,
    pub djot_files: usize,
    pub library_path: String,

    pub wiki_links: usize,
    pub markdown_links: usize,
    pub wiki_pathed: usize,
    pub wiki_bare: usize,
    pub refs_with_extension: usize,
    pub refs_bare: usize,
    pub refs_relative_votes: usize,
    pub refs_absolute_votes: usize,
    pub root_relative_links: usize,
    pub unresolved_links: usize,
    pub broken_link_samples: Vec<String>,

    pub refs_text_matching: usize,
    pub refs_text_diverging: usize,

    pub key_date_matches: Tally<String>,
    pub title_date_matches: Tally<String>,
    pub frontmatter_title_files: usize,
    pub frontmatter_title_without_header: usize,
    pub header_files: usize,

    pub language_scores: Tally<String>,
    pub locale_language: Option<String>,

    pub list_tokens: Tally<String>,
    pub emphasis_tokens: Tally<String>,
    pub strong_tokens: Tally<String>,
    pub ordered_tokens: Tally<String>,
    pub ordered_incrementing: usize,
    pub ordered_flat: usize,
    pub code_fence_tokens: Tally<String>,
    pub code_fence_lengths: Tally<usize>,
    pub rule_tokens: Tally<String>,
    pub rule_lengths: Tally<usize>,
    pub bullet_indents: Tally<usize>,
    pub ordered_indents: Tally<usize>,

    pub wrapped_line_lengths: Tally<usize>,
    pub multiline_paragraphs: usize,
    pub single_line_paragraphs: usize,
    pub hard_break_backslash: usize,
    pub hard_break_spaces: usize,
    pub blank_run_files: usize,

    pub key_styles: Tally<String>,

    pub crlf_files: usize,
    pub bom_files: usize,
    pub setext_headers: usize,
    pub embeds: usize,
    pub callouts: usize,
    pub comment_lines: usize,
    pub inline_tags: usize,
    pub frontmatter_tag_files: usize,
    pub frontmatter_keys: Tally<String>,
    pub case_collisions: Vec<String>,
    pub keys_with_spaces: usize,
    pub duplicate_titles: Vec<String>,
    pub unreadable_files: Vec<String>,
}

impl Evidence {
    pub fn local_links(&self) -> usize {
        self.wiki_links + self.markdown_links
    }
}

struct FileFacts {
    key: String,
    title: Option<String>,
    links: Vec<LinkRef>,
}

pub fn scan(root: &Path) -> Evidence {
    let mut evidence = Evidence::default();

    let markdown_paths = walk_md_paths(root, Format::Markdown);
    let djot_paths = walk_md_paths(root, Format::Djot);

    evidence.markdown_files = markdown_paths.len();
    evidence.djot_files = djot_paths.len();
    evidence.files = markdown_paths.len() + djot_paths.len();

    let dominant_format = if djot_paths.len() > markdown_paths.len() {
        Format::Djot
    } else {
        Format::Markdown
    };

    let mut paths = match dominant_format {
        Format::Djot => djot_paths,
        Format::Markdown => markdown_paths,
    };
    paths.sort();

    evidence.library_path = detect_library_path(&paths);

    let prefix = if evidence.library_path.is_empty() {
        String::new()
    } else {
        format!("{}/", evidence.library_path)
    };

    let mut in_library: Vec<(String, PathBuf)> = paths
        .into_iter()
        .filter(|(key, _)| prefix.is_empty() || key.starts_with(&prefix))
        .map(|(key, path)| (key[prefix.len()..].to_string(), path))
        .collect();

    if in_library.len() > SCAN_CAP {
        evidence.capped = true;
        in_library.truncate(SCAN_CAP);
    }
    evidence.scanned_files = in_library.len();

    let keys: BTreeSet<String> = in_library.iter().map(|(key, _)| key.clone()).collect();

    let mut facts = Vec::new();
    for (key, path) in &in_library {
        match std::fs::read(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(raw) => facts.push(scan_file(key, &raw, &mut evidence)),
                Err(_) => evidence.unreadable_files.push(key.clone()),
            },
            Err(_) => evidence.unreadable_files.push(key.clone()),
        }
    }

    resolve_links(&facts, &keys, &mut evidence);
    check_key_hygiene(&keys, &facts, &mut evidence);
    evidence.locale_language = locale_language();

    evidence
}

const ROOT_META_FILES: [&str; 9] = [
    "readme",
    "agents",
    "claude",
    "contributing",
    "changelog",
    "license",
    "code_of_conduct",
    "security",
    "index",
];

fn is_root_meta_file(key: &str) -> bool {
    !key.contains('/') && ROOT_META_FILES.contains(&key.to_lowercase().as_str())
}

fn detect_library_path(all_paths: &[(String, PathBuf)]) -> String {
    let paths: Vec<&(String, PathBuf)> = all_paths
        .iter()
        .filter(|(key, _)| !is_root_meta_file(key))
        .collect();

    if paths.is_empty() {
        return String::new();
    }

    let mut current = String::new();
    loop {
        let prefix = if current.is_empty() {
            String::new()
        } else {
            format!("{}/", current)
        };

        let under: Vec<&String> = paths
            .iter()
            .map(|(key, _)| key)
            .filter(|key| prefix.is_empty() || key.starts_with(&prefix))
            .collect();

        let mut children: BTreeMap<String, usize> = BTreeMap::new();
        for key in &under {
            let rest = &key[prefix.len()..];
            if let Some((head, _)) = rest.split_once('/') {
                *children.entry(head.to_string()).or_default() += 1;
            }
        }

        let qualifying: Vec<(&String, &usize)> = children
            .iter()
            .filter(|(_, count)| **count * 10 >= paths.len() * 9)
            .collect();

        match qualifying.as_slice() {
            [(head, _)] => {
                current = if current.is_empty() {
                    (*head).clone()
                } else {
                    format!("{}/{}", current, head)
                };
            }
            _ => return current,
        }
    }
}

fn scan_file(key: &str, raw: &str, evidence: &mut Evidence) -> FileFacts {
    if raw.starts_with('\u{FEFF}') {
        evidence.bom_files += 1;
    }
    if raw.contains("\r\n") {
        evidence.crlf_files += 1;
    }

    let text = raw.trim_start_matches('\u{FEFF}').replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();

    let mut cursor = 0;
    let mut title = None;

    if lines.first().map(|line| line.trim_end()) == Some("---") {
        let end = lines
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, line)| line.trim_end() == "---")
            .map(|(index, _)| index);
        if let Some(end) = end {
            let mut has_title = false;
            for line in &lines[1..end] {
                if let Some((field, value)) = line.split_once(':') {
                    if !field.starts_with([' ', '\t', '-']) && !field.trim().is_empty() {
                        let field = field.trim();
                        evidence.frontmatter_keys.bump(field.to_string());
                        if field == "title" {
                            has_title = true;
                            let value = value.trim().trim_matches(['"', '\'']).to_string();
                            if !value.is_empty() {
                                title = Some(value);
                            }
                        }
                        if field == "tags" {
                            evidence.frontmatter_tag_files += 1;
                        }
                    }
                }
            }
            if has_title {
                evidence.frontmatter_title_files += 1;
            }
            cursor = end + 1;
        }
    }

    let body = &lines[cursor.min(lines.len())..];
    let facts = scan_body(key, body, title, evidence);

    if let Some(pattern) = KEY_DATE_PATTERNS
        .iter()
        .find(|pattern| parses_as_date(last_segment(key), pattern))
    {
        evidence.key_date_matches.bump((*pattern).to_string());
        if let Some(header) = &facts.title {
            if let Some(display) = DISPLAY_DATE_PATTERNS
                .iter()
                .find(|pattern| parses_as_date(header, pattern))
            {
                evidence.title_date_matches.bump((*display).to_string());
            }
        }
    }

    evidence.key_styles.bump(key_style(last_segment(key)));

    facts
}

fn scan_body(
    key: &str,
    lines: &[&str],
    frontmatter_title: Option<String>,
    evidence: &mut Evidence,
) -> FileFacts {
    let mut header_title = None;
    let mut has_header = false;
    let mut fence: Option<(char, usize)> = None;
    let mut prose = String::new();
    let mut paragraph_lines: Vec<&str> = Vec::new();
    let mut ordered_run: Vec<usize> = Vec::new();
    let mut blank_streak = 0;
    let mut counted_blank_run = false;
    let mut links = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if let Some((fence_char, fence_len)) = fence {
            if is_fence(trimmed)
                .is_some_and(|(character, length)| character == fence_char && length >= fence_len)
            {
                fence = None;
            }
            continue;
        }

        if let Some((fence_char, fence_len)) = is_fence(trimmed) {
            flush_paragraph(&mut paragraph_lines, evidence);
            flush_ordered_run(&mut ordered_run, evidence);
            evidence.code_fence_tokens.bump(fence_char.to_string());
            evidence.code_fence_lengths.bump(fence_len);
            fence = Some((fence_char, fence_len));
            continue;
        }

        if line.trim().is_empty() {
            blank_streak += 1;
            if blank_streak >= 2 && !counted_blank_run {
                evidence.blank_run_files += 1;
                counted_blank_run = true;
            }
            flush_paragraph(&mut paragraph_lines, evidence);
            flush_ordered_run(&mut ordered_run, evidence);
            continue;
        }
        blank_streak = 0;

        if let Some((rule_char, rule_len)) = is_rule(trimmed) {
            let underlines_text = paragraph_lines.last().is_some() && rule_char != '*';
            if rule_char == '=' || (underlines_text && rule_char == '-') {
                evidence.setext_headers += 1;
                if header_title.is_none() {
                    header_title = paragraph_lines.last().map(|line| line.trim().to_string());
                    has_header = true;
                }
                paragraph_lines.clear();
            } else {
                flush_paragraph(&mut paragraph_lines, evidence);
                evidence.rule_tokens.bump(rule_char.to_string());
                evidence.rule_lengths.bump(rule_len);
            }
            flush_ordered_run(&mut ordered_run, evidence);
            continue;
        }

        if trimmed.starts_with('=') && trimmed.chars().all(|character| character == '=') {
            evidence.setext_headers += 1;
            if header_title.is_none() {
                header_title = paragraph_lines.last().map(|line| line.trim().to_string());
                has_header = true;
            }
            paragraph_lines.clear();
            continue;
        }

        collect_links(key, line, &mut links, evidence);

        if trimmed.starts_with('#') {
            let level = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            flush_paragraph(&mut paragraph_lines, evidence);
            flush_ordered_run(&mut ordered_run, evidence);
            has_header = true;
            if level == 1 && header_title.is_none() {
                header_title = Some(trimmed[level..].trim().to_string());
            }
            prose.push_str(trimmed);
            prose.push(' ');
            continue;
        }

        if CALLOUT.is_match(line) {
            evidence.callouts += 1;
        }
        if line.contains("%%") {
            evidence.comment_lines += 1;
        }
        if INLINE_TAG.is_match(line) && !trimmed.starts_with('#') {
            evidence.inline_tags += 1;
        }

        if let Some((token, content_indent)) = bullet_item(trimmed, indent) {
            flush_paragraph(&mut paragraph_lines, evidence);
            flush_ordered_run(&mut ordered_run, evidence);
            evidence.list_tokens.bump(token.to_string());
            if indent == 0 {
                evidence.bullet_indents.bump(content_indent);
            }
            count_inline_tokens(trimmed, evidence);
            prose.push_str(trimmed);
            prose.push(' ');
            continue;
        }

        if let Some((number, token, content_indent)) = ordered_item(trimmed, indent) {
            flush_paragraph(&mut paragraph_lines, evidence);
            evidence.ordered_tokens.bump(token.to_string());
            if indent == 0 {
                evidence.ordered_indents.bump(content_indent);
                ordered_run.push(number);
            }
            count_inline_tokens(trimmed, evidence);
            prose.push_str(trimmed);
            prose.push(' ');
            continue;
        }

        flush_ordered_run(&mut ordered_run, evidence);

        if line.ends_with('\\') {
            let next_is_text = lines
                .get(index + 1)
                .is_some_and(|next| !next.trim().is_empty());
            if next_is_text {
                evidence.hard_break_backslash += 1;
            }
        } else if line.ends_with("  ") {
            let next_is_text = lines
                .get(index + 1)
                .is_some_and(|next| !next.trim().is_empty());
            if next_is_text {
                evidence.hard_break_spaces += 1;
            }
        }

        count_inline_tokens(line, evidence);
        prose.push_str(line);
        prose.push(' ');
        paragraph_lines.push(line);
    }

    flush_paragraph(&mut paragraph_lines, evidence);
    flush_ordered_run(&mut ordered_run, evidence);

    score_language(&prose, evidence);

    let title = frontmatter_title.clone().or_else(|| header_title.clone());
    if frontmatter_title.is_some() && !has_header {
        evidence.frontmatter_title_without_header += 1;
    }
    if has_header {
        evidence.header_files += 1;
    }

    FileFacts {
        key: key.to_string(),
        title,
        links,
    }
}

fn flush_paragraph(paragraph: &mut Vec<&str>, evidence: &mut Evidence) {
    if paragraph.is_empty() {
        return;
    }
    if paragraph.len() == 1 {
        evidence.single_line_paragraphs += 1;
    } else {
        evidence.multiline_paragraphs += 1;
        for line in &paragraph[..paragraph.len() - 1] {
            evidence.wrapped_line_lengths.bump(line.chars().count());
        }
    }
    paragraph.clear();
}

fn flush_ordered_run(run: &mut Vec<usize>, evidence: &mut Evidence) {
    if run.len() >= 2 {
        if run.windows(2).all(|pair| pair[1] == pair[0] + 1) {
            evidence.ordered_incrementing += 1;
        } else if run.iter().all(|number| *number == run[0]) {
            evidence.ordered_flat += 1;
        }
    }
    run.clear();
}

fn is_fence(trimmed: &str) -> Option<(char, usize)> {
    for character in ['`', '~'] {
        let length = trimmed
            .chars()
            .take_while(|candidate| *candidate == character)
            .count();
        if length >= 3 {
            return Some((character, length));
        }
    }
    None
}

fn is_rule(trimmed: &str) -> Option<(char, usize)> {
    for character in ['-', '*', '_', '='] {
        let stripped: String = trimmed
            .chars()
            .filter(|candidate| !candidate.is_whitespace())
            .collect();
        if stripped.len() >= 3 && stripped.chars().all(|candidate| candidate == character) {
            return Some((character, stripped.len()));
        }
    }
    None
}

fn bullet_item(trimmed: &str, indent: usize) -> Option<(char, usize)> {
    let token = trimmed.chars().next()?;
    if !matches!(token, '-' | '*' | '+') {
        return None;
    }
    let rest = &trimmed[1..];
    let spaces = rest
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    if spaces == 0 || rest.trim().is_empty() {
        return None;
    }
    Some((token, indent + 1 + spaces))
}

fn ordered_item(trimmed: &str, indent: usize) -> Option<(usize, char, usize)> {
    let digits: String = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    if digits.is_empty() || digits.len() > 9 {
        return None;
    }
    let token = trimmed[digits.len()..].chars().next()?;
    if !matches!(token, '.' | ')') {
        return None;
    }
    let rest = &trimmed[digits.len() + 1..];
    let spaces = rest
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    if spaces == 0 || rest.trim().is_empty() {
        return None;
    }
    let number = digits.parse().ok()?;
    Some((number, token, indent + digits.len() + 1 + spaces))
}

fn count_inline_tokens(line: &str, evidence: &mut Evidence) {
    let stripped = strip_inline_code(line);

    let double_star = stripped.matches("**").count() / 2;
    let double_underscore = stripped.matches("__").count() / 2;
    if double_star > 0 {
        evidence.strong_tokens.add("**".to_string(), double_star);
    }
    if double_underscore > 0 {
        evidence
            .strong_tokens
            .add("__".to_string(), double_underscore);
    }

    let without_strong = stripped.replace("**", "").replace("__", "");
    let single_star = without_strong.matches('*').count() / 2;
    if single_star > 0 {
        evidence.emphasis_tokens.add("*".to_string(), single_star);
    }
    let boundary_underscores = count_boundary_underscores(&without_strong) / 2;
    if boundary_underscores > 0 {
        evidence
            .emphasis_tokens
            .add("_".to_string(), boundary_underscores);
    }
}

fn count_boundary_underscores(text: &str) -> usize {
    let characters: Vec<char> = text.chars().collect();
    characters
        .iter()
        .enumerate()
        .filter(|(index, character)| {
            **character == '_' && {
                let before = index.checked_sub(1).and_then(|at| characters.get(at));
                let after = characters.get(index + 1);
                let before_word = before.is_some_and(|character| character.is_alphanumeric());
                let after_word = after.is_some_and(|character| character.is_alphanumeric());
                before_word != after_word
            }
        })
        .count()
}

fn strip_inline_code(line: &str) -> String {
    let mut output = String::new();
    let mut inside = false;
    for character in line.chars() {
        if character == '`' {
            inside = !inside;
            continue;
        }
        if !inside {
            output.push(character);
        }
    }
    output
}

fn collect_links(key: &str, line: &str, links: &mut Vec<LinkRef>, evidence: &mut Evidence) {
    let stripped = strip_inline_code(line);

    for capture in WIKI_LINK.captures_iter(&stripped) {
        let target = capture[2].split('|').next().unwrap_or_default().trim();
        let text = capture[2]
            .split_once('|')
            .map(|(_, alias)| alias.trim().to_string())
            .unwrap_or_default();
        if &capture[1] == "!" {
            evidence.embeds += 1;
            continue;
        }
        evidence.wiki_links += 1;
        if target.contains('/') {
            evidence.wiki_pathed += 1;
        } else {
            evidence.wiki_bare += 1;
        }
        links.push(LinkRef {
            from_key: key.to_string(),
            target: target.to_string(),
            text,
            is_wiki: true,
        });
    }

    let without_wiki = WIKI_LINK.replace_all(&stripped, " ");
    for capture in MARKDOWN_LINK.captures_iter(&without_wiki) {
        if &capture[1] == "!" {
            continue;
        }
        let target = capture[3].split_whitespace().next().unwrap_or_default();
        if target.is_empty() || is_external(target) || target.starts_with('#') {
            continue;
        }
        let path = target.split('#').next().unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        let decoded = path.replace("%20", " ");
        if is_asset(&decoded) {
            continue;
        }
        evidence.markdown_links += 1;
        if has_document_extension(&decoded) {
            evidence.refs_with_extension += 1;
        } else {
            evidence.refs_bare += 1;
        }
        links.push(LinkRef {
            from_key: key.to_string(),
            target: decoded,
            text: capture[2].to_string(),
            is_wiki: false,
        });
    }
}

fn is_external(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("ftp://")
        || target.contains("://")
}

fn is_asset(target: &str) -> bool {
    Path::new(target)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| ASSET_EXTENSIONS.contains(&extension.to_lowercase().as_str()))
}

fn has_document_extension(target: &str) -> bool {
    target.ends_with(".md") || target.ends_with(".dj")
}

fn strip_document_extension(target: &str) -> &str {
    target
        .strip_suffix(".md")
        .or_else(|| target.strip_suffix(".dj"))
        .unwrap_or(target)
}

fn resolve_links(facts: &[FileFacts], keys: &BTreeSet<String>, evidence: &mut Evidence) {
    let titles: BTreeMap<&str, &str> = facts
        .iter()
        .filter_map(|fact| {
            fact.title
                .as_deref()
                .map(|title| (fact.key.as_str(), title))
        })
        .collect();

    for fact in facts {
        for link in &fact.links {
            let target = strip_document_extension(&link.target);

            if link.is_wiki {
                let resolved = resolve_wiki(target, keys);
                match resolved {
                    Some(_) => {}
                    None => record_unresolved(&link.from_key, &link.target, evidence),
                }
                continue;
            }

            if target.starts_with('/') {
                let candidate = normalize_path(target.trim_start_matches('/'));
                if keys.contains(&candidate) {
                    evidence.refs_absolute_votes += 1;
                } else {
                    record_unresolved(&link.from_key, &link.target, evidence);
                }
                continue;
            }

            let directory = parent_of(&link.from_key);
            let relative = if directory.is_empty() {
                normalize_path(target)
            } else {
                normalize_path(&format!("{}/{}", directory, target))
            };
            let from_root = normalize_path(target);

            let relative_hit = keys.contains(&relative);
            let root_hit = keys.contains(&from_root);

            if target.contains("../") {
                if relative_hit {
                    evidence.refs_relative_votes += 1;
                } else {
                    record_unresolved(&link.from_key, &link.target, evidence);
                }
                continue;
            }

            match (relative_hit, root_hit, relative == from_root) {
                (_, _, true) if relative_hit => {}
                (true, false, _) => evidence.refs_relative_votes += 1,
                (false, true, _) => {
                    evidence.refs_absolute_votes += 1;
                    evidence.root_relative_links += 1;
                }
                (true, true, _) => {}
                (false, false, _) => record_unresolved(&link.from_key, &link.target, evidence),
            }

            let resolved_key = if relative_hit {
                Some(relative)
            } else if root_hit {
                Some(from_root)
            } else {
                None
            };

            if let (Some(resolved), false) = (resolved_key, link.text.is_empty()) {
                if let Some(title) = titles.get(resolved.as_str()) {
                    if *title == link.text {
                        evidence.refs_text_matching += 1;
                    } else {
                        evidence.refs_text_diverging += 1;
                    }
                }
            }
        }
    }
}

fn resolve_wiki(target: &str, keys: &BTreeSet<String>) -> Option<String> {
    let normalized = normalize_path(target);
    if keys.contains(&normalized) {
        return Some(normalized);
    }
    keys.iter()
        .find(|key| last_segment(key) == last_segment(&normalized))
        .cloned()
}

fn record_unresolved(from_key: &str, target: &str, evidence: &mut Evidence) {
    evidence.unresolved_links += 1;
    if evidence.broken_link_samples.len() < 5 {
        evidence
            .broken_link_samples
            .push(format!("{} → {}", from_key, target));
    }
}

fn parent_of(key: &str) -> &str {
    key.rsplit_once('/').map(|(parent, _)| parent).unwrap_or("")
}

fn last_segment(key: &str) -> &str {
    key.rsplit_once('/').map(|(_, name)| name).unwrap_or(key)
}

fn normalize_path(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

fn check_key_hygiene(keys: &BTreeSet<String>, facts: &[FileFacts], evidence: &mut Evidence) {
    let mut lowercase: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for key in keys {
        lowercase.entry(key.to_lowercase()).or_default().push(key);
        if key.contains(' ') {
            evidence.keys_with_spaces += 1;
        }
    }
    for (_, group) in lowercase {
        if group.len() > 1 {
            evidence.case_collisions.push(
                group
                    .iter()
                    .map(|key| key.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
    }

    let mut by_title: BTreeMap<&str, usize> = BTreeMap::new();
    for fact in facts {
        if let Some(title) = &fact.title {
            *by_title.entry(title.as_str()).or_default() += 1;
        }
    }
    for (title, count) in by_title {
        if count > 1 && evidence.duplicate_titles.len() < 5 {
            evidence
                .duplicate_titles
                .push(format!("{} ({} documents)", title, count));
        }
    }
}

fn parses_as_date(candidate: &str, pattern: &str) -> bool {
    chrono::NaiveDate::parse_from_str(candidate, pattern).is_ok()
}

fn key_style(name: &str) -> String {
    if KEY_DATE_PATTERNS
        .iter()
        .any(|pattern| parses_as_date(name, pattern))
    {
        return "date".to_string();
    }
    if name.len() >= 8 && name.chars().all(|character| character.is_ascii_digit()) {
        return "id".to_string();
    }
    if name.contains(' ') {
        return "title".to_string();
    }
    if name.contains('_') && !name.contains('-') {
        return "snake".to_string();
    }
    "slug".to_string()
}

fn score_language(prose: &str, evidence: &mut Evidence) {
    let lowered = prose.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return;
    }
    for (language, stopwords) in STOPWORDS {
        let hits = words.iter().filter(|word| stopwords.contains(word)).count();
        if hits > 0 {
            evidence.language_scores.add(language.to_string(), hits);
        }
    }
}

fn locale_language() -> Option<String> {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .ok()?;
    let code = raw.split(['_', '.', '-']).next()?.to_lowercase();
    STOPWORDS
        .iter()
        .find(|(language, _)| language.starts_with(&code) || language_code(language) == code)
        .map(|(language, _)| (*language).to_string())
}

fn language_code(language: &str) -> &str {
    match language {
        "english" => "en",
        "german" => "de",
        "french" => "fr",
        "spanish" => "es",
        "italian" => "it",
        "portuguese" => "pt",
        "dutch" => "nl",
        "russian" => "ru",
        "swedish" => "sv",
        "danish" => "da",
        _ => "",
    }
}
