use std::collections::HashSet;

use bm25::{Language, Tokenizer};
use deunicode::deunicode_with_tofu;
use rust_stemmers::{Algorithm, Stemmer};
use stop_words::LANGUAGE;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenizerMode {
    Document,
    Query,
}

pub struct LexicalTokenizer {
    stemmer: Stemmer,
    stop_words: HashSet<String>,
    mode: TokenizerMode,
}

impl LexicalTokenizer {
    pub fn new(language: &Language, mode: TokenizerMode) -> Self {
        let stop_words = LANGUAGE::try_from(language)
            .map(|list| {
                stop_words::get(list)
                    .iter()
                    .map(|word| word.to_string())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            stemmer: Stemmer::create(Algorithm::from(language)),
            stop_words,
            mode,
        }
    }

    fn push_words(&self, run: &str, tokens: &mut Vec<String>) {
        for word in run.unicode_words() {
            if self.stop_words.contains(word) {
                continue;
            }
            let stem = self.stemmer.stem(word);
            if stem.is_ascii() {
                tokens.push(stem.into_owned());
                continue;
            }
            let folded = deunicode_with_tofu(&stem, "").to_lowercase();
            tokens.extend(folded.unicode_words().map(str::to_string));
        }
    }
}

impl Tokenizer for LexicalTokenizer {
    fn tokenize(&self, input_text: &str) -> Vec<String> {
        let text = input_text.to_lowercase();
        let mut tokens = Vec::new();
        let mut rest = text.as_str();
        while let Some(first) = rest.chars().next() {
            let cjk = is_cjk(first);
            let end = rest.find(|c: char| is_cjk(c) != cjk).unwrap_or(rest.len());
            let (run, tail) = rest.split_at(end);
            if cjk {
                push_cjk(run, self.mode, &mut tokens);
            } else {
                self.push_words(run, &mut tokens);
            }
            rest = tail;
        }
        tokens
    }
}

fn push_cjk(run: &str, mode: TokenizerMode, tokens: &mut Vec<String>) {
    let chars: Vec<char> = run.chars().collect();
    if chars.len() == 1 {
        tokens.push(run.to_string());
        return;
    }
    match mode {
        TokenizerMode::Query => {
            tokens.extend(chars.windows(2).map(|pair| pair.iter().collect::<String>()));
        }
        TokenizerMode::Document => {
            for (position, &c) in chars.iter().enumerate() {
                tokens.push(c.to_string());
                if let Some(&next) = chars.get(position + 1) {
                    tokens.push([c, next].iter().collect());
                }
            }
        }
    }
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x11FF
            | 0x3005
            | 0x3041..=0x309F
            | 0x30A1..=0x30FA
            | 0x30FC..=0x30FF
            | 0x3130..=0x318F
            | 0x31F0..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFF66..=0xFF9F
            | 0x20000..=0x2FA1F
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(language: Language, text: &str) -> Vec<String> {
        LexicalTokenizer::new(&language, TokenizerMode::Query).tokenize(text)
    }

    fn document_tokens(language: Language, text: &str) -> Vec<String> {
        LexicalTokenizer::new(&language, TokenizerMode::Document).tokenize(text)
    }

    #[test]
    fn stems_cyrillic_inflections_to_one_token() {
        assert_eq!(
            tokens(Language::Russian, "книгой книги книга"),
            vec!["knig", "knig", "knig"]
        );
    }

    #[test]
    fn stems_greek_inflections_to_one_token() {
        assert_eq!(
            tokens(Language::Greek, "ανθρώπων άνθρωπος ανθρώπους"),
            vec!["anthrop", "anthrop", "anthrop"]
        );
    }

    #[test]
    fn stems_arabic_inflections_to_one_token() {
        assert_eq!(
            tokens(Language::Arabic, "المكتبات مكتبة المكتبة"),
            vec!["mktb", "mktb", "mktb"]
        );
    }

    #[test]
    fn stems_accented_latin_before_folding() {
        assert_eq!(
            tokens(Language::French, "préférées préféré"),
            vec!["prefer", "prefer"]
        );
        assert_eq!(
            tokens(Language::Portuguese, "informações informação"),
            vec!["inform", "inform"]
        );
        assert_eq!(tokens(Language::Turkish, "kızlar kız"), vec!["kiz", "kiz"]);
    }

    #[test]
    fn folds_accents_so_unaccented_queries_match() {
        assert_eq!(tokens(Language::Spanish, "niños nino"), vec!["nin", "nin"]);
    }

    #[test]
    fn drops_stop_words_in_original_script() {
        assert_eq!(tokens(Language::Russian, "и книга не"), vec!["knig"]);
        assert_eq!(tokens(Language::English, "the jumping of"), vec!["jump"]);
    }

    #[test]
    fn splits_han_into_overlapping_pairs() {
        assert_eq!(tokens(Language::English, "苹果树"), vec!["苹果", "果树"]);
    }

    #[test]
    fn keeps_a_lone_cjk_character_as_one_token() {
        assert_eq!(tokens(Language::English, "猫"), vec!["猫"]);
    }

    #[test]
    fn document_mode_emits_single_characters_and_overlapping_pairs() {
        assert_eq!(
            document_tokens(Language::English, "苹果树"),
            vec!["苹", "苹果", "果", "果树", "树"]
        );
    }

    #[test]
    fn document_mode_keeps_a_lone_cjk_character_once() {
        assert_eq!(document_tokens(Language::English, "猫"), vec!["猫"]);
    }

    #[test]
    fn document_mode_breaks_cjk_runs_at_punctuation_and_spaces() {
        assert_eq!(
            document_tokens(Language::English, "猫を。사과 먹"),
            vec!["猫", "猫を", "を", "사", "사과", "과", "먹"]
        );
    }

    #[test]
    fn document_mode_leaves_non_cjk_words_unchanged() {
        assert_eq!(
            document_tokens(Language::English, "apples苹果 trees"),
            vec!["appl", "苹", "苹果", "果", "tree"]
        );
        assert_eq!(
            document_tokens(Language::Russian, "и книга не"),
            tokens(Language::Russian, "и книга не")
        );
    }

    #[test]
    fn breaks_cjk_pairs_at_punctuation_and_spaces() {
        assert_eq!(
            tokens(Language::English, "猫を。사과를 먹다"),
            vec!["猫を", "사과", "과를", "먹다"]
        );
    }

    #[test]
    fn mixes_cjk_pairs_with_stemmed_words() {
        assert_eq!(
            tokens(Language::English, "apples苹果 trees"),
            vec!["appl", "苹果", "tree"]
        );
    }

    #[test]
    fn stems_tamil_without_a_stop_word_list() {
        assert_eq!(
            tokens(Language::Tamil, "வீடுகள் வீட்டில் வீடு"),
            vec!["viittu", "viittu", "viittu"]
        );
    }

    #[test]
    fn lowercases_non_latin_scripts_before_stemming() {
        assert_eq!(
            tokens(Language::Russian, "КНИГОЙ Книги"),
            vec!["knig", "knig"]
        );
    }

    #[test]
    fn stems_english_word_forms_to_one_token() {
        assert_eq!(
            tokens(Language::English, "Jumping jumped jumps"),
            vec!["jump", "jump", "jump"]
        );
    }

    #[test]
    fn folds_a_non_ascii_stem_to_the_same_token_as_its_ascii_spelling() {
        assert_eq!(
            tokens(Language::English, "naïve naive"),
            vec!["naiv", "naiv"]
        );
        assert_eq!(
            tokens(Language::German, "Größe Grüße"),
            vec!["gross", "gruss"]
        );
    }

    #[test]
    fn splits_a_folded_stem_that_becomes_several_words() {
        assert_eq!(tokens(Language::English, "½ ﬁle"), vec!["1", "2", "file"]);
    }

    #[test]
    fn keeps_identifiers_and_numbers_as_word_boundaries_allow() {
        assert_eq!(
            tokens(Language::English, "v2 snake_case well-known 3.14 don't"),
            vec!["v2", "snake_cas", "well", "known", "3.14"]
        );
    }

    #[test]
    fn pairs_kana_with_the_prolonged_sound_mark_and_breaks_at_the_middle_dot() {
        assert_eq!(
            tokens(Language::English, "コーヒー・カップ"),
            vec!["コー", "ーヒ", "ヒー", "カッ", "ップ"]
        );
    }

    #[test]
    fn pairs_mixed_kanji_and_hiragana_as_one_run() {
        assert_eq!(
            tokens(Language::English, "りんご 食べた"),
            vec!["りん", "んご", "食べ", "べた"]
        );
    }

    #[test]
    fn pairs_hangul_syllables_within_a_word() {
        assert_eq!(tokens(Language::English, "먹었다"), vec!["먹었", "었다"]);
    }

    #[test]
    fn drops_cjk_brackets() {
        assert_eq!(tokens(Language::English, "「紅茶」"), vec!["紅茶"]);
    }

    #[test]
    fn pairs_halfwidth_katakana_without_widening() {
        assert_eq!(tokens(Language::English, "ｺｰﾋｰ"), vec!["ｺｰ", "ｰﾋ", "ﾋｰ"]);
    }

    #[test]
    fn empty_text_has_no_tokens() {
        assert_eq!(tokens(Language::English, ""), Vec::<String>::new());
    }
}
