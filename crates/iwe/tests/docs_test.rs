use std::process::Command;

use diwe::config::Configuration;
use liwe::query::block::parse_block_predicate;
use liwe::query::{parse_filter_expression, parse_operation, OperationKind};
use liwe::schema::compile_schema;
use serde_yaml::Value;

const INDEX: &str = include_str!("../docs/index.txt");
const QUERY: &str = include_str!("../docs/query.md");
const CONFIG: &str = include_str!("../docs/config.md");
const SCHEMA: &str = include_str!("../docs/schema.md");

fn run_docs(args: &[&str]) -> std::process::Output {
    Command::new(crate::common::get_iwe_binary_path())
        .arg("docs")
        .args(args)
        .output()
        .expect("Failed to execute iwe docs")
}

fn fenced_blocks(source: &str, language: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        match current.as_mut() {
            Some(block) => {
                if line.trim_end() == "```" {
                    blocks.push(current.take().unwrap());
                } else {
                    block.push_str(line);
                    block.push('\n');
                }
            }
            None => {
                let trimmed = line.trim_end();
                if trimmed == format!("```{}", language) || trimmed == format!("``` {}", language) {
                    current = Some(String::new());
                }
            }
        }
    }
    blocks
}

#[test]
fn test_docs_index() {
    let output = run_docs(&[]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), INDEX);
}

#[test]
fn test_docs_query() {
    let output = run_docs(&["query"]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), QUERY);
}

#[test]
fn test_docs_config() {
    let output = run_docs(&["config"]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), CONFIG);
}

#[test]
fn test_docs_schema() {
    let output = run_docs(&["schema"]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), SCHEMA);
}

#[test]
fn test_docs_rejects_unknown_topic() {
    let output = run_docs(&["unknown"]);
    assert!(!output.status.success());
}

#[test]
fn test_query_doc_examples_parse() {
    let examples = fenced_blocks(QUERY, "yaml");
    assert!(!examples.is_empty());
    for example in examples {
        let operation = [
            OperationKind::Find,
            OperationKind::Count,
            OperationKind::Update,
            OperationKind::Delete,
        ]
        .into_iter()
        .any(|kind| parse_operation(&example, kind).is_ok());
        let filter = parse_filter_expression(&example).is_ok();
        let predicate = serde_yaml::from_str::<Value>(&example)
            .map(|value| parse_block_predicate(&value, "docs").is_ok())
            .unwrap_or(false);
        assert!(
            operation || filter || predicate,
            "query example does not parse as an operation, filter, or block predicate:\n{}",
            example
        );
    }
}

#[test]
fn test_config_doc_examples_parse() {
    let examples = fenced_blocks(CONFIG, "toml");
    assert!(!examples.is_empty());
    for example in examples {
        if let Err(error) = toml::from_str::<Configuration>(&example) {
            panic!("config example does not parse:\n{}\n{}", example, error);
        }
    }
}

#[test]
fn test_schema_doc_examples_compile() {
    let examples = fenced_blocks(SCHEMA, "yaml");
    assert!(!examples.is_empty());
    for example in examples {
        if let Err(errors) = compile_schema(&example) {
            panic!(
                "schema example does not compile:\n{}\n{:?}",
                example, errors
            );
        }
    }
}

#[test]
fn test_schema_doc_config_examples_parse() {
    let examples = fenced_blocks(SCHEMA, "toml");
    assert!(!examples.is_empty());
    for example in examples {
        if let Err(error) = toml::from_str::<Configuration>(&example) {
            panic!(
                "schema config example does not parse:\n{}\n{}",
                example, error
            );
        }
    }
}
