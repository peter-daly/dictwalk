import math
from datetime import datetime, timezone
from decimal import Decimal

import pytest

from dictwalk import dictwalk
from dictwalk.errors import DictWalkParseError


def test_run_filter_function__builtin_inc():
    assert dictwalk.run_filter_function("$inc", 2) == 3


def test_run_filter_function__builtin_dec():
    assert dictwalk.run_filter_function("$dec", 2) == 1


def test_run_filter_function__builtin_double():
    assert dictwalk.run_filter_function("$double", 3) == 6


def test_run_filter_function__builtin_square():
    assert dictwalk.run_filter_function("$square", 4) == 16


def test_run_filter_function__builtin_string():
    assert dictwalk.run_filter_function("$string", 9) == "9"


def test_run_filter_function__builtin_int():
    assert dictwalk.run_filter_function("$int", "9") == 9


def test_run_filter_function__builtin_float():
    assert dictwalk.run_filter_function("$float", "9.5") == 9.5


def test_run_filter_function__builtin_decimal():
    assert dictwalk.run_filter_function("$decimal", "9.5") == Decimal("9.5")


def test_run_filter_function__builtin_round():
    assert dictwalk.run_filter_function("$round", 2.6) == 3.0


def test_run_filter_function__builtin_floor():
    assert dictwalk.run_filter_function("$floor", 2.9) == 2


def test_run_filter_function__builtin_ceil():
    assert dictwalk.run_filter_function("$ceil", 2.1) == 3


def test_run_filter_function__builtin_abs():
    assert dictwalk.run_filter_function("$abs", -7) == 7


def test_run_filter_function__builtin_quote():
    assert dictwalk.run_filter_function("$quote", "hello") == '"hello"'


def test_run_filter_function__builtin_even():
    assert dictwalk.run_filter_function("$even", 4) is True


def test_run_filter_function__builtin_odd():
    assert dictwalk.run_filter_function("$odd", 5) is True


def test_run_filter_function__builtin_gt():
    assert dictwalk.run_filter_function("$gt(2)", 3) is True


def test_run_filter_function__builtin_lt():
    assert dictwalk.run_filter_function("$lt(5)", 3) is True


def test_run_filter_function__builtin_gte():
    assert dictwalk.run_filter_function("$gte(3)", 3) is True


def test_run_filter_function__builtin_lte():
    assert dictwalk.run_filter_function("$lte(3)", 3) is True


def test_run_filter_function__builtin_add():
    assert dictwalk.run_filter_function("$add(2)", 3) == 5


def test_run_filter_function__builtin_sub():
    assert dictwalk.run_filter_function("$sub(2)", 3) == 1


def test_run_filter_function__builtin_mul():
    assert dictwalk.run_filter_function("$mul(2)", 3) == 6


def test_run_filter_function__builtin_div():
    assert dictwalk.run_filter_function("$div(2)", 8) == 4.0


def test_run_filter_function__builtin_mod():
    assert dictwalk.run_filter_function("$mod(3)", 8) == 2


def test_run_filter_function__builtin_neg():
    assert dictwalk.run_filter_function("$neg", 3) == -3


def test_run_filter_function__builtin_pow():
    assert dictwalk.run_filter_function("$pow(3)", 2) == 8


def test_run_filter_function__builtin_rpow():
    assert dictwalk.run_filter_function("$rpow(2)", 3) == 8


def test_run_filter_function__builtin_sqrt():
    assert dictwalk.run_filter_function("$sqrt", 9) == 3.0


def test_run_filter_function__builtin_root():
    assert dictwalk.run_filter_function("$root(2)", 9) == 3.0


def test_run_filter_function__builtin_max():
    assert dictwalk.run_filter_function("$max", [1, 5, 2]) == 5


def test_run_filter_function__builtin_min():
    assert dictwalk.run_filter_function("$min", [1, 5, 2]) == 1


def test_run_filter_function__builtin_len():
    assert dictwalk.run_filter_function("$len", {"a": 1, "b": 2}) == 2


def test_run_filter_function__builtin_pick():
    assert dictwalk.run_filter_function(
        "$pick('a', 'c')", {"a": 1, "b": 2, "c": 3}
    ) == {
        "a": 1,
        "c": 3,
    }


def test_run_filter_function__builtin_unpick():
    assert dictwalk.run_filter_function("$unpick('b')", {"a": 1, "b": 2, "c": 3}) == {
        "a": 1,
        "c": 3,
    }


def test_run_filter_function__builtin_clamp():
    assert dictwalk.run_filter_function("$clamp(0, 10)", 99) == 10


def test_run_filter_function__builtin_sign():
    assert dictwalk.run_filter_function("$sign", -2) == -1


def test_run_filter_function__builtin_log():
    assert dictwalk.run_filter_function("$log(10)", 1000) == pytest.approx(3.0)


def test_run_filter_function__builtin_exp():
    assert dictwalk.run_filter_function("$exp", 1) == pytest.approx(math.e)


def test_run_filter_function__builtin_pct():
    assert dictwalk.run_filter_function("$pct(25)", 200) == 50.0


def test_run_filter_function__builtin_pctile():
    assert dictwalk.run_filter_function("$pctile(50)", [1, 2, 3, 4, 5]) == 3.0


def test_run_filter_function__builtin_pctile_with_interpolation():
    assert dictwalk.run_filter_function("$pctile(25)", [1, 2, 3, 4]) == 1.75


def test_run_filter_function__builtin_pctile_empty_collection():
    assert dictwalk.run_filter_function("$pctile(50)", []) is None


def test_run_filter_function__builtin_median():
    assert dictwalk.run_filter_function("$median", [1, 2, 3, 4]) == 2.5


def test_run_filter_function__builtin_q1():
    assert dictwalk.run_filter_function("$q1", [1, 2, 3, 4]) == 1.75


def test_run_filter_function__builtin_q3():
    assert dictwalk.run_filter_function("$q3", [1, 2, 3, 4]) == 3.25


def test_run_filter_function__builtin_iqr():
    assert dictwalk.run_filter_function("$iqr", [1, 2, 3, 4]) == 1.5


def test_run_filter_function__builtin_mode():
    assert dictwalk.run_filter_function("$mode", [1, 2, 2, 3]) == 2


def test_run_filter_function__builtin_mode_tie_picks_first_seen():
    assert dictwalk.run_filter_function("$mode", [2, 1, 2, 1]) == 2


def test_run_filter_function__builtin_mode_empty_collection():
    assert dictwalk.run_filter_function("$mode", []) is None


def test_run_filter_function__builtin_stdev():
    assert dictwalk.run_filter_function("$stdev", [1, 2, 3, 4]) == pytest.approx(
        1.118033988749895
    )


def test_run_filter_function__builtin_between():
    assert dictwalk.run_filter_function("$between(1, 10)", 5) is True


def test_run_filter_function__builtin_sum():
    assert dictwalk.run_filter_function("$sum", [1, 2, 3]) == 6


def test_run_filter_function__builtin_avg():
    assert dictwalk.run_filter_function("$avg", [1, 2, 3]) == 2.0


def test_run_filter_function__builtin_unique():
    assert dictwalk.run_filter_function("$unique", [1, 2, 2, 3, 1]) == [1, 2, 3]


def test_run_filter_function__builtin_sorted():
    assert dictwalk.run_filter_function("$sorted", [3, 1, 2]) == [1, 2, 3]


def test_run_filter_function__builtin_first():
    assert dictwalk.run_filter_function("$first", [9, 8, 7]) == 9


def test_run_filter_function__builtin_last():
    assert dictwalk.run_filter_function("$last", [9, 8, 7]) == 7


def test_run_filter_function__builtin_contains():
    assert dictwalk.run_filter_function("$contains(2)", [1, 2, 3]) is True


def test_run_filter_function__builtin_in():
    assert dictwalk.run_filter_function("$in([1, 2, 3])", 2) is True


def test_run_filter_function__builtin_lower():
    assert dictwalk.run_filter_function("$lower", "HeLLo") == "hello"


def test_run_filter_function__builtin_upper():
    assert dictwalk.run_filter_function("$upper", "HeLLo") == "HELLO"


def test_run_filter_function__builtin_title():
    assert dictwalk.run_filter_function("$title", "hello world") == "Hello World"


def test_run_filter_function__builtin_strip():
    assert dictwalk.run_filter_function("$strip", "  hi  ") == "hi"


def test_run_filter_function__builtin_replace():
    assert (
        dictwalk.run_filter_function("$replace('world', 'there')", "hello world")
        == "hello there"
    )


def test_run_filter_function__builtin_split():
    assert dictwalk.run_filter_function("$split(',')", "a,b,c") == ["a", "b", "c"]


def test_run_filter_function__builtin_join():
    assert dictwalk.run_filter_function("$join('-')", ["a", "b", "c"]) == "a-b-c"


def test_run_filter_function__builtin_startswith():
    assert dictwalk.run_filter_function("$startswith('he')", "hello") is True


def test_run_filter_function__builtin_endswith():
    assert dictwalk.run_filter_function("$endswith('lo')", "hello") is True


def test_run_filter_function__builtin_matches():
    assert (
        dictwalk.run_filter_function("$matches('hello\\\\s+world')", "hello world")
        is True
    )


def test_run_filter_function__builtin_default():
    assert dictwalk.run_filter_function("$default(9)", None) == 9


def test_run_filter_function__builtin_coalesce():
    assert dictwalk.run_filter_function("$coalesce(None, 7, 8)", None) == 7


def test_run_filter_function__builtin_bool():
    assert dictwalk.run_filter_function("$bool", "YES") is True


def test_run_filter_function__builtin_type_is():
    assert dictwalk.run_filter_function("$type_is('int')", 5) is True


def test_run_filter_function__builtin_is_empty():
    assert dictwalk.run_filter_function("$is_empty", []) is True


def test_run_filter_function__builtin_non_empty():
    assert dictwalk.run_filter_function("$non_empty", [1]) is True


def test_run_filter_function__builtin_to_datetime():
    assert dictwalk.run_filter_function(
        "$to_datetime", "2024-01-02T03:04:05Z"
    ) == datetime(2024, 1, 2, 3, 4, 5, tzinfo=timezone.utc)


def test_run_filter_function__builtin_timestamp():
    expected = datetime(2024, 1, 2, 3, 4, 5, tzinfo=timezone.utc).timestamp()
    assert dictwalk.run_filter_function(
        "$timestamp", "2024-01-02T03:04:05Z"
    ) == pytest.approx(expected)


def test_run_filter_function__builtin_age_seconds():
    result = dictwalk.run_filter_function("$age_seconds", "1970-01-01T00:00:00+00:00")
    assert isinstance(result, float)
    assert result > 1_000_000


def test_run_filter_function__builtin_before():
    assert (
        dictwalk.run_filter_function(
            "$before('2025-01-01T00:00:00+00:00')", "2024-01-01T00:00:00+00:00"
        )
        is True
    )


def test_run_filter_function__builtin_after():
    assert (
        dictwalk.run_filter_function(
            "$after('2024-01-01T00:00:00+00:00')", "2025-01-01T00:00:00+00:00"
        )
        is True
    )


def test_run_filter_function__supports_round_with_args():
    assert dictwalk.run_filter_function("$round(2)", 2.349) == 2.35


def test_run_filter_function__supports_sorted_reverse():
    assert dictwalk.run_filter_function("$sorted(True)", [1, 3, 2]) == [3, 2, 1]


def test_run_filter_function__applies_filter_pipeline():
    assert dictwalk.run_filter_function("$add(2)|$double", 3) == 10


def test_run_filter_function__rejects_path_filter_instance():
    with pytest.raises(DictWalkParseError):
        dictwalk.run_filter_function(object(), 7)


def test_run_filter_function__raises_parse_error_for_non_filter_string():
    with pytest.raises(DictWalkParseError):
        dictwalk.run_filter_function("add(2)", 3)
