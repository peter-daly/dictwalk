# dictwalk Examples

This file contains practical examples for:
- `dictwalk.get`
- `dictwalk.set`
- `dictwalk.unset`

```python
from dictwalk import dictwalk
```

## Shared Sample Data

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

## `get` Examples

### Basic traversal

```python
dictwalk.get(data, "a.b.c")
# 1
```

### Root object

```python
dictwalk.get(data, ".")
# full data object
```

### Root token

```python
dictwalk.get(data, "$$root.x")
# 2

dictwalk.get(data, "a.b.$$root.x")
# 2
```

### Missing path with default

```python
dictwalk.get(data, "a.b.missing", default="n/a")
# "n/a"
```

### Strict mode

```python
dictwalk.get(data, "a.b.missing", strict=True)
# raises DictWalkResolutionError
```

### List map

```python
dictwalk.get(data, "a.users[].name")
# ["Ada", "Lin", "Mia"]
```

### List index and negative index

```python
dictwalk.get(data, "a.users[0].name")
# "Ada"

dictwalk.get(data, "a.users[-1].name")
# "Mia"
```

### List slice

```python
dictwalk.get(data, "a.users[1:3].name[]")
# ["Lin", "Mia"]
```

### Predicate filters

```python
dictwalk.get(data, "a.users[?id==2].name[]")
# ["Lin"]

dictwalk.get(data, "a.users[?score>10].name[]")
# ["Lin", "Mia"]

dictwalk.get(data, "a.users[?score<=20].name[]")
# ["Ada", "Lin"]
```

### Predicate path filters

```python
dictwalk.get(data, "a.users[?id==$even].name[]")
# ["Lin"]

dictwalk.get(data, "a.users[?id==$gt(1)&&$lt(3)].name[]")
# ["Lin"]

dictwalk.get(data, "a.users[?id==!$odd].name[]")
# ["Lin"]
```

### Predicate root (`?.`)

```python
dictwalk.get({"items": ["hi", "hello", "yo"]}, "items[?.|$len>2]")
# ["hello"]
```

### Wildcards

```python
dictwalk.get(data, "a.groups.*.id")
# [1]

dictwalk.get(data, "a.groups.**.id")
# [1, 2]
```

### Output transforms

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

## `set` Examples

All `set` operations mutate and return the same `data` object.

### Basic nested write

```python
obj = {}
dictwalk.set(obj, "a.b.c", 5)
# {"a": {"b": {"c": 5}}}
```

### Create list path via map

```python
obj = {}
dictwalk.set(obj, "a.items[].value", 1)
# {"a": {"items": [{"value": 1}]}}
```

### Update list values with map

```python
obj = {"a": {"nums": [1, 2, 3]}}
dictwalk.set(obj, "a.nums[]", 9)
# {"a": {"nums": [9, 9, 9]}}
```

### Transform existing values

```python
obj = {"a": {"nums": [1, 2, 3]}}
dictwalk.set(obj, "a.nums[]", "$double")
# {"a": {"nums": [2, 4, 6]}}

dictwalk.set(obj, "a.nums[]", "$add(1)|$string")
# {"a": {"nums": ["3", "5", "7"]}}
```

### Filtered write

```python
obj = {"a": {"users": [{"id": 1, "active": False}, {"id": 2, "active": False}]}}
dictwalk.set(obj, "a.users[?id==2].active", True)
# {"a": {"users": [{"id": 1, "active": False}, {"id": 2, "active": True}]}}
```

### Operator filter write

```python
obj = {"a": {"users": [{"id": 1, "score": 10}, {"id": 2, "score": 20}, {"id": 3, "score": 30}]}}
dictwalk.set(obj, "a.users[?id>1].score", 0)
# {"a": {"users": [{"id": 1, "score": 10}, {"id": 2, "score": 0}, {"id": 3, "score": 0}]}}
```

### Index and slice write

```python
obj = {"a": {"nums": [10, 20, 30, 40]}}
dictwalk.set(obj, "a.nums[1]", 99)
# {"a": {"nums": [10, 99, 30, 40]}}

dictwalk.set(obj, "a.nums[1:3]", 0)
# {"a": {"nums": [10, 0, 0, 40]}}
```

### Wildcard write

```python
obj = {"a": {"u1": {"enabled": False}, "u2": {"enabled": False}}}
dictwalk.set(obj, "a.*.enabled", True)
# {"a": {"u1": {"enabled": True}, "u2": {"enabled": True}}}
```

### Deep wildcard write

```python
obj = {"a": {"g1": {"u1": {"enabled": False}}, "g2": {"nested": {"u2": {"enabled": False}}}}}
dictwalk.set(obj, "a.**.enabled", True)
# {"a": {"g1": {"u1": {"enabled": True}}, "g2": {"nested": {"u2": {"enabled": True}}}}}
```

### `$$root` value expressions

```python
obj = {"a": {"items": [{"v": 0}, {"v": 0}]}, "source": 9}
dictwalk.set(obj, "a.items[].v", "$$root.source")
# {"a": {"items": [{"v": 9}, {"v": 9}]}, "source": 9}

dictwalk.set(obj, "a.items[].v", "$$root.source|$double")
# {"a": {"items": [{"v": 18}, {"v": 18}]}, "source": 9}
```

### Strict write

```python
obj = {}
dictwalk.set(obj, "a.b.c", 1, strict=True)
# raises DictWalkResolutionError
```

### Write options

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

## `unset` Examples

All `unset` operations mutate and return the same `data` object.

### Remove nested key

```python
obj = {"a": {"b": {"c": 1, "d": 2}}}
dictwalk.unset(obj, "a.b.c")
# {"a": {"b": {"d": 2}}}
```

### Remove mapped key from all list items

```python
obj = {"a": {"users": [{"id": 1, "name": "Ada"}, {"id": 2, "name": "Lin"}]}}
dictwalk.unset(obj, "a.users[].name")
# {"a": {"users": [{"id": 1}, {"id": 2}]}}
```

### Remove field from filtered matches

```python
obj = {"a": {"users": [{"id": 1, "score": 10}, {"id": 2, "score": 20}]}}
dictwalk.unset(obj, "a.users[?id==2].score")
# {"a": {"users": [{"id": 1, "score": 10}, {"id": 2}]}}
```

### Remove items at terminal filtered path

```python
obj = {"a": {"users": [{"id": 1}, {"id": 2}, {"id": 3}]}}
dictwalk.unset(obj, "a.users[?id>1]")
# {"a": {"users": [{"id": 1}]}}
```

### Remove list index and slice

```python
obj = {"a": {"nums": [10, 20, 30, 40]}}
dictwalk.unset(obj, "a.nums[1]")
# {"a": {"nums": [10, 30, 40]}}

dictwalk.unset(obj, "a.nums[1:3]")
# {"a": {"nums": [10]}}
```

### Unset with slice + nested field

```python
obj = {"a": {"users": [{"id": 1, "debug": True}, {"id": 2, "debug": False}, {"id": 3, "debug": True}]}}
dictwalk.unset(obj, "a.users[1:3].debug")
# {"a": {"users": [{"id": 1, "debug": True}, {"id": 2}, {"id": 3}]}}
```

### Wildcard unset

```python
obj = {"a": {"u1": {"debug": True, "id": 1}, "u2": {"debug": False, "id": 2}}}
dictwalk.unset(obj, "a.*.debug")
# {"a": {"u1": {"id": 1}, "u2": {"id": 2}}}
```

### Deep wildcard unset

```python
obj = {"a": {"g1": {"u1": {"debug": True, "id": 1}}, "g2": {"nested": {"u2": {"debug": False, "id": 2}}}}}
dictwalk.unset(obj, "a.**.debug")
# {"a": {"g1": {"u1": {"id": 1}}, "g2": {"nested": {"u2": {"id": 2}}}}}
```

### Strict unset

```python
obj = {"a": {"b": {}}}
dictwalk.unset(obj, "a.b.c", strict=True)
# raises DictWalkResolutionError
```

