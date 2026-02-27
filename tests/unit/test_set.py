from dictwalk import dictwalk


def test_set__creates_nested_dicts():
    data = {}
    path = "a.b.c"
    value = 5
    expected = {"a": {"b": {"c": 5}}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__creates_list_for_map_path():
    data = {}
    path = "a.b[].c"
    value = 5
    expected = {"a": {"b": [{"c": 5}]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__creates_filtered_item():
    data = {}
    path = "a.b[?.id==3].c"
    value = 5
    expected = {"a": {"b": [{"id": "3", "c": 5}]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__updates_only_filter_matches():
    data = {"a": {"b": [{"id": "3", "c": 1}, {"id": "4", "c": 2}]}}
    path = "a.b[?.id==3].c"
    value = 7
    expected = {"a": {"b": [{"id": "3", "c": 7}, {"id": "4", "c": 2}]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__overwrites_incompatible_intermediate():
    data = {"a": 1}
    path = "a.b"
    value = 2
    expected = {"a": {"b": 2}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__sets_terminal_map_values():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = 9
    expected = {"a": {"b": [9, 9, 9]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_lists_when_mapped_filter_function_is_passed_in_value():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b"
    value = "$double[]"
    expected = {"a": {"b": [2, 4, 6]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__rewrties_list_when_passed_in_without_mapping():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b"
    value = "$len"
    expected = {"a": {"b": 3}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_double_path_filter():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$double"
    expected = {"a": {"b": [2, 4, 6]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_quote_path_filter():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$quote"
    expected = {"a": {"b": ['"1"', '"2"', '"3"']}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_string_path_filter():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$string"
    expected = {"a": {"b": ["1", "2", "3"]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_composed_path_filter():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$double|$inc|$string"
    expected = {"a": {"b": ["3", "5", "7"]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_registered_path_filter():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$double"
    expected = {"a": {"b": [2, 4, 6]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_piped_registered_path_filters():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$double|$inc|$string"
    expected = {"a": {"b": ["3", "5", "7"]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_registered_path_filter_with_args():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$add(2)"
    expected = {"a": {"b": [3, 4, 5]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_piped_registered_path_filter_with_args():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$add(2)|$string"
    expected = {"a": {"b": ["3", "4", "5"]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__transforms_terminal_map_values_with_namespaced_filter():
    data = {"a": {"b": [1, 2, 3]}}
    path = "a.b[]"
    value = "$add(2)"
    expected = {"a": {"b": [3, 4, 5]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__sets_values_using_root_reference_write_expression():
    data = {"a": {"b": [{"c": 0}, {"c": 0}]}, "d": 9}
    path = "a.b[].c"
    value = "$$root.d"
    expected = {"a": {"b": [{"c": 9}, {"c": 9}]}, "d": 9}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__sets_values_using_root_reference_write_expression_with_transform():
    data = {"a": {"b": [{"c": 0}, {"c": 0}]}, "d": 9}
    path = "a.b[].c"
    value = "$$root.d|$double"
    expected = {"a": {"b": [{"c": 18}, {"c": 18}]}, "d": 9}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__updates_only_items_matching_operator_filter():
    data = {
        "a": {
            "b": [
                {"id": 1, "value": 10},
                {"id": 2, "value": 20},
                {"id": 3, "value": 30},
            ]
        }
    }
    path = "a.b[?.id>1].value"
    value = 0
    expected = {
        "a": {
            "b": [{"id": 1, "value": 10}, {"id": 2, "value": 0}, {"id": 3, "value": 0}]
        }
    }

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__updates_only_items_matching_boolean_filter():
    data = {
        "a": {
            "b": [
                {"id": 1, "value": 10},
                {"id": 2, "value": 20},
                {"id": 3, "value": 30},
                {"id": 4, "value": 40},
            ]
        }
    }
    path = "a.b[?.id==$gt(1)&&$lt(4)].value"
    value = -1
    expected = {
        "a": {
            "b": [
                {"id": 1, "value": 10},
                {"id": 2, "value": -1},
                {"id": 3, "value": -1},
                {"id": 4, "value": 40},
            ]
        }
    }

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__set_path_value_with_list_index():
    data = {"a": {"b": [{"id": 1, "value": 10}, {"id": 2, "value": 20}]}}
    path = "a.b[1].value"
    value = 99
    expected = {"a": {"b": [{"id": 1, "value": 10}, {"id": 2, "value": 99}]}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__set_path_value_with_list_slice():
    data = {
        "a": {
            "b": [
                {"id": 1, "value": 10},
                {"id": 2, "value": 20},
                {"id": 3, "value": 30},
                {"id": 4, "value": 40},
            ]
        }
    }
    path = "a.b[1:3].value"
    value = 0
    expected = {
        "a": {
            "b": [
                {"id": 1, "value": 10},
                {"id": 2, "value": 0},
                {"id": 3, "value": 0},
                {"id": 4, "value": 40},
            ]
        }
    }

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__sets_root_list_map_selectors():
    data = [{"v": 1}, {"v": 2}, {"v": 3}]

    dictwalk.set(data, ".[].v", 9)
    assert data == [{"v": 9}, {"v": 9}, {"v": 9}]

    dictwalk.set(data, "$$root[].v", 1)
    assert data == [{"v": 1}, {"v": 1}, {"v": 1}]


def test_set__sets_root_list_index_and_slice_selectors():
    data = [{"v": 1}, {"v": 2}, {"v": 3}, {"v": 4}]

    dictwalk.set(data, ".[1].v", 20)
    dictwalk.set(data, "$$root[2:4].v", 30)
    assert data == [{"v": 1}, {"v": 20}, {"v": 30}, {"v": 30}]


def test_set__sets_root_list_filter_selectors():
    data = [{"id": 1, "v": 10}, {"id": 2, "v": 20}, {"id": 3, "v": 30}]

    dictwalk.set(data, ".[?.id>1].v", 0)
    assert data == [{"id": 1, "v": 10}, {"id": 2, "v": 0}, {"id": 3, "v": 0}]

    dictwalk.set(data, "$$root[?.id==1].v", 99)
    assert data == [{"id": 1, "v": 99}, {"id": 2, "v": 0}, {"id": 3, "v": 0}]


def test_set__sets_root_list_index_with_create_missing():
    data = [{"v": 1}]

    dictwalk.set(data, ".[2].v", 7)
    assert data == [{"v": 1}, {}, {"v": 7}]


def test_set__set_path_value_with_single_layer_wildcard():
    data = {"a": {"u1": {"enabled": False}, "u2": {"enabled": False}}}
    path = "a.*.enabled"
    value = True
    expected = {"a": {"u1": {"enabled": True}, "u2": {"enabled": True}}}

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__set_path_value_with_deep_wildcard():
    data = {
        "a": {
            "g1": {"u1": {"enabled": False}},
            "g2": {"nested": {"u2": {"enabled": False}}},
        }
    }
    path = "a.**.enabled"
    value = True
    expected = {
        "a": {
            "g1": {"u1": {"enabled": True}},
            "g2": {"nested": {"u2": {"enabled": True}}},
        }
    }

    result = dictwalk.set(data, path, value)

    assert result is data
    assert data == expected


def test_set__strict_succeeds_when_parent_exists():
    data = {"a": {"b": {}}}
    dictwalk.set(data, "a.b.c", 1, strict=True)
    assert data == {"a": {"b": {"c": 1}}}


def test_set__does_not_create_missing_when_disabled():
    data = {}
    dictwalk.set(data, "a.b.c", 1, create_missing=False)
    assert data == {}


def test_set__does_not_create_filter_match_when_disabled():
    data = {"a": {"b": [{"id": "1", "c": 10}]}}
    dictwalk.set(data, "a.b[?.id==3].c", 99, create_filter_match=False)
    assert data == {"a": {"b": [{"id": "1", "c": 10}]}}


def test_set__does_not_overwrite_incompatible_when_disabled():
    data = {"a": 1}
    dictwalk.set(data, "a.b", 2, overwrite_incompatible=False)
    assert data == {"a": 1}
