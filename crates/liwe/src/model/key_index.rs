use std::collections::HashMap;

use percent_encoding::percent_decode_str;

use crate::model::config::WikiLinkPath;
use crate::model::reference::ReferenceType;
use crate::model::Key;

#[derive(Clone, Default)]
pub struct KeyIndex {
    by_basename: HashMap<String, Vec<Key>>,
}

impl KeyIndex {
    pub fn build<'a>(keys: impl Iterator<Item = &'a Key>) -> KeyIndex {
        let mut by_basename: HashMap<String, Vec<Key>> = HashMap::new();

        for key in keys {
            by_basename
                .entry(key.source())
                .or_default()
                .push(key.clone());
        }

        for bucket in by_basename.values_mut() {
            bucket.sort_by(resolution_order);
        }

        KeyIndex { by_basename }
    }

    pub fn wiki_target(&self, target: &Key, mode: WikiLinkPath) -> String {
        match mode {
            WikiLinkPath::Short => self.shorten_wiki(target),
            WikiLinkPath::Full | WikiLinkPath::Preserve => target.to_library_url(),
        }
    }

    pub fn insert(&mut self, key: &Key) {
        let bucket = self.by_basename.entry(key.source()).or_default();
        if !bucket.contains(key) {
            bucket.push(key.clone());
            bucket.sort_by(resolution_order);
        }
    }

    pub fn remove(&mut self, key: &Key) {
        if let Some(bucket) = self.by_basename.get_mut(&key.source()) {
            bucket.retain(|existing| existing != key);
            if bucket.is_empty() {
                self.by_basename.remove(&key.source());
            }
        }
    }

    pub fn resolve_wiki(&self, url: &str) -> Key {
        let decoded = percent_decode_str(url).decode_utf8_lossy().into_owned();
        let target = decoded.trim_end_matches(".md").to_string();
        let segs = segments(&target);

        let Some(basename) = segs.last() else {
            return Key::name(&target);
        };
        let Some(bucket) = self.by_basename.get(*basename) else {
            return Key::name(&target);
        };

        bucket
            .iter()
            .find(|key| ends_with_segments(key, &segs))
            .or_else(|| bucket.first())
            .cloned()
            .unwrap_or_else(|| Key::name(&target))
    }

    pub fn shorten_wiki(&self, target: &Key) -> String {
        let path = target.relative_path.to_string();

        let segs = segments(&path);

        let Some(basename) = segs.last() else {
            return path;
        };
        let Some(bucket) = self.by_basename.get(*basename) else {
            return path;
        };

        for length in 1..=segs.len() {
            let suffix = &segs[segs.len() - length..];
            let mut matching = bucket.iter().filter(|key| ends_with_segments(key, suffix));
            if let (Some(only), None) = (matching.next(), matching.next()) {
                if only == target {
                    return suffix.join("/");
                }
            }
        }

        path
    }

    pub fn resolve_link_key(
        &self,
        url: &str,
        relative_to: &str,
        reference_type: ReferenceType,
    ) -> Key {
        match reference_type {
            ReferenceType::Regular => Key::from_rel_link_url(url, relative_to),
            ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => self.resolve_wiki(url),
        }
    }
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

fn ends_with_segments(key: &Key, suffix: &[&str]) -> bool {
    let key_segs = segments(&key.relative_path);
    key_segs.len() >= suffix.len() && key_segs[key_segs.len() - suffix.len()..] == *suffix
}

fn segment_count(key: &Key) -> usize {
    segments(&key.relative_path).len()
}

fn resolution_order(a: &Key, b: &Key) -> std::cmp::Ordering {
    segment_count(a)
        .cmp(&segment_count(b))
        .then_with(|| a.relative_path.cmp(&b.relative_path))
}

#[cfg(test)]
mod test {
    use super::*;

    fn index(paths: &[&str]) -> KeyIndex {
        let keys: Vec<Key> = paths.iter().map(|p| Key::name(p)).collect();
        KeyIndex::build(keys.iter())
    }

    #[test]
    fn bare_name_resolves_across_directories() {
        let index = index(&["first/note", "second/target"]);
        assert_eq!(Key::name("second/target"), index.resolve_wiki("target"));
    }

    #[test]
    fn bare_name_with_no_match_stays_as_root_key() {
        let index = index(&["first/note"]);
        assert_eq!(Key::name("missing"), index.resolve_wiki("missing"));
    }

    #[test]
    fn single_match_is_returned() {
        let index = index(&["folder/only"]);
        assert_eq!(Key::name("folder/only"), index.resolve_wiki("only"));
    }

    #[test]
    fn ambiguous_match_prefers_fewest_segments_then_lexicographic() {
        let index = index(&["zeta/target", "alpha/beta/target", "target"]);
        assert_eq!(Key::name("target"), index.resolve_wiki("target"));
    }

    #[test]
    fn ambiguous_match_with_equal_segments_prefers_lexicographic() {
        let index = index(&["zeta/target", "alpha/target"]);
        assert_eq!(Key::name("alpha/target"), index.resolve_wiki("target"));
    }

    #[test]
    fn prefixed_path_matches_exact_key_from_root() {
        let index = index(&["folder/target", "other/target"]);
        assert_eq!(
            Key::name("folder/target"),
            index.resolve_wiki("folder/target")
        );
    }

    #[test]
    fn prefixed_path_without_exact_match_falls_back_to_basename() {
        let index = index(&["clippings/target"]);
        assert_eq!(
            Key::name("clippings/target"),
            index.resolve_wiki("missing/target")
        );
    }

    #[test]
    fn md_extension_is_trimmed() {
        let index = index(&["folder/target"]);
        assert_eq!(Key::name("folder/target"), index.resolve_wiki("target.md"));
    }

    #[test]
    fn suffix_disambiguates_shared_basename() {
        let index = index(&["x/a/note", "y/b/note"]);
        assert_eq!(Key::name("x/a/note"), index.resolve_wiki("a/note"));
        assert_eq!(Key::name("y/b/note"), index.resolve_wiki("b/note"));
    }

    #[test]
    fn shorten_unique_basename_to_bare_name() {
        let index = index(&["clippings/target", "notes/note"]);
        assert_eq!("target", index.shorten_wiki(&Key::name("clippings/target")));
    }

    #[test]
    fn shorten_shared_basename_to_shortest_unique_suffix() {
        let index = index(&["x/a/note", "y/b/note"]);
        assert_eq!("a/note", index.shorten_wiki(&Key::name("x/a/note")));
        assert_eq!("b/note", index.shorten_wiki(&Key::name("y/b/note")));
    }

    #[test]
    fn shorten_falls_back_to_full_path_when_suffix_still_shared() {
        let index = index(&["x/a/note", "y/a/note"]);
        assert_eq!("x/a/note", index.shorten_wiki(&Key::name("x/a/note")));
        assert_eq!("y/a/note", index.shorten_wiki(&Key::name("y/a/note")));
    }

    #[test]
    fn shorten_then_resolve_round_trips() {
        let index = index(&["x/a/note", "y/b/note"]);
        let shortened = index.shorten_wiki(&Key::name("x/a/note"));
        assert_eq!(Key::name("x/a/note"), index.resolve_wiki(&shortened));
    }

    #[test]
    fn shorten_keeps_full_path_for_target_absent_from_index() {
        let index = index(&["first/note"]);
        assert_eq!("second/note", index.shorten_wiki(&Key::name("second/note")));
    }

    #[test]
    fn wiki_target_full_keeps_full_path() {
        let index = index(&["clippings/target", "notes/note"]);
        assert_eq!(
            "clippings/target",
            index.wiki_target(&Key::name("clippings/target"), WikiLinkPath::Full)
        );
    }

    #[test]
    fn wiki_target_short_uses_shortest_suffix() {
        let index = index(&["clippings/target", "notes/note"]);
        assert_eq!(
            "target",
            index.wiki_target(&Key::name("clippings/target"), WikiLinkPath::Short)
        );
    }

    #[test]
    fn wiki_target_preserve_keeps_full_path() {
        let index = index(&["clippings/target", "notes/note"]);
        assert_eq!(
            "clippings/target",
            index.wiki_target(&Key::name("clippings/target"), WikiLinkPath::Preserve)
        );
    }

    #[test]
    fn insert_makes_a_new_key_resolvable() {
        let mut index = index(&["first/note"]);
        assert_eq!(Key::name("target"), index.resolve_wiki("target"));
        index.insert(&Key::name("second/target"));
        assert_eq!(Key::name("second/target"), index.resolve_wiki("target"));
    }

    #[test]
    fn insert_is_idempotent() {
        let mut index = index(&["first/target"]);
        index.insert(&Key::name("first/target"));
        assert_eq!(Key::name("first/target"), index.resolve_wiki("target"));
    }

    #[test]
    fn insert_keeps_resolution_order() {
        let mut index = index(&["zeta/target"]);
        index.insert(&Key::name("alpha/target"));
        assert_eq!(Key::name("alpha/target"), index.resolve_wiki("target"));
    }

    #[test]
    fn remove_drops_the_key_from_resolution() {
        let mut index = index(&["first/note", "second/target"]);
        index.remove(&Key::name("second/target"));
        assert_eq!(Key::name("target"), index.resolve_wiki("target"));
    }
}
