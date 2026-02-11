from dictwalk import dictwalk

def test_get__returns_root_entity_for_dot_path():
    data = {'a': {'b': {'c': 1}}}
    path = '.'
    default = None
    expected = {'a': {'b': {'c': 1}}}

    assert dictwalk.get(data, path, default=default) == expected

def test_get__returns_root_entity_field_for_root_token_at_start():
    data = {'a': {'b': {'c': 1}}, 'x': 2}
    path = '$$root.x'
    default = None
    expected = 2

    assert dictwalk.get(data, path, default=default) == expected

def test_get__returns_root_entity_field_for_root_token_mid_path():
    data = {'a': {'b': {'c': 1}}, 'x': 2}
    path = 'a.b.$$root.x'
    default = None
    expected = 2

    assert dictwalk.get(data, path, default=default) == expected

def test_get__nested_scalar():
    data = {'a': {'b': {'c': 1}}}
    path = 'a.b.c'
    default = None
    expected = 1

    assert dictwalk.get(data, path, default=default) == expected

def test_get__list_map_values():
    data = {'a': {'b': [{'c': 1}, {'c': 2}]}}
    path = 'a.b.c[]'
    default = None
    expected = [1, 2]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map():
    data = {'a': {'b': [{'id': '1', 'c': 10}, {'id': '2', 'c': 20}]}}
    path = 'a.b[?id==1].c[]'
    default = None
    expected = [10]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_registered_path_filter():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}]}}
    path = 'a.b[?id==$even].c[]'
    default = None
    expected = [20]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_piped_registered_path_filters():
    data = {'a': {'b': [{'id': '1', 'c': 10}, {'id': '2', 'c': 20}, {'id': '3', 'c': 30}]}}
    path = 'a.b[?id==$int|$even].c[]'
    default = None
    expected = [20]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_registered_path_filter_with_args():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}]}}
    path = 'a.b[?id==$gt(1)].c[]'
    default = None
    expected = [20, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_greater_than_operator():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}]}}
    path = 'a.b[?id>1].c[]'
    default = None
    expected = [20, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_less_than_or_equal_operator():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}]}}
    path = 'a.b[?id<=2].c[]'
    default = None
    expected = [10, 20]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_not_equal_operator():
    data = {'a': {'b': [{'id': '1', 'c': 10}, {'id': '2', 'c': 20}, {'id': '3', 'c': 30}]}}
    path = 'a.b[?id!=2].c[]'
    default = None
    expected = [10, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_namespaced_filter():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}]}}
    path = 'a.b[?id==$gt(1)].c[]'
    default = None
    expected = [20, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_boolean_and():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}, {'id': 4, 'c': 40}]}}
    path = 'a.b[?id==$gt(1)&&$lt(4)].c[]'
    default = None
    expected = [20, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_boolean_or():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}, {'id': 4, 'c': 40}]}}
    path = 'a.b[?id==$lt(2)||$gt(3)].c[]'
    default = None
    expected = [10, 40]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_boolean_not():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}, {'id': 4, 'c': 40}]}}
    path = 'a.b[?id==!$even].c[]'
    default = None
    expected = [10, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_and_map_using_boolean_grouping():
    data = {'a': {'b': [{'id': 1, 'c': 10}, {'id': 2, 'c': 20}, {'id': 3, 'c': 30}, {'id': 4, 'c': 40}]}}
    path = 'a.b[?id==($lt(2)||$gt(3))&&$odd].c[]'
    default = None
    expected = [10]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_list_index():
    data = {'a': {'b': [{'c': 10}, {'c': 20}, {'c': 30}]}}
    path = 'a.b[0].c'
    default = None
    expected = 10

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_negative_list_index():
    data = {'a': {'b': [{'c': 10}, {'c': 20}, {'c': 30}]}}
    path = 'a.b[-1].c'
    default = None
    expected = 30

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_list_slice():
    data = {'a': {'b': [{'c': 10}, {'c': 20}, {'c': 30}, {'c': 40}]}}
    path = 'a.b[1:3].c[]'
    default = None
    expected = [20, 30]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_single_layer_wildcard():
    data = {'a': {'u1': {'id': 1}, 'u2': {'id': 2}}}
    path = 'a.*.id'
    default = None
    expected = [1, 2]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_deep_wildcard():
    data = {'a': {'group1': {'u1': {'id': 1}}, 'group2': {'nested': {'u2': {'id': 2}}}}}
    path = 'a.**.id'
    default = None
    expected = [1, 2]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_output_transform_pipeline():
    data = {'a': {'b': {'c': 2}}}
    path = 'a.b.c|$double|$string'
    default = None
    expected = '4'

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_output_transform_args():
    data = {'a': {'b': {'c': 2}}}
    path = 'a.b.c|$add(3)'
    default = None
    expected = 5

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_path_value_with_predicate_filter_and_output_transform():
    data = {'a': {'b': [{'id': '1', 'c': 10}, {'id': '2', 'c': 20}, {'id': '3', 'c': 30}]}}
    path = 'a.b[?id==$int|$even].c[]|$string'
    default = None
    expected = '[20]'

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_list_piped_out_to_filters():
    data = {'a': {'b': [1, 2, 3, 4, 5]}}
    path = 'a.b|$max'
    default = None
    expected = 5

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_list_piped_out_to_filters_that_map_values():
    data = {'a': {'b': ['foo', 'bar', 'hello']}}
    path = 'a.b|$len[]'
    default = None
    expected = [3, 3, 5]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_list_mapped_through_output_filter():
    data = {'a': {'b': [1, 2, 3, 4, 5]}}
    path = 'a.b|$double[]'
    default = None
    expected = [2, 4, 6, 8, 10]

    assert dictwalk.get(data, path, default=default) == expected

def test_get__get_list_mapped_then_reduced_through_output_filters():
    data = {'a': {'b': [1, 2, 3, 4, 5]}}
    path = 'a.b|$double[]|$max'
    default = None
    expected = 10

    assert dictwalk.get(data, path, default=default) == expected

def test_get__filter_list_elements_using_current_item_predicate_root():
    data = {'a': {'b': ['hello', 'world', 'foo', 'bar']}}
    path = 'a.b[?.|$len>3]'
    default = None
    expected = ['hello', 'world']

    assert dictwalk.get(data, path, default=default) == expected

def test_get__missing_returns_default():
    data = {'a': {'b': {'c': 1}}}
    path = 'a.b.missing'
    default = 'not-found'
    expected = 'not-found'

    assert dictwalk.get(data, path, default=default) == expected

def test_get__type_mismatch_returns_default():
    data = {'a': {'b': {'c': 1}}}
    path = 'a.b.c[]'
    default = None
    expected = None

    assert dictwalk.get(data, path, default=default) == expected

def test_get__nested_path_exists():
    data = {'a': {'b': {'c': 1}}}
    path = 'a.b.c'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__path_exists_with_root_token_mid_path():
    data = {'a': {'b': {'c': 1}}, 'x': 2}
    path = 'a.b.$$root.x'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__mapped_path_exists():
    data = {'a': {'b': [{'c': 1}, {'c': 2}]}}
    path = 'a.b.c[]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filtered_path_exists():
    data = {'a': {'b': [{'id': '1'}, {'id': '2'}]}}
    path = 'a.b[?id==1]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_using_registered_path_filter():
    data = {'a': {'b': [{'id': 1}, {'id': 2}]}}
    path = 'a.b[?id==$even]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_using_piped_registered_path_filters():
    data = {'a': {'b': [{'id': '1'}, {'id': '2'}]}}
    path = 'a.b[?id==$int|$even]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_using_registered_path_filter_with_args():
    data = {'a': {'b': [{'id': 1}, {'id': 2}]}}
    path = 'a.b[?id==$gt(1)]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_using_greater_than_or_equal_operator():
    data = {'a': {'b': [{'id': 1}, {'id': 2}]}}
    path = 'a.b[?id>=2]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_even_when_operator_filter_has_no_matches():
    data = {'a': {'b': [{'id': 1}, {'id': 2}]}}
    path = 'a.b[?id<1]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_using_boolean_and():
    data = {'a': {'b': [{'id': 1}, {'id': 2}, {'id': 3}]}}
    path = 'a.b[?id==$gt(1)&&$odd]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__filter_path_exists_using_boolean_not():
    data = {'a': {'b': [{'id': 1}, {'id': 2}, {'id': 3}]}}
    path = 'a.b[?id==!$odd]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__path_exists_with_list_index():
    data = {'a': {'b': [{'id': 1}, {'id': 2}]}}
    path = 'a.b[1].id'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__path_exists_with_list_slice():
    data = {'a': {'b': [{'id': 1}, {'id': 2}, {'id': 3}]}}
    path = 'a.b[1:3].id[]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__path_exists_with_single_layer_wildcard():
    data = {'a': {'u1': {'id': 1}, 'u2': {'id': 2}}}
    path = 'a.*.id'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__path_exists_with_deep_wildcard():
    data = {'a': {'group1': {'u1': {'id': 1}}, 'group2': {'nested': {'u2': {'id': 2}}}}}
    path = 'a.**.id[]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__path_exists_for_filter_list_elements_using_current_item_predicate_root():
    data = {'a': {'b': ['hello', 'world', 'foo', 'bar']}}
    path = 'a.b[?.|$len>3]'
    expected = True

    assert dictwalk.exists(data, path) is expected

def test_get__missing_path():
    data = {'a': {'b': {'c': 1}}}
    path = 'a.b.x'
    expected = False

    assert dictwalk.exists(data, path) is expected

def test_get__type_mismatch():
    data = {'a': {'b': {'c': 1}}}
    path = 'a.b.c[]'
    expected = False

    assert dictwalk.exists(data, path) is expected
