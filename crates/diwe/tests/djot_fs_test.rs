use diwe::config::{DjotOptions, Format, FormatOptions};

fn djot_options() -> FormatOptions {
    FormatOptions::Djot(DjotOptions::default())
}

#[test]
fn discovers_and_reads_dj_files_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("note.dj"), "# Title\n\ntext\n").unwrap();
    std::fs::write(dir.path().join("ignored.md"), "# Md\n").unwrap();

    let graph = diwe::loader::from_path(dir.path(), false, djot_options(), None);

    assert_eq!("# Title\n\ntext\n", graph.to_markdown(&"note".into()));
    assert_eq!("", graph.to_markdown(&"ignored".into()));
}

#[test]
fn writes_dj_extension() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = liwe::model::State::new();
    state.insert("note".to_string(), "# Title\n".to_string());

    diwe::fs::write_store_at_path(&state, dir.path(), Format::Djot).unwrap();

    assert!(dir.path().join("note.dj").exists());
    assert!(!dir.path().join("note.md").exists());
}
