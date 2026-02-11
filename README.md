# dictwalk

> This library is basically overengineered and ridiculous.  
> It mostly exists because I kept asking an AI to add one more feature, then one more, then one more.
> Then I asked the AI to come up with features and I said screw it add them all.
> Please do not use this instead of just writing normal code.  
> This is bored-developer scope creep in library form.
> The original idea was basically “jq/yq, but for Python dicts, and here we are”.
> I hope you get some use out of it, because I didn't.

`dictwalk` is a small utility for traversing and mutating nested Python dict/list data using path expressions.

It supports:
- Deep reads (`get`)
- Existence checks (`exists`)
- In-place writes (`set`)
- In-place removals (`unset`)
- Predicate filtering for lists
- Wildcards (`*`, `**`)
- Transform/filter pipelines (`|$filter`)

## Requirements

- Python `>=3.10`

## Installation

From source:

```bash
pip install .
```

For local development:

```bash
uv sync
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

## Filters

Built-in filters include arithmetic, string, collection, predicate, and datetime utilities.

Examples:
- Arithmetic: `$inc`, `$double`, `$add(2)`, `$round(1)`
- Collection: `$len`, `$max`, `$min`, `$sum`, `$avg`, `$unique`
- String: `$lower`, `$upper`, `$replace("a","b")`, `$split(",")`
- Predicates: `$even`, `$odd`, `$gt(10)`, `$contains("x")`
- Datetime: `$to_datetime`, `$timestamp`, `$age_seconds`, `$before(...)`, `$after(...)`

Register custom filters:

```python
from dictwalk import dictwalk

dictwalk.register_path_filter("triple", lambda x: x * 3)

value = dictwalk.get({"a": {"b": 2}}, "a.b|$triple")
# 6
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
