# Rust Backend Migration Plan

Goal: implement `dictwalk` core logic in Rust, usable from Python, while keeping the current pure-Python implementation for fallback and benchmarking.

## 1. Baseline and Parity

- [x] Keep current Python implementation untouched.
- [x] Treat existing tests as behavioral source of truth.
- [x] Add a backend-aware test harness to run the same assertions against `python` and `rust` backends.

## 2. Dual-Engine Python API

- [x] Keep public API unchanged: `get`, `exists`, `set`, `unset`.
- [x] Add backend selection (`DICTWALK_BACKEND=python|rust|auto`).
- [x] Default to Python until Rust reaches parity.

## 3. Rust Extension Scaffolding

- [x] Create Rust crate for dictwalk core.
- [x] Expose Python-callable API via PyO3.
- [x] Map Rust errors into existing Python error types.

## 4. Incremental Porting

- [x] Phase 1: parser + `get`.
- [x] Phase 1a: Rust parser + Rust `get` traversal for root/get/map/wildcard/deep-wildcard tokens.
- [x] Phase 1b: Replace delegated Python token resolution in Rust `get` (index/slice/filter) with native Rust implementations.
- [x] Phase 2: `exists`.
- [x] Phase 3: `set` + `unset` (in-place mutation parity).
- [x] Phase 3a: native Rust `set` traversal/mutation.
- [x] Phase 3b: native Rust `unset` traversal/mutation.
- [x] After each phase, run full parity tests against both backends.

## 5. Filters and Custom Filters

- [x] Phase 5a: compile filter matcher once per token in Rust (avoid per-item expression resolution).
- [x] Phase 5b: native Rust built-in filter pipeline for common built-ins with Python fallback for unsupported/custom filters.
- [x] Phase 5c: expand native built-ins batch (numeric/string/collection predicates) and benchmark transform-heavy paths.
- [x] Phase 5d: add explicit backend parity tests for migrated built-ins (get/set + predicates + transforms).
- [x] Port built-in filters to Rust.
- [x] Keep custom Python filter registration out of scope for Rust backend.
- [x] In Rust backend mode, reject custom filter registration/use with a clear error.
- [ ] (Optional later) remove custom filter registration support from public API.

## 6. Packaging and Build

- [x] Re-introduce mixed Rust+Python build tooling with `maturin`.
- [x] Keep pure-Python code shipped for fallback/benchmarking.
- [x] Ensure wheel/sdist build for supported Python versions.

## 7. Benchmarking

- [x] Add benchmark suite comparing Python vs Rust for parse-heavy cases.
- [x] Add benchmark suite comparing Python vs Rust for read-heavy cases.
- [x] Add benchmark suite comparing Python vs Rust for write/unset-heavy cases.
- [x] Report both performance and behavior parity.

## 8. CI/CD

- [x] Add Rust toolchain setup in CI.
- [x] Add Rust build/test jobs.
- [x] Add backend parity test jobs.
- [x] Keep existing lint/type/deptry/test jobs.

## 9. Rollout

- [ ] Release with Rust backend optional.
- [x] Switch default to Rust only after parity + benchmark confidence.

## Open Decisions

- [x] When to make Rust the default backend?
