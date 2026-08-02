use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use bm25::{Embedder, EmbedderBuilder, Embedding};
use rayon::prelude::*;

use liwe::model::Key;

pub use bm25::{Language, ScoredDocument};

const DEFAULT_AVGDL: f32 = 256.0;
const PARALLEL_EMBED_THRESHOLD: usize = 128;
const K1: f32 = 1.2;
const B: f32 = 0.75;
const REFIT_DRIFT: f32 = 0.25;

pub const RRF_K: f64 = 60.0;

pub fn rrf_weight(rank: usize) -> f64 {
    1.0 / (RRF_K + rank as f64 + 1.0)
}

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
    keys: Vec<Option<Key>>,
    ids: HashMap<Key, u32>,
    docs: Vec<Option<Embedding<u32>>>,
    postings: HashMap<u32, Vec<(u32, f32)>>,
    free_slots: Vec<u32>,
    total_length: f64,
    language: Language,
}

impl Bm25Index {
    pub fn build(docs: Vec<(Key, String)>, language: Language) -> Self {
        let corpus: Vec<&str> = docs.iter().map(|(_, text)| text.as_str()).collect();
        let embedder: Embedder<u32> =
            EmbedderBuilder::<u32>::with_fit_to_corpus(language.clone(), &corpus)
                .k1(K1)
                .b(B)
                .build();

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
            keys: Vec::new(),
            ids: HashMap::new(),
            docs: Vec::new(),
            postings: HashMap::new(),
            free_slots: Vec::new(),
            total_length: 0.0,
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
            .k1(K1)
            .b(B)
            .build();
        Self {
            embedder,
            keys: Vec::new(),
            ids: HashMap::new(),
            docs: Vec::new(),
            postings: HashMap::new(),
            free_slots: Vec::new(),
            total_length: 0.0,
            language,
        }
    }

    pub fn upsert(&mut self, key: Key, text: String) {
        let embedding = self.embedder.embed(&text);
        self.insert(key, embedding);
        self.refit_if_drifted();
    }

    pub fn remove(&mut self, key: &Key) {
        self.evict(key);
        self.refit_if_drifted();
    }

    /// How far the corpus average document length has moved from the value the embedder was fit
    /// to, as a ratio of the fitted value. Term weights are computed against the fitted value, so
    /// this is the staleness of the index's length normalization.
    pub fn avgdl_drift(&self) -> f32 {
        let fitted = self.embedder.avgdl();
        if fitted <= 0.0 || self.ids.is_empty() {
            return 0.0;
        }
        ((self.current_avgdl() - fitted) / fitted).abs()
    }

    fn current_avgdl(&self) -> f32 {
        if self.ids.is_empty() {
            return DEFAULT_AVGDL;
        }
        (self.total_length / self.ids.len() as f64) as f32
    }

    fn evict(&mut self, key: &Key) {
        let Some(doc_id) = self.ids.remove(key) else {
            return;
        };
        let Some(slot) = self.docs.get_mut(doc_id as usize) else {
            return;
        };
        let Some(embedding) = slot.take() else {
            return;
        };
        self.total_length -= embedding.len() as f64;
        self.drop_postings(doc_id, &embedding);
        self.keys[doc_id as usize] = None;
        self.free_slots.push(doc_id);
    }

    fn refit_if_drifted(&mut self) {
        if self.avgdl_drift() > REFIT_DRIFT {
            self.refit();
        }
    }

    /// Re-fit the embedder to the current corpus and recompute every term weight against the new
    /// average document length. Term frequency and document length are both recoverable from a
    /// stored embedding, so this needs no document text.
    fn refit(&mut self) {
        let avgdl = self.current_avgdl();
        self.embedder = EmbedderBuilder::<u32>::with_avgdl(avgdl)
            .language_mode(self.language.clone())
            .k1(K1)
            .b(B)
            .build();

        self.postings.clear();
        let mut docs = std::mem::take(&mut self.docs);
        for (doc_id, slot) in docs.iter_mut().enumerate() {
            let Some(embedding) = slot.as_mut() else {
                continue;
            };
            reweight(embedding, avgdl);
            self.add_postings(doc_id as u32, embedding);
        }
        self.docs = docs;
    }

    fn add_postings(&mut self, doc_id: u32, embedding: &Embedding<u32>) {
        let mut seen: HashSet<u32> = HashSet::new();
        for token in embedding.iter() {
            if !seen.insert(token.index) {
                continue;
            }
            let postings = self.postings.entry(token.index).or_default();
            match postings.binary_search_by_key(&doc_id, |&(id, _)| id) {
                Ok(position) => postings[position].1 = token.value,
                Err(position) => postings.insert(position, (doc_id, token.value)),
            }
        }
    }

    fn drop_postings(&mut self, doc_id: u32, embedding: &Embedding<u32>) {
        let mut seen: HashSet<u32> = HashSet::new();
        for token in embedding.iter() {
            if !seen.insert(token.index) {
                continue;
            }
            let Some(postings) = self.postings.get_mut(&token.index) else {
                continue;
            };
            if let Ok(position) = postings.binary_search_by_key(&doc_id, |&(id, _)| id) {
                postings.remove(position);
            }
            if postings.is_empty() {
                self.postings.remove(&token.index);
            }
        }
    }

    pub fn search(&self, query: &str) -> Vec<ScoredDocument<Key>> {
        let embedding = self.embedder.embed(query);
        let scores = self.accumulate(&embedding);
        let mut results: Vec<ScoredDocument<Key>> = scores
            .into_iter()
            .enumerate()
            .filter(|(_, score)| *score > 0.0)
            .filter_map(|(doc_id, score)| {
                self.keys[doc_id]
                    .clone()
                    .map(|id| ScoredDocument { id, score })
            })
            .collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        results
    }

    pub fn has_query_terms(&self, query: &str) -> bool {
        !self.embedder.embed(query).is_empty()
    }

    pub fn scores(&self, query: &str) -> HashMap<Key, f32> {
        self.search(query)
            .into_iter()
            .map(|scored| (scored.id, scored.score))
            .collect()
    }

    /// Pages mutually reachable from `key` in one direction: each other document whose accumulated
    /// score against `key`'s own embedding, normalized by `key`'s self-score, is at least `floor`.
    /// Self is excluded; the returned ratios are unsorted (callers sort).
    pub fn similar_to(&self, key: &Key, floor: f32) -> Vec<(Key, f32)> {
        let Some(&own_id) = self.ids.get(key) else {
            return Vec::new();
        };
        let Some(Some(embedding)) = self.docs.get(own_id as usize) else {
            return Vec::new();
        };
        let scores = self.accumulate(embedding);
        let self_score = scores[own_id as usize];
        if self_score <= 0.0 {
            return Vec::new();
        }
        scores
            .into_iter()
            .enumerate()
            .filter_map(|(doc_id, score)| {
                if doc_id as u32 == own_id || score <= 0.0 {
                    return None;
                }
                let ratio = score / self_score;
                (ratio >= floor)
                    .then(|| self.keys[doc_id].clone().map(|key| (key, ratio)))
                    .flatten()
            })
            .collect()
    }

    /// The score of a document against its own embedding: `Σ idf(t) · w(t)` over the document's own
    /// tokens. `None` when `key` is not in the index.
    pub fn self_score(&self, key: &Key) -> Option<f32> {
        let &id = self.ids.get(key)?;
        let embedding = self.docs.get(id as usize)?.as_ref()?;
        let n = self.ids.len() as f32;
        let mut score = 0f32;
        for token in embedding.iter() {
            score += self.idf(token.index, n) * token.value;
        }
        Some(score)
    }

    /// The directional score of `query_key` against `doc_key`: `Σ idf(t) · w_doc(t)` over
    /// `query_key`'s tokens. `None` when either key is absent from the index.
    pub fn score_between(&self, query_key: &Key, doc_key: &Key) -> Option<f32> {
        let &query_id = self.ids.get(query_key)?;
        let query_embedding = self.docs.get(query_id as usize)?.as_ref()?;
        let &doc_id = self.ids.get(doc_key)?;
        let doc_embedding = self.docs.get(doc_id as usize)?.as_ref()?;
        let doc_weights: HashMap<u32, f32> =
            doc_embedding.iter().map(|t| (t.index, t.value)).collect();
        let n = self.ids.len() as f32;
        let mut score = 0f32;
        for token in query_embedding.iter() {
            let weight = doc_weights.get(&token.index).copied().unwrap_or(0.0);
            score += self.idf(token.index, n) * weight;
        }
        Some(score)
    }

    fn idf(&self, token: u32, n: f32) -> f32 {
        let df = self.postings.get(&token).map_or(0, Vec::len) as f32;
        (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
    }

    fn accumulate(&self, query: &Embedding<u32>) -> Vec<f32> {
        let n = self.ids.len() as f32;
        let mut scores = vec![0f32; self.keys.len()];
        for token in query.iter() {
            if let Some(postings) = self.postings.get(&token.index) {
                let idf = self.idf(token.index, n);
                for &(doc_id, weight) in postings {
                    scores[doc_id as usize] += idf * weight;
                }
            }
        }
        scores
    }

    fn insert(&mut self, key: Key, embedding: Embedding<u32>) {
        self.evict(&key);

        let doc_id = match self.free_slots.pop() {
            Some(reused) => reused,
            None => {
                self.keys.push(None);
                self.docs.push(None);
                (self.keys.len() - 1) as u32
            }
        };

        self.add_postings(doc_id, &embedding);
        self.total_length += embedding.len() as f64;
        self.ids.insert(key.clone(), doc_id);
        self.keys[doc_id as usize] = Some(key);
        self.docs[doc_id as usize] = Some(embedding);
    }

    #[cfg(test)]
    pub fn slot_count(&self) -> usize {
        self.keys.len()
    }
}

fn reweight(embedding: &mut Embedding<u32>, avgdl: f32) {
    let length = embedding.len() as f32;
    let mut counts: HashMap<u32, f32> = HashMap::new();
    for token in embedding.iter() {
        *counts.entry(token.index).or_insert(0.0) += 1.0;
    }
    let saturation = K1 * (1.0 - B + B * (length / avgdl));
    for token in embedding.0.iter_mut() {
        let frequency = counts.get(&token.index).copied().unwrap_or(0.0);
        token.value = frequency * (K1 + 1.0) / (frequency + saturation);
    }
}

impl Clone for Bm25Index {
    fn clone(&self) -> Self {
        let embedder = EmbedderBuilder::<u32>::with_avgdl(self.embedder.avgdl())
            .language_mode(self.language.clone())
            .k1(K1)
            .b(B)
            .build();
        Self {
            embedder,
            keys: self.keys.clone(),
            ids: self.ids.clone(),
            docs: self.docs.clone(),
            postings: self.postings.clone(),
            free_slots: self.free_slots.clone(),
            total_length: self.total_length,
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
    fn has_query_terms_detects_stop_words() {
        let index = sample_index();
        assert!(index.has_query_terms("apples"));
        assert!(!index.has_query_terms("the"));
        assert!(!index.has_query_terms(""));
    }

    #[test]
    fn clone_preserves_search_results() {
        let index = sample_index();
        let cloned = index.clone();
        assert_eq!(keys(&cloned, "apples"), vec![key("apples"), key("mixed")]);
    }

    #[test]
    fn identical_documents_tie_break_by_key_ascending() {
        let index = Bm25Index::build(
            vec![
                (
                    key("zebra"),
                    "identical shared content here now".to_string(),
                ),
                (
                    key("alpha"),
                    "identical shared content here now".to_string(),
                ),
            ],
            Language::English,
        );
        assert_eq!(
            keys(&index, "identical shared content"),
            vec![key("alpha"), key("zebra")]
        );
    }

    #[test]
    fn score_between_self_equals_self_score() {
        let index = sample_index();
        for name in ["apples", "mixed", "oranges"] {
            assert_eq!(
                index.score_between(&key(name), &key(name)),
                index.self_score(&key(name))
            );
        }
    }

    fn twins_index() -> Bm25Index {
        Bm25Index::build(
            vec![
                (
                    key("a"),
                    "shared shared shared content here now".to_string(),
                ),
                (
                    key("b"),
                    "shared shared shared content here now".to_string(),
                ),
                (
                    key("c"),
                    "totally different unrelated distinct words".to_string(),
                ),
            ],
            Language::English,
        )
    }

    #[test]
    fn similar_to_excludes_self_and_applies_floor() {
        let index = twins_index();
        assert_eq!(index.similar_to(&key("a"), 0.85), vec![(key("b"), 1.0)]);
        assert_eq!(index.similar_to(&key("b"), 0.85), vec![(key("a"), 1.0)]);
        assert_eq!(index.similar_to(&key("c"), 0.85), Vec::<(Key, f32)>::new());
    }

    #[test]
    fn similar_to_floor_gates_out_weak_matches() {
        let index = twins_index();
        assert_eq!(index.similar_to(&key("a"), 1.5), Vec::<(Key, f32)>::new());
    }

    #[test]
    fn repeated_edits_reuse_one_slot() {
        let mut index = Bm25Index::build(
            vec![(key("note"), "alpha beta gamma".to_string())],
            Language::English,
        );
        assert_eq!(index.slot_count(), 1);

        for edit in 0..1000 {
            index.upsert(key("note"), format!("alpha beta gamma edit {}", edit));
        }

        assert_eq!(index.slot_count(), 1);
        assert_eq!(keys(&index, "alpha"), vec![key("note")]);
    }

    #[test]
    fn removed_slots_are_reused_by_later_documents() {
        let mut index = Bm25Index::build(
            vec![
                (key("a"), "alpha content here".to_string()),
                (key("b"), "beta content here".to_string()),
            ],
            Language::English,
        );
        assert_eq!(index.slot_count(), 2);

        index.remove(&key("a"));
        index.upsert(key("c"), "gamma content here".to_string());

        assert_eq!(index.slot_count(), 2);
        assert_eq!(keys(&index, "alpha"), Vec::<Key>::new());
        assert_eq!(keys(&index, "gamma"), vec![key("c")]);
        assert_eq!(keys(&index, "beta"), vec![key("b")]);
    }

    #[test]
    fn reused_slot_does_not_inherit_the_previous_document_terms() {
        let mut index = Bm25Index::build(
            vec![(key("a"), "alpha unique marker".to_string())],
            Language::English,
        );

        index.remove(&key("a"));
        index.upsert(key("b"), "beta different marker".to_string());

        assert_eq!(keys(&index, "alpha"), Vec::<Key>::new());
        assert_eq!(keys(&index, "unique"), Vec::<Key>::new());
        assert_eq!(keys(&index, "beta"), vec![key("b")]);
    }

    #[test]
    fn postings_stay_ordered_by_document_id() {
        let mut index = Bm25Index::build(
            vec![
                (key("a"), "shared alpha".to_string()),
                (key("b"), "shared beta".to_string()),
                (key("c"), "shared gamma".to_string()),
            ],
            Language::English,
        );
        index.remove(&key("b"));
        index.upsert(key("d"), "shared delta".to_string());

        for postings in index.postings.values() {
            let ids: Vec<u32> = postings.iter().map(|&(id, _)| id).collect();
            let mut sorted = ids.clone();
            sorted.sort();
            assert_eq!(ids, sorted);
        }
    }

    #[test]
    fn growing_documents_refit_the_average_length() {
        let mut index = Bm25Index::build(
            vec![(key("seed"), "alpha beta".to_string())],
            Language::English,
        );
        let fitted = index.embedder.avgdl();

        for document in 0..20 {
            index.upsert(
                key(&format!("long{}", document)),
                (0..200)
                    .map(|word| format!("filler{}", word))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }

        assert!(index.avgdl_drift() <= REFIT_DRIFT);
        assert!(index.embedder.avgdl() > fitted);
    }

    #[test]
    fn refit_matches_a_batch_build_of_the_same_corpus() {
        let documents: Vec<(Key, String)> = (0..40)
            .map(|document| {
                (
                    key(&format!("doc{}", document)),
                    format!(
                        "shared harvest term {}",
                        (0..document * 5)
                            .map(|word| format!("filler{}", word))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                )
            })
            .collect();

        let mut incremental = Bm25Index::empty(Language::English);
        for (document_key, text) in &documents {
            incremental.upsert(document_key.clone(), text.clone());
        }
        incremental.refit();

        let batch = Bm25Index::build(documents.clone(), Language::English);

        for query in ["harvest", "shared", "filler3", "term"] {
            assert_eq!(keys(&incremental, query), keys(&batch, query));
        }
    }

    #[test]
    fn incremental_upserts_match_batch_build() {
        let docs = vec![
            (key("a"), "apples apples orchard harvest season".to_string()),
            (key("b"), "oranges citrus grove harvest season".to_string()),
            (key("c"), "bananas tropical fruit harvest".to_string()),
        ];
        let batch = Bm25Index::build(docs.clone(), Language::English);

        let mut incremental = Bm25Index::empty(Language::English);
        for (k, text) in &docs {
            incremental.upsert(k.clone(), text.clone());
        }

        let mut churned = Bm25Index::build(docs.clone(), Language::English);
        churned.remove(&key("b"));
        churned.upsert(key("b"), "oranges citrus grove harvest season".to_string());

        for query in ["harvest", "apples", "citrus", "tropical", "grove season"] {
            let expected = keys(&batch, query);
            assert_eq!(keys(&incremental, query), expected);
            assert_eq!(keys(&churned, query), expected);
        }

        assert_eq!(churned.search("harvest"), batch.search("harvest"));
    }
}
