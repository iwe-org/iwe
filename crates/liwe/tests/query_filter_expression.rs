use liwe::query::{parse_filter_expression, Filter, KeyOp, ParseError};

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
fn parses_top_level_bare_and_or() {
    let f = parse_filter_expression("{type: tracker, $or: [{status: open}, {status: pending}]}")
        .unwrap();
    let parts = match f {
        Filter::And(p) => p,
        other => panic!("expected And, got {:?}", other),
    };
    assert_eq!(parts.len(), 2);
    match &parts[0] {
        Filter::Or(branches) => assert_eq!(branches.len(), 2),
        other => panic!("expected Or first (dollar group), got {:?}", other),
    }
    match &parts[1] {
        Filter::Field { path, op: _ } => {
            assert_eq!(path.segments(), &["type".to_string()]);
        }
        other => panic!("expected Field second (bare group), got {:?}", other),
    }
}

#[test]
fn parses_top_level_bare_and_and() {
    let f = parse_filter_expression("{a: 1, $and: [{b: 2}]}").unwrap();
    let parts = match f {
        Filter::And(p) => p,
        other => panic!("expected And, got {:?}", other),
    };
    assert_eq!(parts.len(), 2);
    match &parts[0] {
        Filter::And(inner) => assert_eq!(inner.len(), 1),
        other => panic!("expected inner And first, got {:?}", other),
    }
    match &parts[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["a".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn parses_top_level_bare_and_nor() {
    let f = parse_filter_expression("{a: 1, $nor: [{b: 2}]}").unwrap();
    let parts = match f {
        Filter::And(p) => p,
        other => panic!("expected And, got {:?}", other),
    };
    assert_eq!(parts.len(), 2);
    match &parts[0] {
        Filter::Nor(inner) => assert_eq!(inner.len(), 1),
        other => panic!("expected Nor first, got {:?}", other),
    }
    match &parts[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["a".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn rejects_top_level_not() {
    let err = parse_filter_expression("{$not: {b: 2}}").unwrap_err();
    match err {
        ParseError::TopLevelNotNotSupported { ref path } => {
            assert!(path.is_empty());
        }
        other => panic!("expected TopLevelNotNotSupported, got {:?}", other),
    }
}

#[test]
fn parses_top_level_bare_and_key() {
    let f = parse_filter_expression("{type: t, $key: notes/foo}").unwrap();
    let parts = match f {
        Filter::And(p) => p,
        other => panic!("expected And, got {:?}", other),
    };
    assert_eq!(parts.len(), 2);
    match &parts[0] {
        Filter::Key(KeyOp::Eq(k)) => assert_eq!(k.to_string(), "notes/foo"),
        other => panic!("expected Key(Eq) first, got {:?}", other),
    }
    match &parts[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["type".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn parses_top_level_multiple_bare_and_multiple_dollar() {
    let f = parse_filter_expression("{a: 1, b: 2, $or: [{c: 3}], $and: [{d: 4}]}").unwrap();
    let parts = match f {
        Filter::And(p) => p,
        other => panic!("expected And, got {:?}", other),
    };
    assert_eq!(parts.len(), 4);
    assert!(matches!(&parts[0], Filter::And(_) | Filter::Or(_)));
    assert!(matches!(&parts[1], Filter::And(_) | Filter::Or(_)));
    assert!(matches!(&parts[2], Filter::Field { .. }));
    assert!(matches!(&parts[3], Filter::Field { .. }));
}

#[test]
fn parses_mix_inside_or_branch() {
    let f = parse_filter_expression("$or: [{a: 1, $nor: [{b: 2}]}, {c: 3}]").unwrap();
    let branches = match f {
        Filter::Or(b) => b,
        other => panic!("expected Or, got {:?}", other),
    };
    assert_eq!(branches.len(), 2);
    let first = match &branches[0] {
        Filter::And(p) => p,
        other => panic!("expected And inside Or branch, got {:?}", other),
    };
    assert_eq!(first.len(), 2);
    match &first[0] {
        Filter::Nor(inner) => assert_eq!(inner.len(), 1),
        other => panic!("expected Nor first, got {:?}", other),
    }
    match &first[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["a".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn parses_mix_inside_and_branch() {
    let f = parse_filter_expression("$and: [{a: 1, $or: [{b: 2}]}]").unwrap();
    let branches = match f {
        Filter::And(b) => b,
        other => panic!("expected And, got {:?}", other),
    };
    assert_eq!(branches.len(), 1);
    let inner = match &branches[0] {
        Filter::And(p) => p,
        other => panic!("expected And inside And branch, got {:?}", other),
    };
    assert_eq!(inner.len(), 2);
    match &inner[0] {
        Filter::Or(_) => {}
        other => panic!("expected Or first, got {:?}", other),
    }
    match &inner[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["a".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn parses_mix_inside_nor_branch() {
    let f = parse_filter_expression("$nor: [{a: 1, $or: [{b: 2}]}]").unwrap();
    let branches = match f {
        Filter::Nor(b) => b,
        other => panic!("expected Nor, got {:?}", other),
    };
    assert_eq!(branches.len(), 1);
    let inner = match &branches[0] {
        Filter::And(p) => p,
        other => panic!("expected And inside Nor branch, got {:?}", other),
    };
    assert_eq!(inner.len(), 2);
    match &inner[0] {
        Filter::Or(_) => {}
        other => panic!("expected Or first, got {:?}", other),
    }
    match &inner[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["a".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn parses_mix_inside_graph_anchor_match() {
    let f = parse_filter_expression("$includedBy: { match: {a: 1, $key: notes/foo}, maxDepth: 3 }")
        .unwrap();
    let anchor = match f {
        Filter::IncludedBy(a) => a,
        other => panic!("expected IncludedBy, got {:?}", other),
    };
    assert_eq!(anchor.max_depth, 3);
    let parts = match &anchor.match_filter {
        Filter::And(p) => p,
        other => panic!("expected And in match, got {:?}", other),
    };
    assert_eq!(parts.len(), 2);
    match &parts[0] {
        Filter::Key(KeyOp::Eq(k)) => assert_eq!(k.to_string(), "notes/foo"),
        other => panic!("expected Key(Eq) first, got {:?}", other),
    }
    match &parts[1] {
        Filter::Field { path, op: _ } => assert_eq!(path.segments(), &["a".to_string()]),
        other => panic!("expected Field second, got {:?}", other),
    }
}

#[test]
fn rejects_mix_inside_field_value_mapping() {
    let err = parse_filter_expression("{author: {$eq: alice, name: alice}}").unwrap_err();
    match err {
        ParseError::MixedDollarAndBare { ref path } => {
            assert_eq!(path, &vec!["author".to_string()]);
        }
        other => panic!("expected MixedDollarAndBare, got {:?}", other),
    }
}

#[test]
fn rejects_mix_inside_field_level_not() {
    let err = parse_filter_expression("{score: {$not: {$gt: 5, extra: 1}}}").unwrap_err();
    match err {
        ParseError::MixedDollarAndBare { ref path } => {
            assert_eq!(path, &vec!["score".to_string()]);
        }
        other => panic!("expected MixedDollarAndBare, got {:?}", other),
    }
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
    let expr = "$includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }";
    let f = parse_filter_expression(expr).unwrap();
    match f {
        Filter::IncludedBy(anchor) => {
            match &anchor.match_filter {
                Filter::Key(KeyOp::Eq(k)) => assert_eq!(k.to_string(), "projects/alpha"),
                other => panic!("expected Key(Eq), got {:?}", other),
            }
            assert_eq!(anchor.max_depth, 5);
        }
        other => panic!("expected IncludedBy, got {:?}", other),
    }
}

#[test]
fn parses_graph_anchor_scalar_shorthand() {
    let expr = "$includedBy: projects/alpha";
    let f = parse_filter_expression(expr).unwrap();
    match f {
        Filter::IncludedBy(anchor) => {
            match &anchor.match_filter {
                Filter::Key(KeyOp::Eq(k)) => assert_eq!(k.to_string(), "projects/alpha"),
                other => panic!("expected Key(Eq), got {:?}", other),
            }
            assert_eq!(anchor.max_depth, 1);
            assert_eq!(anchor.min_depth, 1);
        }
        other => panic!("expected IncludedBy, got {:?}", other),
    }
}

#[test]
fn malformed_yaml_is_an_error() {
    let err = parse_filter_expression("status: draft, : bad");
    assert!(err.is_err(), "expected parse error");
}
