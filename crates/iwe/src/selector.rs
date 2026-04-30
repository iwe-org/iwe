use clap::Args;

use liwe::model::Key;
use liwe::query::{Filter, InclusionAnchor};

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

    fn anchor(&self, default_depth: Option<u8>) -> InclusionAnchor {
        let max = self
            .depth
            .or(default_depth)
            .map(u32::from)
            .unwrap_or(u32::MAX);
        InclusionAnchor::with_max(self.key.to_string(), max)
    }
}

#[derive(Debug, Args, Clone, Default)]
pub struct SelectorArgs {
    #[clap(
        long = "in",
        value_parser = parse_key_depth,
        help = "Restrict to sub-documents of EVERY listed key (AND). Use KEY or KEY:DEPTH. Repeat for multiple."
    )]
    pub in_: Vec<KeyDepth>,

    #[clap(
        long = "in-any",
        value_parser = parse_key_depth,
        help = "Restrict to sub-documents of AT LEAST ONE listed key (OR). Use KEY or KEY:DEPTH."
    )]
    pub in_any: Vec<KeyDepth>,

    #[clap(
        long = "not-in",
        value_parser = parse_key_depth,
        help = "Exclude sub-documents of any listed key (NOT). Use KEY or KEY:DEPTH."
    )]
    pub not_in: Vec<KeyDepth>,

    #[clap(
        long = "max-depth",
        help = "Default depth applied to in / in-any / not-in entries without their own depth. Omit for unbounded."
    )]
    pub max_depth: Option<u8>,
}

impl SelectorArgs {
    pub fn is_empty(&self) -> bool {
        self.in_.is_empty()
            && self.in_any.is_empty()
            && self.not_in.is_empty()
            && self.max_depth.is_none()
    }

    pub fn to_filter(&self) -> Option<Filter> {
        if self.is_empty() {
            return None;
        }
        let mut conjuncts: Vec<Filter> = Vec::new();
        for kd in &self.in_ {
            conjuncts.push(Filter::IncludedBy(vec![kd.anchor(self.max_depth)]));
        }
        if !self.in_any.is_empty() {
            conjuncts.push(Filter::Or(
                self.in_any
                    .iter()
                    .map(|kd| Filter::IncludedBy(vec![kd.anchor(self.max_depth)]))
                    .collect(),
            ));
        }
        for kd in &self.not_in {
            conjuncts.push(Filter::Not(Box::new(Filter::IncludedBy(vec![
                kd.anchor(self.max_depth),
            ]))));
        }
        Some(Filter::And(conjuncts))
    }
}

fn parse_key_depth(s: &str) -> Result<KeyDepth, String> {
    if let Some((k, d)) = s.rsplit_once(':') {
        let depth: u8 = d
            .parse()
            .map_err(|_| format!("invalid depth in '{}': expected non-negative integer", s))?;
        Ok(KeyDepth::with_depth(Key::name(k), depth))
    } else {
        Ok(KeyDepth::bare(Key::name(s)))
    }
}
