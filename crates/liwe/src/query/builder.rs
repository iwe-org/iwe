use serde_yaml::{Mapping, Value};

use crate::model::Key;
use crate::query::document::{
    CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, InclusionAnchor, KeyOp, Limit,
    Operation, OperationKind, Projection, ProjectionField, ProjectionMode, ProjectionSource,
    PseudoField, ReferenceAnchor, Sort, SortDir, Update, UpdateOp, UpdateOperator, YamlType,
};
use crate::query::wire::{
    self, RawFilter, RawKeyOpMap, RawOperation, RawProjection, RawRelationalObj, RawSort,
    RawUpdate,
};

#[derive(Debug)]
pub enum ParseError {
    Wire(serde_yaml::Error),
    OperationFieldNotAllowed {
        kind: OperationKind,
        field: &'static str,
    },
    MissingRequiredField {
        kind: OperationKind,
        field: &'static str,
    },
    EmptyFilter,
    MixedDollarAndBare {
        path: Vec<String>,
    },
    UnknownOperator {
        op: String,
        path: Vec<String>,
    },
    EmptyOperatorList {
        op: &'static str,
    },
    OperatorExpectedList {
        op: &'static str,
    },
    OperatorExpectedMapping {
        op: &'static str,
    },
    OperatorExpectedString {
        op: &'static str,
    },
    OperatorExpectedBool {
        op: &'static str,
    },
    OperatorExpectedNonNegativeInt {
        op: &'static str,
    },
    OperatorExpectedInteger {
        op: &'static str,
    },
    UnknownTypeName {
        name: String,
    },
    InvalidProjectionValue {
        path: Vec<String>,
    },
    UnknownProjectionSource {
        selector: String,
    },
    ReservedOutputName {
        name: String,
    },
    NestedProjectionOutput {
        name: String,
    },
    ProjectAddFieldsConflict,
    InvalidSortValue {
        key: String,
        value: i64,
    },
    EmptySort,
    MultiKeySortNotSupportedV1,
    NegativeLimit(i64),
    EmptyUpdate,
    UnknownUpdateOperator {
        op: String,
    },
    EmptyUpdateOperator {
        op: &'static str,
    },
    UpdateOperatorExpectedMapping {
        op: &'static str,
    },
    ReservedPrefixField {
        path: Vec<String>,
    },
    SetUnsetConflict {
        path: Vec<String>,
    },
    EmptyFieldPath,
    InvalidPathSegment {
        path: Vec<String>,
        reason: &'static str,
    },
    NonStringKey,
    GraphOpExpectedScalarOrMapping {
        op: &'static str,
    },
    ArrayFormRemoved {
        op: &'static str,
    },
    EmptyAnchorMapping {
        op: &'static str,
    },
    MatchMissing {
        op: &'static str,
    },
    WrongBoundFamily {
        op: &'static str,
        modifier: &'static str,
    },
    DepthRangeInverted {
        op: &'static str,
    },
    KeyOpForbidden {
        op: &'static str,
    },
    InvalidDepthValue {
        op: &'static str,
        modifier: &'static str,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {}


pub fn parse_operation(yaml: &str, kind: OperationKind) -> Result<Operation, ParseError> {
    let raw = wire::parse(yaml).map_err(ParseError::Wire)?;
    match kind {
        OperationKind::Find => Ok(Operation::Find(build_find(raw)?)),
        OperationKind::Count => Ok(Operation::Count(build_count(raw)?)),
        OperationKind::Update => Ok(Operation::Update(build_update(raw)?)),
        OperationKind::Delete => Ok(Operation::Delete(build_delete(raw)?)),
    }
}

pub fn parse_filter_expression(expr: &str) -> Result<Filter, ParseError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Ok(Filter::And(Vec::new()));
    }
    let mapping = parse_to_mapping(trimmed)
        .or_else(|_| parse_to_mapping(&format!("{{{}}}", trimmed)))
        .map_err(ParseError::Wire)?;
    build_filter_at(mapping, &[])
}

fn parse_to_mapping(yaml: &str) -> Result<Mapping, serde_yaml::Error> {
    let value: Value = serde_yaml::from_str(yaml)?;
    match value {
        Value::Mapping(m) => Ok(m),
        Value::Null => Ok(Mapping::new()),
        _ => serde_yaml::from_str::<Mapping>(yaml),
    }
}


fn build_find(raw: RawOperation) -> Result<FindOp, ParseError> {
    if raw.update.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Find,
            field: "update",
        });
    }
    if raw.project.is_some() && raw.add_fields.is_some() {
        return Err(ParseError::ProjectAddFieldsConflict);
    }
    let project = if let Some(p) = raw.project {
        Some(build_projection(p, ProjectionMode::Replace)?)
    } else if let Some(a) = raw.add_fields {
        Some(build_projection(a, ProjectionMode::Extend)?)
    } else {
        None
    };
    Ok(FindOp {
        filter: raw.filter.map(build_filter).transpose()?,
        project,
        sort: raw.sort.map(build_sort).transpose()?,
        limit: raw.limit.map(build_limit).transpose()?,
    })
}

fn build_count(raw: RawOperation) -> Result<CountOp, ParseError> {
    if raw.project.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Count,
            field: "project",
        });
    }
    if raw.add_fields.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Count,
            field: "addFields",
        });
    }
    if raw.update.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Count,
            field: "update",
        });
    }
    Ok(CountOp {
        filter: raw.filter.map(build_filter).transpose()?,
        sort: raw.sort.map(build_sort).transpose()?,
        limit: raw.limit.map(build_limit).transpose()?,
    })
}

fn build_update(raw: RawOperation) -> Result<UpdateOp, ParseError> {
    if raw.project.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Update,
            field: "project",
        });
    }
    if raw.add_fields.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Update,
            field: "addFields",
        });
    }
    let filter = raw
        .filter
        .ok_or(ParseError::MissingRequiredField {
            kind: OperationKind::Update,
            field: "filter",
        })
        .and_then(build_filter)?;
    let update = raw
        .update
        .ok_or(ParseError::MissingRequiredField {
            kind: OperationKind::Update,
            field: "update",
        })
        .and_then(build_update_doc)?;
    Ok(UpdateOp {
        filter,
        sort: raw.sort.map(build_sort).transpose()?,
        limit: raw.limit.map(build_limit).transpose()?,
        update,
    })
}

fn build_delete(raw: RawOperation) -> Result<DeleteOp, ParseError> {
    if raw.project.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Delete,
            field: "project",
        });
    }
    if raw.add_fields.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Delete,
            field: "addFields",
        });
    }
    if raw.update.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Delete,
            field: "update",
        });
    }
    let filter = raw
        .filter
        .ok_or(ParseError::MissingRequiredField {
            kind: OperationKind::Delete,
            field: "filter",
        })
        .and_then(build_filter)?;
    Ok(DeleteOp {
        filter,
        sort: raw.sort.map(build_sort).transpose()?,
        limit: raw.limit.map(build_limit).transpose()?,
    })
}


fn build_filter(raw: RawFilter) -> Result<Filter, ParseError> {
    build_filter_at(raw.0, &[])
}

fn build_filter_at(map: Mapping, path: &[String]) -> Result<Filter, ParseError> {
    if map.is_empty() {


        return Ok(Filter::And(Vec::new()));
    }

    let (dollar_keys, bare_keys) = classify_keys(&map)?;
    if !dollar_keys.is_empty() && !bare_keys.is_empty() {
        return Err(ParseError::MixedDollarAndBare {
            path: path.to_vec(),
        });
    }

    if !dollar_keys.is_empty() {

        let mut clauses: Vec<Filter> = Vec::new();
        for op in dollar_keys {
            let value = &map[Value::String(op.clone())];
            clauses.push(build_filter_op(&op, value, path)?);
        }
        if clauses.len() == 1 {
            Ok(clauses.into_iter().next().unwrap())
        } else {
            Ok(Filter::And(clauses))
        }
    } else {
        let mut clauses: Vec<Filter> = Vec::new();
        for key_str in bare_keys {
            let segments: Vec<String> = if key_str.contains('.') {
                key_str.split('.').map(|s| s.to_string()).collect()
            } else {
                vec![key_str.clone()]
            };
            check_path_segments(&segments)?;
            let mut child_path = path.to_vec();
            child_path.extend(segments.iter().cloned());
            let value = map[Value::String(key_str.clone())].clone();
            clauses.push(build_field_clause(&segments, value, &child_path)?);
        }
        if clauses.len() == 1 {
            Ok(clauses.into_iter().next().unwrap())
        } else {
            Ok(Filter::And(clauses))
        }
    }
}

fn classify_keys(map: &Mapping) -> Result<(Vec<String>, Vec<String>), ParseError> {
    let mut dollar = Vec::new();
    let mut bare = Vec::new();
    for (k, _) in map {
        let s = k
            .as_str()
            .ok_or(ParseError::NonStringKey)?
            .to_string();
        if s.starts_with('$') {
            dollar.push(s);
        } else {
            bare.push(s);
        }
    }
    Ok((dollar, bare))
}

fn build_filter_op(op: &str, value: &Value, path: &[String]) -> Result<Filter, ParseError> {
    match op {
        "$and" => Ok(Filter::And(parse_filter_list(value, "$and", path)?)),
        "$or" => Ok(Filter::Or(parse_filter_list(value, "$or", path)?)),
        "$nor" => Ok(Filter::Nor(parse_filter_list(value, "$nor", path)?)),
        "$not" => Ok(Filter::Not(Box::new(parse_not(value, path)?))),
        "$key" => Ok(Filter::Key(parse_key_op(value, "$key")?)),
        "$includes" => Ok(Filter::Includes(Box::new(parse_inclusion_arg(value, "$includes")?))),
        "$includedBy" => Ok(Filter::IncludedBy(Box::new(parse_inclusion_arg(value, "$includedBy")?))),
        "$references" => Ok(Filter::References(Box::new(parse_reference_arg(value, "$references")?))),
        "$referencedBy" => Ok(Filter::ReferencedBy(Box::new(parse_reference_arg(value, "$referencedBy")?))),
        other => Err(ParseError::UnknownOperator {
            op: other.to_string(),
            path: path.to_vec(),
        }),
    }
}

fn parse_filter_list(
    value: &Value,
    op: &'static str,
    path: &[String],
) -> Result<Vec<Filter>, ParseError> {
    let list = value
        .as_sequence()
        .ok_or(ParseError::OperatorExpectedList { op })?;
    if list.is_empty() {
        return Err(ParseError::EmptyOperatorList { op });
    }
    list.iter()
        .map(|elem| {
            let m = elem
                .as_mapping()
                .ok_or(ParseError::OperatorExpectedMapping { op })?
                .clone();
            build_filter_at(m, path)
        })
        .collect()
}

fn parse_not(value: &Value, path: &[String]) -> Result<Filter, ParseError> {
    let m = value
        .as_mapping()
        .ok_or(ParseError::OperatorExpectedMapping { op: "$not" })?
        .clone();
    build_filter_at(m, path)
}

fn static_op_name(op: &str) -> &'static str {
    match op {
        "$and" => "$and",
        "$or" => "$or",
        "$not" => "$not",
        "$eq" => "$eq",
        "$ne" => "$ne",
        "$gt" => "$gt",
        "$gte" => "$gte",
        "$lt" => "$lt",
        "$lte" => "$lte",
        "$in" => "$in",
        "$nin" => "$nin",
        "$exists" => "$exists",
        "$type" => "$type",
        "$all" => "$all",
        "$size" => "$size",
        "$set" => "$set",
        "$unset" => "$unset",
        _ => "<operator>",
    }
}

fn build_field_clause(
    segments: &[String],
    value: Value,
    path: &[String],
) -> Result<Filter, ParseError> {
    match value {
        Value::Mapping(map) => {
            let (dollar_keys, bare_keys) = classify_keys(&map)?;
            if !dollar_keys.is_empty() && !bare_keys.is_empty() {
                return Err(ParseError::MixedDollarAndBare {
                    path: path.to_vec(),
                });
            }

            if !dollar_keys.is_empty() {

                let mut ops = Vec::with_capacity(dollar_keys.len());
                for op in dollar_keys {
                    let v = map[Value::String(op.clone())].clone();
                    let field_op = build_field_op(&op, v, path)?;
                    ops.push(Filter::Field {
                        path: FieldPath(segments.to_vec()),
                        op: field_op,
                    });
                }
                if ops.len() == 1 {
                    Ok(ops.into_iter().next().unwrap())
                } else {
                    Ok(Filter::And(ops))
                }
            } else {

                build_nested_field(segments, &map, path)
            }
        }
        other => Ok(Filter::Field {
            path: FieldPath(segments.to_vec()),
            op: FieldOp::Eq(other),
        }),
    }
}

fn build_nested_field(
    parent: &[String],
    map: &Mapping,
    path: &[String],
) -> Result<Filter, ParseError> {
    let mut sub = Vec::with_capacity(map.len());
    for (k, v) in map {
        let key_str = k.as_str().ok_or(ParseError::NonStringKey)?;
        let child_segments: Vec<String> = if key_str.contains('.') {
            let mut s = parent.to_vec();
            s.extend(key_str.split('.').map(|s| s.to_string()));
            s
        } else {
            let mut s = parent.to_vec();
            s.push(key_str.to_string());
            s
        };
        check_path_segments(&child_segments)?;
        let mut child_path = path.to_vec();
        for seg in child_segments.iter().skip(parent.len()) {
            child_path.push(seg.clone());
        }
        sub.push(build_field_clause(&child_segments, v.clone(), &child_path)?);
    }
    if sub.len() == 1 {
        Ok(sub.into_iter().next().unwrap())
    } else {
        Ok(Filter::And(sub))
    }
}

fn build_field_op(op: &str, value: Value, path: &[String]) -> Result<FieldOp, ParseError> {
    match op {
        "$eq" => Ok(FieldOp::Eq(value)),
        "$ne" => Ok(FieldOp::Ne(value)),
        "$gt" => Ok(FieldOp::Gt(value)),
        "$gte" => Ok(FieldOp::Gte(value)),
        "$lt" => Ok(FieldOp::Lt(value)),
        "$lte" => Ok(FieldOp::Lte(value)),
        "$in" | "$nin" | "$all" => {
            let list = value
                .as_sequence()
                .ok_or(ParseError::OperatorExpectedList {
                    op: static_op_name(op),
                })?
                .clone();
            if list.is_empty() {
                return Err(ParseError::EmptyOperatorList {
                    op: static_op_name(op),
                });
            }
            match op {
                "$in" => Ok(FieldOp::In(list)),
                "$nin" => Ok(FieldOp::Nin(list)),
                "$all" => Ok(FieldOp::All(list)),
                _ => unreachable!(),
            }
        }
        "$exists" => match value {
            Value::Bool(b) => Ok(FieldOp::Exists(b)),
            _ => Err(ParseError::OperatorExpectedBool { op: "$exists" }),
        },
        "$type" => {
            let names: Vec<String> = match value {
                Value::String(s) => vec![s],
                Value::Sequence(seq) => {
                    if seq.is_empty() {
                        return Err(ParseError::EmptyOperatorList { op: "$type" });
                    }
                    let mut out = Vec::with_capacity(seq.len());
                    for v in seq {
                        out.push(
                            v.as_str()
                                .ok_or(ParseError::OperatorExpectedString { op: "$type" })?
                                .to_string(),
                        );
                    }
                    out
                }
                _ => return Err(ParseError::OperatorExpectedString { op: "$type" }),
            };
            let mut types = Vec::with_capacity(names.len());
            for n in names {
                types.push(parse_type_name(&n)?);
            }
            Ok(FieldOp::Type(types))
        }
        "$size" => match value {
            Value::Number(n) => {
                let i = n
                    .as_i64()
                    .ok_or(ParseError::OperatorExpectedInteger { op: "$size" })?;
                if i < 0 {
                    return Err(ParseError::OperatorExpectedNonNegativeInt { op: "$size" });
                }
                Ok(FieldOp::Size(i as u64))
            }
            _ => Err(ParseError::OperatorExpectedInteger { op: "$size" }),
        },
        "$not" => {
            let m = value
                .as_mapping()
                .ok_or(ParseError::OperatorExpectedMapping { op: "$not" })?
                .clone();
            if m.is_empty() {
                return Err(ParseError::OperatorExpectedMapping { op: "$not" });
            }

            let (dollar_keys, bare_keys) = classify_keys(&m)?;
            if !bare_keys.is_empty() {
                return Err(ParseError::MixedDollarAndBare {
                    path: path.to_vec(),
                });
            }

            let mut inner_ops = Vec::with_capacity(dollar_keys.len());
            for inner_op in dollar_keys {
                let v = m[Value::String(inner_op.clone())].clone();
                inner_ops.push(build_field_op(&inner_op, v, path)?);
            }
            let inner = if inner_ops.len() == 1 {
                inner_ops.into_iter().next().unwrap()
            } else {
                FieldOp::And(inner_ops)
            };
            Ok(FieldOp::Not(Box::new(inner)))
        }
        other => Err(ParseError::UnknownOperator {
            op: other.to_string(),
            path: path.to_vec(),
        }),
    }
}

fn parse_type_name(name: &str) -> Result<YamlType, ParseError> {
    match name {
        "string" => Ok(YamlType::String),
        "number" => Ok(YamlType::Number),
        "boolean" => Ok(YamlType::Boolean),
        "null" => Ok(YamlType::Null),
        "array" => Ok(YamlType::Array),
        "object" => Ok(YamlType::Object),
        "date" => Ok(YamlType::Date),
        "datetime" => Ok(YamlType::Datetime),
        _ => Err(ParseError::UnknownTypeName {
            name: name.to_string(),
        }),
    }
}


pub fn build_projection(raw: RawProjection, mode: ProjectionMode) -> Result<Projection, ParseError> {
    let mut fields: Vec<ProjectionField> = Vec::new();
    for (k, v) in &raw.0 {
        let output = k.as_str().ok_or(ParseError::NonStringKey)?.to_string();
        check_output_name(&output)?;
        let source = build_projection_source(&output, v)?;
        fields.push(ProjectionField { output, source });
    }
    Ok(Projection { fields, mode })
}

fn check_output_name(name: &str) -> Result<(), ParseError> {
    if name.is_empty() {
        return Err(ParseError::EmptyFieldPath);
    }
    if name.chars().any(|c| c.is_whitespace()) {
        return Err(ParseError::InvalidPathSegment {
            path: vec![name.to_string()],
            reason: "segment contains whitespace",
        });
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(ParseError::InvalidPathSegment {
            path: vec![name.to_string()],
            reason: "segment contains a control character",
        });
    }
    if name.starts_with('$') {
        return Err(ParseError::ReservedOutputName {
            name: name.to_string(),
        });
    }
    if name.contains('.') {
        return Err(ParseError::NestedProjectionOutput {
            name: name.to_string(),
        });
    }
    if matches!(name.chars().next(), Some('_' | '#' | '@')) {
        return Err(ParseError::ReservedOutputName {
            name: name.to_string(),
        });
    }
    Ok(())
}

fn build_projection_source(output: &str, v: &Value) -> Result<ProjectionSource, ParseError> {
    match v {
        Value::Number(n) if n.as_i64() == Some(1) => {
            Ok(ProjectionSource::Frontmatter(FieldPath(vec![output.to_string()])))
        }
        Value::Bool(true) => {
            Ok(ProjectionSource::Frontmatter(FieldPath(vec![output.to_string()])))
        }
        Value::Null => {
            Ok(ProjectionSource::Frontmatter(FieldPath(vec![output.to_string()])))
        }
        Value::String(s) => {
            if let Some(stripped) = s.strip_prefix('$') {
                let selector = format!("${}", stripped);
                if let Some(pf) = PseudoField::from_selector(&selector) {
                    Ok(ProjectionSource::Pseudo(pf))
                } else {
                    Err(ParseError::UnknownProjectionSource { selector })
                }
            } else {
                let segments: Vec<String> = s.split('.').map(|p| p.to_string()).collect();
                check_path_segments(&segments)?;
                Ok(ProjectionSource::Frontmatter(FieldPath(segments)))
            }
        }
        _ => Err(ParseError::InvalidProjectionValue {
            path: vec![output.to_string()],
        }),
    }
}


fn build_sort(raw: RawSort) -> Result<Sort, ParseError> {
    let map = raw.0;
    if map.is_empty() {
        return Err(ParseError::EmptySort);
    }
    if map.len() > 1 {
        return Err(ParseError::MultiKeySortNotSupportedV1);
    }
    let (k, v) = map.into_iter().next().unwrap();
    let key_str = k.as_str().ok_or(ParseError::NonStringKey)?.to_string();
    let dir_int = match v {
        Value::Number(n) => n.as_i64().ok_or(ParseError::InvalidSortValue {
            key: key_str.clone(),
            value: 0,
        })?,
        _ => {
            return Err(ParseError::InvalidSortValue {
                key: key_str,
                value: 0,
            });
        }
    };
    let dir = match dir_int {
        1 => SortDir::Asc,
        -1 => SortDir::Desc,
        other => {
            return Err(ParseError::InvalidSortValue {
                key: key_str,
                value: other,
            });
        }
    };
    let path = if key_str.contains('.') {
        FieldPath::from_dotted(&key_str)
    } else {
        FieldPath(vec![key_str])
    };
    check_path_segments(&path.0)?;
    Ok(Sort { key: path, dir })
}


fn build_limit(raw: i64) -> Result<Limit, ParseError> {
    if raw < 0 {
        Err(ParseError::NegativeLimit(raw))
    } else {
        Ok(Limit(raw as u64))
    }
}


pub fn build_update_doc(raw: RawUpdate) -> Result<Update, ParseError> {
    if raw.set.is_none() && raw.unset.is_none() {
        return Err(ParseError::EmptyUpdate);
    }
    let mut operators: Vec<UpdateOperator> = Vec::new();
    if let Some(set) = raw.set {
        if set.is_empty() {
            return Err(ParseError::EmptyUpdateOperator { op: "$set" });
        }
        walk_update_set(&set, &[], &mut operators)?;
    }
    if let Some(unset) = raw.unset {
        if unset.is_empty() {
            return Err(ParseError::EmptyUpdateOperator { op: "$unset" });
        }
        walk_update_unset(&unset, &[], &mut operators)?;
    }
    check_update_conflicts(&operators)?;
    Ok(Update { operators })
}

fn walk_update_set(
    map: &Mapping,
    parent: &[String],
    out: &mut Vec<UpdateOperator>,
) -> Result<(), ParseError> {
    for (k, v) in map {
        let key_str = k.as_str().ok_or(ParseError::NonStringKey)?;
        let segments: Vec<String> = if key_str.contains('.') {
            let mut s = parent.to_vec();
            s.extend(key_str.split('.').map(|s| s.to_string()));
            s
        } else {
            let mut s = parent.to_vec();
            s.push(key_str.to_string());
            s
        };
        check_path_segments(&segments)?;
        check_reserved_prefix(&segments)?;
        check_value_for_reserved(v, &segments)?;
        out.push(UpdateOperator::Set {
            path: FieldPath(segments),
            value: v.clone(),
        });
    }
    Ok(())
}

fn check_value_for_reserved(value: &Value, parent: &[String]) -> Result<(), ParseError> {
    match value {
        Value::Mapping(m) => {
            for (k, inner) in m {
                let key_str = k.as_str().ok_or(ParseError::NonStringKey)?;
                let mut child = parent.to_vec();
                child.push(key_str.to_string());
                if matches!(key_str.chars().next(), Some('_' | '$' | '.' | '#' | '@')) {
                    return Err(ParseError::ReservedPrefixField { path: child });
                }
                check_value_for_reserved(inner, &child)?;
            }
            Ok(())
        }
        Value::Sequence(seq) => {
            for elem in seq {
                check_value_for_reserved(elem, parent)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn walk_update_unset(
    map: &Mapping,
    parent: &[String],
    out: &mut Vec<UpdateOperator>,
) -> Result<(), ParseError> {
    for (k, _v) in map {
        let key_str = k.as_str().ok_or(ParseError::NonStringKey)?;
        let segments: Vec<String> = if key_str.contains('.') {
            let mut s = parent.to_vec();
            s.extend(key_str.split('.').map(|s| s.to_string()));
            s
        } else {
            let mut s = parent.to_vec();
            s.push(key_str.to_string());
            s
        };
        check_path_segments(&segments)?;
        check_reserved_prefix(&segments)?;
        out.push(UpdateOperator::Unset {
            path: FieldPath(segments),
        });
    }
    Ok(())
}

fn check_reserved_prefix(segments: &[String]) -> Result<(), ParseError> {
    for seg in segments {
        match seg.chars().next() {
            None => return Err(ParseError::EmptyFieldPath),
            Some('_' | '$' | '.' | '#' | '@') => {
                return Err(ParseError::ReservedPrefixField {
                    path: segments.to_vec(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_path_segments(segments: &[String]) -> Result<(), ParseError> {
    for seg in segments {
        if seg.is_empty() {
            return Err(ParseError::InvalidPathSegment {
                path: segments.to_vec(),
                reason: "empty segment",
            });
        }
        if seg.chars().any(|c| c.is_whitespace()) {
            return Err(ParseError::InvalidPathSegment {
                path: segments.to_vec(),
                reason: "segment contains whitespace",
            });
        }
        if seg.chars().any(|c| c.is_control()) {
            return Err(ParseError::InvalidPathSegment {
                path: segments.to_vec(),
                reason: "segment contains a control character",
            });
        }
    }
    Ok(())
}

fn parse_key_op(value: &Value, op: &'static str) -> Result<KeyOp, ParseError> {
    if let Some(s) = value.as_str() {
        return Ok(KeyOp::Eq(Key::name(s)));
    }
    if !value.is_mapping() {
        return Err(ParseError::GraphOpExpectedScalarOrMapping { op });
    }
    let m: RawKeyOpMap = serde_yaml::from_value(value.clone())
        .map_err(|_| ParseError::KeyOpForbidden { op })?;
    key_op_from_map(m, op)
}

fn key_op_from_map(m: RawKeyOpMap, op: &'static str) -> Result<KeyOp, ParseError> {
    let count = m.eq.is_some() as u8 + m.ne.is_some() as u8 + m.in_.is_some() as u8
        + m.nin.is_some() as u8;
    if count != 1 {
        return Err(ParseError::KeyOpForbidden { op });
    }
    if let Some(s) = m.eq {
        return Ok(KeyOp::Eq(Key::name(&s)));
    }
    if let Some(s) = m.ne {
        return Ok(KeyOp::Ne(Key::name(&s)));
    }
    if let Some(list) = m.in_ {
        return Ok(KeyOp::In(string_list(list, op)?));
    }
    if let Some(list) = m.nin {
        return Ok(KeyOp::Nin(string_list(list, op)?));
    }
    unreachable!()
}

fn string_list(list: Vec<Value>, op: &'static str) -> Result<Vec<Key>, ParseError> {
    if list.is_empty() {
        return Err(ParseError::EmptyOperatorList { op });
    }
    list.into_iter()
        .map(|v| {
            v.as_str()
                .map(Key::name)
                .ok_or(ParseError::OperatorExpectedString { op })
        })
        .collect()
}

fn pos_u32(i: i64, op: &'static str, modifier: &'static str) -> Result<u32, ParseError> {
    if i >= 1 {
        Ok(i as u32)
    } else {
        Err(ParseError::InvalidDepthValue { op, modifier })
    }
}

fn parse_relational_obj(value: &Value, op: &'static str) -> Result<RawRelationalObj, ParseError> {
    if matches!(value, Value::Sequence(_)) {
        return Err(ParseError::ArrayFormRemoved { op });
    }
    let mapping = value
        .as_mapping()
        .ok_or(ParseError::GraphOpExpectedScalarOrMapping { op })?;
    if mapping.is_empty() {
        return Err(ParseError::EmptyAnchorMapping { op });
    }
    serde_yaml::from_value(value.clone())
        .map_err(|_| ParseError::GraphOpExpectedScalarOrMapping { op })
}

fn match_to_filter(raw: &RawRelationalObj, op: &'static str) -> Result<Filter, ParseError> {
    let m = raw.match_.as_ref().ok_or(ParseError::MatchMissing { op })?;
    build_filter_at(m.clone(), &[])
}

fn parse_inclusion_arg(value: &Value, op: &'static str) -> Result<InclusionAnchor, ParseError> {
    if let Some(s) = value.as_str() {
        return Ok(InclusionAnchor::new(s, 1, 1));
    }
    let raw = parse_relational_obj(value, op)?;
    if raw.max_distance.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "maxDistance",
        });
    }
    if raw.min_distance.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "minDistance",
        });
    }
    let match_filter = match_to_filter(&raw, op)?;
    let max_depth = match raw.max_depth {
        Some(n) => pos_u32(n, op, "maxDepth")?,
        None => u32::MAX,
    };
    let min_depth = match raw.min_depth {
        Some(n) => pos_u32(n, op, "minDepth")?,
        None => 1,
    };
    if min_depth > max_depth {
        return Err(ParseError::DepthRangeInverted { op });
    }
    Ok(InclusionAnchor::with_match(match_filter, min_depth, max_depth))
}

fn parse_reference_arg(value: &Value, op: &'static str) -> Result<ReferenceAnchor, ParseError> {
    if let Some(s) = value.as_str() {
        return Ok(ReferenceAnchor::new(s, 1, 1));
    }
    let raw = parse_relational_obj(value, op)?;
    if raw.max_depth.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "maxDepth",
        });
    }
    if raw.min_depth.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "minDepth",
        });
    }
    let match_filter = match_to_filter(&raw, op)?;
    let max_distance = match raw.max_distance {
        Some(n) => pos_u32(n, op, "maxDistance")?,
        None => u32::MAX,
    };
    let min_distance = match raw.min_distance {
        Some(n) => pos_u32(n, op, "minDistance")?,
        None => 1,
    };
    if min_distance > max_distance {
        return Err(ParseError::DepthRangeInverted { op });
    }
    Ok(ReferenceAnchor::with_match(match_filter, min_distance, max_distance))
}

fn check_update_conflicts(ops: &[UpdateOperator]) -> Result<(), ParseError> {
    use std::collections::HashSet;
    let mut paths: HashSet<Vec<String>> = HashSet::new();
    let mut all_paths: Vec<Vec<String>> = Vec::new();
    for op in ops {
        let p = match op {
            UpdateOperator::Set { path, .. } | UpdateOperator::Unset { path } => &path.0,
        };
        if !paths.insert(p.clone()) {
            return Err(ParseError::SetUnsetConflict { path: p.clone() });
        }
        all_paths.push(p.clone());
    }

    for (i, a) in all_paths.iter().enumerate() {
        for (j, b) in all_paths.iter().enumerate() {
            if i == j {
                continue;
            }
            if is_prefix_of(a, b) {
                return Err(ParseError::SetUnsetConflict { path: b.clone() });
            }
        }
    }
    Ok(())
}

fn is_prefix_of(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() >= path.len() {
        return false;
    }
    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str, kind: OperationKind) -> Result<Operation, ParseError> {
        parse_operation(yaml, kind)
    }

    fn parse_err(yaml: &str, kind: OperationKind) -> ParseError {
        parse(yaml, kind).expect_err("expected parse failure")
    }


    #[test]
    fn find_rejects_update_field() {
        let err = parse_err("update:\n  $set:\n    x: 1\n", OperationKind::Find);
        assert!(matches!(
            err,
            ParseError::OperationFieldNotAllowed {
                kind: OperationKind::Find,
                field: "update"
            }
        ));
    }

    #[test]
    fn count_rejects_project_and_update() {
        let err = parse_err("project:\n  x: 1\n", OperationKind::Count);
        assert!(matches!(
            err,
            ParseError::OperationFieldNotAllowed {
                kind: OperationKind::Count,
                field: "project"
            }
        ));
    }

    #[test]
    fn update_requires_filter() {
        let err = parse_err("update:\n  $set:\n    x: 1\n", OperationKind::Update);
        assert!(matches!(
            err,
            ParseError::MissingRequiredField {
                kind: OperationKind::Update,
                field: "filter"
            }
        ));
    }

    #[test]
    fn update_requires_update_field() {
        let err = parse_err("filter:\n  status: draft\n", OperationKind::Update);
        assert!(matches!(
            err,
            ParseError::MissingRequiredField {
                kind: OperationKind::Update,
                field: "update"
            }
        ));
    }

    #[test]
    fn delete_requires_filter() {
        let err = parse_err("limit: 10\n", OperationKind::Delete);
        assert!(matches!(
            err,
            ParseError::MissingRequiredField {
                kind: OperationKind::Delete,
                field: "filter"
            }
        ));
    }

    #[test]
    fn delete_with_empty_filter_ok() {

        let op = parse("filter: {}\n", OperationKind::Delete).unwrap();
        assert!(matches!(op, Operation::Delete(_)));
    }


    #[test]
    fn scope_field_rejected_at_wire() {

        let err = parse_err(
            "scope:\n  notes/foo: { self: true }\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::Wire(_)));
    }


    #[test]
    fn filter_mixed_dollar_and_bare_rejected() {
        let err = parse_err(
            "filter:\n  author:\n    $eq: dmytro\n    name: dmytro\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::MixedDollarAndBare { .. }));
    }

    #[test]
    fn filter_double_not_top_level_parses() {
        let op = parse(
            "filter:\n  $not:\n    $not:\n      status: draft\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            let f = find.filter.unwrap();
            match f {
                Filter::Not(inner) => match *inner {
                    Filter::Not(_) => {}
                    other => panic!("expected nested Not, got {:?}", other),
                },
                other => panic!("expected Not, got {:?}", other),
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn filter_empty_and_rejected() {
        let err = parse_err("filter:\n  $and: []\n", OperationKind::Find);
        assert!(matches!(
            err,
            ParseError::EmptyOperatorList { op: "$and" }
        ));
    }

    #[test]
    fn filter_empty_or_rejected() {
        let err = parse_err("filter:\n  $or: []\n", OperationKind::Find);
        assert!(matches!(
            err,
            ParseError::EmptyOperatorList { op: "$or" }
        ));
    }

    #[test]
    fn filter_empty_in_rejected() {
        let err = parse_err("filter:\n  status:\n    $in: []\n", OperationKind::Find);
        assert!(matches!(err, ParseError::EmptyOperatorList { op: "$in" }));
    }

    #[test]
    fn filter_empty_nin_rejected() {
        let err = parse_err("filter:\n  status:\n    $nin: []\n", OperationKind::Find);
        assert!(matches!(err, ParseError::EmptyOperatorList { op: "$nin" }));
    }

    #[test]
    fn filter_empty_type_rejected() {
        let err = parse_err("filter:\n  x:\n    $type: []\n", OperationKind::Find);
        assert!(matches!(err, ParseError::EmptyOperatorList { op: "$type" }));
    }

    #[test]
    fn filter_empty_all_rejected() {
        let err = parse_err("filter:\n  tags:\n    $all: []\n", OperationKind::Find);
        assert!(matches!(err, ParseError::EmptyOperatorList { op: "$all" }));
    }

    #[test]
    fn filter_dotted_key_resolves_to_segments() {
        let op = parse("filter:\n  author.name: dmytro\n", OperationKind::Find).unwrap();
        if let Operation::Find(find) = op {
            let f = find.filter.unwrap();
            if let Filter::Field { path, .. } = f {
                assert_eq!(path.0, vec!["author".to_string(), "name".to_string()]);
            } else {
                panic!("expected Field, got {:?}", f);
            }
        } else {
            panic!()
        }
    }


    #[test]
    fn project_accepts_one_true_null() {
        let op = parse(
            "project:\n  a: 1\n  b: true\n  c: ~\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            let p = find.project.unwrap();
            assert_eq!(p.fields.len(), 3);
            assert_eq!(p.fields[0].output, "a");
        } else {
            panic!()
        }
    }

    #[test]
    fn project_rejects_zero() {
        let err = parse_err("project:\n  a: 0\n", OperationKind::Find);
        assert!(matches!(err, ParseError::InvalidProjectionValue { .. }));
    }

    #[test]
    fn project_rejects_false() {
        let err = parse_err("project:\n  a: false\n", OperationKind::Find);
        assert!(matches!(err, ParseError::InvalidProjectionValue { .. }));
    }

    #[test]
    fn project_string_source_resolves_to_path() {
        let op = parse(
            "project:\n  name: author.name\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            let p = find.project.unwrap();
            assert_eq!(p.fields[0].output, "name");
            match &p.fields[0].source {
                ProjectionSource::Frontmatter(fp) => {
                    assert_eq!(fp.0, vec!["author".to_string(), "name".to_string()]);
                }
                _ => panic!("expected frontmatter source"),
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn project_pseudo_source_resolves() {
        let op = parse(
            "project:\n  body: $content\n  parents: $includedBy\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            let p = find.project.unwrap();
            assert_eq!(p.fields.len(), 2);
            assert_eq!(p.fields[0].output, "body");
            assert!(matches!(
                p.fields[0].source,
                ProjectionSource::Pseudo(PseudoField::Content)
            ));
            assert!(matches!(
                p.fields[1].source,
                ProjectionSource::Pseudo(PseudoField::IncludedBy)
            ));
        } else {
            panic!()
        }
    }

    #[test]
    fn project_unknown_pseudo_rejected() {
        let err = parse_err("project:\n  x: $bogus\n", OperationKind::Find);
        assert!(matches!(err, ParseError::UnknownProjectionSource { .. }));
    }

    #[test]
    fn project_reserved_output_rejected() {
        let err = parse_err("project:\n  $x: 1\n", OperationKind::Find);
        assert!(matches!(err, ParseError::ReservedOutputName { .. }));
    }

    #[test]
    fn project_dotted_output_rejected() {
        let err = parse_err("project:\n  author.name: 1\n", OperationKind::Find);
        assert!(matches!(err, ParseError::NestedProjectionOutput { .. }));
    }

    #[test]
    fn project_and_add_fields_conflict() {
        let err = parse_err(
            "project:\n  title: 1\naddFields:\n  status: 1\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::ProjectAddFieldsConflict));
    }

    #[test]
    fn add_fields_extend_mode() {
        let op = parse(
            "addFields:\n  body: $content\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            let p = find.project.unwrap();
            assert_eq!(p.mode, ProjectionMode::Extend);
            assert_eq!(p.fields.len(), 1);
            assert_eq!(p.fields[0].output, "body");
        } else {
            panic!()
        }
    }


    #[test]
    fn sort_accepts_one_ascending() {
        let op = parse("sort:\n  a: 1\n", OperationKind::Find).unwrap();
        if let Operation::Find(find) = op {
            let s = find.sort.unwrap();
            assert_eq!(s.key.0, vec!["a".to_string()]);
            assert_eq!(s.dir, SortDir::Asc);
        } else {
            panic!()
        }
    }

    #[test]
    fn sort_accepts_minus_one_descending() {
        let op = parse("sort:\n  modified_at: -1\n", OperationKind::Find).unwrap();
        if let Operation::Find(find) = op {
            let s = find.sort.unwrap();
            assert_eq!(s.key.0, vec!["modified_at".to_string()]);
            assert_eq!(s.dir, SortDir::Desc);
        } else {
            panic!()
        }
    }

    #[test]
    fn sort_dotted_key_resolves() {
        let op = parse("sort:\n  author.name: 1\n", OperationKind::Find).unwrap();
        if let Operation::Find(find) = op {
            let s = find.sort.unwrap();
            assert_eq!(s.key.0, vec!["author".to_string(), "name".to_string()]);
        } else {
            panic!()
        }
    }

    #[test]
    fn sort_rejects_zero() {
        let err = parse_err("sort:\n  a: 0\n", OperationKind::Find);
        assert!(matches!(err, ParseError::InvalidSortValue { .. }));
    }

    #[test]
    fn sort_rejects_multi_key() {

        let err = parse_err("sort:\n  a: 1\n  b: -1\n", OperationKind::Find);
        assert!(matches!(err, ParseError::MultiKeySortNotSupportedV1));
    }

    #[test]
    fn sort_empty_rejected() {
        let err = parse_err("sort: {}\n", OperationKind::Find);
        assert!(matches!(err, ParseError::EmptySort));
    }


    #[test]
    fn limit_negative_rejected() {
        let err = parse_err("limit: -1\n", OperationKind::Find);
        assert!(matches!(err, ParseError::NegativeLimit(-1)));
    }

    #[test]
    fn limit_zero_accepted() {
        let op = parse("limit: 0\n", OperationKind::Find).unwrap();
        if let Operation::Find(find) = op {
            let l = find.limit.unwrap();
            assert!(l.is_unbounded());
        } else {
            panic!()
        }
    }


    #[test]
    fn update_empty_rejected() {
        let err = parse_err(
            "filter: {}\nupdate: {}\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::EmptyUpdate));
    }

    #[test]
    fn update_empty_set_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set: {}\n",
            OperationKind::Update,
        );
        assert!(matches!(
            err,
            ParseError::EmptyUpdateOperator { op: "$set" }
        ));
    }

    #[test]
    fn update_reserved_prefix_underscore_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    _x: 1\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::ReservedPrefixField { .. }));
    }

    #[test]
    fn update_reserved_prefix_at_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    \"@user\": foo\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::ReservedPrefixField { .. }));
    }

    #[test]
    fn update_reserved_prefix_in_nested_value_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    author:\n      _hidden: 1\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::ReservedPrefixField { .. }));
    }

    #[test]
    fn update_reserved_prefix_in_deeply_nested_value_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    author:\n      review:\n        \"#tag\": foo\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::ReservedPrefixField { .. }));
    }

    #[test]
    fn update_set_unset_same_path_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    a: 1\n  $unset:\n    a: \"\"\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::SetUnsetConflict { .. }));
    }

    #[test]
    fn update_set_prefix_unset_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    a: 1\n  $unset:\n    \"a.b\": \"\"\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::SetUnsetConflict { .. }));
    }

    #[test]
    fn explicit_and_with_single_child_is_preserved() {
        let op = parse(
            "filter:\n  $and:\n    - status: draft\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            match find.filter.unwrap() {
                Filter::And(children) => assert_eq!(children.len(), 1),
                other => panic!("expected And wrapper, got {:?}", other),
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn explicit_or_with_single_child_is_preserved() {
        let op = parse(
            "filter:\n  $or:\n    - status: draft\n",
            OperationKind::Find,
        )
        .unwrap();
        if let Operation::Find(find) = op {
            match find.filter.unwrap() {
                Filter::Or(children) => assert_eq!(children.len(), 1),
                other => panic!("expected Or wrapper, got {:?}", other),
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn size_float_distinguishes_from_negative() {
        let float_err = parse_err(
            "filter:\n  tags:\n    $size: 1.5\n",
            OperationKind::Find,
        );
        assert!(matches!(float_err, ParseError::OperatorExpectedInteger { op: "$size" }));
        let neg_err = parse_err(
            "filter:\n  tags:\n    $size: -1\n",
            OperationKind::Find,
        );
        assert!(matches!(
            neg_err,
            ParseError::OperatorExpectedNonNegativeInt { op: "$size" }
        ));
    }

    #[test]
    fn filter_path_with_whitespace_rejected() {
        let err = parse_err(
            "filter:\n  \"foo .bar\": 1\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::InvalidPathSegment { .. }));
    }

    #[test]
    fn projection_path_with_whitespace_rejected() {
        let err = parse_err(
            "project:\n  \" foo\": 1\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::InvalidPathSegment { .. }));
    }

    #[test]
    fn sort_path_with_empty_segment_rejected() {
        let err = parse_err(
            "sort:\n  \"a..b\": 1\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::InvalidPathSegment { .. }));
    }

    #[test]
    fn update_set_path_with_control_char_rejected() {
        let err = parse_err(
            "filter: {}\nupdate:\n  $set:\n    \"foo\\tbar\": 1\n",
            OperationKind::Update,
        );
        assert!(matches!(err, ParseError::InvalidPathSegment { .. }));
    }

    #[test]
    fn update_set_dotted_path_resolves() {
        let op = parse(
            "filter: {}\nupdate:\n  $set:\n    \"a.b.c\": 1\n",
            OperationKind::Update,
        )
        .unwrap();
        if let Operation::Update(u) = op {
            assert_eq!(u.update.operators.len(), 1);
            if let UpdateOperator::Set { path, .. } = &u.update.operators[0] {
                assert_eq!(path.0, vec!["a", "b", "c"]);
            } else {
                panic!()
            }
        } else {
            panic!()
        }
    }
}
