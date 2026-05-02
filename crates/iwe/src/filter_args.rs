use clap::Args;

use liwe::model::Key;
use liwe::query::{
    parse_filter_expression, Filter, InclusionAnchor, KeyOp, ReferenceAnchor,
};

const LEGACY_ALIAS_DEPTH: u32 = 1;

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

    fn inclusion_anchor(&self, default_depth: Option<u8>) -> InclusionAnchor {
        let max = resolve_depth(self.depth, default_depth);
        InclusionAnchor::with_max(self.key.to_string(), max)
    }

    fn reference_anchor(&self, default_depth: Option<u8>) -> ReferenceAnchor {
        let max = resolve_depth(self.depth, default_depth);
        ReferenceAnchor::with_max(self.key.to_string(), max)
    }
}

fn resolve_depth(explicit: Option<u8>, default: Option<u8>) -> u32 {
    let raw = explicit.or(default).unwrap_or(1);
    if raw == 0 {
        u32::MAX
    } else {
        u32::from(raw)
    }
}

#[derive(Debug, Args, Clone, Default)]
pub struct FilterArgs {
    #[clap(
        long,
        help = "Filter expression. Inline YAML; wrapped in `{}` and parsed as a filter document. Example: --filter 'status: pending'."
    )]
    pub filter: Option<String>,

    #[clap(
        long,
        short = 'k',
        help = "Match by document key. Repeatable: 1 key uses $eq, 2+ uses $in."
    )]
    pub key: Vec<String>,

    #[clap(
        long,
        value_parser = parse_key_depth,
        help = "$includes anchor. KEY or KEY:DEPTH (DEPTH defaults to --max-depth). Lowers to scalar shorthand when DEPTH=1, full form { match: { $key: KEY }, maxDepth: N } otherwise. Repeatable; anchors are ANDed."
    )]
    pub includes: Vec<KeyDepth>,

    #[clap(
        long = "included-by",
        value_parser = parse_key_depth,
        help = "$includedBy anchor. KEY or KEY:DEPTH (DEPTH defaults to --max-depth). Lowers to scalar shorthand when DEPTH=1, full form { match: { $key: KEY }, maxDepth: N } otherwise. Repeatable; anchors are ANDed."
    )]
    pub included_by: Vec<KeyDepth>,

    #[clap(
        long,
        value_parser = parse_key_depth,
        help = "$references anchor. KEY or KEY:DIST (DIST defaults to --max-distance). Lowers to scalar shorthand when DIST=1, full form { match: { $key: KEY }, maxDistance: N } otherwise. Repeatable; anchors are ANDed."
    )]
    pub references: Vec<KeyDepth>,

    #[clap(
        long = "referenced-by",
        value_parser = parse_key_depth,
        help = "$referencedBy anchor. KEY or KEY:DIST (DIST defaults to --max-distance). Lowers to scalar shorthand when DIST=1, full form { match: { $key: KEY }, maxDistance: N } otherwise. Repeatable; anchors are ANDed."
    )]
    pub referenced_by: Vec<KeyDepth>,

    #[clap(
        long = "in",
        hide = true,
        value_parser = parse_key_depth,
    )]
    pub in_: Vec<KeyDepth>,

    #[clap(long = "in-any", hide = true)]
    pub in_any: Vec<String>,

    #[clap(long = "not-in", hide = true)]
    pub not_in: Vec<String>,

    #[clap(long = "refs-to", hide = true)]
    pub refs_to: Option<String>,

    #[clap(long = "refs-from", hide = true)]
    pub refs_from: Option<String>,

    #[clap(
        long,
        help = "Only match root documents (those with no incoming inclusion edges)."
    )]
    pub roots: bool,

    #[clap(
        long = "max-depth",
        help = "Default maxDepth applied to inclusion anchor flags without a colon-suffix. Default 1."
    )]
    pub max_depth: Option<u8>,

    #[clap(
        long = "max-distance",
        help = "Default maxDistance applied to reference anchor flags without a colon-suffix. Default 1."
    )]
    pub max_distance: Option<u8>,
}

impl FilterArgs {
    pub fn has_non_key_clauses(&self) -> bool {
        self.filter.is_some()
            || self.roots
            || !self.includes.is_empty()
            || !self.included_by.is_empty()
            || !self.references.is_empty()
            || !self.referenced_by.is_empty()
            || !self.in_.is_empty()
            || !self.in_any.is_empty()
            || !self.not_in.is_empty()
            || self.refs_to.is_some()
            || self.refs_from.is_some()
            || self.max_depth.is_some()
            || self.max_distance.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.filter.is_none()
            && self.key.is_empty()
            && !self.roots
            && self.includes.is_empty()
            && self.included_by.is_empty()
            && self.references.is_empty()
            && self.referenced_by.is_empty()
            && self.in_.is_empty()
            && self.in_any.is_empty()
            && self.not_in.is_empty()
            && self.refs_to.is_none()
            && self.refs_from.is_none()
            && self.max_depth.is_none()
            && self.max_distance.is_none()
    }

    pub fn to_filter(&self) -> Result<Option<Filter>, String> {
        if self.is_empty() {
            return Ok(None);
        }
        let mut conjuncts: Vec<Filter> = Vec::new();

        if let Some(expr) = &self.filter {
            let parsed = parse_filter_expression(expr)
                .map_err(|e| format!("invalid --filter expression: {}", e))?;
            if !self.key.is_empty() && filter_has_top_level_key(&parsed) {
                return Err(
                    "-k / --key conflicts with a $key predicate at the top level of --filter; \
                     use --filter '$or: [{$key: a}, {$key: b}]' for OR-of-keys, or pick one source"
                        .to_string(),
                );
            }
            conjuncts.push(parsed);
        }

        match self.key.len() {
            0 => {}
            1 => conjuncts.push(Filter::Key(KeyOp::Eq(Key::name(&self.key[0])))),
            _ => conjuncts.push(Filter::Key(KeyOp::In(
                self.key.iter().map(|s| Key::name(s)).collect(),
            ))),
        }

        for kd in &self.includes {
            conjuncts.push(Filter::Includes(Box::new(kd.inclusion_anchor(self.max_depth))));
        }
        for kd in &self.included_by {
            conjuncts.push(Filter::IncludedBy(Box::new(kd.inclusion_anchor(self.max_depth))));
        }
        for kd in &self.references {
            conjuncts.push(Filter::References(Box::new(kd.reference_anchor(self.max_distance))));
        }
        for kd in &self.referenced_by {
            conjuncts.push(Filter::ReferencedBy(Box::new(kd.reference_anchor(self.max_distance))));
        }

        for kd in &self.in_ {
            eprintln!("warning: --in is deprecated; use --included-by");
            conjuncts.push(Filter::IncludedBy(Box::new(kd.inclusion_anchor(self.max_depth))));
        }
        if !self.in_any.is_empty() {
            eprintln!(
                "warning: --in-any is deprecated; use --filter '$or: [{{ $includedBy: K1 }}, {{ $includedBy: K2 }}]'"
            );
            conjuncts.push(Filter::Or(
                self.in_any
                    .iter()
                    .map(|k| Filter::IncludedBy(Box::new(InclusionAnchor::with_max(k, LEGACY_ALIAS_DEPTH))))
                    .collect(),
            ));
        }
        for k in &self.not_in {
            eprintln!("warning: --not-in is deprecated; use --filter '$not: {{ $includedBy: ... }}'");
            conjuncts.push(Filter::Not(Box::new(Filter::IncludedBy(Box::new(
                InclusionAnchor::with_max(k, LEGACY_ALIAS_DEPTH),
            )))));
        }
        if let Some(k) = &self.refs_to {
            eprintln!("warning: --refs-to is deprecated; use --references");
            conjuncts.push(Filter::Or(vec![
                Filter::Includes(Box::new(InclusionAnchor::with_max(k, LEGACY_ALIAS_DEPTH))),
                Filter::References(Box::new(ReferenceAnchor::with_max(k, LEGACY_ALIAS_DEPTH))),
            ]));
        }
        if let Some(k) = &self.refs_from {
            eprintln!("warning: --refs-from is deprecated; use --referenced-by");
            conjuncts.push(Filter::Or(vec![
                Filter::IncludedBy(Box::new(InclusionAnchor::with_max(k, LEGACY_ALIAS_DEPTH))),
                Filter::ReferencedBy(Box::new(ReferenceAnchor::with_max(k, LEGACY_ALIAS_DEPTH))),
            ]));
        }

        if conjuncts.is_empty() {
            return Ok(None);
        }
        if conjuncts.len() == 1 {
            return Ok(Some(conjuncts.into_iter().next().unwrap()));
        }
        Ok(Some(Filter::And(conjuncts)))
    }
}

fn filter_has_top_level_key(filter: &Filter) -> bool {
    match filter {
        Filter::Key(_) => true,
        Filter::And(children) => children.iter().any(filter_has_top_level_key),
        _ => false,
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

