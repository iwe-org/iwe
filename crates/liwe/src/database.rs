use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::graph::{Graph, SearchPath};
use crate::markdown::MarkdownReader;
use crate::model::graph::MarkdownOptions;
use crate::model::{Content, Key, State};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::parser::Parser;

pub struct Database {
    graph: Graph,
    content: State,
    pub sequential_ids: bool,
    paths: Vec<SearchPath>,
}

pub trait DatabaseContext {
    fn lines(&self, key: &Key) -> u32;
    fn parser(&self, key: &Key) -> Option<Parser>;
}

impl DatabaseContext for &Database {
    fn parser(&self, key: &Key) -> Option<Parser> {
        self.content
            .get(key)
            .map(|content| Parser::new(&content, MarkdownReader::new()))
    }

    fn lines(&self, key: &Key) -> u32 {
        self.content
            .get(key)
            .map(|content| content.lines().count() as u32)
            .unwrap_or(0)
    }
}

impl Database {
    pub fn global_search(&self, query: &str) -> Vec<SearchPath> {
        let matcher = SkimMatcherV2::default();
        assert_eq!(None, matcher.fuzzy_match("abc", "abx"));

        self.paths
            .par_iter()
            .map(|path| {
                (
                    path,
                    matcher.fuzzy_match(&path.search_text, query).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|(path_a, rank_a), (path_b, rank_b)| {
                if query.is_empty() {
                    path_b
                        .node_rank
                        .cmp(&path_a.node_rank)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                } else {
                    rank_b
                        .cmp(&rank_a)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                        .then_with(|| path_b.node_rank.cmp(&path_a.node_rank))
                }
            })
            .map(|(path, _)| path)
            .take(100)
            .cloned()
            .collect_vec()
    }

    pub fn new(state: State, sequential_ids: bool, markdown_options: MarkdownOptions) -> Self {
        let mut graph = Graph::import(&state, markdown_options);
        let paths = graph.search_paths();
        graph.set_sequential_keys(sequential_ids);
        Self {
            graph,
            sequential_ids,
            paths,
            content: state,
        }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn get_document(&self, key: &Key) -> Option<Content> {
        self.content.get(key).cloned()
    }

    pub fn insert_document(&mut self, key: Key, content: Content) -> () {
        self.graph.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
        self.paths = self.graph.search_paths();
    }

    pub fn update_document(&mut self, key: Key, content: Content) -> () {
        self.graph.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
        self.paths = self.graph.search_paths();
    }
}
