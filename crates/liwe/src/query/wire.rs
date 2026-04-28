use serde::Deserialize;
use serde_yaml::{Mapping, Value};


#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawOperation {
    #[serde(default)]
    pub filter: Option<Mapping>,
    #[serde(default)]
    pub project: Option<Mapping>,
    #[serde(default)]
    pub sort: Option<Mapping>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub update: Option<Mapping>,
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
        assert_eq!(op.sort.as_ref().unwrap()["modified_at"], -1);
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
