---
name: use-dictwalk
description: Use dictwalk to read, test, transform, set, and unset genuinely complex nested Python dict/list data with dynamic paths, predicates, slices, wildcards, root references, and built-in filter pipelines. Use when code imports dictwalk or repeated/configurable nested-data operations justify declarative paths. Do not introduce it for simple fixed-key access, a short comprehension, or ordinary assignment or deletion.
---

# Use dictwalk

Use `dictwalk` when a declarative path makes complex or dynamic nested-data work clearer. Keep ordinary Python for ordinary data access.

## Decide whether dictwalk earns its place

Prefer plain Python when the operation is obvious and fixed:

```python
name = data["user"]["name"]
active_names = [user["name"] for user in data["users"] if user["active"]]
data["settings"]["enabled"] = True
del data["debug"]
```

Do not replace code like this merely to use the library. Also prefer typed models or domain objects when the real problem is enforcing a stable schema rather than querying arbitrary nested data.

Use `dictwalk` when one or more of these are true:

- paths are supplied dynamically or stored as configuration;
- the same traversal pattern is reused across different payloads;
- one operation targets filtered, sliced, wildcard, or deeply nested matches;
- a built-in transform pipeline replaces substantial traversal and conversion code;
- mutation must create missing containers or update many selected values consistently.

When reviewing existing code, simplify unnecessary `dictwalk` usage if doing so improves clarity without removing required dynamic behavior.

## Use the public object

```python
from dictwalk import dictwalk
from dictwalk.errors import DictWalkParseError, DictWalkResolutionError
```

The Rust backend is required. Do not add fallback backend selection. Custom filter registration is currently unsupported; use the built-in filters instead of calling `register_path_filter(...)` or `get_path_filter(...)`.

## Read without hiding failures accidentally

```python
name = dictwalk.get(data, "user.profile.name")
name = dictwalk.get(data, "user.profile.name", default="unknown")
exists = dictwalk.exists(data, "user.profile.name")
```

Resolution failures return `default` from `get` and `False` from `exists` unless `strict=True`. Parse errors still raise. Use strict mode when a missing or incompatible structure is a defect:

```python
name = dictwalk.get(data, "user.profile.name", strict=True)
```

For read paths, collect a field from list items by putting `[]` on the field being collected:

```python
names = dictwalk.get(data, "users.name[]")
active_names = dictwalk.get(data, "users[?.active==True].name[]")
```

## Treat writes as in-place mutation

`set` and `unset` mutate and return the same root object. Do not treat the result as a fresh immutable value.

```python
dictwalk.set(data, "users[?.id==2].active", True)
dictwalk.unset(data, "users[?.disabled==True]")
```

For `set` and `unset`, put `[]` on the list-bearing key before the child field:

```python
dictwalk.set(data, "users[].reviewed", True)
dictwalk.unset(data, "users[].temporary")
```

By default, `set` may create missing containers, create a matching filtered item, and replace an incompatible intermediate value. Tighten this behavior when silent reshaping is unsafe:

```python
dictwalk.set(
    data,
    "user.profile.name",
    "Ada",
    create_missing=False,
    create_filter_match=False,
    overwrite_incompatible=False,
)
```

With `strict=True`, the parent path must already resolve. A terminal predicate passed to `unset`, such as `users[?.disabled==True]`, removes matching list items; a predicate followed by a field removes that field only from matching items.

## Use transforms deliberately

Append a built-in filter pipeline to a read path, or pass a filter expression as a set value to transform existing values:

```python
total = dictwalk.get(data, "orders.amount[]|$sum|$round(2)")
dictwalk.set(data, "scores[]", "$clamp(0, 100)")
```

Use `[]` on a pipeline step to map that step over a list:

```python
normalized = dictwalk.get(data, "scores|$float[]|$round(2)[]")
```

A set value beginning with `$$root` reads from the original root, optionally followed by transforms:

```python
dictwalk.set(data, "items[].currency", "$$root.default_currency")
dictwalk.set(data, "items[].tax", "$$root.tax_rate|$float")
```

Read [references/path-and-filters.md](references/path-and-filters.md) when selectors, root-list operations, boolean predicates, wildcards, or filter choice are material to the task.

## Verify paths against realistic shapes

Test the actual expression with representative data. Cover missing keys, incompatible intermediate types, empty lists, multiple predicate matches, and root-list input when relevant. For mutation, assert both the final structure and that the returned object is the original object.
