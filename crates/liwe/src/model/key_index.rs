use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use percent_encoding::percent_decode_str;

use crate::model::config::WikiLinkPath;
use crate::model::reference::Target;
use crate::model::{strip_doc_extension, Key};

#[derive(Default)]
pub struct KeyIndex {
    by_basename: HashMap<String, Vec<Key>>,
    resolutions: RwLock<Resolutions>,
}

#[derive(Default)]
struct Resolutions {
    by_written: HashMap<String, Resolution>,
    written_in_bucket: HashMap<String, Vec<String>>,
    shortened: HashMap<Key, String>,
    shortened_in_bucket: HashMap<String, Vec<Key>>,
}

#[derive(Clone)]
struct Resolution {
    key: Key,
    resolved: bool,
}

impl Resolutions {
    fn insert(&mut self, written: &str, bucket: String, resolution: Resolution) {
        if self
            .by_written
            .insert(written.to_string(), resolution)
            .is_none()
        {
            self.written_in_bucket
                .entry(bucket)
                .or_default()
                .push(written.to_string());
        }
    }

    fn insert_shortened(&mut self, key: &Key, bucket: String, shortened: String) {
        if self.shortened.insert(key.clone(), shortened).is_none() {
            self.shortened_in_bucket
                .entry(bucket)
                .or_default()
                .push(key.clone());
        }
    }

    fn invalidate(&mut self, bucket: &str) {
        for written in self.written_in_bucket.remove(bucket).unwrap_or_default() {
            self.by_written.remove(&written);
        }
        for key in self.shortened_in_bucket.remove(bucket).unwrap_or_default() {
            self.shortened.remove(&key);
        }
    }
}

impl Clone for KeyIndex {
    fn clone(&self) -> Self {
        KeyIndex {
            by_basename: self.by_basename.clone(),
            resolutions: RwLock::default(),
        }
    }
}

impl KeyIndex {
    pub fn build<'a>(keys: impl Iterator<Item = &'a Key>) -> KeyIndex {
        let mut by_basename: HashMap<String, Vec<Key>> = HashMap::new();

        for key in keys {
            by_basename
                .entry(key_bucket(key))
                .or_default()
                .push(key.clone());
        }

        for bucket in by_basename.values_mut() {
            bucket.sort_by(resolution_order);
        }

        KeyIndex {
            by_basename,
            resolutions: RwLock::default(),
        }
    }

    pub fn wiki_target(&self, target: &Key, mode: WikiLinkPath) -> String {
        match mode {
            WikiLinkPath::Short => self.shorten_wiki(target),
            WikiLinkPath::Full | WikiLinkPath::Preserve => target.to_workspace_url(),
        }
    }

    pub fn insert(&mut self, key: &Key) {
        let basename = key_bucket(key);
        let bucket = self.by_basename.entry(basename.clone()).or_default();
        if !bucket.contains(key) {
            bucket.push(key.clone());
            bucket.sort_by(resolution_order);
            self.cache_mut().invalidate(&basename);
        }
    }

    pub fn remove(&mut self, key: &Key) {
        let basename = key_bucket(key);
        if let Some(bucket) = self.by_basename.get_mut(&basename) {
            bucket.retain(|existing| existing != key);
            if bucket.is_empty() {
                self.by_basename.remove(&basename);
            }
            self.cache_mut().invalidate(&basename);
        }
    }

    fn cache(&self) -> RwLockReadGuard<'_, Resolutions> {
        self.resolutions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn cache_write(&self) -> RwLockWriteGuard<'_, Resolutions> {
        self.resolutions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn cache_mut(&mut self) -> &mut Resolutions {
        self.resolutions
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn resolution(&self, written: &str) -> Resolution {
        if let Some(resolution) = self.cache().by_written.get(written) {
            return resolution.clone();
        }
        let target = decoded_name(written);
        let resolution = self.resolve_decoded(&target);
        self.cache_write()
            .insert(written, name_bucket(&target), resolution.clone());
        resolution
    }

    pub fn contains(&self, key: &Key) -> bool {
        self.by_basename
            .get(&key_bucket(key))
            .is_some_and(|bucket| bucket.contains(key))
    }

    pub fn bucket(target: &Target) -> Option<String> {
        match target {
            Target::Path { .. } => None,
            Target::Name(written) => Some(Self::name_bucket(written)),
        }
    }

    pub fn name_bucket(written: &str) -> String {
        name_bucket(&decoded_name(written))
    }

    pub fn key_bucket(key: &Key) -> String {
        key_bucket(key)
    }

    pub fn target_key(&self, target: &Target) -> Key {
        match target {
            Target::Path { key, .. } => key.clone(),
            Target::Name(written) => self.resolve_wiki(written),
        }
    }

    pub fn resolve(&self, target: &Target) -> Option<Key> {
        match target {
            Target::Path { key, .. } => Some(key.clone()).filter(|key| self.contains(key)),
            Target::Name(written) => {
                let resolution = self.resolution(written);
                resolution.resolved.then_some(resolution.key)
            }
        }
    }

    pub fn resolve_wiki(&self, url: &str) -> Key {
        self.resolution(url).key
    }

    fn resolve_decoded(&self, target: &str) -> Resolution {
        let bucket = Some(target)
            .filter(|target| !segments(target).is_empty())
            .and_then(|target| self.by_basename.get(&name_bucket(target)));
        match bucket.and_then(|bucket| Self::suffix_match(bucket, target).or(bucket.first())) {
            Some(key) => Resolution {
                key: key.clone(),
                resolved: true,
            },
            None => Resolution {
                key: Key::name(target),
                resolved: false,
            },
        }
    }

    fn matches_suffix(&self, url: &str, key: &Key) -> bool {
        let target = decoded_name(url);
        self.by_basename
            .get(&name_bucket(&target))
            .and_then(|bucket| Self::suffix_match(bucket, &target))
            == Some(key)
    }

    fn suffix_match<'b>(bucket: &'b [Key], target: &str) -> Option<&'b Key> {
        let segs = segments(target);
        bucket
            .iter()
            .find(|key| ends_with_segments(key, &segs))
            .or_else(|| {
                bucket
                    .iter()
                    .find(|key| ends_with_segments_folded(key, &segs))
            })
    }

    pub fn shorten_wiki(&self, target: &Key) -> String {
        if let Some(shortened) = self.cache().shortened.get(target) {
            return shortened.clone();
        }
        let path = target.as_str().to_string();
        let Some(basename) = segments(&path).last().map(|basename| fold(basename)) else {
            return path;
        };
        let shortened = self.shorten_in_bucket(target, &path, &basename);
        self.cache_write()
            .insert_shortened(target, basename, shortened.clone());
        shortened
    }

    fn shorten_in_bucket(&self, target: &Key, path: &str, basename: &str) -> String {
        let segs = segments(path);
        let Some(bucket) = self.by_basename.get(basename) else {
            return path.to_string();
        };

        for length in 1..=segs.len() {
            let suffix = &segs[segs.len() - length..];
            let mut matching = bucket
                .iter()
                .filter(|key| ends_with_segments_folded(key, suffix));
            if let (Some(only), None) = (matching.next(), matching.next()) {
                if only == target {
                    return suffix.join("/");
                }
            }
        }

        path.to_string()
    }
}

pub struct KeyRename<'a> {
    before: &'a KeyIndex,
    after: KeyIndex,
    old: Key,
    new: Key,
}

impl<'a> KeyRename<'a> {
    pub fn new(before: &'a KeyIndex, old: &Key, new: &Key) -> KeyRename<'a> {
        let mut after = before.clone();
        after.remove(old);
        after.insert(new);
        KeyRename {
            before,
            after,
            old: old.clone(),
            new: new.clone(),
        }
    }

    pub fn rewrite(&self, target: &Target) -> Option<Target> {
        match target {
            Target::Path { key, fragment } if *key == self.old => Some(Target::Path {
                key: self.new.clone(),
                fragment: fragment.clone(),
            }),
            Target::Name(written) => {
                let before = self.before.target_key(target);
                if before == self.old {
                    self.rewrite_renamed(written)
                } else if self.before.contains(&before) && self.after.target_key(target) == self.new
                {
                    Some(Target::Name(self.after.shorten_wiki(&before)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl KeyRename<'_> {
    fn rewrite_renamed(&self, written: &str) -> Option<Target> {
        if self.after.matches_suffix(written, &self.new) {
            None
        } else if segments(&decoded_name(written)).len() >= segment_count(&self.old) {
            Some(Target::Name(self.new.to_workspace_url()))
        } else {
            Some(Target::Name(self.after.shorten_wiki(&self.new)))
        }
    }
}

fn decoded_name(url: &str) -> String {
    let decoded = percent_decode_str(url).decode_utf8_lossy().into_owned();
    strip_doc_extension(&decoded).to_string()
}

fn name_bucket(name: &str) -> String {
    fold(segments(name).last().copied().unwrap_or_default())
}

fn key_bucket(key: &Key) -> String {
    fold(&key.source())
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

fn ends_with_segments(key: &Key, suffix: &[&str]) -> bool {
    let key_segs = segments(key.as_str());
    key_segs.len() >= suffix.len() && key_segs[key_segs.len() - suffix.len()..] == *suffix
}

fn ends_with_segments_folded(key: &Key, suffix: &[&str]) -> bool {
    let key_segs = segments(key.as_str());
    key_segs.len() >= suffix.len()
        && key_segs[key_segs.len() - suffix.len()..]
            .iter()
            .zip(suffix)
            .all(|(left, right)| fold(left) == fold(right))
}

fn fold(text: &str) -> String {
    text.to_lowercase()
}

fn segment_count(key: &Key) -> usize {
    segments(key.as_str()).len()
}

fn resolution_order(a: &Key, b: &Key) -> std::cmp::Ordering {
    segment_count(a)
        .cmp(&segment_count(b))
        .then_with(|| a.as_str().cmp(b.as_str()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::reference::ReferenceType;

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

    #[test]
    fn lowercase_name_resolves_capitalized_key() {
        let index = index(&["notes/Target"]);
        assert_eq!(Key::name("notes/Target"), index.resolve_wiki("target"));
    }

    #[test]
    fn capitalized_name_resolves_lowercase_key() {
        let index = index(&["notes/target"]);
        assert_eq!(Key::name("notes/target"), index.resolve_wiki("Target"));
    }

    #[test]
    fn folded_directory_segment_resolves() {
        let index = index(&["Notes/Target"]);
        assert_eq!(
            Key::name("Notes/Target"),
            index.resolve_wiki("notes/target")
        );
    }

    #[test]
    fn exact_case_wins_over_folded_match() {
        let index = index(&["notes/Target", "notes/target"]);
        assert_eq!(Key::name("notes/target"), index.resolve_wiki("target"));
        assert_eq!(Key::name("notes/Target"), index.resolve_wiki("Target"));
    }

    #[test]
    fn unmatched_case_picks_deterministically() {
        let index = index(&["notes/Target", "notes/target"]);
        assert_eq!(Key::name("notes/Target"), index.resolve_wiki("TARGET"));
        assert_eq!(Key::name("notes/Target"), index.resolve_wiki("TARGET"));
    }

    #[test]
    fn shorten_keeps_full_path_when_only_case_differs() {
        let index = index(&["notes/Target", "notes/target"]);
        assert_eq!(
            "notes/Target",
            index.shorten_wiki(&Key::name("notes/Target"))
        );
        assert_eq!(
            "notes/target",
            index.shorten_wiki(&Key::name("notes/target"))
        );
    }

    #[test]
    fn shorten_keeps_full_path_when_case_differs_across_directories() {
        let index = index(&["first/Target", "second/target"]);
        assert_eq!(
            "first/Target",
            index.shorten_wiki(&Key::name("first/Target"))
        );
        assert_eq!(
            "second/target",
            index.shorten_wiki(&Key::name("second/target"))
        );
    }

    #[test]
    fn shorten_then_resolve_round_trips_with_mixed_case() {
        let index = index(&["first/Target", "second/target"]);
        let shortened = index.shorten_wiki(&Key::name("first/Target"));
        assert_eq!(Key::name("first/Target"), index.resolve_wiki(&shortened));
    }

    #[test]
    fn non_ascii_name_folds() {
        let index = index(&["notes/Caf\u{e9}"]);
        assert_eq!(
            Key::name("notes/Caf\u{e9}"),
            index.resolve_wiki("caf\u{e9}")
        );
    }

    #[test]
    fn astral_name_resolves_unchanged() {
        let index = index(&["notes/\u{1f600}note"]);
        assert_eq!(
            Key::name("notes/\u{1f600}note"),
            index.resolve_wiki("\u{1f600}note")
        );
    }

    #[test]
    fn insert_and_remove_keep_case_variants_apart() {
        let mut index = index(&["notes/Target"]);
        index.insert(&Key::name("notes/target"));
        assert_eq!(Key::name("notes/target"), index.resolve_wiki("target"));
        assert_eq!(Key::name("notes/Target"), index.resolve_wiki("Target"));

        index.remove(&Key::name("notes/Target"));
        assert_eq!(Key::name("notes/target"), index.resolve_wiki("target"));
        assert_eq!(Key::name("notes/target"), index.resolve_wiki("Target"));
    }

    #[test]
    fn markdown_links_do_not_fold_case() {
        let index = index(&["notes/target"]);
        assert_eq!(
            Key::name("notes/Target"),
            index.target_key(&Target::from_link_url(
                "Target.md",
                "notes",
                ReferenceType::Regular
            ))
        );
    }

    #[test]
    fn wiki_links_fold_case_through_target_key() {
        let index = index(&["notes/Target"]);
        assert_eq!(
            Key::name("notes/Target"),
            index.target_key(&Target::from_link_url(
                "target",
                "notes",
                ReferenceType::WikiLink
            ))
        );
    }

    #[test]
    fn target_key_returns_path_key_unchanged() {
        let index = index(&["notes/target"]);
        assert_eq!(
            Key::name("other/target"),
            index.target_key(&Target::path(Key::name("other/target")))
        );
    }

    #[test]
    fn target_key_resolves_name_against_current_keys() {
        let mut index = index(&["deep/sub/target"]);
        let target = Target::name("target");
        assert_eq!(Key::name("deep/sub/target"), index.target_key(&target));
        index.insert(&Key::name("target"));
        assert_eq!(Key::name("target"), index.target_key(&target));
    }

    #[test]
    fn target_key_falls_back_to_written_name() {
        let index = index(&["notes/note"]);
        assert_eq!(
            Key::name("sub/missing"),
            index.target_key(&Target::name("sub/missing.md"))
        );
    }

    #[test]
    fn resolve_drops_dangling_targets() {
        let index = index(&["notes/target"]);
        assert_eq!(
            Some(Key::name("notes/target")),
            index.resolve(&Target::name("Target"))
        );
        assert_eq!(None, index.resolve(&Target::name("missing")));
        assert_eq!(None, index.resolve(&Target::path(Key::name("missing"))));
    }

    #[test]
    fn bucket_folds_decoded_file_name() {
        assert_eq!(
            Some("my note".to_string()),
            KeyIndex::bucket(&Target::name("Folder/My%20Note.md"))
        );
        assert_eq!(None, KeyIndex::bucket(&Target::path(Key::name("note"))));
        assert_eq!(
            KeyIndex::key_bucket(&Key::name("folder/My Note")),
            KeyIndex::bucket(&Target::name("My Note")).unwrap()
        );
    }

    #[test]
    fn rename_leaves_name_that_still_matches() {
        let index = index(&["a/target"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("b/target"));
        assert_eq!(None, rename.rewrite(&Target::name("target")));
    }

    #[test]
    fn rename_rewrites_path_suffixed_name() {
        let index = index(&["a/target"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("b/target"));
        assert_eq!(
            Some(Target::name("b/target")),
            rename.rewrite(&Target::name("a/target"))
        );
    }

    #[test]
    fn rename_shortens_bare_name_against_new_keys() {
        let index = index(&["a/target", "c/renamed"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("a/renamed"));
        assert_eq!(
            Some(Target::name("a/renamed")),
            rename.rewrite(&Target::name("target"))
        );
    }

    #[test]
    fn rename_keeps_fragment_of_path_target() {
        let index = index(&["a/target"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("b/target"));
        assert_eq!(
            Some(Target::Path {
                key: Key::name("b/target"),
                fragment: Some("part".to_string()),
            }),
            rename.rewrite(&Target::Path {
                key: Key::name("a/target"),
                fragment: Some("part".to_string()),
            })
        );
    }

    #[test]
    fn rename_ignores_targets_resolving_elsewhere() {
        let index = index(&["a/target", "other"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("b/target"));
        assert_eq!(None, rename.rewrite(&Target::name("other")));
        assert_eq!(None, rename.rewrite(&Target::path(Key::name("other"))));
    }

    #[test]
    fn rename_keeps_link_to_other_note_pointing_there() {
        let index = index(&["a/target", "c/renamed"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("a/renamed"));
        assert_eq!(
            Some(Target::name("c/renamed")),
            rename.rewrite(&Target::name("renamed"))
        );
    }

    #[test]
    fn rename_lets_dangling_link_resolve_to_new_name() {
        let index = index(&["a/target"]);
        let rename = KeyRename::new(&index, &Key::name("a/target"), &Key::name("a/renamed"));
        assert_eq!(None, rename.rewrite(&Target::name("renamed")));
    }

    #[test]
    fn cached_resolution_follows_insert_and_remove() {
        let mut index = index(&["deep/target", "other"]);
        let target = Target::name("target");
        assert_eq!(Some(Key::name("deep/target")), index.resolve(&target));
        assert_eq!(
            Some(Key::name("other")),
            index.resolve(&Target::name("other"))
        );

        index.insert(&Key::name("target"));
        assert_eq!(Some(Key::name("target")), index.resolve(&target));

        index.remove(&Key::name("target"));
        index.remove(&Key::name("deep/target"));
        assert_eq!(None, index.resolve(&target));
        assert_eq!(Key::name("target"), index.target_key(&target));
        assert_eq!(
            Some(Key::name("other")),
            index.resolve(&Target::name("other"))
        );
    }

    #[test]
    fn clone_starts_with_fresh_resolutions() {
        let index = index(&["deep/target"]);
        assert_eq!(Key::name("deep/target"), index.resolve_wiki("target"));
        let mut cloned = index.clone();
        cloned.insert(&Key::name("target"));
        assert_eq!(Key::name("target"), cloned.resolve_wiki("target"));
        assert_eq!(Key::name("deep/target"), index.resolve_wiki("target"));
    }

    #[test]
    fn cached_shortening_follows_insert_and_remove() {
        let mut index = index(&["x/a/note"]);
        assert_eq!("note", index.shorten_wiki(&Key::name("x/a/note")));

        index.insert(&Key::name("y/b/note"));
        assert_eq!("a/note", index.shorten_wiki(&Key::name("x/a/note")));

        index.remove(&Key::name("y/b/note"));
        assert_eq!("note", index.shorten_wiki(&Key::name("x/a/note")));
    }
}
