import importlib

import pytest

import dictwalk as dictwalk_pkg
from dictwalk import backend as backend_module


def test_resolve_backend_returns_rust_backend_when_requested(monkeypatch):
    rust_backend = object()
    monkeypatch.setattr(backend_module, "_load_rust_backend", lambda: rust_backend)

    backend_name, backend = backend_module.resolve_backend("rust")

    assert backend_name == "rust"
    assert backend is rust_backend


def test_resolve_backend_rejects_invalid_backend_name():
    with pytest.raises(ValueError):
        backend_module.resolve_backend("not-a-backend")


def test_resolve_backend_rejects_python_backend_request():
    with pytest.raises(ValueError):
        backend_module.resolve_backend("python")


def test_resolve_backend_raises_when_rust_requested_but_unavailable(monkeypatch):
    monkeypatch.setattr(backend_module, "_load_rust_backend", lambda: None)

    with pytest.raises(RuntimeError):
        backend_module.resolve_backend("rust")


def test_package_defaults_to_rust_backend_when_available(monkeypatch):
    rust_backend = object()

    monkeypatch.setattr(backend_module, "_load_rust_backend", lambda: rust_backend)
    monkeypatch.delenv(backend_module.BACKEND_ENV_VAR, raising=False)

    importlib.reload(dictwalk_pkg)

    assert dictwalk_pkg.backend_name == "rust"
    assert dictwalk_pkg.dictwalk is rust_backend


def test_package_raises_when_rust_unavailable(monkeypatch):
    monkeypatch.setattr(backend_module, "_load_rust_backend", lambda: None)
    monkeypatch.delenv(backend_module.BACKEND_ENV_VAR, raising=False)

    with pytest.raises(RuntimeError):
        importlib.reload(dictwalk_pkg)
