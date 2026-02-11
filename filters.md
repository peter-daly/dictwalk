# dictwalk Filters

This file documents the built-in path filters available in `dictwalk`.

Use filters in:
- Output transforms: `a.b.c|$double|$string`
- Predicate expressions: `a.items[?id==$even]`
- Write transforms: `dictwalk.set(data, "a.items[]", "$inc")`

## Usage Notes

- Syntax: `$name` or `$name(arg1, arg2, ...)`
- Pipe multiple filters with `|`
- Add `[]` to map over list values in transform context (example: `$double[]`)
- Predicate boolean composition supports `&&`, `||`, `!`, and parentheses

## Numeric

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

## Comparison / Predicates

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

## Conversion

- `$string`: `str(x)`
- `$int`: `int(x)`
- `$float`: `float(x)`
- `$decimal`: `Decimal(x)`
- `$bool`: truthy conversion with string handling (`"true"`, `"1"`, `"yes"`, etc.)
- `$quote`: wrap in double quotes

## String

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

## Collections

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

## Null / Fallback

- `$default(value)`: fallback when current value is `None`
- `$coalesce(*values)`: first non-`None` among current value and provided values

## Date / Time

- `$to_datetime(fmt=None)`: parse datetime
- `$timestamp`: convert datetime-like to unix timestamp
- `$age_seconds`: seconds from datetime to now
- `$before(dt)`: datetime comparison
- `$after(dt)`: datetime comparison

## Examples

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

