# dictwalk

> This library is the result started as an idea to write jq queries for a Python Dict for a reason I can't event remember anymore.
> 
> It's current form emerged because I kept asking the AI to add one more feature, then one more, then one more.
>
> Then I asked the AI to come up with features and I said screw it add them all.
> 
> This is bored-developer + tokens to burn + scope creep in library form.
> 
> This is now just a very advanced hammer in search of a nail, could your usecase be that nail?
> 
> I know what you're thinking, *this could replace those horrible nested dict functions that I have to mainain in one of my projects*, but don't do it's not worth it.
> 
> Now in version 1, it's been converted to rust, so now it's much much faster and doing nothing important.

`dictwalk` is a small utility for traversing and mutating nested Python dict/list data using path expressions.
The implementation is now Rust-first (PyO3 extension), called from Python.
There is no pure-Python execution backend.

It supports:
- Deep reads (`get`)
- Existence checks (`exists`)
- In-place writes (`set`)
- In-place removals (`unset`)
- Predicate filtering for lists
- Wildcards (`*`, `**`)
- Transform/filter pipelines (`|$filter`)

## Jump To

- [Quick Start](#quick-start)
- [Path Syntax](#path-syntax)
- [API](#api)
- [Examples](#examples)
- [Filter Functions](#filter-functions)
- [Development](#development)

## Requirements

- Python `>=3.10`
- Rust toolchain (for source builds)

## Installation

From source:

```bash
pip install .
```

This builds the Rust extension module (`dictwalk._dictwalk_rs`) during install.

For local development:

```bash
uv sync
make rust-build
```

## Quick Start

```python
from dictwalk import dictwalk

data = {
    "a": {
        "users": [
            {"id": 1, "name": "Ada", "active": True},
            {"id": 2, "name": "Lin", "active": False},
        ]
    }
}

# Read
names = dictwalk.get(data, "a.users[].name")
# ["Ada", "Lin"]

# Filter and map
active_names = dictwalk.get(data, "a.users[?active==True].name[]")
# ["Ada"]

# Write
dictwalk.set(data, "a.users[?id==2].active", True)

# Unset
dictwalk.unset(data, "a.users[?id==1].name")
```

## Path Syntax

### Dot traversal

```text
a.b.c
```

Read nested object keys.

### List map

```text
a.items[].id
```

Apply the next token to every item in a list.

### List index and slice

```text
a.items[0]
a.items[-1]
a.items[1:3]
```

### Predicates

```text
a.items[?id==1]
a.items[?score>=10]
```

List predicates support:
- `==`, `!=`, `>`, `<`, `>=`, `<=`

### Predicate filters

Use registered filters on predicate values:

```text
a.items[?id==$even]
a.items[?id==$gt(5)&&$lt(10)]
a.items[?id==!$odd]
```

Boolean operators in predicate filters:
- `&&` (and)
- `||` (or)
- `!` (not)
- parentheses for grouping

### Wildcards

```text
a.*.id
a.**.id
```

- `*`: one level
- `**`: deep descendant traversal

### Output transforms

Apply filters to the final read value:

```text
a.value|$double|$string
a.list|$max
a.list|$double[]|$max
```

## API

`dictwalk` exposed from `dictwalk.__init__` is the Rust extension object directly.
Python methods call into Rust for `get`, `exists`, `set`, `unset`, and `run_filter_function`.

## `dictwalk.get(data, path, default=None, strict=False)`

- Returns resolved value.
- If `strict=False`: resolution failures return `default`.
- If `strict=True`: raises `DictWalkResolutionError`.

Special root token support in read paths:

```text
$$root.x
a.b.$$root.x
```

## `dictwalk.exists(data, path, strict=False) -> bool`

- Returns `True` if path resolves, else `False`.
- If `strict=True`, raises `DictWalkResolutionError` on resolution failures.

## `dictwalk.set(data, path, value, *, strict=False, create_missing=True, create_filter_match=True, overwrite_incompatible=True) -> dict`

Mutates and returns the same `data` object.

`value` can be:
- A direct value (`42`, `"x"`, `{"k": 1}`)
- A filter string (`"$double"`, `"$add(2)|$string"`)
- A root reference expression:
  - `$$root`
  - `$$root.some.path`
  - `$$root.some.path|$filter`

Notes:
- `$$root` is valid in `value`, not in write `path`.
- With `strict=True`, parent path must already resolve.

## `dictwalk.unset(data, path, *, strict=False) -> dict`

Removes targeted values in-place and returns the same object.

At terminal paths this can:
- Remove dict keys
- Remove list indexes/slices
- Remove list items matching a filter

## Examples

This section contains practical examples for:
- `dictwalk.get`
- `dictwalk.set`
- `dictwalk.unset`

```python
from dictwalk import dictwalk
```

### Shared Sample Data

```python
data = {
    "a": {
        "b": {"c": 1},
        "users": [
            {"id": 1, "name": "Ada", "active": True, "score": 10},
            {"id": 2, "name": "Lin", "active": False, "score": 20},
            {"id": 3, "name": "Mia", "active": True, "score": 30},
        ],
        "groups": {
            "g1": {"u1": {"id": 1, "debug": True}},
            "g2": {"nested": {"u2": {"id": 2, "debug": False}}},
        },
    },
    "x": 2,
    "profile": {"name": "Dict Walk", "tags": ["py", "paths", "json"]},
}
```

### `get` Examples

Basic traversal:

```python
dictwalk.get(data, "a.b.c")
# 1
```

Root object:

```python
dictwalk.get(data, ".")
# full data object
```

Root token:

```python
dictwalk.get(data, "$$root.x")
# 2

dictwalk.get(data, "a.b.$$root.x")
# 2
```

Missing path with default:

```python
dictwalk.get(data, "a.b.missing", default="n/a")
# "n/a"
```

Strict mode:

```python
dictwalk.get(data, "a.b.missing", strict=True)
# raises DictWalkResolutionError
```

List map:

```python
dictwalk.get(data, "a.users[].name")
# ["Ada", "Lin", "Mia"]
```

List index and negative index:

```python
dictwalk.get(data, "a.users[0].name")
# "Ada"

dictwalk.get(data, "a.users[-1].name")
# "Mia"
```

List slice:

```python
dictwalk.get(data, "a.users[1:3].name[]")
# ["Lin", "Mia"]
```

Predicate filters:

```python
dictwalk.get(data, "a.users[?id==2].name[]")
# ["Lin"]

dictwalk.get(data, "a.users[?score>10].name[]")
# ["Lin", "Mia"]

dictwalk.get(data, "a.users[?score<=20].name[]")
# ["Ada", "Lin"]
```

Predicate path filters:

```python
dictwalk.get(data, "a.users[?id==$even].name[]")
# ["Lin"]

dictwalk.get(data, "a.users[?id==$gt(1)&&$lt(3)].name[]")
# ["Lin"]

dictwalk.get(data, "a.users[?id==!$odd].name[]")
# ["Lin"]
```

Predicate root (`?.`):

```python
dictwalk.get({"items": ["hi", "hello", "yo"]}, "items[?.|$len>2]")
# ["hello"]
```

Wildcards:

```python
dictwalk.get(data, "a.groups.*.id")
# [1]

dictwalk.get(data, "a.groups.**.id")
# [1, 2]
```

Output transforms:

```python
dictwalk.get(data, "a.b.c|$double")
# 2

dictwalk.get(data, "a.b.c|$double|$string")
# "2"

dictwalk.get(data, "a.users[].score|$sum")
# 60

dictwalk.get(data, "profile.tags|$join(',')")
# "py,paths,json"
```

### `set` Examples

All `set` operations mutate and return the same `data` object.

Basic nested write:

```python
obj = {}
dictwalk.set(obj, "a.b.c", 5)
# {"a": {"b": {"c": 5}}}
```

Create list path via map:

```python
obj = {}
dictwalk.set(obj, "a.items[].value", 1)
# {"a": {"items": [{"value": 1}]}}
```

Update list values with map:

```python
obj = {"a": {"nums": [1, 2, 3]}}
dictwalk.set(obj, "a.nums[]", 9)
# {"a": {"nums": [9, 9, 9]}}
```

Transform existing values:

```python
obj = {"a": {"nums": [1, 2, 3]}}
dictwalk.set(obj, "a.nums[]", "$double")
# {"a": {"nums": [2, 4, 6]}}

dictwalk.set(obj, "a.nums[]", "$add(1)|$string")
# {"a": {"nums": ["3", "5", "7"]}}
```

Filtered write:

```python
obj = {"a": {"users": [{"id": 1, "active": False}, {"id": 2, "active": False}]}}
dictwalk.set(obj, "a.users[?id==2].active", True)
# {"a": {"users": [{"id": 1, "active": False}, {"id": 2, "active": True}]}}
```

Operator filter write:

```python
obj = {"a": {"users": [{"id": 1, "score": 10}, {"id": 2, "score": 20}, {"id": 3, "score": 30}]}}
dictwalk.set(obj, "a.users[?id>1].score", 0)
# {"a": {"users": [{"id": 1, "score": 10}, {"id": 2, "score": 0}, {"id": 3, "score": 0}]}}
```

Index and slice write:

```python
obj = {"a": {"nums": [10, 20, 30, 40]}}
dictwalk.set(obj, "a.nums[1]", 99)
# {"a": {"nums": [10, 99, 30, 40]}}

dictwalk.set(obj, "a.nums[1:3]", 0)
# {"a": {"nums": [10, 0, 0, 40]}}
```

Wildcard write:

```python
obj = {"a": {"u1": {"enabled": False}, "u2": {"enabled": False}}}
dictwalk.set(obj, "a.*.enabled", True)
# {"a": {"u1": {"enabled": True}, "u2": {"enabled": True}}}
```

Deep wildcard write:

```python
obj = {"a": {"g1": {"u1": {"enabled": False}}, "g2": {"nested": {"u2": {"enabled": False}}}}}
dictwalk.set(obj, "a.**.enabled", True)
# {"a": {"g1": {"u1": {"enabled": True}}, "g2": {"nested": {"u2": {"enabled": True}}}}}
```

`$$root` value expressions:

```python
obj = {"a": {"items": [{"v": 0}, {"v": 0}]}, "source": 9}
dictwalk.set(obj, "a.items[].v", "$$root.source")
# {"a": {"items": [{"v": 9}, {"v": 9}]}, "source": 9}

dictwalk.set(obj, "a.items[].v", "$$root.source|$double")
# {"a": {"items": [{"v": 18}, {"v": 18}]}, "source": 9}
```

Strict write:

```python
obj = {}
dictwalk.set(obj, "a.b.c", 1, strict=True)
# raises DictWalkResolutionError
```

Write options:

```python
obj = {}
dictwalk.set(obj, "a.b.c", 1, create_missing=False)
# {}

obj = {"a": {"users": [{"id": "1", "c": 10}]}}
dictwalk.set(obj, "a.users[?id==3].c", 99, create_filter_match=False)
# unchanged

obj = {"a": 1}
dictwalk.set(obj, "a.b", 2, overwrite_incompatible=False)
# {"a": 1}
```

### `unset` Examples

All `unset` operations mutate and return the same `data` object.

Remove nested key:

```python
obj = {"a": {"b": {"c": 1, "d": 2}}}
dictwalk.unset(obj, "a.b.c")
# {"a": {"b": {"d": 2}}}
```

Remove mapped key from all list items:

```python
obj = {"a": {"users": [{"id": 1, "name": "Ada"}, {"id": 2, "name": "Lin"}]}}
dictwalk.unset(obj, "a.users[].name")
# {"a": {"users": [{"id": 1}, {"id": 2}]}}
```

Remove field from filtered matches:

```python
obj = {"a": {"users": [{"id": 1, "score": 10}, {"id": 2, "score": 20}]}}
dictwalk.unset(obj, "a.users[?id==2].score")
# {"a": {"users": [{"id": 1, "score": 10}, {"id": 2}]}}
```

Remove items at terminal filtered path:

```python
obj = {"a": {"users": [{"id": 1}, {"id": 2}, {"id": 3}]}}
dictwalk.unset(obj, "a.users[?id>1]")
# {"a": {"users": [{"id": 1}]}}
```

Remove list index and slice:

```python
obj = {"a": {"nums": [10, 20, 30, 40]}}
dictwalk.unset(obj, "a.nums[1]")
# {"a": {"nums": [10, 30, 40]}}

dictwalk.unset(obj, "a.nums[1:3]")
# {"a": {"nums": [10]}}
```

Unset with slice + nested field:

```python
obj = {"a": {"users": [{"id": 1, "debug": True}, {"id": 2, "debug": False}, {"id": 3, "debug": True}]}}
dictwalk.unset(obj, "a.users[1:3].debug")
# {"a": {"users": [{"id": 1, "debug": True}, {"id": 2}, {"id": 3}]}}
```

Wildcard unset:

```python
obj = {"a": {"u1": {"debug": True, "id": 1}, "u2": {"debug": False, "id": 2}}}
dictwalk.unset(obj, "a.*.debug")
# {"a": {"u1": {"id": 1}, "u2": {"id": 2}}}
```

Deep wildcard unset:

```python
obj = {"a": {"g1": {"u1": {"debug": True, "id": 1}}, "g2": {"nested": {"u2": {"debug": False, "id": 2}}}}}
dictwalk.unset(obj, "a.**.debug")
# {"a": {"g1": {"u1": {"id": 1}}, "g2": {"nested": {"u2": {"id": 2}}}}}
```

Strict unset:

```python
obj = {"a": {"b": {}}}
dictwalk.unset(obj, "a.b.c", strict=True)
# raises DictWalkResolutionError
```

## Filter Functions

This section documents the built-in path filters available in `dictwalk`.

Use filters in:
- Output transforms: `a.b.c|$double|$string`
- Predicate expressions: `a.items[?id==$even]`
- Write transforms: `dictwalk.set(data, "a.items[]", "$inc")`

Usage notes:
- Syntax: `$name` or `$name(arg1, arg2, ...)`
- Pipe multiple filters with `|`
- Add `[]` to map over list values in transform context (example: `$double[]`)
- Predicate boolean composition supports `&&`, `||`, `!`, and parentheses

Numeric:
- `$inc`: add 1
- `$dec`: subtract 1
- `$double`: multiply by 2
- `$square`: multiply by itself
- `$add(amount)`: add amount
- `$sub(amount)`: subtract amount
- `$mul(factor)`: multiply by factor
- `$div(divisor)`: divide (returns `None` when divisor is 0)
- `$mod(divisor)`: modulo (returns `None` when divisor is 0)
- `$neg`: negate value
- `$pow(exponent)`: raise value to exponent
- `$rpow(base)`: raise base to value
- `$sqrt`: square root (returns `None` for negative input)
- `$root(degree)`: nth root (returns `None` for invalid input)
- `$round(ndigits=0)`: round value
- `$floor`: floor
- `$ceil`: ceil
- `$abs`: absolute value
- `$clamp(min_value, max_value)`: clamp to bounds
- `$sign`: -1, 0, or 1
- `$log(base=e)`: logarithm (returns `None` for invalid input)
- `$exp`: exponential
- `$pct(percent)`: percent of value (`x * percent/100`)

Comparison/predicates:
- `$even`: true if even int
- `$odd`: true if odd int
- `$gt(threshold)`: greater than threshold
- `$lt(threshold)`: less than threshold
- `$gte(threshold)`: greater than or equal
- `$lte(threshold)`: less than or equal
- `$between(min_value, max_value)`: inclusive range check
- `$contains(value)`: membership for `str/list/tuple/set/dict`
- `$in(values)`: check if current value is in provided container
- `$type_is(name)`: type-name comparison (case-insensitive)
- `$is_empty`: `None` or zero-length container
- `$non_empty`: inverse of `$is_empty`

Conversion:
- `$string`: `str(x)`
- `$int`: `int(x)`
- `$float`: `float(x)`
- `$decimal`: `Decimal(x)`
- `$bool`: truthy conversion with string handling (`"true"`, `"1"`, `"yes"`, etc.)
- `$quote`: wrap in double quotes

String:
- `$lower`: lowercase string
- `$upper`: uppercase string
- `$title`: title case
- `$strip(chars=None)`: strip chars
- `$replace(old, new)`: replace substring
- `$split(sep=None)`: split into list
- `$join(sep)`: join list-like values
- `$startswith(prefix)`: startswith check
- `$endswith(suffix)`: endswith check
- `$matches(pattern)`: regex search check

Collections:
- `$len`: length
- `$max`: max for list/tuple, otherwise passthrough
- `$min`: min for list/tuple, otherwise passthrough
- `$sum`: sum for list/tuple, otherwise passthrough
- `$avg`: average for list/tuple, otherwise passthrough
- `$unique`: deduplicate list while preserving order
- `$sorted(reverse=False)`: sort list/tuple
- `$first`: first item for list/tuple
- `$last`: last item for list/tuple
- `$pick(*keys)`: keep only selected dict keys
- `$unpick(*keys)`: remove selected dict keys

Null/fallback:
- `$default(value)`: fallback when current value is `None`
- `$coalesce(*values)`: first non-`None` among current value and provided values

Date/time:
- `$to_datetime(fmt=None)`: parse datetime
- `$timestamp`: convert datetime-like to unix timestamp
- `$age_seconds`: seconds from datetime to now
- `$before(dt)`: datetime comparison
- `$after(dt)`: datetime comparison

Filter usage examples:

```python
from dictwalk import dictwalk

data = {"a": {"scores": [10, 20, 30], "name": "  ada  ", "created": "2024-01-01T00:00:00Z"}}

dictwalk.get(data, "a.scores|$sum")
# 60

dictwalk.get(data, "a.name|$strip|$title")
# "Ada"

dictwalk.get(data, "a.created|$to_datetime|$timestamp")
# 1704067200.0

dictwalk.get({"a": {"users": [{"id": 1}, {"id": 2}]}}, "a.users[?id==$even].id[]")
# [2]
```

## Errors

From `dictwalk.errors`:
- `DictWalkError` (base)
- `DictWalkParseError`
- `DictWalkOperatorError`
- `DictWalkResolutionError`

Use `strict=True` when you want explicit failures instead of fallback defaults.

## Development

Run tests:

```bash
make test
```

Build Rust extension:

```bash
make rust-build
```

Run lint/type/dependency checks:

```bash
make lint
make type
make deptry
```

Run everything:

```bash
make ci
```

Direct tox usage:

```bash
uv run tox -e py310,py311,py312,py313,py314,lint,type,deptry
```
