use serde_yaml::Value;

use crate::model::Key;

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
    pub project: Option<Projection>,
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

    pub fn project(mut self, project: Projection) -> Self {
        self.project = Some(project);
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

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOp {
    pub filter: Filter,
    pub sort: Option<Sort>,
    pub limit: Option<Limit>,
    pub update: Update,
}

impl UpdateOp {
    pub fn new(filter: Filter, update: Update) -> Self {
        Self {
            filter,
            sort: None,
            limit: None,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteOp {
    pub filter: Filter,
    pub sort: Option<Sort>,
    pub limit: Option<Limit>,
}

impl DeleteOp {
    pub fn new(filter: Filter) -> Self {
        Self {
            filter,
            sort: None,
            limit: None,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
    Field { path: FieldPath, op: FieldOp },
    Key(KeyOp),
    IncludesCount(CountArg),
    IncludedByCount(CountArg),
    Includes(Vec<InclusionAnchor>),
    IncludedBy(Vec<InclusionAnchor>),
    References(Vec<ReferenceAnchor>),
    ReferencedBy(Vec<ReferenceAnchor>),
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
pub struct CountArg {
    pub count: NumExpr,
    pub min_depth: u32,
    pub max_depth: MaxDepth,
}

impl CountArg {
    pub fn direct(count: NumExpr) -> Self {
        CountArg {
            count,
            min_depth: 1,
            max_depth: MaxDepth::Bounded(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxDepth {
    Bounded(u32),
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumExpr(pub Vec<NumOp>);

impl NumExpr {
    pub fn eq(n: u64) -> Self {
        NumExpr(vec![NumOp::Eq(n)])
    }
    pub fn ne(n: u64) -> Self {
        NumExpr(vec![NumOp::Ne(n)])
    }
    pub fn gt(n: u64) -> Self {
        NumExpr(vec![NumOp::Gt(n)])
    }
    pub fn gte(n: u64) -> Self {
        NumExpr(vec![NumOp::Gte(n)])
    }
    pub fn lt(n: u64) -> Self {
        NumExpr(vec![NumOp::Lt(n)])
    }
    pub fn lte(n: u64) -> Self {
        NumExpr(vec![NumOp::Lte(n)])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumOp {
    Eq(u64),
    Ne(u64),
    Gt(u64),
    Gte(u64),
    Lt(u64),
    Lte(u64),
    In(Vec<u64>),
    Nin(Vec<u64>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InclusionAnchor {
    pub key: Key,
    pub min_depth: u32,
    pub max_depth: u32,
}

impl InclusionAnchor {
    pub fn new(key: impl Into<String>, min_depth: u32, max_depth: u32) -> Self {
        InclusionAnchor {
            key: Key::name(&key.into()),
            min_depth,
            max_depth,
        }
    }
    pub fn with_max(key: impl Into<String>, max_depth: u32) -> Self {
        Self::new(key, 1, max_depth)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceAnchor {
    pub key: Key,
    pub min_distance: u32,
    pub max_distance: u32,
}

impl ReferenceAnchor {
    pub fn new(key: impl Into<String>, min_distance: u32, max_distance: u32) -> Self {
        ReferenceAnchor {
            key: Key::name(&key.into()),
            min_distance,
            max_distance,
        }
    }
    pub fn with_max(key: impl Into<String>, max_distance: u32) -> Self {
        Self::new(key, 1, max_distance)
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
    Size(u64),
    Not(Box<FieldOp>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub fields: Vec<FieldPath>,
}

impl Projection {
    pub fn fields(fields: &[&str]) -> Self {
        Projection {
            fields: fields.iter().map(|p| FieldPath::from_dotted(p)).collect(),
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct Update {
    pub operators: Vec<UpdateOperator>,
}

impl Update {
    pub fn new(operators: Vec<UpdateOperator>) -> Self {
        Update { operators }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateOperator {
    Set { path: FieldPath, value: Value },
    Unset { path: FieldPath },
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
