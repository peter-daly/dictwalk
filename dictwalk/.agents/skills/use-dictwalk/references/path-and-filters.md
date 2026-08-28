# dictwalk Path and Filter Reference

Read this reference when a task needs more than fixed dot traversal or a single basic predicate.

## Selector syntax

| Purpose | Read example | Mutation example |
| --- | --- | --- |
| Nested key | `user.profile.name` | `user.profile.name` |
| Collect a field from list items | `users.name[]` | `users[].reviewed` |
| Index | `users[0].name` | `users[0].reviewed` |
| Slice | `users[1:3].name[]` | `users[1:3].reviewed` |
| Predicate | `users[?.active==True].name[]` | `users[?.active==True].reviewed` |
| One-level wildcard | `groups.*.id` | `groups.*.enabled` |
| Recursive wildcard | `groups.**.id[]` | `groups.**.enabled` |
| Root list | `.[].name` | `.[].reviewed` |

Root-list selectors also support `.[0]`, `.[1:3]`, and `.[?.id==2]`. The equivalent explicit forms are `$$root[]`, `$$root[0]`, `$$root[1:3]`, and `$$root[?.id==2]`. A root selector is valid only as the first path token.

`$$root.path` can begin a read path. It can also be used as a `set` value expression when the new value must come from elsewhere in the original document. Never place a root token mid-path, and do not use bare `$$root` as a mutation path.

## Predicates

Comparison operators are `==`, `!=`, `>`, `<`, `>=`, and `<=`:

```python
dictwalk.get(data, "users[?.score>=10].name[]")
```

The left side must start with `.`. Use `?.field` for a field, `?.` for the current scalar item, or `?.|$filter` to transform the current item before comparison:

```python
dictwalk.get(data, "values[?.>3]")
dictwalk.get(data, "labels[?.|$len>3]")
```

The right side may be a built-in predicate pipeline. Combine predicates with `&&`, `||`, `!`, and parentheses:

```python
dictwalk.get(data, "users[?.id==$gt(5)&&$lt(10)].name[]")
dictwalk.get(data, "users[?.id==!$odd].name[]")
```

Nested predicates are supported, but use them only when they are clearer than straightforward Python traversal.

## Output and mutation transforms

Output transforms follow `|` after the path:

```python
dictwalk.get(data, "scores|$max")
dictwalk.get(data, "scores|$double[]|$sum")
```

Passing a valid filter expression as the `set` value applies it to each selected existing value:

```python
dictwalk.set(data, "scores[]", "$double|$clamp(0, 100)")
```

`dictwalk.run_filter_function(expression, value)` runs a built-in pipeline directly when no traversal is needed.

## Built-in filter index

Choose from the installed built-ins; do not invent custom registration APIs.

- Numeric: `$inc`, `$dec`, `$double`, `$square`, `$add`, `$sub`, `$mul`, `$div`, `$idiv`, `$mod`, `$neg`, `$pow`, `$rpow`, `$sqrt`, `$root`, `$round`, `$floor`, `$ceil`, `$abs`, `$clamp`, `$sign`, `$log`, `$exp`, `$pct`
- Predicates: `$even`, `$odd`, `$gt`, `$lt`, `$gte`, `$lte`, `$between`, `$contains`, `$in`, `$type_is`, `$is_empty`, `$non_empty`
- Conversion: `$string`, `$int`, `$float`, `$decimal`, `$bool`, `$quote`
- Strings: `$lower`, `$upper`, `$title`, `$strip`, `$replace`, `$regex_replace`, `$split`, `$join`, `$startswith`, `$endswith`, `$matches`
- Collections: `$len`, `$keys`, `$values`, `$items`, `$max`, `$min`, `$unique`, `$sort_by`, `$unique_by`, `$index_by`, `$group_by`, `$reverse`, `$chunk`, `$flatten`, `$flatten_deep`, `$sorted`, `$first`, `$last`, `$pick`, `$unpick`
- Statistics: `$sum`, `$avg`, `$pctile`, `$median`, `$q1`, `$q3`, `$iqr`, `$mode`, `$stdev`
- Fallbacks: `$const`, `$default`, `$coalesce`, `$compact`
- Date/time: `$to_datetime`, `$strftime`, `$timestamp`, `$age_seconds`, `$before`, `$after`
- Serialization: `$from_json`, `$to_json`

Filters that accept arguments use function syntax such as `$round(2)`, `$join(',')`, or `$sort_by('id', True)`. Literal arguments are parsed as Python literals where possible.

Invalid numeric operations commonly return `None`, including division by zero and square root of a negative value. Collection filters vary between returning `None` and passing through non-collection inputs; verify behavior when input types are not already constrained.

## Failure behavior

- `DictWalkParseError`: malformed path, predicate, root-token placement, or filter expression.
- `DictWalkResolutionError`: missing or incompatible structure when strict resolution is requested.
- `DictWalkOperatorError`: unsupported or failed comparison behavior.

Non-strict mode softens resolution failures, not syntax errors. Prefer strict mode at validation boundaries and non-strict defaults only when absence is expected domain behavior.
