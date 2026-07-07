#![cfg(windows)]

use std::fs;
use std::sync::Once;

use pretty_assertions::assert_str_eq;
use tempfile::TempDir;

use liwe::graph::Graph;
use liwe::model::config::{MarkdownOptions, WikiLinkPath};

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().try_init();
    });
}

#[test]
fn wiki_link_resolves_across_directories_from_windows_disk() {
    setup();

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let diary = base_path.join("diary");
    let clippings = base_path.join("clippings");
    fs::create_dir_all(&diary).unwrap();
    fs::create_dir_all(&clippings).unwrap();

    fs::write(diary.join("today.md"), "# Today\r\n\r\n[[target]]\r\n").unwrap();
    fs::write(clippings.join("target.md"), "# Target\r\n").unwrap();

    let graph = Graph::from_path(
        &base_path,
        false,
        MarkdownOptions {
            wiki_link_path: WikiLinkPath::Full,
            ..Default::default()
        },
        None,
        None,
    );

    assert_str_eq!(
        "# Today\n\n[[clippings/target]]\n",
        graph.to_markdown(&"diary/today".into())
    );
}

#[test]
fn nested_windows_directories_produce_forward_slash_keys() {
    setup();

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let nested = base_path.join("a").join("b").join("c");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("deep.md"), "# Deep\r\n").unwrap();

    let graph = Graph::from_path(&base_path, false, MarkdownOptions::default(), None, None);

    assert_str_eq!(
        "a/b/c/deep",
        graph.key_index().resolve_wiki("deep").to_string()
    );
}

#[test]
fn markdown_link_resolves_relative_across_windows_directories_from_disk() {
    setup();

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let sub_dir = base_path.join("sub").join("dir");
    fs::create_dir_all(&sub_dir).unwrap();

    fs::write(base_path.join("note.md"), "[old title](sub/dir/target)\r\n").unwrap();
    fs::write(sub_dir.join("target.md"), "# title\r\n").unwrap();

    let graph = Graph::from_path(
        &base_path,
        false,
        MarkdownOptions {
            refs_extension: String::default(),
            ..Default::default()
        },
        None,
        None,
    );

    assert_str_eq!(
        "[title](sub/dir/target)\n",
        graph.to_markdown(&"note".into())
    );
}
