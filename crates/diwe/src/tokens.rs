use std::sync::OnceLock;

use tiktoken_rs::{o200k_base, CoreBPE};

fn bpe() -> &'static CoreBPE {
    static BPE: OnceLock<CoreBPE> = OnceLock::new();
    BPE.get_or_init(|| o200k_base().expect("load o200k_base"))
}

pub fn count_tokens(text: &str) -> usize {
    bpe().count_ordinary(text)
}

pub fn truncate_to_tokens(text: &str, max: usize) -> (String, usize) {
    let toks = bpe().encode_ordinary(text);
    if toks.len() <= max {
        return (text.to_string(), 0);
    }
    let bytes = bpe().decode_bytes(&toks[..max]).unwrap_or_default();
    (
        String::from_utf8_lossy(&bytes).into_owned(),
        toks.len() - max,
    )
}

pub fn truncation_marker(omitted: usize) -> String {
    format!("\n\n⋯ truncated ({} tokens omitted)", omitted)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Budget {
    pub limit: Option<usize>,
    pub max_tokens: Option<usize>,
    pub max_document_tokens: Option<usize>,
}

impl Budget {
    pub fn is_active(&self) -> bool {
        self.limit.is_some_and(|l| l > 0)
            || self.max_tokens.is_some_and(|t| t > 0)
            || self.max_document_tokens.is_some_and(|t| t > 0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Truncation {
    pub emitted: usize,
    pub matched: usize,
    pub clipped: Vec<String>,
    pub tokens: usize,
    pub budget: Option<usize>,
}

impl Truncation {
    pub fn is_truncated(&self) -> bool {
        self.emitted < self.matched || !self.clipped.is_empty()
    }
}

pub fn apply_budget<T, K, C, Cap>(
    items: &mut Vec<T>,
    budget: &Budget,
    matched: usize,
    key_of: K,
    content_tokens: C,
    mut cap_content: Cap,
) -> Truncation
where
    K: Fn(&T) -> String,
    C: Fn(&T) -> usize,
    Cap: FnMut(&mut T, usize) -> Option<usize>,
{
    if let Some(limit) = budget.limit.filter(|&l| l > 0) {
        items.truncate(limit);
    }

    let mut clipped = Vec::new();
    if let Some(max_doc) = budget.max_document_tokens.filter(|&m| m > 0) {
        for item in items.iter_mut() {
            if cap_content(item, max_doc).is_some() {
                clipped.push(key_of(item));
            }
        }
    }

    let mut total = 0usize;
    if let Some(max_total) = budget.max_tokens.filter(|&m| m > 0) {
        let mut running = 0usize;
        let mut kept = items.len();
        for (index, item) in items.iter().enumerate() {
            let item_tokens = content_tokens(item);
            if running > 0 && running + item_tokens > max_total {
                kept = index;
                break;
            }
            running += item_tokens;
        }
        items.truncate(kept);
        total = running;
    } else {
        for item in items.iter() {
            total += content_tokens(item);
        }
    }

    Truncation {
        emitted: items.len(),
        matched,
        clipped,
        tokens: total,
        budget: budget.max_tokens.filter(|&m| m > 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_tokens_counts_known_strings() {
        assert_eq!(count_tokens(""), 0);
        assert_eq!(count_tokens("hello"), 1);
        assert_eq!(count_tokens("hello world"), 2);
    }

    #[test]
    fn truncate_to_tokens_exact_fit_returns_input_and_zero() {
        let text = "hello world";
        let (head, omitted) = truncate_to_tokens(text, 2);
        assert_eq!(head, "hello world");
        assert_eq!(omitted, 0);
    }

    #[test]
    fn truncate_to_tokens_under_limit_returns_input_and_zero() {
        let text = "hello world";
        let (head, omitted) = truncate_to_tokens(text, 10);
        assert_eq!(head, "hello world");
        assert_eq!(omitted, 0);
    }

    #[test]
    fn truncate_to_tokens_multibyte_never_drops_the_body() {
        let text = "日本語のテキストをここにたくさん書いています。";
        let full = count_tokens(text);
        assert!(full > 1);
        for max in 1..full {
            let (head, omitted) = truncate_to_tokens(text, max);
            assert!(
                !head.is_empty(),
                "truncating to {} tokens dropped the whole body",
                max
            );
            assert_eq!(omitted, full - max);
        }
    }

    #[test]
    fn truncate_to_tokens_over_limit_returns_head_and_omitted_count() {
        let text = "one two three four five";
        let full = count_tokens(text);
        let (head, omitted) = truncate_to_tokens(text, 2);
        assert_eq!(head, "one two");
        assert_eq!(omitted, full - 2);
    }

    #[test]
    fn apply_budget_limit_keeps_prefix() {
        let mut items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let budget = Budget {
            limit: Some(2),
            max_tokens: None,
            max_document_tokens: None,
        };
        let report = apply_budget(
            &mut items,
            &budget,
            3,
            |s| s.clone(),
            |s| count_tokens(s),
            |_s, _max| None,
        );
        assert_eq!(items, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(report.emitted, 2);
        assert_eq!(report.matched, 3);
        assert!(report.is_truncated());
    }

    #[test]
    fn apply_budget_max_tokens_drops_trailing_whole_documents() {
        let mut items = vec![
            "one two three".to_string(),
            "four five six".to_string(),
            "seven eight nine".to_string(),
        ];
        let budget = Budget {
            limit: None,
            max_tokens: Some(4),
            max_document_tokens: None,
        };
        let report = apply_budget(
            &mut items,
            &budget,
            3,
            |s| s.clone(),
            |s| count_tokens(s),
            |_s, _max| None,
        );
        assert_eq!(items, vec!["one two three".to_string()]);
        assert_eq!(report.emitted, 1);
        assert_eq!(report.tokens, 3);
        assert_eq!(report.budget, Some(4));
    }

    #[test]
    fn apply_budget_always_keeps_first_document() {
        let mut items = vec!["one two three four five".to_string(), "six".to_string()];
        let budget = Budget {
            limit: None,
            max_tokens: Some(1),
            max_document_tokens: None,
        };
        let report = apply_budget(
            &mut items,
            &budget,
            2,
            |s| s.clone(),
            |s| count_tokens(s),
            |_s, _max| None,
        );
        assert_eq!(items, vec!["one two three four five".to_string()]);
        assert_eq!(report.emitted, 1);
    }

    #[test]
    fn apply_budget_max_document_tokens_caps_and_records_clipped() {
        let mut items = vec!["one two three four five".to_string()];
        let budget = Budget {
            limit: None,
            max_tokens: None,
            max_document_tokens: Some(2),
        };
        let report = apply_budget(
            &mut items,
            &budget,
            1,
            |s| s.clone(),
            |s| count_tokens(s),
            |s, max| {
                let (head, omitted) = truncate_to_tokens(s, max);
                if omitted > 0 {
                    *s = head;
                    Some(omitted)
                } else {
                    None
                }
            },
        );
        assert_eq!(items, vec!["one two".to_string()]);
        assert_eq!(report.clipped, vec!["one two".to_string()]);
        assert!(report.is_truncated());
    }
}
