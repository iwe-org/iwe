use regex::Regex;
use serde_yaml::{Mapping, Value};

use crate::model::Key;
use crate::query::builder::ParseError;

#[derive(Debug, Clone)]
pub struct BlockRegex {
    pattern: String,
    regex: Regex,
}

impl BlockRegex {
    pub fn compile(pattern: &str) -> Result<Self, ParseError> {
        Regex::new(pattern)
            .map(|regex| BlockRegex {
                pattern: pattern.to_string(),
                regex,
            })
            .map_err(|e| ParseError::InvalidRegex {
                pattern: pattern.to_string(),
                message: e.to_string(),
            })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

impl PartialEq for BlockRegex {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextMatch {
    Substring(String),
    Exact(String),
}

impl TextMatch {
    pub fn matches(&self, text: &str) -> bool {
        match self {
            TextMatch::Substring(s) => text.to_lowercase().contains(&s.to_lowercase()),
            TextMatch::Exact(s) => text.to_lowercase() == s.to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Header,
    Paragraph,
    Item,
    Code,
    Table,
    Ref,
    Hr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockOp {
    Text(TextMatch),
    Matches(BlockRegex),
    Within(Box<BlockPredicate>),
    Contains(Box<BlockPredicate>),
    Section(Box<BlockPredicate>),
    Quote(Box<BlockPredicate>),
    List(Box<BlockPredicate>),
    Type(BlockType, Box<BlockPredicate>),
    References(Key),
    And(Vec<BlockPredicate>),
    Or(Vec<BlockPredicate>),
    Nor(Vec<BlockPredicate>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlockPredicate(pub Vec<BlockOp>);

pub trait IntoBlockPredicate {
    fn into_block_predicate(self) -> BlockPredicate;
}

impl IntoBlockPredicate for BlockPredicate {
    fn into_block_predicate(self) -> BlockPredicate {
        self
    }
}

impl IntoBlockPredicate for &str {
    fn into_block_predicate(self) -> BlockPredicate {
        BlockPredicate::text_exact(self)
    }
}

impl BlockPredicate {
    pub fn empty() -> Self {
        BlockPredicate(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn text_exact(text: &str) -> Self {
        BlockPredicate(vec![BlockOp::Text(TextMatch::Exact(text.to_string()))])
    }

    fn has_direct_text_op(&self) -> bool {
        self.0
            .iter()
            .any(|op| matches!(op, BlockOp::Text(_) | BlockOp::Matches(_)))
    }

    fn carries_contents(&self) -> bool {
        self.is_empty()
            || self.0.iter().any(|op| match op {
                BlockOp::Section(_) | BlockOp::Quote(_) | BlockOp::List(_) => true,
                BlockOp::And(preds) => preds.iter().any(BlockPredicate::carries_contents),
                BlockOp::Or(preds) => preds.iter().all(BlockPredicate::carries_contents),
                _ => false,
            })
    }

    fn with(mut self, op: BlockOp) -> Self {
        self.0.push(op);
        self
    }

    fn typed(self, block_type: BlockType, arg: impl IntoBlockPredicate) -> Self {
        self.with(BlockOp::Type(
            block_type,
            Box::new(arg.into_block_predicate()),
        ))
    }

    pub fn text(self, text: &str) -> Self {
        self.with(BlockOp::Text(TextMatch::Substring(text.to_string())))
    }

    pub fn text_eq(self, text: &str) -> Self {
        self.with(BlockOp::Text(TextMatch::Exact(text.to_string())))
    }

    pub fn matches(self, pattern: &str) -> Self {
        self.with(BlockOp::Matches(
            BlockRegex::compile(pattern).expect("valid regex"),
        ))
    }

    pub fn within(self, scope: BlockPredicate) -> Self {
        assert!(
            scope.carries_contents(),
            "$within argument must carry contents: {{}}, or a predicate containing $section, $quote, or $list"
        );
        self.with(BlockOp::Within(Box::new(scope)))
    }

    pub fn within_section(self, title: &str) -> Self {
        self.within(BlockPredicate::empty().section(title))
    }

    pub fn contains(self, pred: BlockPredicate) -> Self {
        self.with(BlockOp::Contains(Box::new(pred)))
    }

    pub fn section(self, root: impl IntoBlockPredicate) -> Self {
        self.with(BlockOp::Section(Box::new(root.into_block_predicate())))
    }

    pub fn quote(self, inner: BlockPredicate) -> Self {
        self.with(BlockOp::Quote(Box::new(inner)))
    }

    pub fn list(self, inner: BlockPredicate) -> Self {
        self.with(BlockOp::List(Box::new(inner)))
    }

    pub fn header(self, arg: impl IntoBlockPredicate) -> Self {
        self.typed(BlockType::Header, arg)
    }

    pub fn paragraph(self, arg: impl IntoBlockPredicate) -> Self {
        self.typed(BlockType::Paragraph, arg)
    }

    pub fn item(self, arg: impl IntoBlockPredicate) -> Self {
        self.typed(BlockType::Item, arg)
    }

    pub fn code(self, arg: impl IntoBlockPredicate) -> Self {
        self.typed(BlockType::Code, arg)
    }

    pub fn table(self, arg: impl IntoBlockPredicate) -> Self {
        self.typed(BlockType::Table, arg)
    }

    pub fn reference(self, inner: BlockPredicate) -> Self {
        self.typed(BlockType::Ref, inner)
    }

    pub fn hr(self, inner: BlockPredicate) -> Self {
        self.typed(BlockType::Hr, inner)
    }

    pub fn references(self, key: &str) -> Self {
        self.with(BlockOp::References(Key::name(key)))
    }

    pub fn and(self, preds: Vec<BlockPredicate>) -> Self {
        self.with(BlockOp::And(preds))
    }

    pub fn or(self, preds: Vec<BlockPredicate>) -> Self {
        self.with(BlockOp::Or(preds))
    }

    pub fn nor(self, preds: Vec<BlockPredicate>) -> Self {
        self.with(BlockOp::Nor(preds))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchesSource {
    pub pattern: BlockRegex,
    pub scope: BlockPredicate,
}

pub fn parse_block_predicate(
    value: &Value,
    op: &'static str,
) -> Result<BlockPredicate, ParseError> {
    match value {
        Value::Mapping(m) => parse_mapping(m),
        _ => Err(ParseError::OperatorExpectedMapping { op }),
    }
}

fn parse_mapping(map: &Mapping) -> Result<BlockPredicate, ParseError> {
    let mut ops = Vec::new();
    for (k, v) in map {
        let key = k.as_str().ok_or(ParseError::NonStringKey)?;
        if !key.starts_with('$') {
            return Err(ParseError::BareKeyInBlockPredicate {
                key: key.to_string(),
            });
        }
        ops.push(parse_entry(key, v)?);
    }
    Ok(BlockPredicate(ops))
}

fn parse_entry(key: &str, value: &Value) -> Result<BlockOp, ParseError> {
    match key {
        "$text" => parse_text(value),
        "$matches" => parse_regex(value).map(BlockOp::Matches),
        "$within" => parse_within(value),
        "$contains" => {
            parse_block_predicate(value, "$contains").map(|p| BlockOp::Contains(Box::new(p)))
        }
        "$section" => parse_root_arg(value, "$section").map(|p| BlockOp::Section(Box::new(p))),
        "$quote" => parse_empty_head(value, "$quote").map(|p| BlockOp::Quote(Box::new(p))),
        "$list" => parse_empty_head(value, "$list").map(|p| BlockOp::List(Box::new(p))),
        "$header" => parse_type_arg(value, "$header", BlockType::Header),
        "$paragraph" => parse_type_arg(value, "$paragraph", BlockType::Paragraph),
        "$item" => parse_type_arg(value, "$item", BlockType::Item),
        "$code" => parse_type_arg(value, "$code", BlockType::Code),
        "$table" => parse_type_arg(value, "$table", BlockType::Table),
        "$ref" => match value {
            Value::Mapping(_) => parse_block_predicate(value, "$ref")
                .map(|p| BlockOp::Type(BlockType::Ref, Box::new(p))),
            _ => Err(ParseError::BlockScalarNotAllowed { op: "$ref" }),
        },
        "$hr" => parse_empty_head(value, "$hr").map(|p| BlockOp::Type(BlockType::Hr, Box::new(p))),
        "$references" => match value {
            Value::String(s) => Ok(BlockOp::References(Key::name(s))),
            _ => Err(ParseError::OperatorExpectedString { op: "$references" }),
        },
        "$and" => parse_list(value, "$and").map(BlockOp::And),
        "$or" => parse_list(value, "$or").map(BlockOp::Or),
        "$nor" => parse_list(value, "$nor").map(BlockOp::Nor),
        other => Err(ParseError::UnknownBlockOperator {
            op: other.to_string(),
        }),
    }
}

fn parse_text(value: &Value) -> Result<BlockOp, ParseError> {
    match value {
        Value::String(s) => Ok(BlockOp::Text(TextMatch::Substring(s.clone()))),
        Value::Mapping(m) => {
            if m.len() == 1 {
                if let Some(Value::String(s)) = m.get(Value::String("$eq".to_string())) {
                    return Ok(BlockOp::Text(TextMatch::Exact(s.clone())));
                }
            }
            Err(ParseError::OperatorExpectedString { op: "$text" })
        }
        _ => Err(ParseError::OperatorExpectedString { op: "$text" }),
    }
}

pub fn parse_regex(value: &Value) -> Result<BlockRegex, ParseError> {
    match value {
        Value::String(s) => BlockRegex::compile(s),
        _ => Err(ParseError::OperatorExpectedString { op: "$matches" }),
    }
}

fn parse_within(value: &Value) -> Result<BlockOp, ParseError> {
    match value {
        Value::String(s) => Ok(BlockOp::Within(Box::new(BlockPredicate(vec![
            BlockOp::Section(Box::new(BlockPredicate::text_exact(s))),
        ])))),
        Value::Mapping(_) => {
            let scope = parse_block_predicate(value, "$within")?;
            if !scope.carries_contents() {
                return Err(ParseError::WithinArgumentWithoutContents);
            }
            Ok(BlockOp::Within(Box::new(scope)))
        }
        _ => Err(ParseError::GraphOpExpectedScalarOrMapping { op: "$within" }),
    }
}

fn parse_root_arg(value: &Value, op: &'static str) -> Result<BlockPredicate, ParseError> {
    match value {
        Value::String(s) => Ok(BlockPredicate::text_exact(s)),
        Value::Mapping(_) => parse_block_predicate(value, op),
        _ => Err(ParseError::GraphOpExpectedScalarOrMapping { op }),
    }
}

fn parse_type_arg(value: &Value, op: &'static str, t: BlockType) -> Result<BlockOp, ParseError> {
    let arg = match value {
        Value::String(s) => BlockPredicate::text_exact(s),
        Value::Mapping(_) => parse_block_predicate(value, op)?,
        _ => return Err(ParseError::GraphOpExpectedScalarOrMapping { op }),
    };
    Ok(BlockOp::Type(t, Box::new(arg)))
}

fn parse_empty_head(value: &Value, op: &'static str) -> Result<BlockPredicate, ParseError> {
    match value {
        Value::Mapping(_) => {
            let pred = parse_block_predicate(value, op)?;
            if pred.has_direct_text_op() {
                return Err(ParseError::BlockTextPredicateNotAllowed { op });
            }
            Ok(pred)
        }
        _ => Err(ParseError::BlockScalarNotAllowed { op }),
    }
}

fn parse_list(value: &Value, op: &'static str) -> Result<Vec<BlockPredicate>, ParseError> {
    match value {
        Value::Sequence(items) => {
            if items.is_empty() {
                return Err(ParseError::EmptyOperatorList { op });
            }
            items
                .iter()
                .map(|item| parse_block_predicate(item, op))
                .collect()
        }
        _ => Err(ParseError::OperatorExpectedList { op }),
    }
}

pub fn parse_matches_source(value: &Value) -> Result<MatchesSource, ParseError> {
    match value {
        Value::String(_) => Ok(MatchesSource {
            pattern: parse_regex(value)?,
            scope: BlockPredicate::empty(),
        }),
        Value::Mapping(m) => {
            let mut pattern = None;
            let mut scope_map = Mapping::new();
            for (k, v) in m {
                let key = k.as_str().ok_or(ParseError::NonStringKey)?;
                if key == "pattern" {
                    pattern = Some(parse_regex(v)?);
                } else if key.starts_with('$') {
                    scope_map.insert(k.clone(), v.clone());
                } else {
                    return Err(ParseError::BareKeyInBlockPredicate {
                        key: key.to_string(),
                    });
                }
            }
            let pattern = pattern.ok_or(ParseError::MatchesPatternMissing)?;
            let scope = parse_mapping(&scope_map)?;
            Ok(MatchesSource { pattern, scope })
        }
        _ => Err(ParseError::OperatorExpectedString { op: "$matches" }),
    }
}
