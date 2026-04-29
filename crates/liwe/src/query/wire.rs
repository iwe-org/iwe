use serde::Deserialize;
use serde_yaml::{Mapping, Value};


#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawOperation {
    #[serde(default)]
    pub filter: Option<RawFilter>,
    #[serde(default)]
    pub project: Option<RawProjection>,
    #[serde(default)]
    pub sort: Option<RawSort>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub update: Option<RawUpdate>,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RawFilter(pub Mapping);

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RawProjection(pub Mapping);

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RawSort(pub Mapping);

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawUpdate {
    #[serde(rename = "$set", default)]
    pub set: Option<Mapping>,
    #[serde(rename = "$unset", default)]
    pub unset: Option<Mapping>,
}


pub fn parse(yaml: &str) -> Result<RawOperation, serde_yaml::Error> {
    if yaml.trim().is_empty() {
        return Ok(RawOperation::default());
    }
    let value: Value = serde_yaml::from_str(yaml)?;
    if matches!(value, Value::Null) {
        return Ok(RawOperation::default());
    }
    serde_yaml::from_value(value)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawKeyArg {
    Scalar(String),
    Map(RawKeyOpMap),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawKeyOpMap {
    #[serde(rename = "$eq", default)]
    pub eq: Option<String>,
    #[serde(rename = "$ne", default)]
    pub ne: Option<String>,
    #[serde(rename = "$in", default)]
    pub in_: Option<Vec<Value>>,
    #[serde(rename = "$nin", default)]
    pub nin: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawMaxDepth {
    Number(i64),
    Symbol(String),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawNumExprMap {
    #[serde(rename = "$eq", default)]
    pub eq: Option<i64>,
    #[serde(rename = "$ne", default)]
    pub ne: Option<i64>,
    #[serde(rename = "$gt", default)]
    pub gt: Option<i64>,
    #[serde(rename = "$gte", default)]
    pub gte: Option<i64>,
    #[serde(rename = "$lt", default)]
    pub lt: Option<i64>,
    #[serde(rename = "$lte", default)]
    pub lte: Option<i64>,
    #[serde(rename = "$in", default)]
    pub in_: Option<Vec<i64>>,
    #[serde(rename = "$nin", default)]
    pub nin: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawCountValue {
    Number(i64),
    Map(RawNumExprMap),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCountArgMap {
    #[serde(rename = "$count", default)]
    pub count: Option<RawCountValue>,
    #[serde(rename = "$maxDepth", default)]
    pub max_depth: Option<RawMaxDepth>,
    #[serde(rename = "$minDepth", default)]
    pub min_depth: Option<i64>,
    #[serde(rename = "$maxDistance", default)]
    pub max_distance: Option<Value>,
    #[serde(rename = "$minDistance", default)]
    pub min_distance: Option<Value>,
    #[serde(rename = "$eq", default)]
    pub eq: Option<i64>,
    #[serde(rename = "$ne", default)]
    pub ne: Option<i64>,
    #[serde(rename = "$gt", default)]
    pub gt: Option<i64>,
    #[serde(rename = "$gte", default)]
    pub gte: Option<i64>,
    #[serde(rename = "$lt", default)]
    pub lt: Option<i64>,
    #[serde(rename = "$lte", default)]
    pub lte: Option<i64>,
    #[serde(rename = "$in", default)]
    pub in_: Option<Vec<i64>>,
    #[serde(rename = "$nin", default)]
    pub nin: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawCountArg {
    Number(i64),
    Map(RawCountArgMap),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawKeyValue {
    Scalar(String),
    Other(Value),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAnchor {
    #[serde(rename = "$key", default)]
    pub key: Option<RawKeyValue>,
    #[serde(rename = "$maxDepth", default)]
    pub max_depth: Option<i64>,
    #[serde(rename = "$minDepth", default)]
    pub min_depth: Option<i64>,
    #[serde(rename = "$maxDistance", default)]
    pub max_distance: Option<i64>,
    #[serde(rename = "$minDistance", default)]
    pub min_distance: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawAnchorArg {
    Single(RawAnchor),
    List(Vec<RawAnchor>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn parse_ok(yaml: &str) -> RawOperation {
        parse(yaml).expect("expected wire parse to succeed")
    }

    fn parse_err(yaml: &str) -> String {
        parse(yaml)
            .expect_err("expected wire parse to fail")
            .to_string()
    }

    #[test]
    fn empty_yaml_parses_to_default() {
        let op = parse_ok("");
        assert!(op.filter.is_none() && op.update.is_none());
    }

    #[test]
    fn full_operation_round_trips() {
        let op = parse_ok(indoc! {"
            filter:
              status: draft
            project:
              title: 1
            sort:
              modified_at: -1
            limit: 10
            update:
              $set:
                reviewed: true
        "});
        assert!(op.filter.is_some());
        assert!(op.project.is_some());
        assert_eq!(op.sort.as_ref().unwrap().0["modified_at"], -1);
        assert_eq!(op.limit, Some(10));
        assert!(op.update.is_some());
    }

    #[test]
    fn unknown_top_level_field_rejected() {
        let err = parse_err("bogus: 1\n");
        assert!(err.contains("bogus"), "{}", err);
    }

    #[test]
    fn scope_field_rejected() {

        let err = parse_err("scope:\n  notes/foo: { self: true }\n");
        assert!(err.contains("scope"), "{}", err);
    }

    #[test]
    fn limit_string_rejected() {
        let err = parse_err("limit: \"20\"\n");
        assert!(
            err.to_lowercase().contains("invalid type")
                || err.to_lowercase().contains("expected"),
            "{}",
            err
        );
    }

    #[test]
    fn sort_parses_as_raw_mapping() {


        let op = parse_ok("sort:\n  modified_at: -1\n");
        assert!(op.sort.is_some());
    }
}
