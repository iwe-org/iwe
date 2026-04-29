use serde_yaml::{Mapping, Value};

use crate::model::Key;
use crate::query::document::{
    CountArg, CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, GraphOp, InclusionAnchor,
    KeyOp, Limit, MaxDepth, NumExpr, NumOp, Operation, OperationKind, Projection,
    ReferenceAnchor, Sort, SortDir, Update, UpdateOp, UpdateOperator, YamlType,
};
use crate::query::wire::{
    self, RawAnchor, RawAnchorArg, RawCountArg, RawCountArgMap, RawCountValue, RawFilter,
    RawKeyOpMap, RawKeyValue, RawMaxDepth, RawNumExprMap, RawOperation, RawProjection, RawSort,
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
    DoubleNot,
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
    UnknownTypeName {
        name: String,
    },
    InvalidProjectionValue {
        path: Vec<String>,
    },
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
    NonStringKey,
    GraphOpExpectedScalarOrMapping {
        op: &'static str,
    },
    GraphOpExpectedAnchor {
        op: &'static str,
    },
    EmptyAnchorList {
        op: &'static str,
    },
    AnchorMissingBound {
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
    WalkKeyNotScalar {
        op: &'static str,
    },
    AnchorMissingKey {
        op: &'static str,
    },
    UnknownAnchorField {
        op: &'static str,
        field: String,
    },
    InvalidDepthValue {
        op: &'static str,
        modifier: &'static str,
    },
    MissingCountField {
        op: &'static str,
    },
    InvalidCountValue {
        op: &'static str,
    },
    NumExprMixedInWithComparisons {
        op: &'static str,
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


fn build_find(raw: RawOperation) -> Result<FindOp, ParseError> {
    if raw.update.is_some() {
        return Err(ParseError::OperationFieldNotAllowed {
            kind: OperationKind::Find,
            field: "update",
        });
    }
    Ok(FindOp {
        filter: raw.filter.map(build_filter).transpose()?,
        project: raw.project.map(build_projection).transpose()?,
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
            clauses.push(build_logical_op(&op, value, path)?);
        }
        if clauses.len() == 1 {
            Ok(clauses.into_iter().next().unwrap())
        } else {
            Ok(Filter::And(clauses))
        }
    } else {
        let mut clauses: Vec<Filter> = Vec::new();
        for key_str in bare_keys {
            let segments = if key_str.contains('.') {
                key_str.split('.').map(|s| s.to_string()).collect()
            } else {
                vec![key_str.clone()]
            };
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

fn build_logical_op(op: &str, value: &Value, path: &[String]) -> Result<Filter, ParseError> {
    if let Some(graph_op_name) = graph_op_name(op) {
        return build_graph_op(graph_op_name, value).map(Filter::Graph);
    }
    match op {
        "$and" | "$or" => {
            let list = value
                .as_sequence()
                .ok_or(ParseError::OperatorExpectedList {
                    op: static_op_name(op),
                })?;
            if list.is_empty() {
                return Err(ParseError::EmptyOperatorList {
                    op: static_op_name(op),
                });
            }
            let mut sub = Vec::with_capacity(list.len());
            for elem in list {
                let m = elem
                    .as_mapping()
                    .ok_or(ParseError::OperatorExpectedMapping {
                        op: static_op_name(op),
                    })?
                    .clone();
                sub.push(build_filter_at(m, path)?);
            }
            if op == "$and" {
                Ok(Filter::And(sub))
            } else {
                Ok(Filter::Or(sub))
            }
        }
        "$not" => {
            let m = value
                .as_mapping()
                .ok_or(ParseError::OperatorExpectedMapping { op: "$not" })?
                .clone();

            if m.len() == 1 {
                if let Some(Value::String(s)) = m.keys().next() {
                    if s == "$not" {
                        return Err(ParseError::DoubleNot);
                    }
                }
            }
            let inner = build_filter_at(m, path)?;
            Ok(Filter::Not(Box::new(inner)))
        }
        other => Err(ParseError::UnknownOperator {
            op: other.to_string(),
            path: path.to_vec(),
        }),
    }
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
                    .ok_or(ParseError::OperatorExpectedNonNegativeInt { op: "$size" })?;
                if i < 0 {
                    return Err(ParseError::OperatorExpectedNonNegativeInt { op: "$size" });
                }
                Ok(FieldOp::Size(i as u64))
            }
            _ => Err(ParseError::OperatorExpectedNonNegativeInt { op: "$size" }),
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
            if dollar_keys.len() == 1 && dollar_keys[0] == "$not" {
                return Err(ParseError::DoubleNot);
            }


            if dollar_keys.len() == 1 {
                let inner_op = dollar_keys[0].clone();
                let v = m[Value::String(inner_op.clone())].clone();
                let inner = build_field_op(&inner_op, v, path)?;
                Ok(FieldOp::Not(Box::new(inner)))
            } else {


                Err(ParseError::OperatorExpectedMapping { op: "$not" })
            }
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


fn build_projection(raw: RawProjection) -> Result<Projection, ParseError> {
    let mut fields: Vec<FieldPath> = Vec::new();
    walk_projection(&raw.0, &[], &mut fields)?;
    Ok(Projection { fields })
}

fn walk_projection(
    map: &Mapping,
    parent: &[String],
    out: &mut Vec<FieldPath>,
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
        match v {
            Value::Number(n) if n.as_i64() == Some(1) => {
                out.push(FieldPath(segments));
            }
            Value::Bool(true) => {
                out.push(FieldPath(segments));
            }
            Value::Null => {
                out.push(FieldPath(segments));
            }
            Value::Mapping(inner) => {
                walk_projection(inner, &segments, out)?;
            }
            _ => {
                return Err(ParseError::InvalidProjectionValue { path: segments });
            }
        }
    }
    Ok(())
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
    Ok(Sort { key: path, dir })
}


fn build_limit(raw: i64) -> Result<Limit, ParseError> {
    if raw < 0 {
        Err(ParseError::NegativeLimit(raw))
    } else {
        Ok(Limit(raw as u64))
    }
}


fn build_update_doc(raw: RawUpdate) -> Result<Update, ParseError> {
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
        check_reserved_prefix(&segments)?;


        out.push(UpdateOperator::Set {
            path: FieldPath(segments),
            value: v.clone(),
        });
    }
    Ok(())
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

fn graph_op_name(op: &str) -> Option<&'static str> {
    match op {
        "$key" => Some("$key"),
        "$includesCount" => Some("$includesCount"),
        "$includedByCount" => Some("$includedByCount"),
        "$includes" => Some("$includes"),
        "$includedBy" => Some("$includedBy"),
        "$references" => Some("$references"),
        "$referencedBy" => Some("$referencedBy"),
        _ => None,
    }
}

fn build_graph_op(op: &'static str, value: &Value) -> Result<GraphOp, ParseError> {
    match op {
        "$key" => Ok(GraphOp::Key(parse_key_op(value, op)?)),
        "$includesCount" => Ok(GraphOp::IncludesCount(parse_count_arg(value, op)?)),
        "$includedByCount" => Ok(GraphOp::IncludedByCount(parse_count_arg(value, op)?)),
        "$includes" => Ok(GraphOp::Includes(parse_inclusion_anchors(value, op)?)),
        "$includedBy" => Ok(GraphOp::IncludedBy(parse_inclusion_anchors(value, op)?)),
        "$references" => Ok(GraphOp::References(parse_reference_anchors(value, op)?)),
        "$referencedBy" => Ok(GraphOp::ReferencedBy(parse_reference_anchors(value, op)?)),
        _ => unreachable!(),
    }
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

fn parse_count_arg(value: &Value, op: &'static str) -> Result<CountArg, ParseError> {
    let raw: RawCountArg = serde_yaml::from_value(value.clone())
        .map_err(|_| ParseError::GraphOpExpectedScalarOrMapping { op })?;
    match raw {
        RawCountArg::Number(i) => bare_count(i, op),
        RawCountArg::Map(m) => count_arg_from_map(m, op),
    }
}

fn bare_count(i: i64, op: &'static str) -> Result<CountArg, ParseError> {
    if i < 0 {
        return Err(ParseError::InvalidCountValue { op });
    }
    Ok(CountArg {
        count: NumExpr::eq(i as u64),
        min_depth: 1,
        max_depth: MaxDepth::Bounded(1),
    })
}

fn count_arg_from_map(m: RawCountArgMap, op: &'static str) -> Result<CountArg, ParseError> {
    if m.max_distance.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "$maxDistance",
        });
    }
    if m.min_distance.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "$minDistance",
        });
    }
    let has_depth_key = m.max_depth.is_some() || m.min_depth.is_some();
    let bare_ops = num_expr_from_count_map(&m);
    if let Some(raw_count) = m.count {
        if !bare_ops.0.is_empty() {
            return Err(ParseError::MissingCountField { op });
        }
        let count = num_expr_from_value(raw_count, op)?;
        let min_depth = match m.min_depth {
            Some(i) => pos_u32(i, op, "$minDepth")?,
            None => 1,
        };
        let max_depth = match m.max_depth {
            Some(d) => max_depth_from_raw(d, op)?,
            None => MaxDepth::Bounded(1),
        };
        let max_bound = match max_depth {
            MaxDepth::Bounded(n) => n,
            MaxDepth::Any => u32::MAX,
        };
        if min_depth > max_bound {
            return Err(ParseError::DepthRangeInverted { op });
        }
        return Ok(CountArg {
            count,
            min_depth,
            max_depth,
        });
    }
    if has_depth_key {
        return Err(ParseError::MissingCountField { op });
    }
    if bare_ops.0.is_empty() {
        return Err(ParseError::MissingCountField { op });
    }
    Ok(CountArg {
        count: bare_ops,
        min_depth: 1,
        max_depth: MaxDepth::Bounded(1),
    })
}

fn num_expr_from_count_map(m: &RawCountArgMap) -> NumExpr {
    let mut ops = Vec::new();
    if let Some(n) = m.eq {
        ops.push(NumOp::Eq(n.max(0) as u64));
    }
    if let Some(n) = m.ne {
        ops.push(NumOp::Ne(n.max(0) as u64));
    }
    if let Some(n) = m.gt {
        ops.push(NumOp::Gt(n.max(0) as u64));
    }
    if let Some(n) = m.gte {
        ops.push(NumOp::Gte(n.max(0) as u64));
    }
    if let Some(n) = m.lt {
        ops.push(NumOp::Lt(n.max(0) as u64));
    }
    if let Some(n) = m.lte {
        ops.push(NumOp::Lte(n.max(0) as u64));
    }
    if let Some(list) = &m.in_ {
        ops.push(NumOp::In(list.iter().map(|n| (*n).max(0) as u64).collect()));
    }
    if let Some(list) = &m.nin {
        ops.push(NumOp::Nin(list.iter().map(|n| (*n).max(0) as u64).collect()));
    }
    NumExpr(ops)
}

fn num_expr_from_value(raw: RawCountValue, op: &'static str) -> Result<NumExpr, ParseError> {
    match raw {
        RawCountValue::Number(i) => {
            if i < 0 {
                return Err(ParseError::InvalidCountValue { op });
            }
            Ok(NumExpr::eq(i as u64))
        }
        RawCountValue::Map(m) => num_expr_from_map(m, op),
    }
}

fn num_expr_from_map(m: RawNumExprMap, op: &'static str) -> Result<NumExpr, ParseError> {
    let mut ops = Vec::new();
    let mut comparison_count = 0u8;
    let mut list_count = 0u8;
    macro_rules! push_int {
        ($field:expr, $variant:ident) => {
            if let Some(n) = $field {
                if n < 0 {
                    return Err(ParseError::InvalidCountValue { op });
                }
                ops.push(NumOp::$variant(n as u64));
                comparison_count += 1;
            }
        };
    }
    push_int!(m.eq, Eq);
    push_int!(m.ne, Ne);
    push_int!(m.gt, Gt);
    push_int!(m.gte, Gte);
    push_int!(m.lt, Lt);
    push_int!(m.lte, Lte);
    if let Some(list) = m.in_ {
        if list.is_empty() {
            return Err(ParseError::EmptyOperatorList { op });
        }
        let nums = list
            .into_iter()
            .map(|n| {
                if n < 0 {
                    Err(ParseError::InvalidCountValue { op })
                } else {
                    Ok(n as u64)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        ops.push(NumOp::In(nums));
        list_count += 1;
    }
    if let Some(list) = m.nin {
        if list.is_empty() {
            return Err(ParseError::EmptyOperatorList { op });
        }
        let nums = list
            .into_iter()
            .map(|n| {
                if n < 0 {
                    Err(ParseError::InvalidCountValue { op })
                } else {
                    Ok(n as u64)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        ops.push(NumOp::Nin(nums));
        list_count += 1;
    }
    if list_count > 0 && (comparison_count > 0 || list_count > 1) {
        return Err(ParseError::NumExprMixedInWithComparisons { op });
    }
    if ops.is_empty() {
        return Err(ParseError::InvalidCountValue { op });
    }
    Ok(NumExpr(ops))
}

fn max_depth_from_raw(raw: RawMaxDepth, op: &'static str) -> Result<MaxDepth, ParseError> {
    match raw {
        RawMaxDepth::Symbol(s) if s == "any" => Ok(MaxDepth::Any),
        RawMaxDepth::Symbol(_) => Err(ParseError::InvalidDepthValue {
            op,
            modifier: "$maxDepth",
        }),
        RawMaxDepth::Number(n) => Ok(MaxDepth::Bounded(pos_u32(n, op, "$maxDepth")?)),
    }
}

fn pos_u32(i: i64, op: &'static str, modifier: &'static str) -> Result<u32, ParseError> {
    if i >= 1 {
        Ok(i as u32)
    } else {
        Err(ParseError::InvalidDepthValue { op, modifier })
    }
}

fn parse_inclusion_anchors(
    value: &Value,
    op: &'static str,
) -> Result<Vec<InclusionAnchor>, ParseError> {
    let raws = parse_anchor_arg(value, op)?;
    raws.into_iter()
        .map(|raw| inclusion_anchor_from_raw(raw, op))
        .collect()
}

fn parse_reference_anchors(
    value: &Value,
    op: &'static str,
) -> Result<Vec<ReferenceAnchor>, ParseError> {
    let raws = parse_anchor_arg(value, op)?;
    raws.into_iter()
        .map(|raw| reference_anchor_from_raw(raw, op))
        .collect()
}

fn parse_anchor_arg(value: &Value, op: &'static str) -> Result<Vec<RawAnchor>, ParseError> {
    if matches!(value, Value::Mapping(m) if m.is_empty()) {
        return Err(ParseError::EmptyAnchorList { op });
    }
    if matches!(value, Value::Sequence(s) if s.is_empty()) {
        return Err(ParseError::EmptyAnchorList { op });
    }
    let raw: RawAnchorArg = serde_yaml::from_value(value.clone())
        .map_err(|_| ParseError::GraphOpExpectedAnchor { op })?;
    Ok(match raw {
        RawAnchorArg::Single(a) => vec![a],
        RawAnchorArg::List(l) => l,
    })
}

fn inclusion_anchor_from_raw(raw: RawAnchor, op: &'static str) -> Result<InclusionAnchor, ParseError> {
    if raw.max_distance.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "$maxDistance",
        });
    }
    if raw.min_distance.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "$minDistance",
        });
    }
    let key = anchor_key(raw.key, op)?;
    if raw.max_depth.is_none() && raw.min_depth.is_none() {
        return Err(ParseError::AnchorMissingBound { op });
    }
    let max_depth = match raw.max_depth {
        Some(n) => pos_u32(n, op, "$maxDepth")?,
        None => u32::MAX,
    };
    let min_depth = match raw.min_depth {
        Some(n) => pos_u32(n, op, "$minDepth")?,
        None => 1,
    };
    if min_depth > max_depth {
        return Err(ParseError::DepthRangeInverted { op });
    }
    Ok(InclusionAnchor {
        key,
        min_depth,
        max_depth,
    })
}

fn reference_anchor_from_raw(
    raw: RawAnchor,
    op: &'static str,
) -> Result<ReferenceAnchor, ParseError> {
    if raw.max_depth.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "$maxDepth",
        });
    }
    if raw.min_depth.is_some() {
        return Err(ParseError::WrongBoundFamily {
            op,
            modifier: "$minDepth",
        });
    }
    let key = anchor_key(raw.key, op)?;
    if raw.max_distance.is_none() && raw.min_distance.is_none() {
        return Err(ParseError::AnchorMissingBound { op });
    }
    let max_distance = match raw.max_distance {
        Some(n) => pos_u32(n, op, "$maxDistance")?,
        None => u32::MAX,
    };
    let min_distance = match raw.min_distance {
        Some(n) => pos_u32(n, op, "$minDistance")?,
        None => 1,
    };
    if min_distance > max_distance {
        return Err(ParseError::DepthRangeInverted { op });
    }
    Ok(ReferenceAnchor {
        key,
        min_distance,
        max_distance,
    })
}

fn anchor_key(raw: Option<RawKeyValue>, op: &'static str) -> Result<Key, ParseError> {
    match raw {
        None => Err(ParseError::AnchorMissingKey { op }),
        Some(RawKeyValue::Scalar(s)) => Ok(Key::name(&s)),
        Some(RawKeyValue::Other(_)) => Err(ParseError::WalkKeyNotScalar { op }),
    }
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
    fn filter_double_not_rejected_top_level() {
        let err = parse_err(
            "filter:\n  $not:\n    $not:\n      status: draft\n",
            OperationKind::Find,
        );
        assert!(matches!(err, ParseError::DoubleNot));
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
    fn project_dotted_resolves() {
        let op = parse("project:\n  author.name: 1\n", OperationKind::Find).unwrap();
        if let Operation::Find(find) = op {
            let p = find.project.unwrap();
            assert_eq!(p.fields[0].0, vec!["author".to_string(), "name".to_string()]);
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
