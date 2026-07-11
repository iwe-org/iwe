use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use liwe::graph::Graph;
use liwe::model::Key;
use liwe::schema::{build_document, compile_schema, CompiledSchema, Violation};

use crate::config::{schemas_dir, Configuration, SchemaBinding};
use crate::tokens::count_tokens;

#[derive(Debug)]
pub struct SchemaBindings {
    rules: Vec<(String, Vec<GlobMatcher>)>,
}

impl SchemaBindings {
    pub fn compile(schemas: &HashMap<String, SchemaBinding>) -> Result<Self, Vec<String>> {
        let mut names: Vec<&String> = schemas.keys().collect();
        names.sort();

        let mut rules = Vec::new();
        let mut errors = Vec::new();

        for name in names {
            let mut matchers = Vec::new();
            for pattern in schemas[name].r#match.as_slice() {
                let anchored = pattern.strip_prefix('/').unwrap_or(pattern);
                match GlobBuilder::new(anchored).literal_separator(true).build() {
                    Ok(glob) => matchers.push(glob.compile_matcher()),
                    Err(error) => errors.push(format!(
                        "schema '{name}': invalid pattern '{pattern}': {error}"
                    )),
                }
            }
            rules.push((name.clone(), matchers));
        }

        if errors.is_empty() {
            Ok(SchemaBindings { rules })
        } else {
            Err(errors)
        }
    }

    pub fn schemas_for(&self, key: &str) -> Vec<&str> {
        self.rules
            .iter()
            .filter(|(_, matchers)| matchers.iter().any(|matcher| matcher.is_match(key)))
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

#[derive(Debug)]
pub struct KeyReport {
    pub key: Key,
    pub schema: String,
    pub violations: Vec<Violation>,
}

impl Serialize for KeyReport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("KeyReport", 3)?;
        state.serialize_field("key", &self.key.to_string())?;
        state.serialize_field("schema", &self.schema)?;
        state.serialize_field("violations", &self.violations)?;
        state.end()
    }
}

pub fn validate_documents(
    config: &Configuration,
    graph: &Graph,
    keys: &[Key],
) -> Result<Vec<KeyReport>, Vec<String>> {
    let dir = schemas_dir().map_err(|error| vec![error])?;
    validate_documents_in(&dir, config, graph, keys)
}

fn validate_documents_in(
    dir: &Path,
    config: &Configuration,
    graph: &Graph,
    keys: &[Key],
) -> Result<Vec<KeyReport>, Vec<String>> {
    let bindings = SchemaBindings::compile(&config.schemas)?;
    let compiled = compile_schemas(dir, &config.schemas)?;

    let mut reports = Vec::new();
    for key in keys {
        let names = bindings.schemas_for(&key.to_string());
        if names.is_empty() {
            continue;
        }
        let document = build_document(graph, key, count_tokens);
        for name in names {
            let violations = compiled[name].validate(&document);
            if !violations.is_empty() {
                reports.push(KeyReport {
                    key: key.clone(),
                    schema: name.to_string(),
                    violations,
                });
            }
        }
    }
    Ok(reports)
}

fn compile_schemas(
    dir: &Path,
    schemas: &HashMap<String, SchemaBinding>,
) -> Result<HashMap<String, CompiledSchema>, Vec<String>> {
    let mut names: Vec<&String> = schemas.keys().collect();
    names.sort();

    let mut compiled = HashMap::new();
    let mut errors = Vec::new();

    for name in names {
        let path = dir.join(format!("{name}.yaml"));
        let source = match read_to_string(&path) {
            Ok(source) => source,
            Err(_) => {
                errors.push(format!(
                    "schema '{name}': .iwe/schemas/{name}.yaml not found"
                ));
                continue;
            }
        };
        match compile_schema(&source) {
            Ok(schema) => {
                compiled.insert(name.clone(), schema);
            }
            Err(schema_errors) => {
                for error in schema_errors {
                    if error.pointer.is_empty() {
                        errors.push(format!("schema '{name}': {}", error.message));
                    } else {
                        errors.push(format!(
                            "schema '{name}' {}: {}",
                            error.pointer, error.message
                        ));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(compiled)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{create_dir_all, write};

    use liwe::markdown::MarkdownReader;
    use liwe::schema::Crumb;
    use tempfile::TempDir;

    use crate::config::Patterns;

    fn bindings(entries: &[(&str, Patterns)]) -> SchemaBindings {
        let schemas = entries
            .iter()
            .map(|(name, patterns)| {
                (
                    name.to_string(),
                    SchemaBinding {
                        r#match: patterns.clone(),
                    },
                )
            })
            .collect();
        SchemaBindings::compile(&schemas).expect("compiles")
    }

    #[test]
    fn single_glob_matches_by_prefix() {
        let bindings = bindings(&[("person", Patterns::One("people/**".to_string()))]);
        assert_eq!(bindings.schemas_for("people/alice"), vec!["person"]);
        assert_eq!(bindings.schemas_for("teams/core"), Vec::<&str>::new());
    }

    #[test]
    fn list_form_matches_any_pattern() {
        let bindings = bindings(&[(
            "session",
            Patterns::Many(vec!["journal/*".to_string(), "meetings/**".to_string()]),
        )]);
        assert_eq!(bindings.schemas_for("journal/monday"), vec!["session"]);
        assert_eq!(
            bindings.schemas_for("meetings/2026/standup"),
            vec!["session"]
        );
        assert_eq!(
            bindings.schemas_for("journal/2026/monday"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn single_star_stops_at_separator_double_star_crosses() {
        let single = bindings(&[("one", Patterns::One("notes/*".to_string()))]);
        assert_eq!(single.schemas_for("notes/today"), vec!["one"]);
        assert_eq!(single.schemas_for("notes/2026/today"), Vec::<&str>::new());

        let double = bindings(&[("all", Patterns::One("notes/**".to_string()))]);
        assert_eq!(double.schemas_for("notes/today"), vec!["all"]);
        assert_eq!(double.schemas_for("notes/2026/today"), vec!["all"]);
    }

    #[test]
    fn leading_slash_in_pattern_is_stripped() {
        let bindings = bindings(&[("person", Patterns::One("/people/**".to_string()))]);
        assert_eq!(bindings.schemas_for("people/alice"), vec!["person"]);
    }

    #[test]
    fn overlapping_schemas_both_apply_sorted_by_name() {
        let bindings = bindings(&[
            ("zeta", Patterns::One("people/**".to_string())),
            ("alpha", Patterns::One("people/*".to_string())),
        ]);
        assert_eq!(bindings.schemas_for("people/alice"), vec!["alpha", "zeta"]);
    }

    #[test]
    fn invalid_globs_report_every_bad_pattern() {
        let schemas = HashMap::from([(
            "broken".to_string(),
            SchemaBinding {
                r#match: Patterns::Many(vec!["[".to_string(), "people/[".to_string()]),
            },
        )]);
        let errors = SchemaBindings::compile(&schemas).unwrap_err();
        assert_eq!(
            errors,
            vec![
                "schema 'broken': invalid pattern '[': error parsing glob '[': unclosed character class; missing ']'".to_string(),
                "schema 'broken': invalid pattern 'people/[': error parsing glob 'people/[': unclosed character class; missing ']'".to_string(),
            ]
        );
    }

    #[test]
    fn binding_round_trips_through_toml_as_string_and_list() {
        let source = "\
[schemas.person]
match = \"people/**\"

[schemas.session]
match = [\"journal/*\", \"meetings/**\"]
";
        let config: Configuration = toml::from_str(source).expect("parses");
        assert_eq!(
            config.schemas["person"],
            SchemaBinding {
                r#match: Patterns::One("people/**".to_string()),
            }
        );
        assert_eq!(
            config.schemas["session"],
            SchemaBinding {
                r#match: Patterns::Many(vec!["journal/*".to_string(), "meetings/**".to_string()]),
            }
        );

        let reparsed: Configuration =
            toml::from_str(&toml::to_string(&config).expect("serializes")).expect("reparses");
        assert_eq!(reparsed.schemas, config.schemas);
    }

    fn graph_with(entries: &[(&str, &str)]) -> Graph {
        let mut graph = Graph::new();
        for (key, content) in entries {
            graph.from_markdown(Key::name(key), content, MarkdownReader::new());
        }
        graph
    }

    fn config_with(entries: &[(&str, Patterns)]) -> Configuration {
        let mut config = Configuration::default();
        config.schemas = entries
            .iter()
            .map(|(name, patterns)| {
                (
                    name.to_string(),
                    SchemaBinding {
                        r#match: patterns.clone(),
                    },
                )
            })
            .collect();
        config
    }

    fn write_schema(dir: &Path, name: &str, source: &str) {
        let schemas = dir.join(".iwe").join("schemas");
        create_dir_all(&schemas).unwrap();
        write(schemas.join(format!("{name}.yaml")), source).unwrap();
    }

    #[test]
    fn validate_documents_reports_per_schema_in_key_and_name_order() {
        let temp = TempDir::new().unwrap();
        write_schema(
            temp.path(),
            "person",
            "sections:\n  - header: { const: Summary }\n  - header: { const: Tasks }\n",
        );
        write_schema(
            temp.path(),
            "audited",
            "sections:\n  - header: { const: Review }\n",
        );

        let graph = graph_with(&[("people/alice", "# Summary\n"), ("teams/core", "# Team\n")]);
        let config = config_with(&[
            ("person", Patterns::One("people/**".to_string())),
            ("audited", Patterns::One("people/**".to_string())),
        ]);
        let keys = vec![Key::name("people/alice"), Key::name("teams/core")];

        let reports = validate_documents_in(
            temp.path().join(".iwe").join("schemas").as_path(),
            &config,
            &graph,
            &keys,
        )
        .expect("no config errors");

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].key, Key::name("people/alice"));
        assert_eq!(reports[0].schema, "audited");
        assert_eq!(
            reports[0].violations,
            vec![Violation {
                breadcrumb: vec![],
                message: "required section 'Review' missing".to_string(),
                hint: None,
                schema_pointer: "/sections/0/minContains".to_string(),
                keyword: "minContains".to_string(),
            }]
        );
        assert_eq!(reports[1].key, Key::name("people/alice"));
        assert_eq!(reports[1].schema, "person");
        assert_eq!(
            reports[1].violations,
            vec![Violation {
                breadcrumb: vec![],
                message: "required section 'Tasks' missing".to_string(),
                hint: None,
                schema_pointer: "/sections/1/minContains".to_string(),
                keyword: "minContains".to_string(),
            }]
        );
    }

    #[test]
    fn clean_document_yields_no_report() {
        let temp = TempDir::new().unwrap();
        write_schema(
            temp.path(),
            "person",
            "sections:\n  - header: { const: Summary }\n",
        );
        let graph = graph_with(&[("people/alice", "# Summary\n\ntext\n")]);
        let config = config_with(&[("person", Patterns::One("people/**".to_string()))]);
        let keys = vec![Key::name("people/alice")];

        let reports = validate_documents_in(
            temp.path().join(".iwe").join("schemas").as_path(),
            &config,
            &graph,
            &keys,
        )
        .expect("no config errors");
        assert!(reports.is_empty());
    }

    #[test]
    fn nested_breadcrumb_survives_into_report() {
        let temp = TempDir::new().unwrap();
        write_schema(
            temp.path(),
            "person",
            "sections:\n  - header: { const: Summary }\nadditionalSections: false\n",
        );
        let graph = graph_with(&[("people/alice", "# Summary\n\n# Extra\n")]);
        let config = config_with(&[("person", Patterns::One("people/**".to_string()))]);
        let keys = vec![Key::name("people/alice")];

        let reports = validate_documents_in(
            temp.path().join(".iwe").join("schemas").as_path(),
            &config,
            &graph,
            &keys,
        )
        .expect("no config errors");
        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0].violations,
            vec![Violation {
                breadcrumb: vec![Crumb::Header("Extra".to_string())],
                message: "unexpected section".to_string(),
                hint: None,
                schema_pointer: "/additionalSections".to_string(),
                keyword: "additionalSections".to_string(),
            }]
        );
    }

    #[test]
    fn missing_schema_file_is_a_config_error() {
        let temp = TempDir::new().unwrap();
        create_dir_all(temp.path().join(".iwe").join("schemas")).unwrap();
        let graph = graph_with(&[("people/alice", "# Summary\n")]);
        let config = config_with(&[("person", Patterns::One("people/**".to_string()))]);
        let keys = vec![Key::name("people/alice")];

        let errors = validate_documents_in(
            temp.path().join(".iwe").join("schemas").as_path(),
            &config,
            &graph,
            &keys,
        )
        .unwrap_err();
        assert_eq!(
            errors,
            vec!["schema 'person': .iwe/schemas/person.yaml not found".to_string()]
        );
    }

    #[test]
    fn uncompilable_schema_surfaces_schema_error_text() {
        let temp = TempDir::new().unwrap();
        write_schema(temp.path(), "person", "sections:\n  - minContains: -1\n");
        let graph = graph_with(&[("people/alice", "# Summary\n")]);
        let config = config_with(&[("person", Patterns::One("people/**".to_string()))]);
        let keys = vec![Key::name("people/alice")];

        let errors = validate_documents_in(
            temp.path().join(".iwe").join("schemas").as_path(),
            &config,
            &graph,
            &keys,
        )
        .unwrap_err();
        assert_eq!(
            errors,
            vec![
                "schema 'person' /sections/0/minContains: minContains must not be negative"
                    .to_string()
            ]
        );
    }
}
