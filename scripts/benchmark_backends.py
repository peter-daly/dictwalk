from __future__ import annotations

import argparse
import importlib.util
import statistics
import sys
import sysconfig
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from types import ModuleType
from typing import Any, Callable

from dictwalk.dictwalk import dictwalk as python_backend


Backend = Any
Workload = Callable[[Backend], int]


@dataclass(frozen=True)
class Case:
    name: str
    kind: str
    workload: Workload


@dataclass(frozen=True)
class CaseResult:
    name: str
    kind: str
    backend: str
    calls: int
    mean_seconds: float
    median_seconds: float
    stdev_seconds: float


def _make_read_data(size: int = 500) -> dict[str, Any]:
    items = [
        {
            "id": i,
            "value": i * 3,
            "meta": {"enabled": i % 2 == 0, "score": i % 13},
        }
        for i in range(size)
    ]
    return {"root": {"items": items}}


def _make_write_data(size: int = 500) -> dict[str, Any]:
    items: list[dict[str, Any]] = []
    for i in range(size):
        item: dict[str, Any] = {
            "id": i,
            "value": i,
            "flags": {"enabled": False, "debug": True},
        }
        for j in range(40):
            item[f"scratch{j}"] = j
        items.append(item)
    return {"root": {"items": items}}


def _make_deep_complex_read_data(size: int = 1200) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    for i in range(size):
        records.append(
            {
                "id": i,
                "active": i % 3 != 0,
                "category": f"group-{i % 9}",
                "metrics": {
                    "raw": (i * 1.25) - 400,
                    "score": i % 97,
                    "weight": (i % 10) + 1,
                },
            }
        )

    return {
        "l1": {
            "l2": {
                "l3": {
                    "l4": {
                        "l5": {
                            "l6": {
                                "l7": {
                                    "l8": {
                                        "l9": {
                                            "l10": {
                                                "records": records,
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }


def case_parse_heavy_get(backend: Backend) -> int:
    data = _make_read_data()
    paths = [f"root.items[{i}].meta.enabled" for i in range(500)]
    for path in paths:
        backend.get(data, path, default=None)
    return len(paths)


def case_read_heavy_get(backend: Backend) -> int:
    data = _make_read_data()
    path = "root.items[?id>=250].value[]"
    calls = 300
    for _ in range(calls):
        backend.get(data, path, default=None)
    return calls


def case_read_heavy_builtin_predicate_get(backend: Backend) -> int:
    data = _make_read_data()
    path = "root.items[?id==$even].value[]"
    calls = 300
    for _ in range(calls):
        backend.get(data, path, default=None)
    return calls


def case_transform_heavy_numeric_pipeline_get(backend: Backend) -> int:
    data = _make_read_data()
    path = "root.items[10].value|$double|$add(3)|$pow(2)|$sqrt|$int"
    calls = 20000
    for _ in range(calls):
        backend.get(data, path, default=None)
    return calls


def case_transform_heavy_string_pipeline_get(backend: Backend) -> int:
    data = _make_read_data()
    path = "root.items[10].id|$string|$replace('10','item-10')|$startswith('item-')"
    calls = 20000
    for _ in range(calls):
        backend.get(data, path, default=None)
    return calls


def case_deep_filter_pipeline_get(backend: Backend) -> int:
    data = _make_deep_complex_read_data()
    path = (
        "l1.l2.l3.l4.l5.l6.l7.l8.l9.l10.records"
        "[?id==$even&&$gt(200)&&$lt(1100)].metrics.raw[]"
        "|$abs[]|$mul(1.1)[]|$add(2)[]|$round(3)[]|$sorted|$unique|$sum"
    )
    calls = 120
    for _ in range(calls):
        backend.get(data, path, default=None)
    return calls


def case_write_heavy_set(backend: Backend) -> int:
    data = _make_write_data()
    paths = [f"root.items[?id>250].new_key_{i}" for i in range(40)]
    for i, path in enumerate(paths):
        backend.set(data, path, i)
    return len(paths)


def case_write_heavy_unset(backend: Backend) -> int:
    data = _make_write_data()
    paths = [f"root.items[?id>250].scratch{i}" for i in range(40)]
    for path in paths:
        backend.unset(data, path)
    return len(paths)


def _built_module_candidates() -> list[Path]:
    release = Path("rust/target/release")
    debug = Path("rust/target/debug")
    names = [
        "lib_dictwalk_rs.dylib",
        "lib_dictwalk_rs.so",
        "_dictwalk_rs.dll",
        "_dictwalk_rs.pyd",
    ]
    candidates: list[Path] = []
    for base in (release, debug):
        for name in names:
            candidates.append(base / name)
    return candidates


def _load_module_from_binary(path: Path) -> tuple[ModuleType, tempfile.TemporaryDirectory[str]]:
    suffix = sysconfig.get_config_var("EXT_SUFFIX")
    if not suffix:
        raise RuntimeError("Unable to determine Python extension suffix for this interpreter.")

    temp_dir = tempfile.TemporaryDirectory(prefix="dictwalk-rs-bench-")
    extension_path = Path(temp_dir.name) / f"_dictwalk_rs{suffix}"
    extension_path.write_bytes(path.read_bytes())

    spec = importlib.util.spec_from_file_location("_dictwalk_rs", extension_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Failed to create module spec for {extension_path}.")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module, temp_dir


def load_rust_backend() -> tuple[Backend, str]:
    module = None
    keepalive: tempfile.TemporaryDirectory[str] | None = None

    # Prefer a fresh on-disk release/debug artifact over importing the currently
    # installed extension module. This avoids accidentally benchmarking a local
    # editable debug build (e.g. from `maturin develop`).
    for candidate in _built_module_candidates():
        if not candidate.exists():
            continue
        loaded_module, keepalive = _load_module_from_binary(candidate)
        module = loaded_module
        break

    if module is None:
        try:
            from dictwalk import _dictwalk_rs as module  # type: ignore[attr-defined]
        except Exception:
            module = None

    if module is None:
        raise RuntimeError(
            "Rust backend is unavailable. Build it first with "
            "`uv run --with maturin maturin build --manifest-path rust/Cargo.toml --release -o rust/target/wheels`."
        )

    if keepalive is not None:
        # Keep temporary module path alive while benchmarks run.
        setattr(module, "_bench_tempdir_keepalive", keepalive)

    backend = getattr(module, "dictwalk", module)
    required = ("get", "exists", "set", "unset")
    if not all(hasattr(backend, name) for name in required):
        raise RuntimeError("Loaded Rust backend module does not expose DictWalk methods.")
    return backend, "rust"


def run_case(
    case: Case,
    backend_name: str,
    backend: Backend,
    *,
    repeats: int,
    warmup: int,
) -> CaseResult:
    for _ in range(warmup):
        case.workload(backend)

    timings: list[float] = []
    calls = 0
    for _ in range(repeats):
        start = time.perf_counter()
        calls = case.workload(backend)
        end = time.perf_counter()
        timings.append(end - start)

    stdev = statistics.stdev(timings) if len(timings) > 1 else 0.0
    return CaseResult(
        name=case.name,
        kind=case.kind,
        backend=backend_name,
        calls=calls,
        mean_seconds=statistics.mean(timings),
        median_seconds=statistics.median(timings),
        stdev_seconds=stdev,
    )


def print_results(results: list[CaseResult]) -> None:
    grouped: dict[str, list[CaseResult]] = {}
    for result in results:
        grouped.setdefault(result.name, []).append(result)

    print("\nDictWalk backend benchmark")
    print(f"Python: {sys.version.split()[0]}")
    print()

    for case_name, rows in grouped.items():
        rows = sorted(rows, key=lambda row: row.backend)
        kind = rows[0].kind
        print(f"[{kind}] {case_name}")
        print("backend  mean(ms)  median(ms)  stdev(ms)  calls  ops/s")
        for row in rows:
            mean_ms = row.mean_seconds * 1000
            median_ms = row.median_seconds * 1000
            stdev_ms = row.stdev_seconds * 1000
            ops_per_second = row.calls / row.mean_seconds if row.mean_seconds else 0.0
            print(
                f"{row.backend:7} {mean_ms:8.3f}  {median_ms:10.3f}  "
                f"{stdev_ms:8.3f}  {row.calls:5d}  {ops_per_second:7.1f}"
            )

        by_backend = {row.backend: row for row in rows}
        py_row = by_backend.get("python")
        rs_row = by_backend.get("rust")
        if py_row and rs_row and rs_row.mean_seconds > 0:
            speedup = py_row.mean_seconds / rs_row.mean_seconds
            print(f"speedup (rust vs python): {speedup:.2f}x")
        print()


def append_markdown_log(
    results: list[CaseResult],
    *,
    repeats: int,
    warmup: int,
    description: str,
    log_file: Path,
) -> None:
    grouped: dict[str, list[CaseResult]] = {}
    for result in results:
        grouped.setdefault(result.name, []).append(result)

    timestamp = datetime.now(timezone.utc).isoformat(timespec="seconds")
    is_new = not log_file.exists()
    log_file.parent.mkdir(parents=True, exist_ok=True)

    with log_file.open("a", encoding="utf-8") as handle:
        if is_new:
            handle.write("# Benchmarks\n\n")

        handle.write(f"## {timestamp}\n\n")
        if description.strip():
            handle.write(f"**Description:** {description.strip()}\n\n")
        handle.write(f"- Python: `{sys.version.split()[0]}`\n")
        handle.write(f"- Repeats: `{repeats}`\n")
        handle.write(f"- Warmup: `{warmup}`\n\n")

        for case_name, rows in grouped.items():
            rows = sorted(rows, key=lambda row: row.backend)
            kind = rows[0].kind
            handle.write(f"### {kind}: {case_name}\n\n")
            handle.write(
                "| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |\n"
            )
            handle.write("|---|---:|---:|---:|---:|---:|\n")
            for row in rows:
                mean_ms = row.mean_seconds * 1000
                median_ms = row.median_seconds * 1000
                stdev_ms = row.stdev_seconds * 1000
                ops_per_second = row.calls / row.mean_seconds if row.mean_seconds else 0.0
                handle.write(
                    f"| {row.backend} | {mean_ms:.3f} | {median_ms:.3f} | "
                    f"{stdev_ms:.3f} | {row.calls} | {ops_per_second:.1f} |\n"
                )

            by_backend = {row.backend: row for row in rows}
            py_row = by_backend.get("python")
            rs_row = by_backend.get("rust")
            if py_row and rs_row and rs_row.mean_seconds > 0:
                speedup = py_row.mean_seconds / rs_row.mean_seconds
                handle.write(
                    f"\nSpeedup (rust vs python): `{speedup:.2f}x`\n\n"
                )
            else:
                handle.write("\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Benchmark Python backend vs Rust backend for dictwalk."
    )
    parser.add_argument("--repeats", type=int, default=7, help="Timed repeats per case.")
    parser.add_argument(
        "--warmup", type=int, default=2, help="Warmup runs per case/backend."
    )
    parser.add_argument(
        "--description",
        default="",
        help="Optional summary of changes for this benchmark run log entry.",
    )
    parser.add_argument(
        "--log-file",
        default="benchmarks.md",
        help="Markdown file used to append benchmark run logs.",
    )
    parser.add_argument(
        "--no-log",
        action="store_true",
        help="Disable benchmark logging to markdown for this run.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    rust_backend, rust_name = load_rust_backend()
    backends: list[tuple[str, Backend]] = [
        ("python", python_backend),
        (rust_name, rust_backend),
    ]

    cases = [
        Case("get_unique_paths", "parse-heavy", case_parse_heavy_get),
        Case("get_repeated_filter_path", "read-heavy", case_read_heavy_get),
        Case(
            "get_repeated_builtin_predicate_filter_path",
            "read-heavy",
            case_read_heavy_builtin_predicate_get,
        ),
        Case(
            "get_numeric_output_pipeline",
            "transform-heavy",
            case_transform_heavy_numeric_pipeline_get,
        ),
        Case(
            "get_string_output_pipeline",
            "transform-heavy",
            case_transform_heavy_string_pipeline_get,
        ),
        Case(
            "get_deep_10_layer_filter_pipeline",
            "transform-heavy",
            case_deep_filter_pipeline_get,
        ),
        Case("set_filter_writes", "write-heavy", case_write_heavy_set),
        Case("unset_filter_writes", "write-heavy", case_write_heavy_unset),
    ]

    results: list[CaseResult] = []
    for case in cases:
        for backend_name, backend in backends:
            results.append(
                run_case(
                    case,
                    backend_name,
                    backend,
                    repeats=args.repeats,
                    warmup=args.warmup,
                )
            )

    print_results(results)
    if not args.no_log:
        append_markdown_log(
            results,
            repeats=args.repeats,
            warmup=args.warmup,
            description=args.description,
            log_file=Path(args.log_file),
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
