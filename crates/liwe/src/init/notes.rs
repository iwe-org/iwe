use std::collections::{BTreeMap, BTreeSet};
use std::fs::{read_dir, read_to_string};
use std::path::Path;
use std::sync::LazyLock;

use crate::fs::walk_md_paths;
use crate::model::config::Format;
use regex::Regex;
use serde::Serialize;

use crate::init::evidence::Evidence;

pub const CLUSTER_SHARE: usize = 90;
pub const CANDIDATE_SHARE: usize = 10;

pub const CANDIDATE_NAMES: [&str; 6] = ["docs", "kb", "notes", "wiki", "doc", "knowledge"];

pub const NEW_FOLDER_NAMES: [&str; 3] = ["docs", "kb", "notes"];

pub const PROJECT_MARKERS: [&str; 5] = [
    "Cargo.toml",
    "package.json",
    "Package.swift",
    "pyproject.toml",
    "go.mod",
];

pub const SOURCE_FOLDERS: [&str; 1] = ["src"];

pub const SOURCE_EXTENSIONS: [&str; 24] = [
    "rs", "swift", "js", "mjs", "ts", "tsx", "jsx", "py", "go", "java", "kt", "c", "h", "cc",
    "cpp", "hpp", "cs", "rb", "php", "scala", "m", "mm", "sh", "zig",
];

pub const BUILD_FILES: [&str; 8] = [
    "Makefile",
    "CMakeLists.txt",
    "Gemfile",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "setup.py",
    "requirements.txt",
];

pub const GENERATED_FOLDERS: [&str; 8] = [
    "site", "_site", "public", "build", "dist", "out", "target", ".next",
];

pub const DEPENDENCY_FOLDERS: [&str; 5] =
    ["node_modules", "vendor", "Pods", "third_party", ".venv"];

pub const FURNITURE_FOLDERS: [&str; 2] = [".github", ".gitlab"];

pub const TEST_DATA_FOLDERS: [&str; 7] = [
    "test",
    "tests",
    "fixtures",
    "__snapshots__",
    "testdata",
    "examples",
    "samples",
];

pub const META_FILES: [&str; 8] = [
    "readme",
    "agents",
    "claude",
    "contributing",
    "changelog",
    "license",
    "security",
    "code_of_conduct",
];

const PAGES_MARKERS: [&str; 3] = ["index.html", "CNAME", ".nojekyll"];

static MKDOCS_DOCS_DIR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^docs_dir:\s*['"]?([^'"\s#]+)"#).expect("valid docs_dir pattern")
});

static MDBOOK_SRC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*src\s*=\s*['"]([^'"]+)['"]"#).expect("valid src pattern")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SiteGenerator {
    MkDocs,
    Docusaurus,
    VitePress,
    MdBook,
    Sphinx,
    Jekyll,
    Hugo,
    Astro,
    Antora,
    Pages,
}

impl SiteGenerator {
    pub fn name(&self) -> &'static str {
        match self {
            SiteGenerator::MkDocs => "MkDocs",
            SiteGenerator::Docusaurus => "Docusaurus",
            SiteGenerator::VitePress => "VitePress",
            SiteGenerator::MdBook => "mdBook",
            SiteGenerator::Sphinx => "Sphinx",
            SiteGenerator::Jekyll => "Jekyll",
            SiteGenerator::Hugo => "Hugo",
            SiteGenerator::Astro => "Astro",
            SiteGenerator::Antora => "Antora",
            SiteGenerator::Pages => "GitHub Pages",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "generator")]
pub enum ExclusionKind {
    Site(SiteGenerator),
    Generated,
    Dependencies,
    Furniture,
    TestData,
}

impl ExclusionKind {
    pub fn describe(&self) -> String {
        match self {
            ExclusionKind::Site(generator) => {
                format!("a published documentation site ({})", generator.name())
            }
            ExclusionKind::Generated => "generated output".to_string(),
            ExclusionKind::Dependencies => "dependencies".to_string(),
            ExclusionKind::Furniture => "repository furniture".to_string(),
            ExclusionKind::TestData => "test data and samples".to_string(),
        }
    }

    pub fn is_site(&self) -> bool {
        matches!(self, ExclusionKind::Site(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExcludedFolder {
    pub path: String,
    pub kind: ExclusionKind,
    pub files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CandidateReason {
    Cluster,
    ConventionalName,
    Vault,
    Share,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotesCandidate {
    pub path: String,
    pub files: usize,
    pub share: usize,
    pub reasons: Vec<CandidateReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NotesConfidence {
    #[default]
    Strong,
    Weak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NotesFolderKind {
    Root,
    Existing,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProposalReason {
    NotACodeProject,
    Vault,
    Cluster,
    NoNotes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotesFolder {
    pub path: String,
    pub kind: NotesFolderKind,
    pub reason: ProposalReason,
    pub files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct NotesProposal {
    pub candidates: Vec<NotesCandidate>,
    pub proposal: Option<NotesFolder>,
    pub confidence: NotesConfidence,
    pub excluded: Vec<ExcludedFolder>,
    pub code_project: bool,
    pub markdown_files: usize,
    pub remaining_files: usize,
    pub cluster: Option<String>,
}

impl NotesProposal {
    pub fn site_excluded(&self) -> bool {
        self.excluded.iter().any(|folder| folder.kind.is_site())
    }

    pub fn cli_workspace_path(&self) -> String {
        match &self.proposal {
            Some(folder) if folder.kind == NotesFolderKind::Existing => folder.path.clone(),
            Some(folder) if folder.kind == NotesFolderKind::Root => {
                self.cluster.clone().unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    pub fn evidence_line(&self) -> String {
        let folder = match &self.proposal {
            Some(folder) => folder,
            None => return format!("{} candidates, none clear", self.candidates.len()),
        };
        match folder.reason {
            ProposalReason::NotACodeProject => "the whole folder — a folder of notes".to_string(),
            ProposalReason::Vault if folder.path.is_empty() => {
                "the whole folder — an Obsidian vault".to_string()
            }
            ProposalReason::Vault => format!("{}/ — an Obsidian vault", folder.path),
            ProposalReason::Cluster => format!(
                "{}/ — {} of {} markdown files are here",
                folder.path, folder.files, self.remaining_files
            ),
            ProposalReason::NoNotes => match self.excluded.iter().find(|f| f.kind.is_site()) {
                Some(site) if site.path.is_empty() => format!(
                    "{}/ — new folder; the folder is a published site",
                    folder.path
                ),
                Some(site) => format!(
                    "{}/ — new folder; {}/ is the published site",
                    folder.path, site.path
                ),
                None => format!("{}/ — new folder", folder.path),
            },
        }
    }
}

pub fn notes_candidates(evidence: &Evidence) -> NotesProposal {
    propose(&evidence.root, &evidence.markdown_keys)
}

pub fn scan_notes(root: &Path) -> NotesProposal {
    let mut keys: Vec<String> = walk_md_paths(root, Format::Markdown)
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    keys.sort();
    propose(root, &keys)
}

pub fn propose(root: &Path, keys: &[String]) -> NotesProposal {
    let code_project = is_code_project(root);
    let sites = site_folders(root);

    let mut excluded: BTreeMap<String, ExcludedFolder> = BTreeMap::new();
    for (path, generator) in &sites {
        excluded.insert(
            path.clone(),
            ExcludedFolder {
                path: path.clone(),
                kind: ExclusionKind::Site(*generator),
                files: 0,
            },
        );
    }

    let mut remaining: Vec<&str> = Vec::new();
    for key in keys {
        if is_meta_file(key) {
            continue;
        }
        if let Some((path, kind)) = exclusion_of(key, &sites) {
            excluded
                .entry(path.clone())
                .or_insert(ExcludedFolder {
                    path,
                    kind,
                    files: 0,
                })
                .files += 1;
            continue;
        }
        remaining.push(key);
    }

    let excluded: Vec<ExcludedFolder> = excluded
        .into_values()
        .filter(|folder| folder.kind.is_site() || folder.files > 0)
        .collect();

    let vaults = vault_folders(root);
    let cluster = cluster_of(&remaining);
    let candidates = candidates_of(&remaining, cluster.as_deref(), &vaults);

    let mut proposal = NotesProposal {
        candidates,
        proposal: None,
        confidence: NotesConfidence::Weak,
        excluded,
        code_project,
        markdown_files: keys.len(),
        remaining_files: remaining.len(),
        cluster: cluster.clone(),
    };

    let root_is_site = sites.iter().any(|(path, _)| path.is_empty());
    let decided = if root.join(".obsidian").is_dir() {
        Some(NotesFolder {
            path: String::new(),
            kind: NotesFolderKind::Root,
            reason: ProposalReason::Vault,
            files: keys.len(),
        })
    } else if !code_project && !root_is_site {
        Some(NotesFolder {
            path: String::new(),
            kind: NotesFolderKind::Root,
            reason: ProposalReason::NotACodeProject,
            files: keys.len(),
        })
    } else if let Some(vault) = vaults.first() {
        Some(NotesFolder {
            path: vault.clone(),
            kind: NotesFolderKind::Existing,
            reason: ProposalReason::Vault,
            files: files_under(&remaining, vault),
        })
    } else if let Some(cluster) = cluster {
        Some(NotesFolder {
            files: files_under(&remaining, &cluster),
            path: cluster,
            kind: NotesFolderKind::Existing,
            reason: ProposalReason::Cluster,
        })
    } else if remaining.is_empty() {
        first_free_folder(root).map(|path| NotesFolder {
            path,
            kind: NotesFolderKind::New,
            reason: ProposalReason::NoNotes,
            files: 0,
        })
    } else {
        None
    };

    if decided.is_some() {
        proposal.confidence = NotesConfidence::Strong;
    }
    proposal.proposal = decided;
    proposal
}

pub fn is_code_project(root: &Path) -> bool {
    if !project_markers(root).is_empty() {
        return true;
    }
    root.join(".git").exists() && has_source_code(root)
}

pub fn has_source_code(root: &Path) -> bool {
    if SOURCE_FOLDERS
        .iter()
        .any(|folder| root.join(folder).is_dir())
        || BUILD_FILES.iter().any(|name| root.join(name).is_file())
    {
        return true;
    }
    top_level_entries(root).iter().any(|name| {
        Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension))
            && root.join(name).is_file()
    })
}

pub fn project_markers(root: &Path) -> Vec<String> {
    let mut markers: Vec<String> = PROJECT_MARKERS
        .iter()
        .filter(|marker| root.join(marker).is_file())
        .map(|marker| marker.to_string())
        .collect();
    markers.extend(
        top_level_entries(root)
            .into_iter()
            .filter(|name| name.ends_with(".xcodeproj")),
    );
    markers
}

pub fn is_free(root: &Path, folder: &str) -> bool {
    let path = root.join(folder);
    if !path.exists() {
        return true;
    }
    path.is_dir()
        && read_dir(&path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
}

pub fn first_free_folder(root: &Path) -> Option<String> {
    NEW_FOLDER_NAMES
        .iter()
        .find(|name| is_free(root, name))
        .map(|name| name.to_string())
}

pub fn is_meta_file(key: &str) -> bool {
    let stem = last_segment(key).to_lowercase();
    if META_FILES.contains(&stem.as_str()) {
        return true;
    }
    !key.contains('/') && stem == "index"
}

pub fn exclusion_of(
    key: &str,
    sites: &[(String, SiteGenerator)],
) -> Option<(String, ExclusionKind)> {
    for (site, generator) in sites {
        if site.is_empty() || key.starts_with(&format!("{}/", site)) {
            return Some((site.clone(), ExclusionKind::Site(*generator)));
        }
    }

    let segments: Vec<&str> = key.split('/').collect();
    let folders = &segments[..segments.len().saturating_sub(1)];
    for (index, segment) in folders.iter().enumerate() {
        let kind = if GENERATED_FOLDERS.contains(segment) {
            ExclusionKind::Generated
        } else if DEPENDENCY_FOLDERS.contains(segment) {
            ExclusionKind::Dependencies
        } else if FURNITURE_FOLDERS.contains(segment) {
            ExclusionKind::Furniture
        } else if TEST_DATA_FOLDERS.contains(segment) {
            ExclusionKind::TestData
        } else {
            continue;
        };
        return Some((folders[..=index].join("/"), kind));
    }
    None
}

pub fn site_folders(root: &Path) -> Vec<(String, SiteGenerator)> {
    let mut sites: BTreeMap<String, SiteGenerator> = BTreeMap::new();

    for (folder, generator) in root_claims(root) {
        if root.join(&folder).is_dir() {
            sites.entry(folder).or_insert(generator);
        }
    }

    for folder in top_level_folders(root) {
        if sites.contains_key(&folder) {
            continue;
        }
        if let Some(generator) = folder_generator(&root.join(&folder)) {
            sites.insert(folder, generator);
        }
    }

    sites.into_iter().collect()
}

fn root_claims(root: &Path) -> Vec<(String, SiteGenerator)> {
    let mut claims = Vec::new();

    for name in ["mkdocs.yml", "mkdocs.yaml"] {
        if let Ok(content) = read_to_string(root.join(name)) {
            let folder = MKDOCS_DOCS_DIR
                .captures(&content)
                .map(|captures| captures[1].trim_end_matches('/').to_string())
                .unwrap_or_else(|| "docs".to_string());
            claims.push((normalize(&folder), SiteGenerator::MkDocs));
        }
    }

    if has_any_file(root, &docusaurus_configs()) {
        claims.push(("docs".to_string(), SiteGenerator::Docusaurus));
        claims.push(("blog".to_string(), SiteGenerator::Docusaurus));
    }

    if let Ok(content) = read_to_string(root.join("book.toml")) {
        let folder = MDBOOK_SRC
            .captures(&content)
            .map(|captures| captures[1].trim_end_matches('/').to_string())
            .unwrap_or_else(|| "src".to_string());
        claims.push((normalize(&folder), SiteGenerator::MdBook));
    }

    if is_hugo(root) {
        claims.push(("content".to_string(), SiteGenerator::Hugo));
    }

    if has_any_file(root, &astro_configs()) {
        claims.push(("src/content/docs".to_string(), SiteGenerator::Astro));
    }

    if root.join("antora.yml").is_file() {
        claims.push(("modules".to_string(), SiteGenerator::Antora));
    }

    if root.join(".vitepress").is_dir() {
        claims.push((String::new(), SiteGenerator::VitePress));
    }

    if root.join("conf.py").is_file() {
        claims.push((String::new(), SiteGenerator::Sphinx));
    } else if root.join("source").join("conf.py").is_file() {
        claims.push(("source".to_string(), SiteGenerator::Sphinx));
    }

    if root.join("_config.yml").is_file() {
        claims.push((String::new(), SiteGenerator::Jekyll));
    }

    if root.join("CNAME").is_file()
        || root.join(".nojekyll").is_file()
        || root.join("index.html").is_file() && project_markers(root).is_empty()
    {
        claims.push((String::new(), SiteGenerator::Pages));
    }

    claims
}

fn folder_generator(folder: &Path) -> Option<SiteGenerator> {
    if folder.join("mkdocs.yml").is_file() || folder.join("mkdocs.yaml").is_file() {
        return Some(SiteGenerator::MkDocs);
    }
    if has_any_file(folder, &docusaurus_configs()) {
        return Some(SiteGenerator::Docusaurus);
    }
    if folder.join(".vitepress").is_dir() {
        return Some(SiteGenerator::VitePress);
    }
    if folder.join("book.toml").is_file() {
        return Some(SiteGenerator::MdBook);
    }
    if folder.join("conf.py").is_file() || folder.join("source").join("conf.py").is_file() {
        return Some(SiteGenerator::Sphinx);
    }
    if folder.join("_config.yml").is_file() {
        return Some(SiteGenerator::Jekyll);
    }
    if is_hugo(folder) {
        return Some(SiteGenerator::Hugo);
    }
    if has_any_file(folder, &astro_configs()) && folder.join("src/content/docs").is_dir() {
        return Some(SiteGenerator::Astro);
    }
    if folder.join("antora.yml").is_file() {
        return Some(SiteGenerator::Antora);
    }
    if PAGES_MARKERS
        .iter()
        .any(|marker| folder.join(marker).is_file())
    {
        return Some(SiteGenerator::Pages);
    }
    None
}

fn is_hugo(folder: &Path) -> bool {
    let has_config = ["hugo.toml", "hugo.yaml", "hugo.yml", "hugo.json"]
        .iter()
        .any(|name| folder.join(name).is_file())
        || folder.join("config.toml").is_file() && folder.join("archetypes").is_dir();
    has_config && folder.join("content").is_dir()
}

fn docusaurus_configs() -> Vec<String> {
    ["js", "ts", "mjs", "cjs", "mts"]
        .iter()
        .map(|extension| format!("docusaurus.config.{}", extension))
        .collect()
}

fn astro_configs() -> Vec<String> {
    ["mjs", "js", "ts", "mts", "cjs"]
        .iter()
        .map(|extension| format!("astro.config.{}", extension))
        .collect()
}

fn has_any_file(folder: &Path, names: &[String]) -> bool {
    names.iter().any(|name| folder.join(name).is_file())
}

fn vault_folders(root: &Path) -> Vec<String> {
    top_level_folders(root)
        .into_iter()
        .filter(|folder| root.join(folder).join(".obsidian").is_dir())
        .collect()
}

fn cluster_of(remaining: &[&str]) -> Option<String> {
    if remaining.is_empty() {
        return None;
    }

    let mut children: BTreeMap<&str, usize> = BTreeMap::new();
    for key in remaining {
        if let Some((head, _)) = key.split_once('/') {
            *children.entry(head).or_default() += 1;
        }
    }

    let qualifying: Vec<&str> = children
        .iter()
        .filter(|(_, count)| **count * 100 >= remaining.len() * CLUSTER_SHARE)
        .map(|(head, _)| *head)
        .collect();

    match qualifying.as_slice() {
        [head] => Some(head.to_string()),
        _ => None,
    }
}

fn candidates_of(
    remaining: &[&str],
    cluster: Option<&str>,
    vaults: &[String],
) -> Vec<NotesCandidate> {
    let mut reasons: BTreeMap<String, BTreeSet<CandidateReason>> = BTreeMap::new();

    if let Some(cluster) = cluster {
        reasons
            .entry(cluster.to_string())
            .or_default()
            .insert(CandidateReason::Cluster);
    }
    for vault in vaults {
        reasons
            .entry(vault.clone())
            .or_default()
            .insert(CandidateReason::Vault);
    }

    let mut top_level: BTreeMap<&str, usize> = BTreeMap::new();
    for key in remaining {
        if let Some((head, _)) = key.split_once('/') {
            *top_level.entry(head).or_default() += 1;
        }
    }
    for (folder, count) in &top_level {
        if CANDIDATE_NAMES.contains(folder) {
            reasons
                .entry(folder.to_string())
                .or_default()
                .insert(CandidateReason::ConventionalName);
        }
        if count * 100 >= remaining.len() * CANDIDATE_SHARE {
            reasons
                .entry(folder.to_string())
                .or_default()
                .insert(CandidateReason::Share);
        }
    }

    let mut candidates: Vec<NotesCandidate> = reasons
        .into_iter()
        .map(|(path, reasons)| {
            let files = files_under(remaining, &path);
            NotesCandidate {
                share: share(files, remaining.len()),
                files,
                path,
                reasons: reasons.into_iter().collect(),
            }
        })
        .collect();
    candidates.sort_by(|left, right| {
        right
            .files
            .cmp(&left.files)
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates
}

fn files_under(remaining: &[&str], folder: &str) -> usize {
    let prefix = format!("{}/", folder);
    remaining
        .iter()
        .filter(|key| key.starts_with(&prefix))
        .count()
}

fn share(part: usize, total: usize) -> usize {
    match total {
        0 => 0,
        _ => part * 100 / total,
    }
}

fn top_level_entries(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = read_dir(root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn top_level_folders(root: &Path) -> Vec<String> {
    top_level_entries(root)
        .into_iter()
        .filter(|name| !name.starts_with('.') && root.join(name).is_dir())
        .collect()
}

fn last_segment(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

fn normalize(folder: &str) -> String {
    folder
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests;
