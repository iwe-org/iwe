use std::fs;
use std::sync::Once;

use liwe::fs::new_for_path;
use tempfile::TempDir;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}

#[test]
fn test_gitignore_excludes_files() {
    setup();

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    fs::write(
        base_path.join("included.md"),
        "# Included File\n\nThis should be included.",
    )
    .unwrap();
    fs::write(
        base_path.join("excluded.md"),
        "# Excluded File\n\nThis should be excluded.",
    )
    .unwrap();
    fs::write(
        base_path.join("also_included.md"),
        "# Also Included\n\nThis should also be included.",
    )
    .unwrap();

    fs::write(base_path.join(".gitignore"), "excluded.md\n").unwrap();

    let state = new_for_path(&base_path);

    assert!(state.contains_key("included"));
    assert!(state.contains_key("also_included"));
    assert!(!state.contains_key("excluded"));

    assert_eq!(
        state.get("included").unwrap(),
        "# Included File\n\nThis should be included."
    );
    assert_eq!(
        state.get("also_included").unwrap(),
        "# Also Included\n\nThis should also be included."
    );
}
