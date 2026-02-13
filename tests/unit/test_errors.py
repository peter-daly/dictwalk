import pytest
from dictwalk import dictwalk
from dictwalk.errors import (
    DictWalkError,
    DictWalkResolutionError,
    DictWalkParseError,
)


def test_get_path_value_raises_parse_error_for_invalid_boolean_filter_expression():
    with pytest.raises(DictWalkParseError):
        dictwalk.get({"a": {"b": [{"id": 1}]}}, "a.b[?id==$gt(1)&&].id[]")


def test_get_path_value_raises_parse_error_for_legacy_left_side_filter_function_syntax():
    with pytest.raises(DictWalkParseError):
        dictwalk.get({"a": {"b": ["hello", "world", "foo", "bar"]}}, "a.b[?$len>3]")


def test_path_exists_raises_parse_error_for_legacy_left_side_filter_function_syntax():
    with pytest.raises(DictWalkParseError):
        dictwalk.exists({"a": {"b": ["hello", "world", "foo", "bar"]}}, "a.b[?$len>3]")


def test_set_raises_parse_error_for_root_token():
    with pytest.raises(DictWalkParseError):
        dictwalk.set({"a": {"b": {"c": 1}}, "x": 2}, "a.b.$$root.x", 9)


def test_unset_raises_parse_error_for_root_token():
    with pytest.raises(DictWalkParseError):
        dictwalk.unset({"a": {"b": {"c": 1}}, "x": 2}, "a.b.$$root.x")


def test_get_path_value_strict_raises_resolution_error_for_missing_path():
    with pytest.raises(DictWalkResolutionError):
        dictwalk.get({"a": {"b": {}}}, "a.b.c", strict=True)


def test_path_exists_strict_raises_resolution_error_for_missing_path():
    with pytest.raises(DictWalkResolutionError):
        dictwalk.exists({"a": {"b": {}}}, "a.b.c", strict=True)


def test_set_path_value_strict_raises_when_parent_path_missing():
    with pytest.raises(DictWalkResolutionError):
        dictwalk.set({}, "a.b.c", 1, strict=True)


def test_unset_path_strict_raises_when_path_missing():
    with pytest.raises(DictWalkResolutionError):
        dictwalk.unset({"a": {"b": {}}}, "a.b.c", strict=True)


def test_register_path_filter_raises_when_custom_filter_registration_is_attempted():
    with pytest.raises(DictWalkError):
        dictwalk.register_path_filter("triple", lambda value: value * 3)


def test_get_path_filter_raises_when_accessing_filter_objects():
    with pytest.raises(DictWalkError):
        dictwalk.get_path_filter("double")
