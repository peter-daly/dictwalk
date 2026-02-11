from datetime import datetime, timezone
from decimal import Decimal
import math
import re
from types import MappingProxyType
from typing import Any


def _as_datetime(value: Any, fmt: str | None = None) -> datetime | None:
    if isinstance(value, datetime):
        return value
    if isinstance(value, int | float):
        return datetime.fromtimestamp(value, tz=timezone.utc)
    if not isinstance(value, str):
        return None
    if fmt is not None:
        return datetime.strptime(value, fmt)
    normalized = value.replace("Z", "+00:00")
    return datetime.fromisoformat(normalized)


def _date_before(value: Any, dt: Any) -> bool:
    left = _as_datetime(value)
    right = _as_datetime(dt)
    if left is None or right is None:
        return False
    return left < right


def _date_after(value: Any, dt: Any) -> bool:
    left = _as_datetime(value)
    right = _as_datetime(dt)
    if left is None or right is None:
        return False
    return left > right


def _age_seconds(value: Any) -> float | None:
    dt = _as_datetime(value)
    if dt is None:
        return None
    now = datetime.now(tz=dt.tzinfo or timezone.utc)
    return (now - dt).total_seconds()


DEFAULT_FILTER_FUNCTION_REGISTRY = MappingProxyType(
    {
        "inc": lambda x: x + 1,
        "dec": lambda x: x - 1,
        "double": lambda x: x * 2,
        "square": lambda x: x * x,
        "string": lambda x: str(x),
        "int": lambda x: int(x),
        "float": lambda x: float(x),
        "decimal": lambda x: Decimal(x),
        "round": lambda x, ndigits=0: round(x, ndigits),
        "floor": lambda x: math.floor(x),
        "ceil": lambda x: math.ceil(x),
        "abs": lambda x: abs(x),
        "quote": lambda x: f'"{x}"',
        "even": lambda x: isinstance(x, int) and x % 2 == 0,
        "odd": lambda x: isinstance(x, int) and x % 2 == 1,
        "gt": lambda x, threshold: x > threshold,
        "lt": lambda x, threshold: x < threshold,
        "gte": lambda x, threshold: x >= threshold,
        "lte": lambda x, threshold: x <= threshold,
        "add": lambda x, amount: x + amount,
        "sub": lambda x, amount: x - amount,
        "mul": lambda x, factor: x * factor,
        "div": lambda x, divisor: x / divisor if divisor != 0 else None,
        "mod": lambda x, divisor: x % divisor if divisor != 0 else None,
        "neg": lambda x: -x,
        "pow": lambda x, exponent: x**exponent,
        "rpow": lambda x, base: base**x,
        "sqrt": lambda x: x**0.5 if x >= 0 else None,
        "root": lambda x, degree: x ** (1 / degree) if x >= 0 and degree > 0 else None,
        "max": lambda x: max(x) if isinstance(x, list | tuple) else x,
        "min": lambda x: min(x) if isinstance(x, list | tuple) else x,
        "len": lambda x: len(x),
        "pick": lambda x, *keys: (
            {k: x[k] for k in keys if k in x} if isinstance(x, dict) else None
        ),
        "unpick": lambda x, *keys: (
            {k: v for k, v in x.items() if k not in keys}
            if isinstance(x, dict)
            else None
        ),
        "clamp": lambda x, min_value, max_value: max(min_value, min(max_value, x)),
        "sign": lambda x: (x > 0) - (x < 0),
        "log": lambda x, base=math.e: (
            math.log(x, base) if x > 0 and base > 0 and base != 1 else None
        ),
        "exp": lambda x: math.exp(x),
        "pct": lambda x, percent: x * (percent / 100),
        "between": lambda x, min_value, max_value: min_value <= x <= max_value,
        "sum": lambda x: sum(x) if isinstance(x, list | tuple) else x,
        "avg": lambda x: (
            (sum(x) / len(x) if x else None) if isinstance(x, list | tuple) else x
        ),
        "unique": lambda x: list(dict.fromkeys(x)) if isinstance(x, list) else x,
        "sorted": lambda x, reverse=False: (
            sorted(x, reverse=reverse) if isinstance(x, list | tuple) else x
        ),
        "first": lambda x: (x[0] if x else None) if isinstance(x, list | tuple) else x,
        "last": lambda x: (x[-1] if x else None) if isinstance(x, list | tuple) else x,
        "contains": lambda x, value: (
            value in x if isinstance(x, str | list | tuple | set | dict) else False
        ),
        "in": lambda x, values: x in values,
        "lower": lambda x: str(x).lower(),
        "upper": lambda x: str(x).upper(),
        "title": lambda x: str(x).title(),
        "strip": lambda x, chars=None: str(x).strip(chars),
        "replace": lambda x, old, new: str(x).replace(old, new),
        "split": lambda x, sep=None: str(x).split(sep),
        "join": lambda x, sep: (
            sep.join(str(item) for item in x) if isinstance(x, list | tuple) else str(x)
        ),
        "startswith": lambda x, prefix: str(x).startswith(prefix),
        "endswith": lambda x, suffix: str(x).endswith(suffix),
        "matches": lambda x, pattern: re.search(pattern, str(x)) is not None,
        "default": lambda x, value: value if x is None else x,
        "coalesce": lambda x, *values: (
            x if x is not None else next((v for v in values if v is not None), None)
        ),
        "bool": lambda x: (
            x.strip().lower() in {"1", "true", "yes", "y", "on"}
            if isinstance(x, str)
            else bool(x)
        ),
        "type_is": lambda x, name: type(x).__name__.lower() == name.lower(),
        "is_empty": lambda x: x is None or (hasattr(x, "__len__") and len(x) == 0),
        "non_empty": lambda x: (
            not (x is None or (hasattr(x, "__len__") and len(x) == 0))
        ),
        "to_datetime": lambda x, fmt=None: _as_datetime(x, fmt=fmt),
        "timestamp": lambda x: (
            value.timestamp() if (value := _as_datetime(x)) is not None else None
        ),
        "age_seconds": _age_seconds,
        "before": _date_before,
        "after": _date_after,
    }
)
