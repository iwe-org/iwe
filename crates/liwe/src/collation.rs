use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

use icu_collator::options::CollatorOptions;
use icu_collator::preferences::CollationNumericOrdering;
use icu_collator::{Collator, CollatorBorrowed, CollatorPreferences};
use icu_locale_core::Locale;
use icu_properties::props::{
    EmojiComponent, EmojiPresentation, ExtendedPictographic, RegionalIndicator,
};
use icu_properties::CodePointSetData;

use crate::locale::strip_locale_encoding;

const FALLBACK_LOCALE: &str = "en";
const VARIATION_SELECTOR_EMOJI: char = '\u{FE0F}';
const ZERO_WIDTH_JOINER: char = '\u{200D}';

#[derive(Clone, Default)]
pub struct TitleCollator {
    localized: Option<Localized>,
}

struct Localized {
    locale: Locale,
    tag: String,
    collator: CollatorBorrowed<'static>,
}

impl Localized {
    fn new(locale: Locale) -> Localized {
        let collator = new_collator(&locale)
            .or_else(|| new_collator(&fallback_locale()))
            .expect("collation data for the fallback locale is compiled in");
        Localized {
            tag: locale.to_string(),
            locale,
            collator,
        }
    }
}

impl Clone for Localized {
    fn clone(&self) -> Self {
        Localized::new(self.locale.clone())
    }
}

impl TitleCollator {
    pub fn binary() -> TitleCollator {
        TitleCollator { localized: None }
    }

    pub fn for_locale(configured: Option<&str>) -> TitleCollator {
        let requested = configured
            .map(String::from)
            .or_else(sys_locale::get_locale)
            .unwrap_or_else(|| FALLBACK_LOCALE.to_string());
        let locale = parse_locale(&requested).unwrap_or_else(fallback_locale);
        TitleCollator {
            localized: Some(Localized::new(locale)),
        }
    }

    pub fn locale(&self) -> Option<&str> {
        self.localized
            .as_ref()
            .map(|localized| localized.tag.as_str())
    }

    pub fn compare(&self, a: &str, b: &str) -> Ordering {
        self.compare_keys(&self.sort_key(a), &self.sort_key(b))
    }

    pub fn sort_key<'a>(&self, text: &'a str) -> SortKey<'a> {
        let stripped = match &self.localized {
            None => Cow::Borrowed(text),
            Some(_) => strip_emoji(text),
        };
        SortKey { text, stripped }
    }

    pub fn compare_keys(&self, a: &SortKey<'_>, b: &SortKey<'_>) -> Ordering {
        let Some(localized) = &self.localized else {
            return a.text.cmp(b.text);
        };
        let mut ordering = localized.collator.compare(&a.stripped, &b.stripped);
        if ordering == Ordering::Equal && (a.has_emoji() || b.has_emoji()) {
            ordering = localized.collator.compare(a.text, b.text);
        }
        ordering.then_with(|| a.text.cmp(b.text))
    }

    pub fn sort_by_text<T>(&self, items: &mut [T], text: impl Fn(&T) -> &str) {
        let keys: Vec<SortKey<'_>> = items.iter().map(|item| self.sort_key(text(item))).collect();
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by(|&a, &b| self.compare_keys(&keys[a], &keys[b]));
        drop(keys);
        apply_order(items, &order);
    }
}

pub struct SortKey<'a> {
    text: &'a str,
    stripped: Cow<'a, str>,
}

impl SortKey<'_> {
    fn has_emoji(&self) -> bool {
        matches!(self.stripped, Cow::Owned(_))
    }
}

pub(crate) fn apply_order<T>(items: &mut [T], order: &[usize]) {
    for target in 0..order.len() {
        let mut source = order[target];
        while source < target {
            source = order[source];
        }
        items.swap(target, source);
    }
}

impl fmt::Debug for TitleCollator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TitleCollator")
            .field("locale", &self.locale())
            .finish()
    }
}

fn new_collator(locale: &Locale) -> Option<CollatorBorrowed<'static>> {
    let mut preferences = CollatorPreferences::from(locale);
    preferences.numeric_ordering = Some(CollationNumericOrdering::True);
    Collator::try_new(preferences, CollatorOptions::default()).ok()
}

fn fallback_locale() -> Locale {
    Locale::try_from_str(FALLBACK_LOCALE).expect("fallback locale is valid")
}

fn parse_locale(value: &str) -> Option<Locale> {
    let tag = strip_locale_encoding(value).replace('_', "-");
    if tag.is_empty() || tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        return None;
    }
    Locale::try_from_str(&tag).ok()
}

fn strip_emoji(text: &str) -> Cow<'_, str> {
    let mut chars = text.char_indices().peekable();
    let mut stripped: Option<String> = None;
    while let Some((index, c)) = chars.next() {
        let next = chars.peek().map(|(_, next)| *next);
        if !is_emoji_start(c, next) {
            if let Some(buffer) = &mut stripped {
                buffer.push(c);
            }
            continue;
        }
        let buffer = stripped.get_or_insert_with(|| text[..index].to_string());
        let mut previous = c;
        while let Some((_, candidate)) = chars.peek().copied() {
            let after = chars.clone().nth(1).map(|(_, after)| after);
            if !is_emoji_continuation(candidate, previous, after) {
                break;
            }
            previous = candidate;
            chars.next();
        }
        let next_is_space = chars.peek().is_some_and(|(_, c)| c.is_whitespace());
        if buffer.is_empty() || buffer.ends_with(char::is_whitespace) {
            while chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
                chars.next();
            }
        } else if chars.peek().is_some() && !next_is_space {
            buffer.push(' ');
        }
    }
    match stripped {
        None => Cow::Borrowed(text),
        Some(buffer) => Cow::Owned(buffer.trim_end().to_string()),
    }
}

fn is_emoji_start(c: char, next: Option<char>) -> bool {
    if c.is_ascii() {
        return false;
    }
    let pictographic = CodePointSetData::new::<ExtendedPictographic>();
    let presentation = CodePointSetData::new::<EmojiPresentation>();
    let regional = CodePointSetData::new::<RegionalIndicator>();
    presentation.contains(c)
        || regional.contains(c)
        || (pictographic.contains(c) && next == Some(VARIATION_SELECTOR_EMOJI))
}

fn is_emoji_continuation(c: char, previous: char, next: Option<char>) -> bool {
    let pictographic = CodePointSetData::new::<ExtendedPictographic>();
    let component = CodePointSetData::new::<EmojiComponent>();
    let regional = CodePointSetData::new::<RegionalIndicator>();
    regional.contains(c)
        || (!c.is_ascii() && component.contains(c))
        || (pictographic.contains(c) && (previous == ZERO_WIDTH_JOINER || is_emoji_start(c, next)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(collator: &TitleCollator, titles: &[&str]) -> Vec<String> {
        let mut items: Vec<String> = titles.iter().map(|t| t.to_string()).collect();
        items.sort_by(|a, b| collator.compare(a, b));
        items
    }

    #[test]
    fn mixed_case_groups_by_letter_and_lowercase_first() {
        let collator = TitleCollator::for_locale(Some("en"));
        assert_eq!(
            sorted(&collator, &["Zebra", "apple", "Apple", "banana", "Banana"]),
            vec!["apple", "Apple", "banana", "Banana", "Zebra"]
        );
    }

    #[test]
    fn accented_letters_sort_with_their_base_letter() {
        let collator = TitleCollator::for_locale(Some("en"));
        assert_eq!(
            sorted(
                &collator,
                &["Zoe", "Émile", "Eve", "Adam", "ähnlich", "Zack"]
            ),
            vec!["Adam", "ähnlich", "Émile", "Eve", "Zack", "Zoe"]
        );
    }

    #[test]
    fn numbers_compare_by_value() {
        let collator = TitleCollator::for_locale(Some("en"));
        assert_eq!(
            sorted(&collator, &["Note 10", "Note 2", "Note 1", "Note 100"]),
            vec!["Note 1", "Note 2", "Note 10", "Note 100"]
        );
    }

    #[test]
    fn leading_emoji_is_ignored() {
        let collator = TitleCollator::for_locale(Some("en"));
        assert_eq!(
            sorted(
                &collator,
                &[
                    "🚀 Launch",
                    "Apples",
                    "📚 Books",
                    "👨‍👩‍👧 Family",
                    "🇺🇸 Travel",
                    "Zoo"
                ]
            ),
            vec![
                "Apples",
                "📚 Books",
                "👨‍👩‍👧 Family",
                "🚀 Launch",
                "🇺🇸 Travel",
                "Zoo"
            ]
        );
    }

    #[test]
    fn leading_emoji_breaks_ties_after_text() {
        let collator = TitleCollator::for_locale(Some("en"));
        assert_eq!(
            sorted(
                &collator,
                &["🚀 Launch", "Launch", "📚 Launch", "Launch pad"]
            ),
            vec!["📚 Launch", "🚀 Launch", "Launch", "Launch pad"]
        );
    }

    #[test]
    fn swedish_places_a_umlaut_after_z() {
        let collator = TitleCollator::for_locale(Some("sv-SE"));
        assert_eq!(
            sorted(&collator, &["ähnlich", "zebra", "apple"]),
            vec!["apple", "zebra", "ähnlich"]
        );
    }

    #[test]
    fn czech_places_ch_after_h() {
        let collator = TitleCollator::for_locale(Some("cs"));
        assert_eq!(
            sorted(&collator, &["chata", "cesta", "hora", "dům"]),
            vec!["cesta", "dům", "hora", "chata"]
        );
    }

    #[test]
    fn danish_places_double_a_after_z() {
        let collator = TitleCollator::for_locale(Some("da"));
        assert_eq!(
            sorted(&collator, &["Aalborg", "Zebra", "Arhus"]),
            vec!["Arhus", "Zebra", "Aalborg"]
        );
    }

    #[test]
    fn binary_collator_keeps_codepoint_order() {
        let collator = TitleCollator::binary();
        assert_eq!(collator.locale(), None);
        assert_eq!(
            sorted(
                &collator,
                &["apple", "Zebra", "Note 10", "Note 2", "🚀 Launch", "Émile"]
            ),
            vec!["Note 10", "Note 2", "Zebra", "apple", "Émile", "🚀 Launch"]
        );
    }

    #[test]
    fn sort_by_text_orders_items_and_keeps_ties_stable() {
        let collator = TitleCollator::for_locale(Some("en"));
        let mut items = vec![
            (1, "Zebra"),
            (2, "apple"),
            (3, "🚀 apple"),
            (4, "Note 10"),
            (5, "apple"),
            (6, "Note 2"),
        ];
        collator.sort_by_text(&mut items, |(_, text)| text);
        assert_eq!(
            items,
            vec![
                (3, "🚀 apple"),
                (2, "apple"),
                (5, "apple"),
                (6, "Note 2"),
                (4, "Note 10"),
                (1, "Zebra"),
            ]
        );
    }

    #[test]
    fn apply_order_permutes_in_place() {
        let mut items = vec!["a", "b", "c", "d", "e"];
        apply_order(&mut items, &[4, 2, 0, 1, 3]);
        assert_eq!(items, vec!["e", "c", "a", "b", "d"]);
    }

    #[test]
    fn default_collator_is_binary() {
        assert_eq!(TitleCollator::default().locale(), None);
    }

    #[test]
    fn posix_style_locale_is_accepted() {
        let collator = TitleCollator::for_locale(Some("sv_SE.UTF-8"));
        assert_eq!(collator.locale(), Some("sv-SE"));
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        let collator = TitleCollator::for_locale(Some("not a locale"));
        assert_eq!(collator.locale(), Some("en"));
    }

    #[test]
    fn strip_emoji_keeps_digits_text_and_text_symbols() {
        assert_eq!(strip_emoji("2026 plan"), "2026 plan");
        assert_eq!(strip_emoji("#tag"), "#tag");
        assert_eq!(strip_emoji("© 2024 Notes"), "© 2024 Notes");
        assert_eq!(strip_emoji("™ Brand"), "™ Brand");
        assert_eq!(strip_emoji("🚀© Notes"), "© Notes");
        assert_eq!(strip_emoji("Plain"), "Plain");
        assert_eq!(strip_emoji(""), "");
        assert!(matches!(strip_emoji("Plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn strip_emoji_removes_emoji_clusters_anywhere() {
        assert_eq!(strip_emoji("🔥🔥 Hot"), "Hot");
        assert_eq!(strip_emoji("✅ Done"), "Done");
        assert_eq!(strip_emoji("☀️ Sunny"), "Sunny");
        assert_eq!(strip_emoji("🏳️‍🌈 Pride"), "Pride");
        assert_eq!(strip_emoji("🇺🇸 Travel"), "Travel");
        assert_eq!(strip_emoji("👍🏽 Good"), "Good");
        assert_eq!(strip_emoji("👨‍👩‍👧 Family"), "Family");
        assert_eq!(strip_emoji("Launch 🚀 pad"), "Launch pad");
        assert_eq!(strip_emoji("Done ✅"), "Done");
        assert_eq!(strip_emoji("Trip 🇺🇸🇫🇷 notes"), "Trip notes");
        assert_eq!(strip_emoji("Rocket🚀Science"), "Rocket Science");
        assert_eq!(strip_emoji("Rocket🚀 Science"), "Rocket Science");
        assert_eq!(strip_emoji("🚀"), "");
    }
}
