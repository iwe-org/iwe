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
    Nor(Vec<Filter>),
    Not(Box<Filter>),
    Field { path: FieldPath, op: FieldOp },
    Key(KeyOp),
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
}

impl InclusionAnchor {
    pub fn new(key: impl Into<String>, min_depth: u32, max_depth: u32) -> Self {
        InclusionAnchor {
            match_filter: Filter::Key(KeyOp::Eq(Key::name(&key.into()))),
            min_depth,
            max_depth,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceAnchor {
    pub match_filter: Filter,
    pub min_distance: u32,
    pub max_distance: u32,
}

impl ReferenceAnchor {
    pub fn new(key: impl Into<String>, min_distance: u32, max_distance: u32) -> Self {
        ReferenceAnchor {
            match_filter: Filter::Key(KeyOp::Eq(Key::name(&key.into()))),
            min_distance,
            max_distance,
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
        }
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
    And(Vec<FieldOp>),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMode {
    Replace,
    Extend,
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
    IncludedByCount,
    IncludesCount,
    ReferencedByCount,
    ReferencesCount,
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
            "$includedByCount" => Some(PseudoField::IncludedByCount),
            "$includesCount" => Some(PseudoField::IncludesCount),
            "$referencedByCount" => Some(PseudoField::ReferencedByCount),
            "$referencesCount" => Some(PseudoField::ReferencesCount),
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
            PseudoField::IncludedByCount => "includedByCount",
            PseudoField::IncludesCount => "includesCount",
            PseudoField::ReferencedByCount => "referencedByCount",
            PseudoField::ReferencesCount => "referencesCount",
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionField {
    pub output: String,
    pub source: ProjectionSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub fields: Vec<ProjectionField>,
    pub mode: ProjectionMode,
}

impl Projection {
    pub fn replace(fields: Vec<ProjectionField>) -> Self {
        Projection {
            fields,
            mode: ProjectionMode::Replace,
        }
    }

    pub fn extend(fields: Vec<ProjectionField>) -> Self {
        Projection {
            fields,
            mode: ProjectionMode::Extend,
        }
    }

    pub fn fields(fields: &[&str]) -> Self {
        Projection {
            fields: fields
                .iter()
                .map(|name| ProjectionField {
                    output: (*name).to_string(),
                    source: ProjectionSource::Frontmatter(FieldPath::from_dotted(name)),
                })
                .collect(),
            mode: ProjectionMode::Replace,
        }
    }

    pub fn default_for_find() -> Self {
        let entries = [
            ("key", PseudoField::Key),
            ("title", PseudoField::Title),
            ("references", PseudoField::References),
            ("includes", PseudoField::Includes),
            ("referencedBy", PseudoField::ReferencedBy),
            ("includedBy", PseudoField::IncludedBy),
        ];
        Projection {
            fields: entries
                .iter()
                .map(|(name, p)| ProjectionField {
                    output: (*name).to_string(),
                    source: ProjectionSource::Pseudo(*p),
                })
                .collect(),
            mode: ProjectionMode::Replace,
        }
    }

    pub fn has_content_or_edge_source(&self) -> bool {
        self.fields.iter().any(|f| match &f.source {
            ProjectionSource::Pseudo(p) => p.is_content_or_edge(),
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
