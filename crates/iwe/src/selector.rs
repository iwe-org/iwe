use clap::Args;

use liwe::model::Key;
use liwe::selector::{KeyDepth, Selector};

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

impl From<SelectorArgs> for Selector {
    fn from(args: SelectorArgs) -> Self {
        Selector {
            in_: args.in_,
            in_any: args.in_any,
            not_in: args.not_in,
            max_depth: args.max_depth,
        }
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
