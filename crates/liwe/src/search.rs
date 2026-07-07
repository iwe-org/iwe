use std::collections::HashMap;

use bm25::{Embedder, EmbedderBuilder, Embedding};
use rayon::prelude::*;

use crate::model::Key;

pub use bm25::{Language, ScoredDocument};

const DEFAULT_AVGDL: f32 = 256.0;
const PARALLEL_EMBED_THRESHOLD: usize = 128;

pub const RRF_K: f64 = 60.0;

pub fn rrf_weight(rank: usize) -> f64 {
    1.0 / (RRF_K + rank as f64 + 1.0)
}

type Scorer = bm25::Scorer<Key, u32>;

pub fn parse_language(name: &str) -> Language {
    match name.trim().to_lowercase().as_str() {
        "arabic" => Language::Arabic,
        "danish" => Language::Danish,
        "dutch" => Language::Dutch,
        "english" => Language::English,
        "french" => Language::French,
        "german" => Language::German,
        "greek" => Language::Greek,
        "hungarian" => Language::Hungarian,
        "italian" => Language::Italian,
        "norwegian" => Language::Norwegian,
        "portuguese" => Language::Portuguese,
        "romanian" => Language::Romanian,
        "russian" => Language::Russian,
        "spanish" => Language::Spanish,
        "swedish" => Language::Swedish,
        "tamil" => Language::Tamil,
        "turkish" => Language::Turkish,
        _ => Language::English,
    }
}

pub struct Bm25Index {
    embedder: Embedder<u32>,
    scorer: Scorer,
    embeddings: HashMap<Key, Embedding<u32>>,
    language: Language,
}

impl Bm25Index {
    pub fn build(docs: Vec<(Key, String)>, language: Language) -> Self {
        let corpus: Vec<&str> = docs.iter().map(|(_, text)| text.as_str()).collect();
        let embedder: Embedder<u32> =
            EmbedderBuilder::<u32>::with_fit_to_corpus(language.clone(), &corpus).build();

        let embedded: Vec<(Key, Embedding<u32>)> = if docs.len() < PARALLEL_EMBED_THRESHOLD {
            docs.into_iter()
                .map(|(key, text)| {
                    let embedding = embedder.embed(&text);
                    (key, embedding)
                })
                .collect()
        } else {
            docs.into_par_iter()
                .map(|(key, text)| {
                    let embedding = embedder.embed(&text);
                    (key, embedding)
                })
                .collect()
        };

        let mut index = Self {
            embedder,
            scorer: Scorer::new(),
            embeddings: HashMap::new(),
            language,
        };
        for (key, embedding) in embedded {
            index.insert(key, embedding);
        }
        index
    }

    pub fn empty(language: Language) -> Self {
        let embedder = EmbedderBuilder::<u32>::with_avgdl(DEFAULT_AVGDL)
            .language_mode(language.clone())
            .build();
        Self {
            embedder,
            scorer: Scorer::new(),
            embeddings: HashMap::new(),
            language,
        }
    }

    pub fn upsert(&mut self, key: Key, text: String) {
        let embedding = self.embedder.embed(&text);
        self.insert(key, embedding);
    }

    pub fn remove(&mut self, key: &Key) {
        self.scorer.remove(key);
        self.embeddings.remove(key);
    }

    pub fn search(&self, query: &str) -> Vec<ScoredDocument<Key>> {
        let embedding = self.embedder.embed(query);
        self.scorer.matches(&embedding)
    }

    pub fn scores(&self, query: &str) -> HashMap<Key, f32> {
        self.search(query)
            .into_iter()
            .map(|scored| (scored.id, scored.score))
            .collect()
    }

    fn insert(&mut self, key: Key, embedding: Embedding<u32>) {
        self.scorer.upsert(&key, embedding.clone());
        self.embeddings.insert(key, embedding);
    }
}

impl Clone for Bm25Index {
    fn clone(&self) -> Self {
        let embedder = EmbedderBuilder::<u32>::with_avgdl(self.embedder.avgdl())
            .language_mode(self.language.clone())
            .build();
        let mut scorer = Scorer::new();
        for (key, embedding) in &self.embeddings {
            scorer.upsert(key, embedding.clone());
        }
        Self {
            embedder,
            scorer,
            embeddings: self.embeddings.clone(),
            language: self.language.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> Key {
        name.into()
    }

    fn keys(index: &Bm25Index, query: &str) -> Vec<Key> {
        index
            .search(query)
            .into_iter()
            .map(|scored| scored.id)
            .collect()
    }

    fn sample_index() -> Bm25Index {
        Bm25Index::build(
            vec![
                (key("apples"), "apples apples apples orchard".to_string()),
                (key("mixed"), "apples and oranges in a basket".to_string()),
                (key("oranges"), "oranges citrus grove".to_string()),
            ],
            Language::English,
        )
    }

    #[test]
    fn ranks_term_dense_document_first() {
        let index = sample_index();
        assert_eq!(keys(&index, "apples"), vec![key("apples"), key("mixed")]);
    }

    #[test]
    fn title_and_body_terms_both_match() {
        let index = Bm25Index::build(
            vec![(
                key("note"),
                "Weather Report\nThunderstorms expected tomorrow".to_string(),
            )],
            Language::English,
        );
        assert_eq!(keys(&index, "weather"), vec![key("note")]);
        assert_eq!(keys(&index, "thunderstorms"), vec![key("note")]);
    }

    #[test]
    fn upsert_new_key_appears() {
        let mut index = sample_index();
        assert_eq!(keys(&index, "pineapple"), Vec::<Key>::new());
        index.upsert(key("tropical"), "pineapple mango".to_string());
        assert_eq!(keys(&index, "pineapple"), vec![key("tropical")]);
    }

    #[test]
    fn upsert_existing_key_replaces_terms() {
        let mut index = sample_index();
        index.upsert(key("note"), "alpha unique".to_string());
        assert_eq!(keys(&index, "alpha"), vec![key("note")]);

        index.upsert(key("note"), "beta different".to_string());
        assert_eq!(keys(&index, "alpha"), Vec::<Key>::new());
        assert_eq!(keys(&index, "beta"), vec![key("note")]);
    }

    #[test]
    fn remove_deletes_document() {
        let mut index = sample_index();
        assert_eq!(keys(&index, "orchard"), vec![key("apples")]);
        index.remove(&key("apples"));
        assert_eq!(keys(&index, "orchard"), Vec::<Key>::new());
    }

    #[test]
    fn empty_index_returns_no_results() {
        let index = Bm25Index::empty(Language::English);
        assert_eq!(keys(&index, "anything"), Vec::<Key>::new());
    }

    #[test]
    fn clone_preserves_search_results() {
        let index = sample_index();
        let cloned = index.clone();
        assert_eq!(keys(&cloned, "apples"), vec![key("apples"), key("mixed")]);
    }
}
