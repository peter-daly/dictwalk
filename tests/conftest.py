import os
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


def _discover_backends() -> list[tuple[str, Any]]:
    rust_backend = _load_rust_backend()
    if rust_backend is None:
        raise RuntimeError(
            "Rust backend is required for tests but unavailable. "
            "Build/install dictwalk._dictwalk_rs first."
        )
    available: dict[str, Any] = {"rust": rust_backend}

    requested = os.getenv("DICTWALK_TEST_BACKENDS")
    if requested is None:
        return list(available.items())

    requested_ids = [item.strip() for item in requested.split(",") if item.strip()]
    missing = [
        backend_id for backend_id in requested_ids if backend_id not in available
    ]
    if missing:
        raise RuntimeError(
            "Requested test backends are unavailable: "
            f"{', '.join(missing)}. Available: {', '.join(available)}"
        )

    return [(backend_id, available[backend_id]) for backend_id in requested_ids]


_BACKENDS = _discover_backends()


@pytest.fixture(scope="session", params=_BACKENDS, ids=lambda backend: backend[0])
def backend_pair(request: pytest.FixtureRequest) -> tuple[str, Any]:
    return request.param


@pytest.fixture
def backend_name(backend_pair: tuple[str, Any]) -> str:
    return backend_pair[0]


@pytest.fixture(autouse=True)
def _inject_backend_dictwalk(
    request: pytest.FixtureRequest, backend_pair: tuple[str, Any]
) -> Iterator[None]:
    _, backend = backend_pair
    module = request.module

    if module is None or not hasattr(module, "dictwalk"):
        yield
        return

    with pytest.MonkeyPatch.context() as monkeypatch:
        monkeypatch.setattr(module, "dictwalk", backend, raising=False)
        yield
