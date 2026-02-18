from typing import Any

from .dictwalk import DictWalk, dictwalk


def run_filter_function(path_filter: str, value: Any) -> Any:
    return dictwalk.run_filter_function(path_filter, value)


__all__ = ["dictwalk", "DictWalk", "run_filter_function"]
