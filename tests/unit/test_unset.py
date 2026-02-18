from dictwalk import dictwalk


def test_unset__unsets_nested_key():
    data = {"a": {"b": {"c": 1, "d": 2}}}
    path = "a.b.c"
    expected = {"a": {"b": {"d": 2}}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_mapped_key_from_all_list_items():
    data = {"a": {"b": [{"c": 1, "d": 10}, {"c": 2, "d": 20}]}}
    path = "a.b[].c"
    expected = {"a": {"b": [{"d": 10}, {"d": 20}]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_key_from_filter_matches():
    data = {"a": {"b": [{"id": 1, "c": 10}, {"id": 2, "c": 20}]}}
    path = "a.b[?.id==2].c"
    expected = {"a": {"b": [{"id": 1, "c": 10}, {"id": 2}]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__removes_filter_matched_items_at_terminal_filter_path():
    data = {"a": {"b": [{"id": 1}, {"id": 2}, {"id": 3}]}}
    path = "a.b[?.id>1]"
    expected = {"a": {"b": [{"id": 1}]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__keeps_even_numbers_in_scalar_list_using_current_item_predicate():
    data = {"a": {"b": [1, 2, 3, 4, 5]}}
    path = "a.b[?.|$even==False]"
    expected = {"a": {"b": [2, 4]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__removes_even_numbers_in_scalar_list_using_current_item_predicate():
    data = {"a": {"b": [1, 2, 3, 4, 5]}}
    path = "a.b[?.|$even==True]"
    expected = {"a": {"b": [1, 3, 5]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__keeps_values_greater_than_threshold_in_scalar_list():
    data = {"a": {"b": [1, 2, 3, 4, 5]}}
    path = "a.b[?.|$gt(3)==False]"
    expected = {"a": {"b": [4, 5]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_list_index_at_terminal_path():
    data = {"a": {"b": [{"id": 1}, {"id": 2}, {"id": 3}]}}
    path = "a.b[1]"
    expected = {"a": {"b": [{"id": 1}, {"id": 3}]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_list_slice_at_terminal_path():
    data = {"a": {"b": [{"id": 1}, {"id": 2}, {"id": 3}, {"id": 4}]}}
    path = "a.b[1:3]"
    expected = {"a": {"b": [{"id": 1}, {"id": 4}]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_nested_key_using_list_slice():
    data = {"a": {"b": [{"id": 1, "c": 10}, {"id": 2, "c": 20}, {"id": 3, "c": 30}]}}
    path = "a.b[1:3].c"
    expected = {"a": {"b": [{"id": 1, "c": 10}, {"id": 2}, {"id": 3}]}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_single_layer_wildcard_key():
    data = {"a": {"u1": {"debug": True, "id": 1}, "u2": {"debug": False, "id": 2}}}
    path = "a.*.debug"
    expected = {"a": {"u1": {"id": 1}, "u2": {"id": 2}}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__unsets_deep_wildcard_key():
    data = {
        "a": {
            "g1": {"u1": {"debug": True, "id": 1}},
            "g2": {"nested": {"u2": {"debug": False, "id": 2}}},
        }
    }
    path = "a.**.debug"
    expected = {"a": {"g1": {"u1": {"id": 1}}, "g2": {"nested": {"u2": {"id": 2}}}}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected


def test_unset__no_op_when_target_does_not_exist():
    data = {"a": {"b": {"c": 1}}}
    path = "a.b.x"
    expected = {"a": {"b": {"c": 1}}}

    result = dictwalk.unset(data, path)

    assert result is data
    assert data == expected
