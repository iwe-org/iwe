use jsonschema::ValidatorMap;
use liwe::query::block::parse_block_predicate;
use liwe::query::{
    current_query_schema, parse_filter_expression, parse_operation, query_schema, query_schema_uri,
    OperationKind, CURRENT_QUERY_SCHEMA_DRAFT, QUERY_SCHEMA_DRAFTS,
};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::common::fenced_blocks;

const QUERY: &str = include_str!("../docs/query.md");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    Find,
    Count,
    Update,
    Delete,
    Filter,
    Block,
}

const READINGS: [Reading; 6] = [
    Reading::Find,
    Reading::Count,
    Reading::Update,
    Reading::Delete,
    Reading::Filter,
    Reading::Block,
];

impl Reading {
    fn pointer(self) -> &'static str {
        match self {
            Reading::Find => "#/$defs/findOperation",
            Reading::Count => "#/$defs/countOperation",
            Reading::Update => "#/$defs/updateOperation",
            Reading::Delete => "#/$defs/deleteOperation",
            Reading::Filter => "#/$defs/filter",
            Reading::Block => "#/$defs/blockPredicate",
        }
    }

    fn parser_accepts(self, yaml: &str, value: &YamlValue) -> bool {
        match self {
            Reading::Find => parse_operation(yaml, OperationKind::Find).is_ok(),
            Reading::Count => parse_operation(yaml, OperationKind::Count).is_ok(),
            Reading::Update => parse_operation(yaml, OperationKind::Update).is_ok(),
            Reading::Delete => parse_operation(yaml, OperationKind::Delete).is_ok(),
            Reading::Filter => parse_filter_expression(yaml).is_ok(),
            Reading::Block => parse_block_predicate(value, "schema").is_ok(),
        }
    }
}

fn current_schema_json() -> JsonValue {
    serde_json::from_str(current_query_schema()).expect("the current draft is valid JSON")
}

fn query_validators() -> ValidatorMap {
    jsonschema::validator_map_for(&current_schema_json()).expect("query schema compiles")
}

fn to_json(value: &YamlValue) -> JsonValue {
    serde_json::to_value(value).expect("YAML document converts to JSON")
}

fn yaml_of(document: &str) -> YamlValue {
    serde_yaml::from_str(document).unwrap_or_else(|error| {
        panic!("document is not valid YAML:\n{}\n{}", document, error);
    })
}

#[test]
fn every_embedded_draft_is_valid_and_declares_its_own_address() {
    assert!(QUERY_SCHEMA_DRAFTS
        .iter()
        .any(|(draft, _)| *draft == CURRENT_QUERY_SCHEMA_DRAFT));

    for (draft, body) in QUERY_SCHEMA_DRAFTS {
        let schema: JsonValue = serde_json::from_str(body).expect("draft is valid JSON");
        assert_eq!(
            schema
                .get("$schema")
                .and_then(JsonValue::as_str)
                .unwrap_or_default(),
            "https://json-schema.org/draft/2020-12/schema",
            "draft {} declares the wrong dialect",
            draft
        );
        assert_eq!(
            schema
                .get("$id")
                .and_then(JsonValue::as_str)
                .unwrap_or_default(),
            query_schema_uri(draft),
            "draft {} declares an address other than its own",
            draft
        );
        if let Err(error) = jsonschema::meta::validate(&schema) {
            panic!("draft {} is not a valid schema: {}", draft, error);
        }
    }
}

#[test]
fn the_current_draft_is_the_one_docs_prints() {
    assert_eq!(
        current_query_schema(),
        query_schema(CURRENT_QUERY_SCHEMA_DRAFT).expect("the current draft is embedded")
    );
}

#[test]
fn query_schema_agrees_with_the_parser_on_every_doc_example() {
    let validators = query_validators();
    let examples = fenced_blocks(QUERY, "yaml");
    assert!(!examples.is_empty());

    for example in examples {
        let value = yaml_of(&example);
        let json = to_json(&value);
        for reading in READINGS {
            let parser = reading.parser_accepts(&example, &value);
            let schema = validators[reading.pointer()].is_valid(&json);
            assert_eq!(
                parser, schema,
                "parser and schema disagree reading this as {:?} \
                 (parser accepts: {}, schema accepts: {}):\n{}",
                reading, parser, schema, example
            );
        }
    }
}

#[test]
fn query_schema_rejects_every_parse_time_error() {
    let validators = query_validators();
    for (reading, document, parser_only) in NEGATIVES {
        let value = yaml_of(document);
        let json = to_json(&value);
        assert!(
            !reading.parser_accepts(document, &value),
            "parser accepts a document the corpus lists as rejected, as {:?}:\n{}",
            reading,
            document
        );
        let schema = validators[reading.pointer()].is_valid(&json);
        if *parser_only {
            assert!(
                schema,
                "the schema rejects a rule listed as parser-only, as {:?}:\n{}",
                reading, document
            );
        } else {
            assert!(
                !schema,
                "the schema accepts a document the parser rejects, as {:?}:\n{}",
                reading, document
            );
        }
    }
}

#[test]
fn query_schema_accepts_every_shape_the_parser_accepts() {
    let validators = query_validators();
    for (reading, document) in POSITIVES {
        let value = yaml_of(document);
        let json = to_json(&value);
        assert!(
            reading.parser_accepts(document, &value),
            "the parser rejects a document the corpus lists as accepted, as {:?}:\n{}",
            reading,
            document
        );
        assert!(
            validators[reading.pointer()].is_valid(&json),
            "the schema rejects a document the parser accepts, as {:?}:\n{}",
            reading,
            document
        );
    }
}

const POSITIVES: &[(Reading, &str)] = &[
    (Reading::Filter, "{}\n"),
    (Reading::Filter, "labels: {}\n"),
    (Reading::Filter, "tags: [rust, async]\n"),
    (Reading::Filter, "reviewed: null\n"),
    (Reading::Filter, "status: { $not: { $eq: draft } }\n"),
    (Reading::Filter, "status: { $type: [string, \"null\"] }\n"),
    (Reading::Filter, "$key: { $in: [notes/alpha, notes/beta] }\n"),
    (Reading::Filter, "$content: {}\n"),
    (Reading::Filter, "$includes: { $size: { $gte: 2, $lt: 9 } }\n"),
    (
        Reading::Filter,
        "review:\n  status: done\n  reviewer: { $exists: false }\n",
    ),
    (Reading::Find, "limit: 0\n"),
    (Reading::Find, "project: {}\n"),
    (Reading::Find, "project: { title: null, author: true, key: $key }\n"),
    (Reading::Find, "addFields: { body: $content }\n"),
    (Reading::Find, "search: { fuzzy: alpha }\nsort: { created: 1 }\n"),
    (
        Reading::Update,
        "filter: {}\nupdate: { $delete: {} }\nexpect: 0\n",
    ),
    (
        Reading::Update,
        "filter: {}\nupdate:\n  $set: { \"review.status\": done }\n  $unset: { draft_notes: \"\" }\n",
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $append: { $section: Goals, content: \"- ship it\", expect: { min: 1 } } }\n",
    ),
    (Reading::Delete, "filter: { $key: notes/alpha }\nexpect: 1\n"),
    (Reading::Block, "{}\n"),
    (Reading::Block, "$quote: { $contains: { $text: Goals } }\n"),
    (Reading::Block, "$ref: { $references: notes/alpha }\n"),
    (Reading::Block, "$nor: [{ $within: {} }]\n"),
];

const NEGATIVES: &[(Reading, &str, bool)] = &[
    (Reading::Filter, "$bogus: 1\n", false),
    (Reading::Filter, "$not:\n  status: draft\n", false),
    (Reading::Filter, "status: { $bogus: 1 }\n", false),
    (Reading::Filter, "$and: []\n", false),
    (Reading::Filter, "$or: []\n", false),
    (Reading::Filter, "$nor: []\n", false),
    (Reading::Filter, "status: { $in: [] }\n", false),
    (Reading::Filter, "status: { $nin: [] }\n", false),
    (Reading::Filter, "tags: { $all: [] }\n", false),
    (Reading::Filter, "status: { $type: [] }\n", false),
    (Reading::Filter, "status: { $type: bogus }\n", false),
    (Reading::Filter, "status: { $exists: \"true\" }\n", false),
    (
        Reading::Filter,
        "author: { $eq: alice, name: alice }\n",
        false,
    ),
    (Reading::Filter, "$includedBy: {}\n", false),
    (
        Reading::Filter,
        "$includes: { match: {}, maxDistance: 2 }\n",
        false,
    ),
    (Reading::Filter, "$includes: [notes/alpha]\n", false),
    (Reading::Filter, "$includes: { minDepth: 0 }\n", false),
    (Reading::Filter, "$includes: { maxDepth: -1 }\n", false),
    (
        Reading::Filter,
        "$key: { $eq: notes/alpha, $ne: notes/beta }\n",
        false,
    ),
    (Reading::Filter, "$key: { $bogus: notes/alpha }\n", false),
    (Reading::Filter, "$includes: { $size: -1 }\n", false),
    (Reading::Filter, "tags: { $size: { $bogus: 1 } }\n", false),
    (Reading::Filter, "\"author.$name\": alice\n", false),
    (Reading::Filter, "\"author..name\": alice\n", false),
    (Reading::Filter, "\"author name\": alice\n", false),
    (Reading::Filter, "\"author\\x85name\": alice\n", false),
    (Reading::Filter, "\"author\\x7fname\": alice\n", false),
    (
        Reading::Find,
        "project: { \"author\\x85name\": 1 }\n",
        false,
    ),
    (Reading::Find, "sort: { \"author\\x9fname\": 1 }\n", false),
    (
        Reading::Update,
        "filter: {}\nupdate: { $set: { \"author\\x85name\": alice } }\n",
        false,
    ),
    (Reading::Find, "sort: { created: 2 }\n", false),
    (Reading::Find, "sort: { created: 1, title: -1 }\n", false),
    (Reading::Find, "sort: {}\n", false),
    (Reading::Find, "limit: -1\n", false),
    (Reading::Find, "search: {}\n", false),
    (Reading::Find, "search: { bogus: alpha }\n", false),
    (
        Reading::Find,
        "project: { title: 1 }\naddFields: { author: 1 }\n",
        false,
    ),
    (Reading::Find, "expect: 1\n", false),
    (Reading::Count, "filter: {}\nexpect: 1\n", false),
    (
        Reading::Find,
        "update: { $set: { reviewed: true } }\n",
        false,
    ),
    (Reading::Find, "bogus: 1\n", false),
    (
        Reading::Update,
        "update: { $set: { reviewed: true } }\n",
        false,
    ),
    (Reading::Update, "filter: {}\n", false),
    (Reading::Delete, "limit: 1\n", false),
    (Reading::Update, "filter: {}\nupdate: {}\n", false),
    (Reading::Update, "filter: {}\nupdate: { $set: {} }\n", false),
    (
        Reading::Update,
        "filter: {}\nupdate: { $unset: {} }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $bogus: {} }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $replace: { $header: Goals } }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $replaceText: { $header: Goals, from: Goals } }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $delete: { bogus: 1 } }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $replaceText: { to: Aims, content: Aims } }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $delete: { expect: { bogus: 1 } } }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $delete: { expect: {} } }\n",
        false,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $delete: { expect: -1 } }\n",
        false,
    ),
    (Reading::Block, "$bogus: {}\n", false),
    (Reading::Block, "title: Goals\n", false),
    (Reading::Block, "$ref: notes/alpha\n", false),
    (Reading::Block, "$hr: Goals\n", false),
    (Reading::Block, "$quote: Goals\n", false),
    (Reading::Block, "$list: Goals\n", false),
    (Reading::Block, "$hr: { $text: Goals }\n", false),
    (Reading::Block, "$quote: { $matches: Goals }\n", false),
    (Reading::Block, "$list: { $text: Goals }\n", false),
    (Reading::Block, "$and: []\n", false),
    (Reading::Block, "$references: 5\n", false),
    (Reading::Block, "$text: { $ne: Goals }\n", false),
    (Reading::Block, "$matches: 5\n", false),
    (Reading::Find, "project: { title: $bogus }\n", false),
    (Reading::Find, "project: { $key: 1 }\n", false),
    (Reading::Find, "project: { \"author.name\": 1 }\n", false),
    (Reading::Find, "project: { title: 2 }\n", false),
    (
        Reading::Find,
        "project: { body: { $content: {}, $blocks: {} } }\n",
        false,
    ),
    (Reading::Find, "addFields: { $header: {} }\n", false),
    (
        Reading::Find,
        "project: { found: { $matches: { $within: Goals } } }\n",
        false,
    ),
    (
        Reading::Find,
        "project: { found: { $matches: { pattern: todo, bogus: 1 } } }\n",
        false,
    ),
    (Reading::Block, "$matches: \"[\"\n", true),
    (Reading::Block, "$within: { $header: Goals }\n", true),
    (
        Reading::Filter,
        "$includes: { minDepth: 3, maxDepth: 2 }\n",
        true,
    ),
    (
        Reading::Filter,
        "$references: { minDistance: 3, maxDistance: 2 }\n",
        true,
    ),
    (
        Reading::Delete,
        "filter: {}\nexpect: { min: 5, max: 1 }\n",
        true,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $set: { reviewed: true }, $unset: { reviewed: \"\" } }\n",
        true,
    ),
    (
        Reading::Update,
        "filter: {}\nupdate: { $set: { review: {}, \"review.status\": done } }\n",
        true,
    ),
    (
        Reading::Find,
        "project: { found: { $matches: { pattern: \"[\" } } }\n",
        true,
    ),
    (Reading::Find, "limit: 1.0\n", true),
    (Reading::Find, "limit: 1e3\n", true),
    (Reading::Find, "sort: { created: 1.0 }\n", true),
    (Reading::Find, "project: { title: 1.0 }\n", true),
    (Reading::Delete, "filter: {}\nexpect: 1.0\n", true),
    (Reading::Filter, "tags: { $size: 1.0 }\n", true),
    (Reading::Filter, "$includes: { maxDepth: 1.0 }\n", true),
    (
        Reading::Update,
        "filter: {}\nupdate: { $delete: { expect: { max: 1.0 } } }\n",
        true,
    ),
];
