from collections.abc import Iterator
from typing import Any

import pytest


def _load_rust_backend() -> Any | None:
    try:
        from dictwalk import _dictwalk_rs  # type: ignore[attr-defined]
    except Exception:
        return None

    candidate = getattr(_dictwalk_rs, "dictwalk", _dictwalk_rs)
    required_methods = ("get", "exists", "set", "unset", "run_filter_function")
    if all(hasattr(candidate, method_name) for method_name in required_methods):
        return candidate
    return None


def _load_test_backend() -> Any:
    rust_backend = _load_rust_backend()
    if rust_backend is None:
        raise RuntimeError(
            "Rust backend is required for tests but unavailable. "
            "Build/install dictwalk._dictwalk_rs first."
        )
    return rust_backend


_BACKEND = _load_test_backend()


@pytest.fixture(scope="session")
def backend() -> Any:
    return _BACKEND


@pytest.fixture
def backend_name() -> str:
    return "rust"


@pytest.fixture(autouse=True)
def _inject_backend_dictwalk(
    request: pytest.FixtureRequest, backend: Any
) -> Iterator[None]:
    module = request.module

    if module is None or not hasattr(module, "dictwalk"):
        yield
        return

    with pytest.MonkeyPatch.context() as monkeypatch:
        monkeypatch.setattr(module, "dictwalk", backend, raising=False)
        yield
