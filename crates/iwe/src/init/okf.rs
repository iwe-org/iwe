use std::fs::{create_dir_all, write};
use std::path::Path;

pub const OKF_VERSION: &str = "0.2";

pub const SCHEMA_CONCEPT: &str = include_str!("schemas/okf.yaml");
pub const SCHEMA_INDEX: &str = include_str!("schemas/okf-index.yaml");
pub const SCHEMA_LOG: &str = include_str!("schemas/okf-log.yaml");

pub const REFS_EXTENSION: &str = ".md";

pub const INDEX_FILE_NAME: &str = "index.md";

pub const BINDINGS: &str = "\
[schemas.okf]
match = [\"**\", \"!index\", \"!**/index\", \"!log\", \"!**/log\"]

[schemas.okf-index]
match = [\"index\", \"**/index\"]

[schemas.okf-log]
match = [\"log\", \"**/log\"]
";

pub fn index_document() -> String {
    format!("---\nokf_version: \"{OKF_VERSION}\"\n---\n")
}

fn schema_files() -> [(&'static str, &'static str); 3] {
    [
        ("okf.yaml", SCHEMA_CONCEPT),
        ("okf-index.yaml", SCHEMA_INDEX),
        ("okf-log.yaml", SCHEMA_LOG),
    ]
}

pub fn planned_artifacts(root: &Path, library: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for (name, _) in schema_files() {
        lines.push(format!("would write .iwe/schemas/{name}"));
    }
    let index = index_path(root, library);
    if index.exists() {
        lines.push(format!(
            "would keep the existing {}",
            index_display(library)
        ));
    } else {
        lines.push(format!("would write {}", index_display(library)));
    }
    lines
}

pub fn write_artifacts(root: &Path, library: &str) -> Vec<String> {
    let mut lines = Vec::new();

    let dir = root.join(".iwe").join("schemas");
    if let Err(error) = create_dir_all(&dir) {
        lines.push(format!(
            "warning: failed to create {}: {error}",
            dir.display()
        ));
        return lines;
    }
    for (name, source) in schema_files() {
        match write(dir.join(name), source) {
            Ok(()) => lines.push(format!("wrote .iwe/schemas/{name}")),
            Err(error) => lines.push(format!("warning: failed to write {name}: {error}")),
        }
    }

    let index = index_path(root, library);
    if index.exists() {
        lines.push(format!("kept the existing {}", index_display(library)));
        return lines;
    }
    if let Some(parent) = index.parent() {
        if let Err(error) = create_dir_all(parent) {
            lines.push(format!(
                "warning: failed to create {}: {error}",
                parent.display()
            ));
            return lines;
        }
    }
    match write(&index, index_document()) {
        Ok(()) => lines.push(format!("wrote {}", index_display(library))),
        Err(error) => lines.push(format!(
            "warning: failed to write {}: {error}",
            index_display(library)
        )),
    }
    lines
}

fn index_path(root: &Path, library: &str) -> std::path::PathBuf {
    if library.is_empty() {
        root.join(INDEX_FILE_NAME)
    } else {
        root.join(library).join(INDEX_FILE_NAME)
    }
}

fn index_display(library: &str) -> String {
    if library.is_empty() {
        INDEX_FILE_NAME.to_string()
    } else {
        format!("{library}/{INDEX_FILE_NAME}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn every_embedded_schema_compiles() {
        for (name, source) in schema_files() {
            assert!(
                liwe::schema::compile_schema(source).is_ok(),
                "{name} failed to compile"
            );
        }
    }

    #[test]
    fn bindings_parse_as_configuration_schema_entries() {
        let config: diwe::config::Configuration = toml::from_str(BINDINGS).expect("parses");
        let mut names: Vec<&String> = config.schemas.keys().collect();
        names.sort();
        assert_eq!(names, vec!["okf", "okf-index", "okf-log"]);
        assert_eq!(
            config.schemas["okf"],
            diwe::config::SchemaBinding {
                r#match: diwe::config::Patterns::Many(vec![
                    "**".to_string(),
                    "!index".to_string(),
                    "!**/index".to_string(),
                    "!log".to_string(),
                    "!**/log".to_string(),
                ]),
            }
        );
    }

    #[test]
    fn index_document_carries_only_the_version() {
        assert_eq!(index_document(), "---\nokf_version: \"0.2\"\n---\n");
    }

    #[test]
    fn write_artifacts_creates_schemas_and_a_root_index() {
        let temp = TempDir::new().unwrap();
        let lines = write_artifacts(temp.path(), "");
        assert_eq!(
            lines,
            vec![
                "wrote .iwe/schemas/okf.yaml".to_string(),
                "wrote .iwe/schemas/okf-index.yaml".to_string(),
                "wrote .iwe/schemas/okf-log.yaml".to_string(),
                "wrote index.md".to_string(),
            ]
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("index.md")).unwrap(),
            index_document()
        );
    }

    #[test]
    fn write_artifacts_keeps_an_existing_index() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("index.md"), "# Existing\n").unwrap();
        let lines = write_artifacts(temp.path(), "");
        assert_eq!(lines.last().unwrap(), "kept the existing index.md");
        assert_eq!(
            std::fs::read_to_string(temp.path().join("index.md")).unwrap(),
            "# Existing\n"
        );
    }

    #[test]
    fn write_artifacts_places_the_index_inside_the_library() {
        let temp = TempDir::new().unwrap();
        let lines = write_artifacts(temp.path(), "data");
        assert_eq!(lines.last().unwrap(), "wrote data/index.md");
        assert!(temp.path().join("data").join("index.md").exists());
    }

    #[test]
    fn planned_artifacts_writes_nothing() {
        let temp = TempDir::new().unwrap();
        let lines = planned_artifacts(temp.path(), "");
        assert_eq!(
            lines,
            vec![
                "would write .iwe/schemas/okf.yaml".to_string(),
                "would write .iwe/schemas/okf-index.yaml".to_string(),
                "would write .iwe/schemas/okf-log.yaml".to_string(),
                "would write index.md".to_string(),
            ]
        );
        assert!(!temp.path().join(".iwe").exists());
        assert!(!temp.path().join("index.md").exists());
    }
}
