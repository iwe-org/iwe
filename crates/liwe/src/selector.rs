use std::collections::HashSet;

use crate::graph::walk::descendants_inclusion;
use crate::graph::Graph;
use crate::model::Key;

#[derive(Debug, Clone)]
pub struct KeyDepth {
    pub key: Key,
    pub depth: Option<u8>,
}

impl KeyDepth {
    pub fn bare(key: Key) -> Self {
        Self { key, depth: None }
    }

    pub fn with_depth(key: Key, depth: u8) -> Self {
        Self { key, depth: Some(depth) }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Selector {
    pub in_: Vec<KeyDepth>,
    pub in_any: Vec<KeyDepth>,
    pub not_in: Vec<KeyDepth>,
    pub max_depth: Option<u8>,
}

impl Selector {
    pub fn is_empty(&self) -> bool {
        self.in_.is_empty()
            && self.in_any.is_empty()
            && self.not_in.is_empty()
            && self.max_depth.is_none()
    }

    pub fn resolve(&self, graph: &Graph) -> HashSet<Key> {
        let mut candidates: Option<HashSet<Key>> = None;

        for kd in &self.in_ {
            let depth = kd.depth.or(self.max_depth);
            let set = descendants_at_depth(graph, &kd.key, depth);
            candidates = Some(match candidates {
                None => set,
                Some(c) => c.intersection(&set).cloned().collect(),
            });
        }

        if !self.in_any.is_empty() {
            let mut union: HashSet<Key> = HashSet::new();
            for kd in &self.in_any {
                let depth = kd.depth.or(self.max_depth);
                union.extend(descendants_at_depth(graph, &kd.key, depth));
            }
            candidates = Some(match candidates {
                None => union,
                Some(c) => c.intersection(&union).cloned().collect(),
            });
        }

        let mut result =
            candidates.unwrap_or_else(|| graph.keys().into_iter().collect());

        for kd in &self.not_in {
            let depth = kd.depth.or(self.max_depth);
            let exclude = descendants_at_depth(graph, &kd.key, depth);
            result = result.difference(&exclude).cloned().collect();
        }

        result
    }
}

pub fn descendants_at_depth(graph: &Graph, origin: &Key, depth: Option<u8>) -> HashSet<Key> {
    let max = depth.map(u32::from).unwrap_or(u32::MAX);
    descendants_inclusion(graph, origin, max).into_keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::config::MarkdownOptions;
    use std::collections::HashMap;

    fn build(docs: &[(&str, &str)]) -> Graph {
        let map: HashMap<String, String> = docs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Graph::import(&map, MarkdownOptions::default(), None)
    }

    fn k(s: &str) -> Key {
        Key::name(s)
    }

    fn keys(set: &HashSet<Key>) -> Vec<String> {
        let mut v: Vec<String> = set.iter().map(|k| k.to_string()).collect();
        v.sort();
        v
    }

    #[test]
    fn descendants_unbounded() {
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n"),
            ("b", "# B\n\n[C](c)\n"),
            ("c", "# C\n"),
        ]);
        let result = descendants_at_depth(&graph, &k("a"), None);
        assert_eq!(keys(&result), vec!["b", "c"]);
    }

    #[test]
    fn descendants_depth_one() {
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n"),
            ("b", "# B\n\n[C](c)\n"),
            ("c", "# C\n"),
        ]);
        let result = descendants_at_depth(&graph, &k("a"), Some(1));
        assert_eq!(keys(&result), vec!["b"]);
    }

    #[test]
    fn descendants_depth_zero_is_empty() {
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n"),
            ("b", "# B\n"),
        ]);
        let result = descendants_at_depth(&graph, &k("a"), Some(0));
        assert!(result.is_empty());
    }

    #[test]
    fn descendants_handle_cycles() {
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n"),
            ("b", "# B\n\n[A](a)\n"),
        ]);
        let result = descendants_at_depth(&graph, &k("a"), None);
        assert_eq!(keys(&result), vec!["b"]);
    }

    #[test]
    fn descendants_diamond_visits_each_once() {
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n\n[C](c)\n"),
            ("b", "# B\n\n[D](d)\n"),
            ("c", "# C\n\n[D](d)\n"),
            ("d", "# D\n"),
        ]);
        let result = descendants_at_depth(&graph, &k("a"), None);
        assert_eq!(keys(&result), vec!["b", "c", "d"]);
    }

    #[test]
    fn descendants_short_path_does_not_starve_long_path() {
        // X reachable at depth 1 via Y, and at depth 3 via B→C→D→X→Z.
        // Y→X is the short path; B→C→D→X→Z must still expose Z within depth 4.
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n\n[Y](y)\n"),
            ("b", "# B\n\n[C](c)\n"),
            ("c", "# C\n\n[D](d)\n"),
            ("d", "# D\n\n[X](x)\n"),
            ("y", "# Y\n\n[X](x)\n"),
            ("x", "# X\n\n[Z](z)\n"),
            ("z", "# Z\n"),
        ]);
        let result = descendants_at_depth(&graph, &k("a"), Some(4));
        // BFS layer order: A→{B,Y}→{C,X}→{D,Z}→{}.
        // Expect everything below A is reached within 4 hops.
        assert_eq!(keys(&result), vec!["b", "c", "d", "x", "y", "z"]);
    }

    #[test]
    fn descendants_missing_key_returns_empty() {
        let graph = build(&[("a", "# A\n")]);
        let result = descendants_at_depth(&graph, &k("missing"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn selector_empty_returns_all_keys() {
        let graph = build(&[
            ("a", "# A\n"),
            ("b", "# B\n"),
            ("c", "# C\n"),
        ]);
        let result = Selector::default().resolve(&graph);
        assert_eq!(keys(&result), vec!["a", "b", "c"]);
    }

    #[test]
    fn selector_in_intersects_two_parents() {
        // X is sub-doc of both A and B; Y is sub-doc of A only.
        let graph = build(&[
            ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
            ("b", "# B\n\n[X](x)\n"),
            ("x", "# X\n"),
            ("y", "# Y\n"),
        ]);
        let sel = Selector {
            in_: vec![KeyDepth::bare(k("a")), KeyDepth::bare(k("b"))],
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert_eq!(keys(&result), vec!["x"]);
    }

    #[test]
    fn selector_in_any_unions_two_parents() {
        let graph = build(&[
            ("a", "# A\n\n[X](x)\n"),
            ("b", "# B\n\n[Y](y)\n"),
            ("x", "# X\n"),
            ("y", "# Y\n"),
            ("z", "# Z\n"),
        ]);
        let sel = Selector {
            in_any: vec![KeyDepth::bare(k("a")), KeyDepth::bare(k("b"))],
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert_eq!(keys(&result), vec!["x", "y"]);
    }

    #[test]
    fn selector_not_in_subtracts() {
        let graph = build(&[
            ("a", "# A\n\n[X](x)\n\n[Y](y)\n"),
            ("archive", "# Archive\n\n[Y](y)\n"),
            ("x", "# X\n"),
            ("y", "# Y\n"),
        ]);
        let sel = Selector {
            in_: vec![KeyDepth::bare(k("a"))],
            not_in: vec![KeyDepth::bare(k("archive"))],
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert_eq!(keys(&result), vec!["x"]);
    }

    #[test]
    fn selector_per_key_depth_overrides_max_depth() {
        // A→B→C, X→Y. max_depth=1 normally limits A's set to {B}.
        // Per-key depth=2 on A widens to {B, C}.
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n"),
            ("b", "# B\n\n[C](c)\n"),
            ("c", "# C\n"),
        ]);
        let sel = Selector {
            in_: vec![KeyDepth::with_depth(k("a"), 2)],
            max_depth: Some(1),
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert_eq!(keys(&result), vec!["b", "c"]);
    }

    #[test]
    fn selector_max_depth_applies_when_no_per_key_override() {
        let graph = build(&[
            ("a", "# A\n\n[B](b)\n"),
            ("b", "# B\n\n[C](c)\n"),
            ("c", "# C\n"),
        ]);
        let sel = Selector {
            in_: vec![KeyDepth::bare(k("a"))],
            max_depth: Some(1),
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert_eq!(keys(&result), vec!["b"]);
    }

    #[test]
    fn selector_empty_intersection_returns_empty() {
        let graph = build(&[
            ("a", "# A\n\n[X](x)\n"),
            ("b", "# B\n\n[Y](y)\n"),
            ("x", "# X\n"),
            ("y", "# Y\n"),
        ]);
        let sel = Selector {
            in_: vec![KeyDepth::bare(k("a")), KeyDepth::bare(k("b"))],
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert!(result.is_empty());
    }

    #[test]
    fn selector_missing_parent_yields_empty_intersection() {
        let graph = build(&[("a", "# A\n\n[X](x)\n"), ("x", "# X\n")]);
        let sel = Selector {
            in_: vec![KeyDepth::bare(k("a")), KeyDepth::bare(k("nonexistent"))],
            ..Default::default()
        };
        let result = sel.resolve(&graph);
        assert!(result.is_empty());
    }
}
