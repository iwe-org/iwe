use serde_yaml::Value;

use crate::model::Key;
use crate::query::block::{BlockPredicate, MatchesSource};
use crate::query::search::SearchSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationKind {
    Find,
    Count,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Find(FindOp),
    Count(CountOp),
    Update(UpdateOp),
    Delete(DeleteOp),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FindOp {
    pub filter: Option<Filter>,
    pub search: Option<SearchSpec>,
    pub project: Projection,
    pub sort: Option<Sort>,
    pub limit: Option<Limit>,
}

impl FindOp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn search(mut self, search: SearchSpec) -> Self {
        self.search = Some(search);
        self
    }

    pub fn project(mut self, project: impl Into<Projection>) -> Self {
        self.project = project.into();
        self
    }

    pub fn add_fields(mut self, fields: Vec<ProjectionField>) -> Self {
        self.project = Projection::extend(fields);
        self
    }

    pub fn sort(mut self, sort: Sort) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(Limit(limit));
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CountOp {
    pub filter: Option<Filter>,
    pub sort: Option<Sort>,
    pub limit: Option<Limit>,
}

impl CountOp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn sort(mut self, sort: Sort) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(Limit(limit));
        self
    }
}

impl From<FindOp> for CountOp {
    fn from(op: FindOp) -> Self {
        CountOp {
            filter: op.filter,
            sort: op.sort,
            limit: op.limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOp {
    pub filter: Filter,
    pub sort: Option<Sort>,
    pub limit: Option<Limit>,
    pub expect: Option<Expect>,
    pub update: Update,
}

impl UpdateOp {
    pub fn new(filter: Filter, update: Update) -> Self {
        Self {
            filter,
            sort: None,
            limit: None,
            expect: None,
            update,
        }
    }

    pub fn sort(mut self, sort: Sort) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(Limit(limit));
        self
    }

    pub fn expect(mut self, expect: Expect) -> Self {
        self.expect = Some(expect);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteOp {
    pub filter: Filter,
    pub sort: Option<Sort>,
    pub limit: Option<Limit>,
    pub expect: Option<Expect>,
}

impl DeleteOp {
    pub fn new(filter: Filter) -> Self {
        Self {
            filter,
            sort: None,
            limit: None,
            expect: None,
        }
    }

    pub fn sort(mut self, sort: Sort) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(Limit(limit));
        self
    }

    pub fn expect(mut self, expect: Expect) -> Self {
        self.expect = Some(expect);
        self
    }
}

impl From<FindOp> for DeleteOp {
    fn from(op: FindOp) -> Self {
        DeleteOp {
            filter: op.filter.unwrap_or_else(Filter::all),
            sort: op.sort,
            limit: op.limit,
            expect: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Nor(Vec<Filter>),
    Field { path: FieldPath, op: FieldOp },
    Key(KeyOp),
    Content(BlockPredicate),
    Includes(Box<InclusionAnchor>),
    IncludedBy(Box<InclusionAnchor>),
    References(Box<ReferenceAnchor>),
    ReferencedBy(Box<ReferenceAnchor>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyOp {
    Eq(Key),
    Ne(Key),
    In(Vec<Key>),
    Nin(Vec<Key>),
}

impl KeyOp {
    pub fn eq(key: impl Into<String>) -> Self {
        KeyOp::Eq(Key::name(&key.into()))
    }
    pub fn ne(key: impl Into<String>) -> Self {
        KeyOp::Ne(Key::name(&key.into()))
    }
    pub fn in_(keys: &[&str]) -> Self {
        KeyOp::In(keys.iter().map(|s| Key::name(s)).collect())
    }
    pub fn nin(keys: &[&str]) -> Self {
        KeyOp::Nin(keys.iter().map(|s| Key::name(s)).collect())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InclusionAnchor {
    pub match_filter: Filter,
    pub min_depth: u32,
    pub max_depth: u32,
    pub size: Option<CountPred>,
}

impl InclusionAnchor {
    pub fn new(key: impl Into<String>, min_depth: u32, max_depth: u32) -> Self {
        InclusionAnchor {
            match_filter: Filter::Key(KeyOp::Eq(Key::name(&key.into()))),
            min_depth,
            max_depth,
            size: None,
        }
    }
    pub fn with_max(key: impl Into<String>, max_depth: u32) -> Self {
        Self::new(key, 1, max_depth)
    }
    pub fn with_match(match_filter: Filter, min_depth: u32, max_depth: u32) -> Self {
        InclusionAnchor {
            match_filter,
            min_depth,
            max_depth,
            size: None,
        }
    }
    pub fn with_size(mut self, size: CountPred) -> Self {
        self.size = Some(size);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceAnchor {
    pub match_filter: Filter,
    pub min_distance: u32,
    pub max_distance: u32,
    pub size: Option<CountPred>,
}

impl ReferenceAnchor {
    pub fn new(key: impl Into<String>, min_distance: u32, max_distance: u32) -> Self {
        ReferenceAnchor {
            match_filter: Filter::Key(KeyOp::Eq(Key::name(&key.into()))),
            min_distance,
            max_distance,
            size: None,
        }
    }
    pub fn with_max(key: impl Into<String>, max_distance: u32) -> Self {
        Self::new(key, 1, max_distance)
    }
    pub fn with_match(match_filter: Filter, min_distance: u32, max_distance: u32) -> Self {
        ReferenceAnchor {
            match_filter,
            min_distance,
            max_distance,
            size: None,
        }
    }
    pub fn with_size(mut self, size: CountPred) -> Self {
        self.size = Some(size);
        self
    }
}

impl Filter {
    pub fn all() -> Self {
        Filter::And(Vec::new())
    }

    pub fn and(filters: Vec<Filter>) -> Self {
        Filter::And(filters)
    }

    pub fn or(filters: Vec<Filter>) -> Self {
        Filter::Or(filters)
    }

    pub fn eq(path: &str, v: impl Into<Value>) -> Self {
        Self::field(path, FieldOp::Eq(v.into()))
    }

    pub fn ne(path: &str, v: impl Into<Value>) -> Self {
        Self::field(path, FieldOp::Ne(v.into()))
    }

    pub fn gt(path: &str, v: impl Into<Value>) -> Self {
        Self::field(path, FieldOp::Gt(v.into()))
    }

    pub fn gte(path: &str, v: impl Into<Value>) -> Self {
        Self::field(path, FieldOp::Gte(v.into()))
    }

    pub fn lt(path: &str, v: impl Into<Value>) -> Self {
        Self::field(path, FieldOp::Lt(v.into()))
    }

    pub fn lte(path: &str, v: impl Into<Value>) -> Self {
        Self::field(path, FieldOp::Lte(v.into()))
    }

    pub fn exists(path: &str, present: bool) -> Self {
        Self::field(path, FieldOp::Exists(present))
    }

    pub fn key(op: KeyOp) -> Self {
        Filter::Key(op)
    }

    pub fn content(pred: BlockPredicate) -> Self {
        Filter::Content(pred)
    }

    fn field(path: &str, op: FieldOp) -> Self {
        Filter::Field {
            path: FieldPath::from_dotted(path),
            op,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldPath(pub Vec<String>);

impl FieldPath {
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    pub fn from_dotted(s: &str) -> Self {
        FieldPath(s.split('.').map(|seg| seg.to_string()).collect())
    }

    pub fn leaf(&self) -> Option<&str> {
        self.0.last().map(|s| s.as_str())
    }

    pub fn starts_with(&self, other: &FieldPath) -> bool {
        if other.0.len() > self.0.len() {
            return false;
        }
        self.0.iter().zip(other.0.iter()).all(|(a, b)| a == b)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldOp {
    Eq(Value),
    Ne(Value),
    Gt(Value),
    Gte(Value),
    Lt(Value),
    Lte(Value),
    In(Vec<Value>),
    Nin(Vec<Value>),
    Exists(bool),
    Type(Vec<YamlType>),
    All(Vec<Value>),
    Size(CountPred),
    Not(Box<FieldOp>),
    And(Vec<FieldOp>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountPred {
    pub comparisons: Vec<CountCmp>,
}

impl CountPred {
    pub fn new(comparisons: Vec<CountCmp>) -> Self {
        CountPred { comparisons }
    }

    pub fn eq(n: u64) -> Self {
        CountPred {
            comparisons: vec![CountCmp::Eq(n)],
        }
    }

    pub fn at_least_one() -> Self {
        CountPred {
            comparisons: vec![CountCmp::Gte(1)],
        }
    }

    pub fn satisfied_by(&self, count: u64) -> bool {
        self.comparisons.iter().all(|c| c.satisfied_by(count))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountCmp {
    Eq(u64),
    Ne(u64),
    Gt(u64),
    Gte(u64),
    Lt(u64),
    Lte(u64),
}

impl CountCmp {
    pub fn satisfied_by(&self, count: u64) -> bool {
        match self {
            CountCmp::Eq(n) => count == *n,
            CountCmp::Ne(n) => count != *n,
            CountCmp::Gt(n) => count > *n,
            CountCmp::Gte(n) => count >= *n,
            CountCmp::Lt(n) => count < *n,
            CountCmp::Lte(n) => count <= *n,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YamlType {
    String,
    Number,
    Boolean,
    Null,
    Array,
    Object,
    Date,
    Datetime,
}

impl std::fmt::Display for YamlType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlType::String => write!(f, "string"),
            YamlType::Number => write!(f, "number"),
            YamlType::Boolean => write!(f, "boolean"),
            YamlType::Null => write!(f, "null"),
            YamlType::Array => write!(f, "array"),
            YamlType::Object => write!(f, "object"),
            YamlType::Date => write!(f, "date"),
            YamlType::Datetime => write!(f, "datetime"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionBase {
    Empty,
    Frontmatter,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoField {
    Key,
    Title,
    TitleSlug,
    Content,
    Frontmatter,
    IncludedBy,
    Includes,
    ReferencedBy,
    References,
}

impl PseudoField {
    pub fn from_selector(s: &str) -> Option<Self> {
        match s {
            "$key" => Some(PseudoField::Key),
            "$title" => Some(PseudoField::Title),
            "$titleSlug" => Some(PseudoField::TitleSlug),
            "$content" => Some(PseudoField::Content),
            "$frontmatter" => Some(PseudoField::Frontmatter),
            "$includedBy" => Some(PseudoField::IncludedBy),
            "$includes" => Some(PseudoField::Includes),
            "$referencedBy" => Some(PseudoField::ReferencedBy),
            "$references" => Some(PseudoField::References),
            _ => None,
        }
    }

    pub fn default_output_name(&self) -> &'static str {
        match self {
            PseudoField::Key => "key",
            PseudoField::Title => "title",
            PseudoField::TitleSlug => "titleSlug",
            PseudoField::Content => "content",
            PseudoField::Frontmatter => "frontmatter",
            PseudoField::IncludedBy => "includedBy",
            PseudoField::Includes => "includes",
            PseudoField::ReferencedBy => "referencedBy",
            PseudoField::References => "references",
        }
    }

    pub fn is_content_or_edge(&self) -> bool {
        matches!(
            self,
            PseudoField::Content
                | PseudoField::IncludedBy
                | PseudoField::Includes
                | PseudoField::ReferencedBy
                | PseudoField::References
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionSource {
    Frontmatter(FieldPath),
    Pseudo(PseudoField),
    ContentBlocks(BlockPredicate),
    Blocks(BlockPredicate),
    Matches(MatchesSource),
}

impl ProjectionSource {
    pub fn is_content_shaped(&self) -> bool {
        matches!(
            self,
            ProjectionSource::Pseudo(PseudoField::Content) | ProjectionSource::ContentBlocks(_)
        )
    }

    pub fn is_block_lines(&self) -> bool {
        matches!(
            self,
            ProjectionSource::Blocks(_) | ProjectionSource::Matches(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionField {
    pub output: String,
    pub source: ProjectionSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub fields: Vec<ProjectionField>,
    pub base: ProjectionBase,
}

impl Default for Projection {
    fn default() -> Self {
        Projection {
            fields: Vec::new(),
            base: ProjectionBase::Frontmatter,
        }
    }
}

impl From<Vec<ProjectionField>> for Projection {
    fn from(fields: Vec<ProjectionField>) -> Self {
        Projection::replace(fields)
    }
}

impl Projection {
    pub fn replace(fields: Vec<ProjectionField>) -> Self {
        Projection {
            fields,
            base: ProjectionBase::Empty,
        }
    }

    pub fn extend(fields: Vec<ProjectionField>) -> Self {
        Projection {
            fields,
            base: ProjectionBase::Document,
        }
    }

    pub fn document() -> Self {
        Projection {
            fields: Vec::new(),
            base: ProjectionBase::Document,
        }
    }

    pub fn fields(fields: &[&str]) -> Self {
        Projection::replace(
            fields
                .iter()
                .map(|name| ProjectionField {
                    output: (*name).to_string(),
                    source: ProjectionSource::Frontmatter(FieldPath::from_dotted(name)),
                })
                .collect(),
        )
    }

    pub fn document_fields() -> Vec<ProjectionField> {
        [
            ("key", PseudoField::Key),
            ("title", PseudoField::Title),
            ("references", PseudoField::References),
            ("includes", PseudoField::Includes),
            ("referencedBy", PseudoField::ReferencedBy),
            ("includedBy", PseudoField::IncludedBy),
        ]
        .iter()
        .map(|(name, p)| ProjectionField {
            output: (*name).to_string(),
            source: ProjectionSource::Pseudo(*p),
        })
        .collect()
    }

    pub fn has_content_or_edge_source(&self) -> bool {
        self.fields.iter().any(|f| match &f.source {
            ProjectionSource::Pseudo(p) => p.is_content_or_edge(),
            ProjectionSource::ContentBlocks(_)
            | ProjectionSource::Blocks(_)
            | ProjectionSource::Matches(_) => true,
            _ => false,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sort {
    pub key: FieldPath,
    pub dir: SortDir,
}

impl Sort {
    pub fn asc(path: &str) -> Self {
        Sort {
            key: FieldPath::from_dotted(path),
            dir: SortDir::Asc,
        }
    }

    pub fn desc(path: &str) -> Self {
        Sort {
            key: FieldPath::from_dotted(path),
            dir: SortDir::Desc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit(pub u64);

impl Limit {
    pub fn is_unbounded(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Update {
    pub operators: Vec<UpdateOperator>,
    pub block_ops: Vec<BlockUpdate>,
}

impl Update {
    pub fn new(operators: Vec<UpdateOperator>) -> Self {
        Update {
            operators,
            block_ops: Vec::new(),
        }
    }

    pub fn blocks(block_ops: Vec<BlockUpdate>) -> Self {
        Update {
            operators: Vec::new(),
            block_ops,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.operators.is_empty() && self.block_ops.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateOperator {
    Set { path: FieldPath, value: Value },
    Unset { path: FieldPath },
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockUpdate {
    pub selector: BlockPredicate,
    pub op: BlockUpdateOp,
    pub expect: Option<Expect>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockUpdateOp {
    Replace { content: String },
    ReplaceText { from: Option<String>, to: String },
    InsertBefore { content: String },
    InsertAfter { content: String },
    Append { content: String },
    Delete,
}

impl BlockUpdateOp {
    pub fn name(&self) -> &'static str {
        match self {
            BlockUpdateOp::Replace { .. } => "$replace",
            BlockUpdateOp::ReplaceText { .. } => "$replaceText",
            BlockUpdateOp::InsertBefore { .. } => "$insertBefore",
            BlockUpdateOp::InsertAfter { .. } => "$insertAfter",
            BlockUpdateOp::Append { .. } => "$append",
            BlockUpdateOp::Delete => "$delete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Exactly(u64),
    Range { min: Option<u64>, max: Option<u64> },
}

impl Expect {
    pub fn satisfied_by(&self, count: u64) -> bool {
        match self {
            Expect::Exactly(n) => count == *n,
            Expect::Range { min, max } => {
                min.map(|m| count >= m).unwrap_or(true) && max.map(|m| count <= m).unwrap_or(true)
            }
        }
    }
}

impl UpdateOperator {
    pub fn set(path: &str, value: impl Into<Value>) -> Self {
        UpdateOperator::Set {
            path: FieldPath::from_dotted(path),
            value: value.into(),
        }
    }

    pub fn unset(path: &str) -> Self {
        UpdateOperator::Unset {
            path: FieldPath::from_dotted(path),
        }
    }
}
