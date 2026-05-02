# IWE Query Language — Grammar Reference

This document collects the full BNF grammar for the IWE query language: operation documents, filter, projection, sort, limit, update operators, and graph operators. The semantic rules — type coercion, missing-field behavior, equality, edge model, walk semantics — live in `query-language-spec.md` and `query-graph-spec.md`. This file is the syntactic source of truth.

## Notation

- `::=` defines a production.
- `|` separates alternatives.
- `[X, ...]` is a YAML sequence of `X`. Empty sequences are noted as parse-time errors where they apply.
- `{ K: V, ... }` is a YAML mapping. Required vs optional entries are annotated inline.
- `# ...` introduces a comment.
- All literal `$`-prefixed names are operator keywords; user frontmatter field names cannot begin with `$` (see `query-language-spec.md` §2.3).

## 1. Operation documents

```
operation ::= find_op | count_op | update_op | delete_op

find_op ::= {
    filter:    filter                               (optional, default {})
    project:   projection                           (optional, mutually exclusive with addFields)
    addFields: projection                           (optional, mutually exclusive with project)
    sort:      sort                                 (optional)
    limit:     limit                                (optional)
}

count_op ::= {
    filter: filter                                  (optional, default {})
    sort:   sort                                    (optional)
    limit:  limit                                   (optional)
}

update_op ::= {
    filter: filter                                  (required)
    sort:   sort                                    (optional)
    limit:  limit                                   (optional)
    update: update_doc                              (required)
}

delete_op ::= {
    filter: filter                                  (required)
    sort:   sort                                    (optional)
    limit:  limit                                   (optional)
}
```

Operation-inappropriate fields are parse-time errors (e.g. `project` outside `find`, `update` outside `update`). `project` and `addFields` cannot both be set in a single `find_op` (see `query-projection-spec.md` §3.5).

## 2. Filter

```
filter ::= { (filter_entry)* }                     # entries AND-composed at top level

filter_entry ::=
    field_path : field_predicate
  | logical_op
  | graph_op

field_predicate ::=
    value                                          # implicit $eq (§4.1)
  | operator_expr
  | nested_filter

operator_expr ::= { ($_field_op : V)+ }            # all keys $-prefixed; multiple keys ANDed

nested_filter ::= { (sub_field_entry)+ }           # all keys non-$-prefixed

sub_field_entry ::= field_path : field_predicate

# A mapping that mixes $-prefixed and non-$-prefixed keys at the same level is a parse-time error.
```

### 2.1 Logical operators

```
logical_op ::=
    $and : [filter, ...]                           # non-empty
  | $or  : [filter, ...]                           # non-empty
  | $nor : [filter, ...]                           # non-empty
  | $not : filter                                  # single filter, not list
```

### 2.2 Field operators

```
$_field_op ::=
    comparison_op
  | element_op
  | array_op
  | $not : operator_expr                           # per-field negation

comparison_op ::=
    $eq:  value
  | $ne:  value
  | $gt:  value
  | $gte: value
  | $lt:  value
  | $lte: value
  | $in:  [value, ...]                             # non-empty
  | $nin: [value, ...]                             # non-empty

element_op ::=
    $exists: bool
  | $type:   type_name | [type_name, ...]          # non-empty list

array_op ::=
    $all:  [value, ...]                            # non-empty
  | $size: non_neg_int

type_name ::=
    "string" | "number" | "boolean" | "null"
  | "array"  | "object" | "date"    | "datetime"

# Type names are YAML strings only. The bare YAML null literal ($type: null) is a
# parse-time error — write $type: "null" to test for the null type.
```

### 2.3 Field paths

```
field_path ::= segment ("." segment)*              # dotted shorthand
segment    ::= identifier                          # see query-language-spec.md §2.3
                                                   # non-empty; no whitespace; no control chars; no `.`;
                                                   # first char not in $, _, ., #, @
```

A nested mapping (`author: { name: ... }`) is equivalent to the dotted form (`author.name: ...`). Field names containing a literal `.` are not addressable in v1 — neither shorthand nor nested-mapping form can reference them, because path resolution always splits on `.`.

## 3. Graph operators

```
graph_op ::=
    $key          : key_op
  | $includes     : relational_arg
  | $includedBy   : relational_arg
  | $references   : relational_arg
  | $referencedBy : relational_arg
```

The `filter` production used inside relational operators (`match` field, §3.2) is the same `filter` production from §2 — the grammar is mutually recursive (filter contains graph_op contains relational_arg.match contains filter).

### 3.1 Identity

```
key_op ::= key | key_expr

key_expr ::=
    { $eq:  key }
  | { $ne:  key }
  | { $in:  [key, ...] }                           # non-empty
  | { $nin: [key, ...] }                           # non-empty

# $gt / $gte / $lt / $lte on $key are parse-time errors.
```

### 3.2 Relational operators

```
relational_arg ::= key | relational_obj

relational_obj ::= {
    match:       filter                            (required)
    maxDepth:    pos_int                           (inclusion ops, optional; absent = unbounded)
    minDepth:    pos_int                           (inclusion ops, optional; absent = 1)
    maxDistance: pos_int                           (reference ops, optional; absent = unbounded)
    minDistance: pos_int                           (reference ops, optional; absent = 1)
}

# Scalar `key` shorthand expands to:
#   - inclusion ops:  { match: { $key: KEY }, maxDepth: 1 }
#   - reference ops:  { match: { $key: KEY }, maxDistance: 1 }
# Inclusion-edge ops accept maxDepth / minDepth only;
#   maxDistance / minDistance are parse-time errors.
# Reference-edge ops accept maxDistance / minDistance only;
#   maxDepth / minDepth are parse-time errors.
# match is required; an object without match is a parse-time error.
# Empty mapping {} is a parse-time error. The array form [...] is a parse-time error.
# All walk-parameter values are positive integers (>= 1).
# No -1 sentinel; absence is the unbounded signal in the full relational_obj form.
# Field names inside relational_obj are bare — $-prefix is reserved for evaluating operators.
# The recognized key set is closed: any key other than match / maxDepth / minDepth /
#   maxDistance / minDistance is a parse-time error (unknown keys are not silently ignored).
# The filter inside `match` is the §2 filter production — the grammar is mutually recursive.
```

## 4. Projection

```
projection ::= { (project_entry)+ }

project_entry ::=
    field_path : include_marker
  | field_path : nested_projection

include_marker  ::= 1 | true | null                # all three mean "include"
                                                   # type-strict: integer 1, bool true, or YAML null
                                                   # 0, false, "1", "true", "null", 1.0 → parse-time error

nested_projection ::= { (project_entry)+ }
```

## 5. Sort

```
sort     ::= { field_path : sort_dir }             # exactly one entry in v1
sort_dir ::= 1 | -1                                # type-strict integer; YAML +1 normalizes to 1 and is accepted;
                                                   # 1.0, "1", true, null → parse-time error
```

## 6. Limit

```
limit ::= non_neg_int                              # 0 = no limit
```

## 7. Update document

```
update_doc ::= { (update_op_entry)+ }              # at least one operator

update_op_entry ::=
    $set:   { (field_path : value)+ }              # body must be non-empty
  | $unset: { (field_path : any_value)+ }          # body must be non-empty; values ignored

# Empty $set: {} / $unset: {} is a parse-time error (grammar requires +).
# Targeting a reserved-prefix segment (_, $, ., #, @ as first character of any segment in
#   any path — top-level, dotted, or nested mapping key, recursively) is a parse-time error.
# Two paths in $set / $unset conflict when, after canonicalizing nested-mapping form
#   to dotted form per §4.4, one path equals or is a prefix of the other. Conflicts are
#   parse-time errors. The check applies across operators ($set vs $unset) and within
#   a single operator (two $set entries).
# A dotted $set path that traverses a present-but-non-mapping intermediate coerces the
#   intermediate to a fresh mapping holding the new leaf (per language spec §8.1, $set).
#   Not a parse-time error and not a runtime failure.
# Mapping values in $set replace wholesale; use dotted shorthand to write subset leaves.
```

## 8. Primitives

```
key         ::= string                             # document key (relative path without .md)
identifier  ::= YAML name; non-empty; no whitespace; no control chars; no `.`;
                first char not in $, _, ., #, @
value       ::= scalar | array | mapping | null
scalar      ::= string | number | boolean | date | datetime
array       ::= [value, ...]
mapping     ::= { (string : value)+ }
bool        ::= true | false
non_neg_int ::= integer ≥ 0
pos_int     ::= integer ≥ 1
int         ::= integer ≥ 0
any_value   ::= value                              # placeholder; ignored by $unset
```
