use liwe::query::{parse_filter_expression, Filter, KeyOp};

#[test]
fn parses_block_style_field_eq() {
    let f = parse_filter_expression("status: draft").unwrap();
    match f {
        Filter::Field { path, op } => {
            assert_eq!(path.segments(), &["status".to_string()]);
            let _ = op;
        }
        other => panic!("expected Field, got {:?}", other),
    }
}

#[test]
fn parses_flow_style_mapping() {
    let f = parse_filter_expression("{status: draft, priority: 5}").unwrap();
    match f {
        Filter::And(parts) => assert_eq!(parts.len(), 2),
        other => panic!("expected And, got {:?}", other),
    }
}

#[test]
fn parses_top_level_dollar_operator() {
    let f = parse_filter_expression("$key: notes/foo").unwrap();
    match f {
        Filter::Key(KeyOp::Eq(k)) => assert_eq!(k.to_string(), "notes/foo"),
        other => panic!("expected Key($eq), got {:?}", other),
    }
}

#[test]
fn parses_field_op_expression() {
    let f = parse_filter_expression("priority: { $gt: 3 }").unwrap();
    match f {
        Filter::Field { path, op: _ } => {
            assert_eq!(path.segments(), &["priority".to_string()]);
        }
        other => panic!("expected Field, got {:?}", other),
    }
}

#[test]
fn empty_input_yields_empty_and() {
    let f = parse_filter_expression("").unwrap();
    match f {
        Filter::And(parts) => assert!(parts.is_empty()),
        other => panic!("expected empty And, got {:?}", other),
    }
}

#[test]
fn whitespace_only_input_yields_empty_and() {
    let f = parse_filter_expression("   \n  ").unwrap();
    match f {
        Filter::And(parts) => assert!(parts.is_empty()),
        other => panic!("expected empty And, got {:?}", other),
    }
}

#[test]
fn rejects_mixed_dollar_and_bare_at_same_level() {
    let err = parse_filter_expression("{$eq: foo, name: bar}");
    assert!(err.is_err(), "expected MixedDollarAndBare error");
}

#[test]
fn rejects_double_not() {
    let err = parse_filter_expression("$not: { $not: { status: draft } }");
    assert!(err.is_err(), "expected DoubleNot error");
}

#[test]
fn parses_or_with_list_of_filters() {
    let expr = "$or: [{ status: draft }, { status: review }]";
    let f = parse_filter_expression(expr).unwrap();
    match f {
        Filter::Or(parts) => assert_eq!(parts.len(), 2),
        other => panic!("expected Or, got {:?}", other),
    }
}

#[test]
fn parses_graph_anchor_with_max_depth() {
    let expr = "$includedBy: { $key: projects/alpha, $maxDepth: 5 }";
    let f = parse_filter_expression(expr).unwrap();
    match f {
        Filter::IncludedBy(anchors) => {
            assert_eq!(anchors.len(), 1);
            assert_eq!(anchors[0].key.to_string(), "projects/alpha");
            assert_eq!(anchors[0].max_depth, 5);
        }
        other => panic!("expected IncludedBy, got {:?}", other),
    }
}

#[test]
fn malformed_yaml_is_an_error() {
    let err = parse_filter_expression("status: draft, : bad");
    assert!(err.is_err(), "expected parse error");
}
