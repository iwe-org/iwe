use std::fs::{create_dir_all, write};
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;
use crate::init::evidence::scan;

fn file(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
    write(path, content).expect("write file");
}

fn folder(root: &Path, relative: &str) {
    create_dir_all(root.join(relative)).expect("create folder");
}

fn notes(root: &Path, folder: &str, count: usize) {
    for index in 0..count {
        file(
            root,
            &format!("{}/note-{}.md", folder, index),
            &format!("# Note {}\n", index),
        );
    }
}

fn repository() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    file(temp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
    temp
}

fn proposal_of(root: &Path) -> NotesProposal {
    scan(root).notes
}

fn proposed_path(root: &Path) -> Option<(String, NotesFolderKind)> {
    proposal_of(root)
        .proposal
        .map(|folder| (folder.path, folder.kind))
}

fn excluded_kind(root: &Path, path: &str) -> Option<ExclusionKind> {
    proposal_of(root)
        .excluded
        .into_iter()
        .find(|folder| folder.path == path)
        .map(|folder| folder.kind)
}

#[test]
fn a_folder_that_is_not_a_code_project_proposes_itself() {
    let temp = TempDir::new().expect("temp dir");
    notes(temp.path(), "journal", 3);
    notes(temp.path(), "ideas", 3);

    let proposal = proposal_of(temp.path());

    assert_eq!(false, proposal.code_project);
    assert_eq!(NotesConfidence::Strong, proposal.confidence);
    assert_eq!(
        Some((String::new(), NotesFolderKind::Root)),
        proposal.proposal.map(|folder| (folder.path, folder.kind))
    );
}

#[test]
fn a_subfolder_with_an_obsidian_vault_is_proposed() {
    let temp = repository();
    notes(temp.path(), "vault", 2);
    folder(temp.path(), "vault/.obsidian");
    notes(temp.path(), "notes", 10);

    let proposal = proposal_of(temp.path());

    assert_eq!(NotesConfidence::Strong, proposal.confidence);
    let folder = proposal.proposal.expect("a proposal");
    assert_eq!("vault", folder.path);
    assert_eq!(ProposalReason::Vault, folder.reason);
}

#[test]
fn one_candidate_holding_ninety_percent_is_proposed() {
    let temp = repository();
    notes(temp.path(), "notes", 19);
    file(temp.path(), "design.md", "# Design\n");

    let proposal = proposal_of(temp.path());

    assert_eq!(NotesConfidence::Strong, proposal.confidence);
    let folder = proposal.proposal.expect("a proposal");
    assert_eq!("notes", folder.path);
    assert_eq!(ProposalReason::Cluster, folder.reason);
    assert_eq!(19, folder.files);
}

#[test]
fn the_cluster_is_the_shallowest_qualifying_folder() {
    let temp = repository();
    notes(temp.path(), "knowledge/notes", 10);

    assert_eq!(
        Some(("knowledge".to_string(), NotesFolderKind::Existing)),
        proposed_path(temp.path())
    );
}

#[test]
fn an_obsidian_vault_at_the_root_is_the_whole_folder_even_in_git() {
    let temp = TempDir::new().expect("temp dir");
    folder(temp.path(), ".git");
    folder(temp.path(), ".obsidian");
    notes(temp.path(), "daily", 4);
    notes(temp.path(), "projects", 4);

    let proposal = proposal_of(temp.path());

    assert_eq!(NotesConfidence::Strong, proposal.confidence);
    let folder = proposal.proposal.clone().expect("a proposal");
    assert_eq!("", folder.path);
    assert_eq!(NotesFolderKind::Root, folder.kind);
    assert_eq!(ProposalReason::Vault, folder.reason);
    assert_eq!(
        "the whole folder — an Obsidian vault",
        proposal.evidence_line()
    );
}

#[test]
fn a_git_tracked_folder_of_notes_is_the_whole_folder() {
    let temp = TempDir::new().expect("temp dir");
    folder(temp.path(), ".git");
    notes(temp.path(), "daily", 4);
    notes(temp.path(), "projects", 4);

    assert_eq!(
        Some((String::new(), NotesFolderKind::Root)),
        proposed_path(temp.path())
    );
}

#[test]
fn a_jekyll_site_at_the_root_claims_the_whole_folder() {
    let temp = TempDir::new().expect("temp dir");
    file(temp.path(), "_config.yml", "title: Blog\n");
    notes(temp.path(), "_posts", 3);

    let proposal = proposal_of(temp.path());

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Jekyll)),
        excluded_kind(temp.path(), "")
    );
    let folder = proposal.proposal.clone().expect("a proposal");
    assert_eq!(NotesFolderKind::New, folder.kind);
    assert_eq!(
        "docs/ — new folder; the folder is a published site",
        proposal.evidence_line()
    );
}

#[test]
fn root_level_site_generators_claim_the_root() {
    for (marker, generator) in [
        ("conf.py", SiteGenerator::Sphinx),
        (".vitepress/config.mts", SiteGenerator::VitePress),
        ("CNAME", SiteGenerator::Pages),
        ("index.html", SiteGenerator::Pages),
    ] {
        let temp = TempDir::new().expect("temp dir");
        file(temp.path(), marker, "");
        notes(temp.path(), "guide", 2);

        assert_eq!(
            Some(ExclusionKind::Site(generator)),
            excluded_kind(temp.path(), ""),
            "{marker}"
        );
    }
}

#[test]
fn a_root_index_html_in_a_code_project_is_not_a_site() {
    let temp = repository();
    file(temp.path(), "index.html", "<html></html>");
    notes(temp.path(), "notes", 3);

    assert_eq!(None, excluded_kind(temp.path(), ""));
}

#[test]
fn a_root_sphinx_source_folder_is_claimed() {
    let temp = repository();
    file(temp.path(), "source/conf.py", "project = 'demo'\n");
    notes(temp.path(), "source", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Sphinx)),
        excluded_kind(temp.path(), "source")
    );
}

#[test]
fn no_markdown_after_exclusions_proposes_a_new_docs_folder() {
    let temp = repository();
    file(temp.path(), "README.md", "# Demo\n");

    let proposal = proposal_of(temp.path());

    assert_eq!(NotesConfidence::Strong, proposal.confidence);
    let folder = proposal.proposal.expect("a proposal");
    assert_eq!("docs", folder.path);
    assert_eq!(NotesFolderKind::New, folder.kind);
    assert_eq!(ProposalReason::NoNotes, folder.reason);
}

#[test]
fn an_empty_docs_folder_is_free() {
    let temp = repository();
    folder(temp.path(), "docs");

    assert_eq!(
        Some(("docs".to_string(), NotesFolderKind::New)),
        proposed_path(temp.path())
    );
}

#[test]
fn two_candidates_are_weak_and_leave_no_proposal() {
    let temp = repository();
    notes(temp.path(), "notes", 6);
    notes(temp.path(), "design", 4);

    let proposal = proposal_of(temp.path());

    assert_eq!(NotesConfidence::Weak, proposal.confidence);
    assert_eq!(None, proposal.proposal);
    let paths: Vec<String> = proposal
        .candidates
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect();
    assert_eq!(vec!["notes".to_string(), "design".to_string()], paths);
    assert_eq!(
        vec![CandidateReason::ConventionalName, CandidateReason::Share],
        proposal.candidates[0].reasons
    );
    assert_eq!(60, proposal.candidates[0].share);
}

#[test]
fn markdown_spread_with_no_candidate_over_ninety_percent_is_weak() {
    let temp = repository();
    file(temp.path(), "plan.md", "# Plan\n");
    file(temp.path(), "ideas.md", "# Ideas\n");
    notes(temp.path(), "src/module", 1);

    let proposal = proposal_of(temp.path());

    assert_eq!(NotesConfidence::Weak, proposal.confidence);
    assert_eq!(None, proposal.proposal);
}

#[test]
fn a_docs_site_next_to_a_free_kb_proposes_kb() {
    let temp = repository();
    file(temp.path(), "mkdocs.yml", "site_name: Demo\n");
    notes(temp.path(), "docs", 12);

    let proposal = proposal_of(temp.path());

    let folder = proposal.proposal.clone().expect("a proposal");
    assert_eq!("kb", folder.path);
    assert_eq!(NotesFolderKind::New, folder.kind);
    assert_eq!(true, proposal.site_excluded());
    assert_eq!(
        "kb/ — new folder; docs/ is the published site",
        proposal.evidence_line()
    );
}

#[test]
fn a_docs_site_with_notes_beside_it_proposes_the_notes() {
    let temp = repository();
    file(temp.path(), "mkdocs.yml", "site_name: Demo\n");
    notes(temp.path(), "docs", 12);
    notes(temp.path(), "kb", 3);

    assert_eq!(
        Some(("kb".to_string(), NotesFolderKind::Existing)),
        proposed_path(temp.path())
    );
}

#[test]
fn mkdocs_without_docs_dir_claims_docs() {
    let temp = repository();
    file(temp.path(), "mkdocs.yml", "site_name: Demo\n");
    notes(temp.path(), "docs", 1);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::MkDocs)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn mkdocs_with_docs_dir_claims_that_folder() {
    let temp = repository();
    file(
        temp.path(),
        "mkdocs.yml",
        "site_name: Demo\ndocs_dir: 'handbook'\n",
    );
    notes(temp.path(), "handbook", 2);
    notes(temp.path(), "docs", 4);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::MkDocs)),
        excluded_kind(temp.path(), "handbook")
    );
    assert_eq!(None, excluded_kind(temp.path(), "docs"));
    assert_eq!(
        Some(("docs".to_string(), NotesFolderKind::Existing)),
        proposed_path(temp.path())
    );
}

#[test]
fn docusaurus_claims_docs() {
    let temp = repository();
    file(temp.path(), "docusaurus.config.ts", "export default {};\n");
    notes(temp.path(), "docs", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Docusaurus)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn docusaurus_in_a_website_folder_claims_that_folder() {
    let temp = repository();
    file(
        temp.path(),
        "website/docusaurus.config.js",
        "module.exports = {};\n",
    );
    notes(temp.path(), "website/docs", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Docusaurus)),
        excluded_kind(temp.path(), "website")
    );
}

#[test]
fn vitepress_claims_its_source_folder() {
    let temp = repository();
    file(
        temp.path(),
        "docs/.vitepress/config.mts",
        "export default {}\n",
    );
    notes(temp.path(), "docs", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::VitePress)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn mdbook_claims_its_src_folder() {
    let temp = repository();
    file(temp.path(), "book.toml", "[book]\nsrc = \"book\"\n");
    notes(temp.path(), "book", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::MdBook)),
        excluded_kind(temp.path(), "book")
    );
}

#[test]
fn mdbook_in_a_docs_folder_claims_that_folder() {
    let temp = repository();
    file(temp.path(), "docs/book.toml", "[book]\n");
    notes(temp.path(), "docs/src", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::MdBook)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn sphinx_claims_the_folder_holding_conf_py() {
    let temp = repository();
    file(temp.path(), "docs/source/conf.py", "project = 'demo'\n");
    notes(temp.path(), "docs/source", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Sphinx)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn jekyll_claims_the_folder_holding_its_config() {
    let temp = repository();
    file(temp.path(), "docs/_config.yml", "title: Demo\n");
    notes(temp.path(), "docs", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Jekyll)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn hugo_claims_its_content_folder() {
    let temp = repository();
    file(temp.path(), "hugo.toml", "title = 'Demo'\n");
    notes(temp.path(), "content", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Hugo)),
        excluded_kind(temp.path(), "content")
    );
}

#[test]
fn astro_claims_its_content_docs_folder() {
    let temp = repository();
    file(temp.path(), "astro.config.mjs", "export default {}\n");
    notes(temp.path(), "src/content/docs", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Astro)),
        excluded_kind(temp.path(), "src/content/docs")
    );
}

#[test]
fn antora_claims_the_folder_holding_its_component() {
    let temp = repository();
    file(temp.path(), "docs/antora.yml", "name: demo\n");
    notes(temp.path(), "docs/modules/ROOT/pages", 2);

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Antora)),
        excluded_kind(temp.path(), "docs")
    );
}

#[test]
fn a_docs_folder_holding_only_index_html_is_a_pages_site() {
    let temp = repository();
    file(temp.path(), "docs/index.html", "<html></html>\n");

    let proposal = proposal_of(temp.path());

    assert_eq!(
        Some(ExclusionKind::Site(SiteGenerator::Pages)),
        excluded_kind(temp.path(), "docs")
    );
    assert_eq!(
        Some(("kb".to_string(), NotesFolderKind::New)),
        proposal.proposal.map(|folder| (folder.path, folder.kind))
    );
}

#[test]
fn generated_output_is_excluded() {
    let temp = repository();
    notes(temp.path(), "dist", 5);
    notes(temp.path(), "notes", 2);

    assert_eq!(
        Some(ExclusionKind::Generated),
        excluded_kind(temp.path(), "dist")
    );
    assert_eq!(
        Some(("notes".to_string(), NotesFolderKind::Existing)),
        proposed_path(temp.path())
    );
}

#[test]
fn gitignored_folders_are_never_seen() {
    let temp = repository();
    folder(temp.path(), ".git");
    file(temp.path(), ".gitignore", "generated/\n");
    notes(temp.path(), "generated", 5);
    notes(temp.path(), "notes", 2);

    let proposal = proposal_of(temp.path());

    assert_eq!(2, proposal.markdown_files);
    assert_eq!(
        Some(("notes".to_string(), NotesFolderKind::Existing)),
        proposal.proposal.map(|folder| (folder.path, folder.kind))
    );
}

#[test]
fn dependencies_are_excluded() {
    let temp = repository();
    notes(temp.path(), "vendor/lib", 5);

    assert_eq!(
        Some(ExclusionKind::Dependencies),
        excluded_kind(temp.path(), "vendor")
    );
}

#[test]
fn repository_furniture_is_excluded() {
    let key = ".github/ISSUE_TEMPLATE/bug";

    assert_eq!(
        Some((".github".to_string(), ExclusionKind::Furniture)),
        exclusion_of(key, &[])
    );
}

#[test]
fn test_data_is_excluded_at_any_depth() {
    let temp = repository();
    notes(temp.path(), "crates/core/tests/fixtures", 4);

    assert_eq!(
        Some(ExclusionKind::TestData),
        excluded_kind(temp.path(), "crates/core/tests")
    );
}

#[test]
fn meta_files_are_ignored_at_any_level() {
    let temp = repository();
    file(temp.path(), "CHANGELOG.md", "# Changes\n");
    file(temp.path(), "packages/web/README.md", "# Web\n");
    file(
        temp.path(),
        "packages/api/CONTRIBUTING.md",
        "# Contributing\n",
    );

    let proposal = proposal_of(temp.path());

    assert_eq!(0, proposal.remaining_files);
    assert_eq!(
        Some(("docs".to_string(), NotesFolderKind::New)),
        proposal.proposal.map(|folder| (folder.path, folder.kind))
    );
}

#[test]
fn a_git_folder_with_source_code_makes_a_code_project() {
    for source in ["main.rs", "src/lib.txt", "Makefile"] {
        let temp = TempDir::new().expect("temp dir");
        folder(temp.path(), ".git");
        file(temp.path(), source, "");

        assert_eq!(true, is_code_project(temp.path()), "{source}");
    }
}

#[test]
fn a_git_folder_holding_only_notes_is_not_a_code_project() {
    let temp = TempDir::new().expect("temp dir");
    folder(temp.path(), ".git");
    notes(temp.path(), "notes", 2);
    file(temp.path(), "README.md", "# Notes\n");

    assert_eq!(false, is_code_project(temp.path()));
}

#[test]
fn an_xcode_project_makes_a_code_project() {
    let temp = TempDir::new().expect("temp dir");
    folder(temp.path(), "Demo.xcodeproj");

    assert_eq!(true, is_code_project(temp.path()));
    assert_eq!(
        vec!["Demo.xcodeproj".to_string()],
        project_markers(temp.path())
    );
}

#[test]
fn the_cli_keeps_the_cluster_of_a_plain_folder() {
    let temp = TempDir::new().expect("temp dir");
    notes(temp.path(), "notes", 4);

    assert_eq!("notes", proposal_of(temp.path()).cli_workspace_path());
}

#[test]
fn the_cli_leaves_the_path_unset_when_weak() {
    let temp = repository();
    notes(temp.path(), "notes", 6);
    notes(temp.path(), "design", 4);

    assert_eq!("", proposal_of(temp.path()).cli_workspace_path());
}

#[test]
fn scan_notes_matches_the_evidence_scan() {
    let temp = repository();
    notes(temp.path(), "notes", 6);
    notes(temp.path(), "design", 4);

    assert_eq!(proposal_of(temp.path()), scan_notes(temp.path()));
}
