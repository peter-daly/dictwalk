import os
from typing import Any

BACKEND_ENV_VAR = "DICTWALK_BACKEND"
_VALID_BACKENDS = {"rust", "auto"}


def _load_rust_backend() -> Any | None:
    try:
        from . import _dictwalk_rs  # type: ignore[attr-defined]
    except Exception:
        return None

    candidate = getattr(_dictwalk_rs, "dictwalk", _dictwalk_rs)
    required_methods = ("get", "exists", "set", "unset", "run_filter_function")
    if all(hasattr(candidate, method_name) for method_name in required_methods):
        return candidate
    return None


def resolve_backend(preference: str | None = None) -> tuple[str, Any]:
    requested = (preference or os.getenv(BACKEND_ENV_VAR, "rust")).strip().lower()

    if requested not in _VALID_BACKENDS:
        valid_options = ", ".join(sorted(_VALID_BACKENDS))
        raise ValueError(
            f"Invalid backend '{requested}'. Expected one of: {valid_options}."
        )

    rust_backend = _load_rust_backend()

    if rust_backend is None:
        raise RuntimeError(
            "Rust backend is required but unavailable. "
            "Build/install the extension (dictwalk._dictwalk_rs)."
        )
    return "rust", rust_backend
