from typing import Any

from .dictwalk import DictWalk
from .backend import resolve_backend

backend_name, dictwalk = resolve_backend()


def run_filter_function(path_filter: str, value: Any) -> Any:
    return dictwalk.run_filter_function(path_filter, value)


__all__ = ["dictwalk", "DictWalk", "backend_name", "run_filter_function"]
