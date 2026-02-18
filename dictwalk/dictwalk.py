from typing import TYPE_CHECKING, Any, Callable, Protocol, TypeVar, cast, overload

TData = TypeVar("TData")
TDefault = TypeVar("TDefault")


class DictWalkProtocol(Protocol):
    @overload
    def get(
        self, data: Any, path: str, default: None = None, *, strict: bool = False
    ) -> Any | None: ...

    @overload
    def get(
        self, data: Any, path: str, default: TDefault, *, strict: bool = False
    ) -> Any | TDefault: ...

    def exists(self, data: Any, path: str, *, strict: bool = False) -> bool: ...

    def set(
        self,
        data: TData,
        path: str,
        value: Any,
        *,
        strict: bool = False,
        create_missing: bool = True,
        create_filter_match: bool = True,
        overwrite_incompatible: bool = True,
    ) -> TData: ...

    def unset(self, data: TData, path: str, *, strict: bool = False) -> TData: ...

    def run_filter_function(self, path_filter: str, value: Any) -> Any: ...

    def register_path_filter(
        self, name: str, path_filter: Callable[[Any], Any]
    ) -> None: ...

    def get_path_filter(self, name: str) -> Callable[[Any], Any]: ...


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


_backend = _load_rust_backend()
dictwalk: DictWalkProtocol = cast(DictWalkProtocol, _backend)

if TYPE_CHECKING:
    DictWalk = DictWalkProtocol
else:
    DictWalk = type(_backend)


def register_path_filter(name: str, path_filter: Callable[[Any], Any]) -> None:
    return dictwalk.register_path_filter(name, path_filter)


def run_filter_function(path_filter: str, value: Any) -> Any:
    return dictwalk.run_filter_function(path_filter, value)
