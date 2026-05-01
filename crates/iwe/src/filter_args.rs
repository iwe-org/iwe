use std::sync::atomic::{AtomicBool, Ordering};

use clap::Args;

use liwe::model::Key;
use liwe::query::{
    parse_filter_expression, CountArg, Filter, InclusionAnchor, KeyOp, MaxDepth, NumExpr, NumOp,
    ReferenceAnchor,
};

const LEGACY_REFS_DEPTH: u32 = 1;

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
    explicit
        .or(default)
        .map(u32::from)
        .unwrap_or(1)
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
        long = "includes-count",
        help = "$includesCount predicate. Lowers to direct-edge shorthand when --max-depth is unset or 1, full form { count: N, maxDepth: M } otherwise."
    )]
    pub includes_count: Option<u64>,

    #[clap(
        long = "included-by-count",
        help = "$includedByCount predicate. Lowers to direct-edge shorthand when --max-depth is unset or 1, full form { count: N, maxDepth: M } otherwise."
    )]
    pub included_by_count: Option<u64>,

    #[clap(
        long = "in",
        hide = true,
        value_parser = parse_key_depth,
    )]
    pub in_: Vec<KeyDepth>,

    #[clap(
        long = "in-any",
        hide = true,
        value_parser = parse_key_depth,
    )]
    pub in_any: Vec<KeyDepth>,

    #[clap(
        long = "not-in",
        hide = true,
        value_parser = parse_key_depth,
    )]
    pub not_in: Vec<KeyDepth>,

    #[clap(long = "refs-to", hide = true)]
    pub refs_to: Option<String>,

    #[clap(long = "refs-from", hide = true)]
    pub refs_from: Option<String>,

    #[clap(long, hide = true)]
    pub roots: bool,

    #[clap(
        long = "max-depth",
        help = "Default maxDepth applied to inclusion anchor flags without a colon-suffix and to count predicates. Default 1."
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
            || !self.includes.is_empty()
            || !self.included_by.is_empty()
            || !self.references.is_empty()
            || !self.referenced_by.is_empty()
            || self.includes_count.is_some()
            || self.included_by_count.is_some()
            || !self.in_.is_empty()
            || !self.in_any.is_empty()
            || !self.not_in.is_empty()
            || self.refs_to.is_some()
            || self.refs_from.is_some()
            || self.roots
            || self.max_depth.is_some()
            || self.max_distance.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.filter.is_none()
            && self.key.is_empty()
            && self.includes.is_empty()
            && self.included_by.is_empty()
            && self.references.is_empty()
            && self.referenced_by.is_empty()
            && self.includes_count.is_none()
            && self.included_by_count.is_none()
            && self.in_.is_empty()
            && self.in_any.is_empty()
            && self.not_in.is_empty()
            && self.refs_to.is_none()
            && self.refs_from.is_none()
            && !self.roots
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
            conjuncts.push(Filter::Includes(vec![kd.inclusion_anchor(self.max_depth)]));
        }
        for kd in &self.included_by {
            conjuncts.push(Filter::IncludedBy(vec![kd.inclusion_anchor(self.max_depth)]));
        }
        for kd in &self.references {
            conjuncts.push(Filter::References(vec![kd.reference_anchor(self.max_distance)]));
        }
        for kd in &self.referenced_by {
            conjuncts.push(Filter::ReferencedBy(vec![kd.reference_anchor(self.max_distance)]));
        }

        if let Some(n) = self.includes_count {
            conjuncts.push(Filter::IncludesCount(count_with_default_depth(n, self.max_depth)));
        }
        if let Some(n) = self.included_by_count {
            conjuncts.push(Filter::IncludedByCount(count_with_default_depth(n, self.max_depth)));
        }

        for kd in &self.in_ {
            warn_once_in();
            conjuncts.push(Filter::IncludedBy(vec![kd.inclusion_anchor(self.max_depth)]));
        }
        if !self.in_any.is_empty() {
            warn_once_in_any();
            conjuncts.push(Filter::Or(
                self.in_any
                    .iter()
                    .map(|kd| Filter::IncludedBy(vec![kd.inclusion_anchor(self.max_depth)]))
                    .collect(),
            ));
        }
        for kd in &self.not_in {
            warn_once_not_in();
            conjuncts.push(Filter::Not(Box::new(Filter::IncludedBy(vec![
                kd.inclusion_anchor(self.max_depth),
            ]))));
        }
        if let Some(k) = &self.refs_to {
            warn_once_refs_to();
            conjuncts.push(Filter::Or(vec![
                Filter::Includes(vec![InclusionAnchor::with_max(k, LEGACY_REFS_DEPTH)]),
                Filter::References(vec![ReferenceAnchor::with_max(k, LEGACY_REFS_DEPTH)]),
            ]));
        }
        if let Some(k) = &self.refs_from {
            warn_once_refs_from();
            conjuncts.push(Filter::Or(vec![
                Filter::IncludedBy(vec![InclusionAnchor::with_max(k, LEGACY_REFS_DEPTH)]),
                Filter::ReferencedBy(vec![ReferenceAnchor::with_max(k, LEGACY_REFS_DEPTH)]),
            ]));
        }
        if self.roots {
            warn_once_roots();
            conjuncts.push(Filter::IncludedByCount(direct_count(0)));
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

fn direct_count(n: u64) -> CountArg {
    CountArg {
        count: NumExpr(vec![NumOp::Eq(n)]),
        min_depth: 1,
        max_depth: MaxDepth::Bounded(1),
    }
}

fn count_with_default_depth(n: u64, default_depth: Option<u8>) -> CountArg {
    let depth = default_depth.map(u32::from).unwrap_or(1).max(1);
    CountArg {
        count: NumExpr(vec![NumOp::Eq(n)]),
        min_depth: 1,
        max_depth: MaxDepth::Bounded(depth),
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

macro_rules! warn_once {
    ($flag:ident, $msg:literal) => {{
        static $flag: AtomicBool = AtomicBool::new(false);
        if !$flag.swap(true, Ordering::Relaxed) {
            eprintln!("warning: {}", $msg);
        }
    }};
}

fn warn_once_in() {
    warn_once!(WARNED_IN, "--in is deprecated; use --included-by");
}

fn warn_once_in_any() {
    warn_once!(
        WARNED_IN_ANY,
        "--in-any is deprecated; use --filter '$or: [{ $includedBy: K1 }, { $includedBy: K2 }]'"
    );
}

fn warn_once_not_in() {
    warn_once!(
        WARNED_NOT_IN,
        "--not-in is deprecated; use --filter '$not: { $includedBy: ... }'"
    );
}

fn warn_once_refs_to() {
    warn_once!(WARNED_REFS_TO, "--refs-to is deprecated; use --references");
}

fn warn_once_refs_from() {
    warn_once!(
        WARNED_REFS_FROM,
        "--refs-from is deprecated; use --referenced-by"
    );
}

fn warn_once_roots() {
    warn_once!(
        WARNED_ROOTS,
        "--roots is deprecated; use --included-by-count 0"
    );
}
