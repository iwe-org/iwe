use std::fs;
use std::sync::Once;

use pretty_assertions::assert_str_eq;
use tempfile::TempDir;

use diwe::config::{MarkdownOptions, RefsText};
use diwe::graph_from_path;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().try_init();
    });
}

#[test]
fn link_into_subdirectory_resolves_when_loaded_from_disk() {
    setup();

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let sub_dir = base_path.join("sub").join("dir");
    fs::create_dir_all(&sub_dir).unwrap();

    fs::write(base_path.join("note.md"), "[old title](sub/dir/target)\n").unwrap();
    fs::write(sub_dir.join("target.md"), "# title\n").unwrap();

    let graph = graph_from_path(
        &base_path,
        false,
        MarkdownOptions {
            refs_extension: String::default(),
            refs_text: RefsText::Normalize,
            ..Default::default()
        },
        None,
    );

    assert_str_eq!(
        "[title](sub/dir/target)\n",
        graph.to_markdown(&"note".into())
    );
}

#[test]
fn double_extension_file_strips_only_one_extension() {
    setup();

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    fs::write(base_path.join("note.md.md"), "# title\n").unwrap();

    let graph = graph_from_path(&base_path, false, MarkdownOptions::default(), None);

    let keys: Vec<String> = graph.keys().iter().map(|k| k.to_library_url()).collect();

    assert_eq!(keys, vec!["note.md".to_string()]);
}
