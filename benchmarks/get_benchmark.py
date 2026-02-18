from benchbro import Case
from dictwalk import dictwalk

get_case = Case(
    name="get",
    case_type="cpu",
    metric_type="time",
    tags=["dictwalk", "get"],
    warmup_iterations=5,
    min_iterations=50,
    repeats=10,
)


@get_case.benchmark()
def simple_path():
    data = {"a": {"b": {"c": 1}}}
    path = "a.b.c"

    dictwalk.get(data, path)


@get_case.benchmark()
def list_path_with_predicate_and_chained_filters():
    data = {
        "a": {
            "b": [
                {"id": 1, "c": 1},
                {"id": 2, "c": 2},
                {"id": 3, "c": 3},
                {"id": 4, "c": 4},
                {"id": 5, "c": 5},
                {"id": 6, "c": 6},
            ]
        }
    }
    path = "a.b[?.id==$even].c[]|$add(2)[]|$double[]|$pow(2)[]|$sum"

    dictwalk.get(data, path)


@get_case.benchmark()
def deep_nested_path():

    data = {"a": {"b": {"c": {"d": {"e": {"f": {"g": {"h": {"i": {"j": 1}}}}}}}}}}
    path = "a.b.c.d.e.f.g.h.i.j | $pow(2) | $add(3) | $double | $rpow(2)"

    dictwalk.get(data, path)
