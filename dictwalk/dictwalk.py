from typing import Any


def _load_rust_backend() -> Any:
    try:
        from . import _dictwalk_rs  # type: ignore[attr-defined]
    except Exception as ex:
        raise RuntimeError(
            "Rust backend is required but unavailable. "
            "Build/install the extension (dictwalk._dictwalk_rs)."
        ) from ex

    backend = getattr(_dictwalk_rs, "dictwalk", _dictwalk_rs)
    required_methods = ("get", "exists", "set", "unset", "run_filter_function")
    if not all(hasattr(backend, method_name) for method_name in required_methods):
        raise RuntimeError(
            "Rust backend module does not expose required DictWalk methods."
        )
    return backend


dictwalk = _load_rust_backend()
DictWalk = type(dictwalk)


def register_path_filter(name: str, path_filter: Any) -> Any:
    return dictwalk.register_path_filter(name, path_filter)


def run_filter_function(path_filter: str, value: Any) -> Any:
    return dictwalk.run_filter_function(path_filter, value)
