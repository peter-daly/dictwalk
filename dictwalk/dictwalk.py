import ast
import re
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, Protocol

from .path_filters import DEFAULT_FILTER_FUNCTION_REGISTRY

from .errors import (
    DictWalkError,
    DictWalkOperatorError,
    DictWalkParseError,
    DictWalkResolutionError,
)


@dataclass(kw_only=True)
class PathWriteOptions:
    create_missing: bool = True
    create_filter_match: bool = True
    overwrite_incompatible: bool = True


class PathToken(Protocol):
    def resolve(self, current: Any) -> Any: ...

    def write(
        self,
        current: Any,
        remaining: list["PathToken"],
        new_value: Any,
        write_options: PathWriteOptions,
        registry: dict[str, "PathFilter"],
        root_data: dict,
    ) -> Any: ...

    def unset(
        self,
        current: Any,
        remaining: list["PathToken"],
    ) -> Any: ...


class PathFilter:
    def __init__(self, filter_fn: Callable[..., Any], *args: Any, **kwargs: Any):
        self._filter_fn = filter_fn
        self._args = args
        self._kwargs = kwargs

    def __call__(self, current_value: Any) -> Any:
        return self._filter_fn(current_value, *self._args, **self._kwargs)

    def with_args(self, *args: Any, **kwargs: Any) -> "PathFilter":
        return PathFilter(self._filter_fn, *args, **kwargs)

    def __or__(self, other: "PathFilter") -> "PathFilter":
        if not isinstance(other, PathFilter):
            raise TypeError(f"Cannot compose PathFilter with {type(other).__name__}.")
        return PathFilter(lambda current_value: other(self(current_value)))

    def __ror__(self, other: "PathFilter") -> "PathFilter":
        if not isinstance(other, PathFilter):
            raise TypeError(f"Cannot compose {type(other).__name__} with PathFilter.")
        return PathFilter(lambda current_value: self(other(current_value)))


def _register_path_filter(
    registry: dict[str, PathFilter],
    name: str,
    path_filter: PathFilter | Callable[..., Any],
) -> PathFilter:
    if not isinstance(path_filter, PathFilter):
        path_filter = PathFilter(path_filter)
    registry[name] = path_filter
    return path_filter


def _get_path_filter(registry: dict[str, PathFilter], name: str) -> PathFilter:
    try:
        return registry[name]
    except KeyError as ex:
        raise KeyError(f"Path filter '{name}' is not registered.") from ex


def _tokenize_boolean_filter_expression(expression: str) -> list[str]:
    tokens: list[str] = []
    i = 0
    while i < len(expression):
        ch = expression[i]
        if ch.isspace():
            i += 1
            continue
        if expression.startswith("&&", i):
            tokens.append("&&")
            i += 2
            continue
        if expression.startswith("||", i):
            tokens.append("||")
            i += 2
            continue
        if ch in {"(", ")", "!"}:
            tokens.append(ch)
            i += 1
            continue

        start = i
        paren_depth = 0
        while i < len(expression):
            if expression[i] == "(":
                paren_depth += 1
                i += 1
                continue
            if expression[i] == ")":
                if paren_depth == 0:
                    break
                paren_depth -= 1
                i += 1
                continue
            if paren_depth == 0 and (
                expression.startswith("&&", i)
                or expression.startswith("||", i)
                or expression[i] == "!"
            ):
                break
            i += 1
        operand = expression[start:i].strip()
        if operand:
            tokens.append(operand)

    return tokens


def _resolve_path_filter_string(
    value: str, registry: dict[str, PathFilter]
) -> PathFilter | None:
    if not value.startswith("$"):
        return None

    segments = [segment.strip() for segment in value.split("|")]
    composed: PathFilter | None = None
    for segment in segments:
        match = re.match(r"^\$([a-zA-Z_]\w*)(?:\((.*)\))?(\[\])?$", segment)
        if not match:
            raise DictWalkParseError(
                path=value,
                token=segment,
                message=(
                    f"Invalid path filter segment '{segment}'. Expected '$<name>', '$<name>(...)', "
                    "or either with trailing '[]' for list mapping."
                ),
            )

        filter_name, args_string, map_suffix = match.groups()
        current = _get_path_filter(registry, filter_name)
        if args_string is not None:
            args = ast.literal_eval(f"({args_string},)")
            current = current.with_args(*args)
        if map_suffix:
            current = PathFilter(
                lambda current_value, inner=current: (
                    [inner(item) for item in current_value]
                    if isinstance(current_value, list)
                    else inner(current_value)
                )
            )
        composed = current if composed is None else (composed | current)
    return composed


def _resolve_path_filter(
    value: Any, registry: dict[str, PathFilter]
) -> PathFilter | None:
    if isinstance(value, PathFilter):
        return value
    if isinstance(value, str):
        return _resolve_path_filter_string(value, registry)
    return None


class _BooleanPathFilterParser:
    def __init__(self, expression: str, registry: dict[str, PathFilter]):
        self._tokens = _tokenize_boolean_filter_expression(expression)
        self._idx = 0
        self._registry = registry

    def parse(self) -> PathFilter:
        result = self._parse_or()
        if self._idx != len(self._tokens):
            raise DictWalkParseError(
                path=" ".join(self._tokens),
                token=self._tokens[self._idx],
                message=f"Unexpected token '{self._tokens[self._idx]}' in boolean path filter expression.",
            )
        return result

    def _parse_or(self) -> PathFilter:
        left = self._parse_and()
        while self._peek() == "||":
            self._consume("||")
            right = self._parse_and()
            left = PathFilter(
                lambda value, left=left, right=right: (
                    bool(left(value)) or bool(right(value))
                )
            )
        return left

    def _parse_and(self) -> PathFilter:
        left = self._parse_not()
        while self._peek() == "&&":
            self._consume("&&")
            right = self._parse_not()
            left = PathFilter(
                lambda value, left=left, right=right: (
                    bool(left(value)) and bool(right(value))
                )
            )
        return left

    def _parse_not(self) -> PathFilter:
        if self._peek() == "!":
            self._consume("!")
            inner = self._parse_not()
            return PathFilter(lambda value, inner=inner: not bool(inner(value)))
        return self._parse_primary()

    def _parse_primary(self) -> PathFilter:
        if self._peek() == "(":
            self._consume("(")
            inner = self._parse_or()
            self._consume(")")
            return inner

        token = self._peek()
        if token is None:
            raise DictWalkParseError(
                path=" ".join(self._tokens),
                token=None,
                message="Unexpected end of boolean path filter expression.",
            )

        self._idx += 1
        path_filter = _resolve_path_filter_string(token, self._registry)
        if path_filter is None:
            raise DictWalkParseError(
                path=" ".join(self._tokens),
                token=token,
                message=f"Invalid path filter token '{token}' in boolean expression.",
            )
        return path_filter

    def _peek(self) -> str | None:
        if self._idx >= len(self._tokens):
            return None
        return self._tokens[self._idx]

    def _consume(self, expected: str):
        token = self._peek()
        if token != expected:
            raise DictWalkParseError(
                path=" ".join(self._tokens),
                token=token,
                message=f"Expected '{expected}' in boolean path filter expression, got '{token}'.",
            )
        self._idx += 1


def _resolve_predicate_path_filter(
    value: str, registry: dict[str, PathFilter]
) -> PathFilter | None:
    if "&&" in value or "||" in value or "!" in value:
        return _BooleanPathFilterParser(value, registry).parse()
    return _resolve_path_filter_string(value, registry)


def _resolve_root_reference_value(
    value: str, root_data: dict, registry: dict[str, PathFilter]
) -> Any:
    if value == "$$root":
        root_path = "."
    elif value.startswith("$$root."):
        root_path = value[len("$$root.") :]
    elif value.startswith("$$root|"):
        root_path = "." + value[len("$$root") :]
    else:
        raise DictWalkParseError(
            path=value,
            token=value,
            message=(
                "Invalid '$$root' value expression. Expected '$$root', '$$root.<path>', "
                "or '$$root|$filter'."
            ),
        )
    return _get_path_value(root_data, root_path, strict=True, registry=registry)


def _resolve_new_value(
    existing_value: Any,
    new_value: Any,
    registry: dict[str, PathFilter],
    root_data: dict,
) -> Any:
    if isinstance(new_value, str) and new_value.startswith("$$root"):
        return _resolve_root_reference_value(new_value, root_data, registry)
    path_filter = _resolve_path_filter(new_value, registry)
    if path_filter is not None:
        return path_filter(existing_value)
    return new_value


class _GetToken(PathToken):
    def __init__(self, key: str):
        self.key = key

    def resolve(self, current):
        if isinstance(current, dict):
            return current[self.key]
        if isinstance(current, list):
            # When previous tokens fan out (e.g. wildcard), map the key read over items.
            return [
                item[self.key]
                for item in current
                if isinstance(item, dict) and self.key in item
            ]

        raise TypeError(f"Key '{self.key}' not found in current context.")

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        next_tok = remaining[1] if len(remaining) > 1 else None
        if not isinstance(current, dict):
            if not write_options.overwrite_incompatible:
                return current
            if not write_options.create_missing:
                return current
            current = {}
        if len(remaining) == 1:
            if self.key not in current and not write_options.create_missing:
                return current
            current[self.key] = _resolve_new_value(
                current.get(self.key), new_value, registry, root_data
            )
            return current
        child = current.get(self.key)
        if child is None:
            if not write_options.create_missing:
                return current
            child = _new_container_for_next_write(next_tok)
        elif next_tok and not isinstance(child, dict | list):
            if not write_options.overwrite_incompatible:
                return current
            child = _new_container_for_next_write(next_tok)
        child = _set_recurse(
            child, remaining[1:], new_value, write_options, registry, root_data
        )
        current[self.key] = child
        return current

    def unset(self, current, remaining):
        if not isinstance(current, dict):
            return current
        if len(remaining) == 1:
            current.pop(self.key, None)
            return current

        child = current.get(self.key)
        if child is None:
            return current
        current[self.key] = _unset_recurse(child, remaining[1:])
        return current


class _RootToken(PathToken):
    def resolve(self, current):
        return current

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        return current

    def unset(self, current, remaining):
        return current


class _MapToken(PathToken):
    def __init__(self, key: str):
        self.key = key

    def resolve(self, current):
        if isinstance(current, list):
            return [
                item[self.key]
                for item in current
                if isinstance(item, dict) and self.key in item
            ]
        raise TypeError(
            f"Expected a list for key '{self.key}', got {type(current).__name__}."
        )

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        next_tok = remaining[1] if len(remaining) > 1 else None
        if not isinstance(current, dict):
            if not write_options.overwrite_incompatible:
                return current
            if not write_options.create_missing:
                return current
            current = {}
        lst = current.get(self.key)
        if not isinstance(lst, list):
            if lst is None and not write_options.create_missing:
                return current
            if lst is not None and not write_options.overwrite_incompatible:
                return current
            lst = []
        if len(remaining) == 1:
            for i in range(len(lst)):
                lst[i] = _resolve_new_value(lst[i], new_value, registry, root_data)
            current[self.key] = lst
            return current
        if not lst:
            if not write_options.create_missing:
                return current
            lst.append(_new_container_for_next_write(next_tok))
        for i, item in enumerate(lst):
            if not isinstance(item, dict | list) and next_tok:
                if not write_options.overwrite_incompatible:
                    continue
                item = _new_container_for_next_write(next_tok)
            lst[i] = _set_recurse(
                item, remaining[1:], new_value, write_options, registry, root_data
            )
        current[self.key] = lst
        return current

    def unset(self, current, remaining):
        if not isinstance(current, dict):
            return current
        lst = current.get(self.key)
        if not isinstance(lst, list):
            return current

        if len(remaining) == 1:
            current[self.key] = []
            return current

        for i, item in enumerate(lst):
            lst[i] = _unset_recurse(item, remaining[1:])
        current[self.key] = lst
        return current


def _iter_child_nodes(node: Any) -> list[Any]:
    if isinstance(node, dict):
        return list(node.values())
    if isinstance(node, list):
        return list(node)
    return []


class _WildcardToken(PathToken):
    def resolve(self, current):
        children = _iter_child_nodes(current)
        if not children and not isinstance(current, dict | list):
            raise TypeError(
                f"Expected dict or list for wildcard '*', got {type(current).__name__}."
            )
        return children

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        if isinstance(current, dict):
            if len(remaining) == 1:
                for key in list(current.keys()):
                    current[key] = _resolve_new_value(
                        current[key], new_value, registry, root_data
                    )
                return current
            for key in list(current.keys()):
                current[key] = _set_recurse(
                    current[key],
                    remaining[1:],
                    new_value,
                    write_options,
                    registry,
                    root_data,
                )
            return current

        if isinstance(current, list):
            if len(remaining) == 1:
                for idx in range(len(current)):
                    current[idx] = _resolve_new_value(
                        current[idx], new_value, registry, root_data
                    )
                return current
            for idx in range(len(current)):
                current[idx] = _set_recurse(
                    current[idx],
                    remaining[1:],
                    new_value,
                    write_options,
                    registry,
                    root_data,
                )
            return current

        return current

    def unset(self, current, remaining):
        if isinstance(current, dict):
            if len(remaining) == 1:
                current.clear()
                return current
            for key in list(current.keys()):
                current[key] = _unset_recurse(current[key], remaining[1:])
            return current

        if isinstance(current, list):
            if len(remaining) == 1:
                current.clear()
                return current
            for idx in range(len(current)):
                current[idx] = _unset_recurse(current[idx], remaining[1:])
            return current

        return current


class _DeepWildcardToken(PathToken):
    def _descendants(self, node: Any) -> list[Any]:
        result: list[Any] = []
        for child in _iter_child_nodes(node):
            result.append(child)
            result.extend(self._descendants(child))
        return result

    def resolve(self, current):
        descendants = self._descendants(current)
        if not descendants and not isinstance(current, dict | list):
            raise TypeError(
                f"Expected dict or list for wildcard '**', got {type(current).__name__}."
            )
        return descendants

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        if not isinstance(current, dict | list):
            return current

        apply_options = PathWriteOptions(
            create_missing=False,
            create_filter_match=write_options.create_filter_match,
            overwrite_incompatible=write_options.overwrite_incompatible,
        )

        def _walk(node: Any):
            if isinstance(node, dict):
                for key in list(node.keys()):
                    child = node[key]
                    if len(remaining) > 1:
                        node[key] = _set_recurse(
                            child,
                            remaining[1:],
                            new_value,
                            apply_options,
                            registry,
                            root_data,
                        )
                    if isinstance(node[key], dict | list):
                        _walk(node[key])
                return

            if isinstance(node, list):
                for idx in range(len(node)):
                    child = node[idx]
                    if len(remaining) > 1:
                        node[idx] = _set_recurse(
                            child,
                            remaining[1:],
                            new_value,
                            apply_options,
                            registry,
                            root_data,
                        )
                    if isinstance(node[idx], dict | list):
                        _walk(node[idx])

        _walk(current)
        return current

    def unset(self, current, remaining):
        if not isinstance(current, dict | list):
            return current

        def _walk(node: Any):
            if isinstance(node, dict):
                for key in list(node.keys()):
                    child = node[key]
                    if len(remaining) > 1:
                        node[key] = _unset_recurse(child, remaining[1:])
                    if isinstance(node[key], dict | list):
                        _walk(node[key])
                return

            if isinstance(node, list):
                for idx in range(len(node)):
                    child = node[idx]
                    if len(remaining) > 1:
                        node[idx] = _unset_recurse(child, remaining[1:])
                    if isinstance(node[idx], dict | list):
                        _walk(node[idx])

        _walk(current)
        return current


class _IndexToken(PathToken):
    def __init__(self, key: str, index: int | slice):
        self.key = key
        self.index = index

    def _ensure_list(self, current: Any) -> tuple[dict, list]:
        if not isinstance(current, dict):
            current = {}
        lst = current.get(self.key)
        if not isinstance(lst, list):
            lst = []
        return current, lst

    def _iter_slice_indexes(self, lst: list[Any]) -> list[int]:
        assert isinstance(self.index, slice)
        start, stop, step = self.index.indices(len(lst))
        return list(range(start, stop, step))

    def resolve(self, current):
        if not isinstance(current, dict):
            raise TypeError(
                f"Expected a dict for key '{self.key}', got {type(current).__name__}."
            )
        lst = current[self.key]
        if not isinstance(lst, list):
            raise TypeError(
                f"Expected a list for key '{self.key}', got {type(lst).__name__}."
            )
        return lst[self.index]

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        next_tok = remaining[1] if len(remaining) > 1 else None
        if not isinstance(current, dict):
            if not write_options.overwrite_incompatible:
                return current
            if not write_options.create_missing:
                return current
            current = {}

        lst = current.get(self.key)
        if not isinstance(lst, list):
            if lst is None and not write_options.create_missing:
                return current
            if lst is not None and not write_options.overwrite_incompatible:
                return current
            lst = []

        if isinstance(self.index, int):
            idx = self.index
            if idx < 0:
                if abs(idx) > len(lst):
                    current[self.key] = lst
                    return current
            else:
                if not write_options.create_missing:
                    current[self.key] = lst
                    return current
                while len(lst) <= idx:
                    lst.append(
                        _new_container_for_next_write(next_tok) if next_tok else None
                    )

            if len(remaining) == 1:
                lst[idx] = _resolve_new_value(lst[idx], new_value, registry, root_data)
                current[self.key] = lst
                return current

            item = lst[idx]
            if not isinstance(item, dict | list) and next_tok:
                if not write_options.overwrite_incompatible:
                    current[self.key] = lst
                    return current
                item = _new_container_for_next_write(next_tok)
            lst[idx] = _set_recurse(
                item, remaining[1:], new_value, write_options, registry, root_data
            )
            current[self.key] = lst
            return current

        # Slice writes target all selected entries.
        indexes = self._iter_slice_indexes(lst)
        if len(remaining) == 1:
            for idx in indexes:
                lst[idx] = _resolve_new_value(lst[idx], new_value, registry, root_data)
            current[self.key] = lst
            return current

        for idx in indexes:
            item = lst[idx]
            if not isinstance(item, dict | list) and next_tok:
                if not write_options.overwrite_incompatible:
                    continue
                item = _new_container_for_next_write(next_tok)
            lst[idx] = _set_recurse(
                item, remaining[1:], new_value, write_options, registry, root_data
            )

        current[self.key] = lst
        return current

    def unset(self, current, remaining):
        if not isinstance(current, dict):
            return current
        lst = current.get(self.key)
        if not isinstance(lst, list):
            return current

        if isinstance(self.index, int):
            idx = self.index
            if len(remaining) == 1:
                if -len(lst) <= idx < len(lst):
                    lst.pop(idx)
                current[self.key] = lst
                return current

            if -len(lst) <= idx < len(lst):
                lst[idx] = _unset_recurse(lst[idx], remaining[1:])
            current[self.key] = lst
            return current

        indexes = self._iter_slice_indexes(lst)
        if len(remaining) == 1:
            for idx in reversed(indexes):
                lst.pop(idx)
            current[self.key] = lst
            return current

        for idx in indexes:
            lst[idx] = _unset_recurse(lst[idx], remaining[1:])
        current[self.key] = lst
        return current


class _FilterToken(PathToken):
    def __init__(
        self,
        list_key: str,
        field: str,
        operator: str,
        value: str,
        registry: dict[str, PathFilter],
    ):
        self.list_key = list_key
        self.field = field
        self.operator = operator
        self.value = value
        self.field_uses_item_root = False
        self.field_path_filter: PathFilter | None = None
        if field == ".":
            self.field_uses_item_root = True
        elif field.startswith(".|"):
            self.field_uses_item_root = True
            self.field_path_filter = _resolve_path_filter_string(field[2:], registry)
            if self.field_path_filter is None:
                raise DictWalkParseError(
                    path=f"{list_key}[?{field}{operator}{value}]",
                    token=field,
                    message=f"Invalid left-hand predicate expression '{field}'.",
                )
        elif field.startswith("$"):
            raise DictWalkParseError(
                path=f"{list_key}[?{field}{operator}{value}]",
                token=field,
                message=(
                    "Left-hand predicate filter functions must use '?.|$name' syntax (for example: '[?.|$len>3]')."
                ),
            )
        else:
            self.field_path_filter = _resolve_predicate_path_filter(field, registry)
        self.path_filter = _resolve_predicate_path_filter(value, registry)

    @staticmethod
    def _parse_literal(value: str) -> Any:
        try:
            return ast.literal_eval(value)
        except (ValueError, SyntaxError):
            return value

    def _compare(self, left: Any, right: Any) -> bool:
        if self.operator == "==":
            return left == right
        if self.operator == "!=":
            return left != right
        if self.operator == ">":
            return left > right
        if self.operator == "<":
            return left < right
        if self.operator == ">=":
            return left >= right
        if self.operator == "<=":
            return left <= right
        raise DictWalkOperatorError(f"Unsupported operator '{self.operator}'.")

    def _field_value(self, item: Any) -> Any:
        if self.field_uses_item_root:
            if self.field_path_filter is not None:
                return self.field_path_filter(item)
            return item
        if self.field_path_filter is not None:
            return self.field_path_filter(item)
        if isinstance(item, dict):
            return item.get(self.field)
        return None

    def _matches(self, item: Any) -> bool:
        field_value = self._field_value(item)
        if self.path_filter is not None:
            if self.operator == "==":
                return bool(self.path_filter(field_value))
            if self.operator == "!=":
                return not bool(self.path_filter(field_value))
            raise DictWalkOperatorError(
                f"Operator '{self.operator}' is not supported with path filters."
            )

        expected_value = self._parse_literal(self.value)

        # Backward compatibility for unquoted string comparisons such as [?id==1] on {"id": "1"}.
        if self.operator in {"==", "!="}:
            result = field_value == expected_value or str(field_value) == self.value
            return result if self.operator == "==" else not result

        try:
            return self._compare(field_value, expected_value)
        except TypeError:
            pass

        # Try parsing the field value when it is a string and expected value is typed.
        if isinstance(field_value, str):
            try:
                parsed_field_value = self._parse_literal(field_value)
                return self._compare(parsed_field_value, expected_value)
            except (TypeError, ValueError):
                pass

        # Last fallback for ordered ops: compare stringified values.
        return self._compare(str(field_value), str(self.value))

    def resolve(self, current):
        if isinstance(current, dict):
            current = current.get(self.list_key, [])
        if isinstance(current, list):
            return [item for item in current if self._matches(item)]

        raise TypeError(
            f"Expected a list for key '{self.list_key}', got {type(current).__name__}."
        )

    def write(
        self,
        current,
        remaining,
        new_value,
        write_options: PathWriteOptions,
        registry: dict[str, PathFilter],
        root_data: dict,
    ):
        if isinstance(current, dict):
            lst = current.get(self.list_key)
        else:
            lst = None
        if not isinstance(lst, list):
            if lst is None and not write_options.create_missing:
                return current
            if lst is not None and not write_options.overwrite_incompatible:
                return current
            lst = []
        matches = [item for item in lst if self._matches(item)]
        if not matches:
            if (
                not self.field_uses_item_root
                and self.field_path_filter is None
                and self.path_filter is None
                and self.operator == "=="
                and write_options.create_missing
                and write_options.create_filter_match
            ):
                new_item = {self.field: self.value}
                lst.append(new_item)
                matches = [new_item]
        if len(remaining) == 1:
            for i, item in enumerate(lst):
                if item in matches:
                    lst[i] = _resolve_new_value(item, new_value, registry, root_data)
            if isinstance(current, dict):
                current[self.list_key] = lst
                return current
            return lst
        for item in matches:
            _set_recurse(
                item, remaining[1:], new_value, write_options, registry, root_data
            )
        if isinstance(current, dict):
            current[self.list_key] = lst
            return current
        return lst

    def unset(self, current, remaining):
        if isinstance(current, dict):
            lst = current.get(self.list_key)
        else:
            lst = None

        if not isinstance(lst, list):
            return current

        if len(remaining) == 1:
            filtered = [item for item in lst if not self._matches(item)]
            if isinstance(current, dict):
                current[self.list_key] = filtered
                return current
            return filtered

        for item in lst:
            if self._matches(item):
                _unset_recurse(item, remaining[1:])

        if isinstance(current, dict):
            current[self.list_key] = lst
            return current
        return lst


def _new_container_for_next_write(next_tok: PathToken | None) -> Any:
    if next_tok is None:
        return {}
    return {}


def _set_recurse(
    current: Any,
    remaining: list[PathToken],
    new_value: Any,
    write_options: PathWriteOptions,
    registry: dict[str, PathFilter],
    root_data: dict,
) -> Any:
    if not remaining:
        return new_value

    tok = remaining[0]
    return tok.write(current, remaining, new_value, write_options, registry, root_data)


def _unset_recurse(current: Any, remaining: list[PathToken]) -> Any:
    if not remaining:
        return current

    tok = remaining[0]
    return tok.unset(current, remaining)


def _path_uses_root_token(raw_tokens: list[str]) -> bool:
    return any(raw_token == "$$root" for raw_token in raw_tokens)


def _raise_if_path_uses_root_token(path: str, raw_tokens: list[str]):
    if _path_uses_root_token(raw_tokens):
        raise DictWalkParseError(
            path=path,
            token="$$root",
            message="The '$$root' token is only supported in read paths.",
        )


def _parse_token(token: str, registry: dict[str, PathFilter]) -> PathToken:
    if token == "$$root":
        return _RootToken()

    if token == "*":
        return _WildcardToken()

    if token == "**":
        return _DeepWildcardToken()

    if token.endswith("[]"):
        return _MapToken(token[:-2])

    index_match = re.match(r"(.+)\[(-?\d+)\]$", token)
    if index_match:
        key, index = index_match.groups()
        return _IndexToken(key, int(index))

    slice_match = re.match(r"(.+)\[(-?\d*):(-?\d*)\]$", token)
    if slice_match:
        key, start, end = slice_match.groups()
        slice_start = int(start) if start else None
        slice_end = int(end) if end else None
        return _IndexToken(key, slice(slice_start, slice_end))

    filter_match = re.match(r"(.+)\[\?(.+?)(==|!=|>=|<=|>|<)(.+?)\]", token)
    if filter_match:
        list_key, field, operator, value = filter_match.groups()
        return _FilterToken(list_key, field, operator, value, registry)
    return _GetToken(token)


def _split_raw_path_tokens(path: str) -> list[str]:
    tokens: list[str] = []
    current: list[str] = []
    bracket_depth = 0

    for ch in path:
        if ch == "[":
            bracket_depth += 1
            current.append(ch)
            continue
        if ch == "]":
            bracket_depth = max(0, bracket_depth - 1)
            current.append(ch)
            continue
        if ch == "." and bracket_depth == 0:
            tokens.append("".join(current))
            current = []
            continue
        current.append(ch)

    tokens.append("".join(current))
    return tokens


def _parse_path(
    path: str, registry: dict[str, PathFilter]
) -> tuple[list[str], list[PathToken]]:
    if not path:
        raise DictWalkParseError(path=path, token=None, message="Path cannot be empty.")

    raw_tokens = _split_raw_path_tokens(path)
    parsed_tokens: list[PathToken] = []
    for raw_token in raw_tokens:
        try:
            parsed_tokens.append(_parse_token(raw_token, registry))
        except DictWalkError:
            raise
        except Exception as ex:
            raise DictWalkParseError(
                path=path, token=raw_token, message="Failed to parse path token."
            ) from ex

    return raw_tokens, parsed_tokens


def _split_path_and_transform(path: str) -> tuple[str, str | None]:
    """
    Split a path into base dictwalk + optional top-level transform pipeline.

    Example:
        "a.b.c|$double|$string" -> ("a.b.c", "$double|$string")

    Pipes inside bracket expressions are ignored (e.g. list predicate filters).
    """
    bracket_depth = 0
    for i, ch in enumerate(path):
        if ch == "[":
            bracket_depth += 1
            continue
        if ch == "]":
            bracket_depth = max(0, bracket_depth - 1)
            continue
        if ch == "|" and bracket_depth == 0:
            if i + 1 < len(path) and path[i + 1] == "$":
                return path[:i], path[i + 1 :]
    return path, None


def _raise_resolution_error(path: str, token: str | None, ex: Exception):
    raise DictWalkResolutionError(path=path, token=token, message=str(ex)) from ex


def _ensure_path_resolves(
    path: str, data: dict, raw_tokens: list[str], tokens: list[PathToken], *, until: int
):
    current: Any = data
    for raw_token, token in zip(raw_tokens[:until], tokens[:until]):
        try:
            current = token.resolve(current)
        except (KeyError, TypeError, DictWalkOperatorError) as ex:
            _raise_resolution_error(path, raw_token, ex)


def _get_path_value(
    data: dict,
    path: str,
    default=None,
    *,
    strict: bool = False,
    registry: dict[str, PathFilter],
):
    base_path, output_transform = _split_path_and_transform(path)
    if base_path == ".":
        current = data
        if output_transform is not None:
            path_filter = _resolve_path_filter_string(output_transform, registry)
            if path_filter is not None:
                current = path_filter(current)
        return current

    raw_tokens, tokens = _parse_path(base_path, registry)
    current = data
    for raw_token, token in zip(raw_tokens, tokens):
        if isinstance(token, _RootToken):
            current = data
            continue
        if strict:
            try:
                current = token.resolve(current)
            except (KeyError, TypeError, DictWalkOperatorError) as ex:
                _raise_resolution_error(base_path, raw_token, ex)
        else:
            try:
                current = token.resolve(current)
            except (KeyError, TypeError, DictWalkOperatorError):
                return default
    if output_transform is not None:
        path_filter = _resolve_path_filter_string(output_transform, registry)
        if path_filter is None:
            return current
        current = path_filter(current)
    return current


def _path_exists(
    data: dict,
    path: str,
    *,
    strict: bool = False,
    registry: dict[str, PathFilter],
) -> bool:
    raw_tokens, tokens = _parse_path(path, registry)
    current = data
    for raw_token, token in zip(raw_tokens, tokens):
        if isinstance(token, _RootToken):
            current = data
            continue
        if strict:
            try:
                current = token.resolve(current)
            except (KeyError, TypeError, DictWalkOperatorError) as ex:
                _raise_resolution_error(path, raw_token, ex)
        else:
            try:
                current = token.resolve(current)
            except (KeyError, TypeError, DictWalkOperatorError):
                return False
    return True


def _set_path_value(
    data: dict,
    path: str,
    value: Any,
    *,
    strict: bool = False,
    create_missing: bool = True,
    create_filter_match: bool = True,
    overwrite_incompatible: bool = True,
    registry: dict[str, PathFilter],
) -> dict:
    raw_tokens, tokens = _parse_path(path, registry)
    _raise_if_path_uses_root_token(path, raw_tokens)

    if strict and tokens:
        _ensure_path_resolves(path, data, raw_tokens, tokens, until=len(tokens) - 1)

    _set_recurse(
        data,
        tokens,
        value,
        PathWriteOptions(
            create_missing=create_missing,
            create_filter_match=create_filter_match,
            overwrite_incompatible=overwrite_incompatible,
        ),
        registry,
        data,
    )
    return data


def _unset_path(
    data: dict,
    path: str,
    *,
    strict: bool = False,
    registry: dict[str, PathFilter],
) -> dict:
    raw_tokens, tokens = _parse_path(path, registry)
    _raise_if_path_uses_root_token(path, raw_tokens)

    if strict and tokens:
        _ensure_path_resolves(path, data, raw_tokens, tokens, until=len(tokens))

    _unset_recurse(data, tokens)
    return data


class DictWalk:
    def __init__(self):
        self._path_filter_registry: dict[str, PathFilter] = {}
        for name, filter_fn in DEFAULT_FILTER_FUNCTION_REGISTRY.items():
            self.register_path_filter(name, filter_fn)

    def register_path_filter(
        self, name: str, path_filter: PathFilter | Callable[..., Any]
    ) -> PathFilter:
        """
        Register a path filter in this `DictWalk` instance.

        Registered filters can be used in path expressions and value transforms
        via `$name` syntax, including piped forms such as `$a|$b` and argument forms
        such as `$add(2)`.

        Args:
            name: Filter name used in path/value expressions.
            path_filter: A `PathFilter` or callable to wrap as `PathFilter`.

        Returns:
            The registered `PathFilter` instance.
        """
        return _register_path_filter(self._path_filter_registry, name, path_filter)

    def get_path_filter(self, name: str) -> PathFilter:
        """
        Retrieve a previously registered path filter by name.

        Args:
            name: Registered filter name.

        Returns:
            The matching `PathFilter`.

        Raises:
            KeyError: If the filter name is not registered.
        """
        return _get_path_filter(self._path_filter_registry, name)

    def get(
        self,
        data: dict,
        path: str,
        default=None,
        *,
        strict: bool = False,
    ):
        """
        Resolve and return a value from `data` using DictWalk syntax.

        This method supports:
        - Dot traversal: `a.b.c`
        - List mapping: `a.items[]`
        - List index/slice: `a.items[0]`, `a.items[-1]`, `a.items[1:3]`
        - Filter predicates: `a.items[?id==1]`, `a.items[?id>10]`
        - Predicate path filters: `a.items[?id==$even]`, including boolean composition
          such as `&&`, `||`, `!` and grouping with parentheses.
        - Wildcards: `*` (single level) and `**` (deep traversal)
        - Optional output transform pipeline: `a.b.c|$double|$string`

        Args:
            data: Root dictionary to read from.
            path: DictWalk expression describing the traversal.
            default: Value returned when the path cannot be resolved in non-strict mode.
            strict: If True, raises `DictWalkResolutionError` on resolution failures.
                If False, returns `default` on resolution failures.

        Returns:
            The resolved value, or `default` when resolution fails in non-strict mode.

        Raises:
            DictWalkParseError: If the path expression is invalid.
            DictWalkResolutionError: If strict mode is enabled and path resolution fails.

        Examples:
            >>> dictwalk.get({"a": {"b": {"c": 1}}}, "a.b.c")
            1
            >>> dictwalk.get({"a": {"b": [{"c": 1}, {"c": 2}]}}, "a.b.c[]")
            [1, 2]
            >>> dictwalk.get({"a": {"b": [{"c": 10}, {"c": 20}, {"c": 30}]}}, "a.b[1].c")
            20
            >>> dictwalk.get({"a": {"b": [{"c": 10}, {"c": 20}, {"c": 30}, {"c": 40}]}}, "a.b[1:3].c[]")
            [20, 30]
            >>> dictwalk.get({"a": {"b": [{"id": 1}, {"id": 2}]}}, "a.b[?id==$even].id[]")
            [2]
            >>> dictwalk.get({"a": {"u1": {"id": 1}, "u2": {"id": 2}}}, "a.*.id")
            [1, 2]
            >>> dictwalk.get(
            ...     {"a": {"g1": {"u1": {"id": 1}}, "g2": {"nested": {"u2": {"id": 2}}}}},
            ...     "a.**.id",
            ... )
            [1, 2]
            >>> dictwalk.get({"a": {"b": {"c": 2}}}, "a.b.c|$double|$string")
            '4'
            >>> dictwalk.get({"a": {"b": {}}}, "a.b.missing", default="n/a")
            'n/a'
            >>> dictwalk.get({"a": {"b": {}}}, "a.b.c", strict=True)
            Traceback (most recent call last):
            ...
            bark_core.utilities.dictwalk.core.DictWalkResolutionError: ...
        """
        return _get_path_value(
            data,
            path,
            default=default,
            strict=strict,
            registry=self._path_filter_registry,
        )

    def exists(self, data: dict, path: str, *, strict: bool = False) -> bool:
        """
        Check whether a path can be resolved from `data`.

        Args:
            data: Root dictionary to inspect.
            path: DictWalk expression to resolve.
            strict: If True, raises `DictWalkResolutionError` on resolution failures.
                If False, returns `False` on resolution failures.

        Returns:
            `True` when the path resolves, otherwise `False` in non-strict mode.

        Raises:
            DictWalkParseError: If the path syntax is invalid.
            DictWalkResolutionError: If strict mode is enabled and resolution fails.
        """
        return _path_exists(
            data,
            path,
            strict=strict,
            registry=self._path_filter_registry,
        )

    def set(
        self,
        data: dict,
        path: str,
        value: Any,
        *,
        strict: bool = False,
        create_missing: bool = True,
        create_filter_match: bool = True,
        overwrite_incompatible: bool = True,
    ) -> dict:
        """
        Write `value` at `path` in `data`.

        Supports object keys, list mapping/index/slice, predicates, and wildcards.
        When `value` is a `PathFilter` (or `$filter` string), the existing value at
        each target location is transformed.

        Args:
            data: Root dictionary to mutate in place.
            path: DictWalk expression describing write targets.
            value: Direct value or filter transform to apply.
            strict: If True, requires the path to resolve up to the parent before write.
            create_missing: Whether to create missing intermediate containers.
            create_filter_match: Whether filter writes may create a new matching item.
            overwrite_incompatible: Whether incompatible intermediate values may be replaced.

        Returns:
            The same mutated `data` object.

        Raises:
            DictWalkParseError: If path syntax is invalid.
            DictWalkResolutionError: If strict mode is enabled and resolution fails.
        """
        return _set_path_value(
            data,
            path,
            value,
            strict=strict,
            create_missing=create_missing,
            create_filter_match=create_filter_match,
            overwrite_incompatible=overwrite_incompatible,
            registry=self._path_filter_registry,
        )

    def unset(self, data: dict, path: str, *, strict: bool = False) -> dict:
        """
        Remove values targeted by `path` from `data`.

        Supports object keys, list mapping/index/slice, predicates, and wildcards.

        Args:
            data: Root dictionary to mutate in place.
            path: DictWalk expression describing removal targets.
            strict: If True, raises on unresolved path segments.

        Returns:
            The same mutated `data` object.

        Raises:
            DictWalkParseError: If path syntax is invalid.
            DictWalkResolutionError: If strict mode is enabled and resolution fails.
        """
        return _unset_path(
            data,
            path,
            strict=strict,
            registry=self._path_filter_registry,
        )


dictwalk = DictWalk()


def register_path_filter(
    name: str, path_filter: PathFilter | Callable[..., Any]
) -> PathFilter:
    """
    Register a path filter in the default `DictWalk` instance.
    The default path instance is bark_core.utilities.dictwalk.dp, which is used by the top-level utility functions such as
    Registered filters can be used in path expressions and value transforms
    via `$name` syntax, including piped forms such as `$a|$b` and argument forms
    such as `$add(2)`.

    Args:
        name: Filter name used in path/value expressions.
        path_filter: A `PathFilter` or callable to wrap as `PathFilter`.
    Returns:
        The registered `PathFilter` instance.
    """
    return dictwalk.register_path_filter(name, path_filter)
