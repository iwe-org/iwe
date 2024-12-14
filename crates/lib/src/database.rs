use crate::graph::Graph;
use crate::model::{Content, Key, State};

use crate::parser::Parser;

type FilePath = String;

#[derive(Default)]
pub struct Database {
    graph: Graph,
    content: State,
    pub sequential_ids: bool,
}

pub trait DatabaseContext {
    fn parser(&self, key: &Key) -> Option<Parser>;
}

impl DatabaseContext for &Database {
    fn parser(&self, key: &Key) -> Option<Parser> {
        if key.ends_with(".md") {
            panic!("Key should not end with .md")
        }

        self.content.get(key).map(|content| Parser::new(&content))
    }
}

impl Database {
    pub fn new(state: State, sequential_ids: bool) -> Self {
        let mut graph = Graph::import(state.clone());
        graph.set_sequential_keys(sequential_ids);
        Self {
            graph,
            sequential_ids,
            content: state
                .iter()
                .map(|(k, v)| (k.trim_end_matches(".md").to_string(), v.clone()))
                .collect(),
        }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn get_document(&self, key: &Key) -> Option<Content> {
        self.content.get(key).cloned()
    }

    pub fn insert_document(&mut self, key: &Key, content: Content) -> () {
        if key.ends_with(".md") {
            panic!("Key should not end with .md")
        }

        self.graph.update_key(key, &content);
        self.content.insert(key.clone(), content);
    }

    pub fn update_document(&mut self, key: &Key, content: Content) -> () {
        if key.ends_with(".md") {
            panic!("Key should not end with .md")
        }

        self.graph.update_key(key, &content);
        self.content.insert(key.to_string(), content);
    }
}
