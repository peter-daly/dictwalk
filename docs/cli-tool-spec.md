# dictwalk CLI Spec (Separate Repo)

## Goal

Build a standalone CLI that reads structured input, applies `dictwalk` operations, and writes results to stdout or back to file.

This spec is for a separate repository. Do not implement in this repo.

## Scope

- Commands: `get`, `set`, `unset`
- Input formats (v1): JSON, YAML
- Input sources: file path or stdin stream
- Output: stdout by default
- In-place edit: only when explicit `--in-place` is provided

Out of scope (v1):
- XML
- TOML
- Batch/multi-file execution in one command

## Binary and Package

- Package name: `dictwalk-cli`
- Console entrypoint: `dictwalk`
- Runtime Python dependencies:
  - `dictwalk` (latest stable)
  - `PyYAML` (for YAML parse/dump support)
- Python standard library usage:
  - `json`
  - `argparse` (if chosen as CLI framework)

Example `pyproject.toml` snippet:

```toml
[project]
name = "dictwalk-cli"
dependencies = [
  "dictwalk",
  "PyYAML",
]

[project.scripts]
dictwalk = "dictwalk_cli.main:main"
```

## Command Model

Top-level:

```bash
dictwalk [global options] <command> [command options]
```

Commands:

- `get`: query value(s) from input document
- `set`: mutate at path with provided value
- `unset`: remove values at path

## Global Options

- `-i, --input <path>`
  - Optional. If omitted, read from stdin.
- `--input-format <json|yaml>`
  - Optional. Auto-detect from file extension when `--input` is provided.
  - For stdin, default to JSON unless explicitly set.
- `--output-format <json|yaml>`
  - Optional. Default: preserve detected input format.
- `--strict`
  - Optional. Default is non-strict.
  - Applies library strict behavior for all commands.
- `--pretty / --no-pretty`
  - Optional. Default `--pretty`.
- `-q, --quiet`
  - Suppress non-essential logs; only payload/errors.

## `get` Command

Usage:

```bash
dictwalk get <path> [--default <value>] [--default-type <json|string>]
```

Behavior:

- Executes `dictwalk.get(data, path, default=?, strict=?)`.
- If multiple matches occur, emit an array in selected output format.
- `--default` is optional.
  - `--default-type json` (default): parse default as JSON scalar/object/array.
  - `--default-type string`: treat as raw string.

## `set` Command

Usage:

```bash
dictwalk set <path> --value <value> [--value-type <json|string>] [--in-place]
```

Behavior:

- Executes `dictwalk.set(data, path, value, strict=?)`.
- Default value parsing is typed literal:
  - `--value-type json` (default): parse as JSON literal.
  - `--value-type string`: raw text.
- Output rules:
  - Without `--in-place`: print full mutated document to stdout.
  - With `--in-place`: write full mutated document back to `--input` file and print nothing.
- Validation:
  - `--in-place` requires `--input` file path (stdin is invalid for in-place).

## `unset` Command

Usage:

```bash
dictwalk unset <path> [--in-place]
```

Behavior:

- Executes `dictwalk.unset(data, path, strict=?)`.
- Output rules match `set`.
- Validation: `--in-place` requires `--input`.

## I/O and Serialization Rules

- Parse input into Python dict/list structure.
- JSON parser: standard library `json`.
- YAML parser: `yaml.safe_load`.
- YAML dump should preserve data semantics; exact comment/format preservation is not required.
- When output format is not explicitly set:
  - Preserve detected input format.
- Pretty output defaults:
  - JSON: 2-space indent, stable key order off.
  - YAML: block style, human-readable defaults.

## Error Handling and Exit Codes

Exit codes:

- `0`: success
- `2`: CLI usage/argument error
- `3`: input read/parse error
- `4`: dictwalk parse/resolution error
- `5`: output serialization or write error

Error policy:

- Print concise actionable message to stderr.
- Include exception class in `--quiet` off mode if useful.
- Never emit partial document on write failure.

## Safety Rules

- In-place writes must be atomic:
  - Write temp file in same directory, then replace target.
- Refuse in-place on stdin.
- Refuse in-place if input path does not exist or is not a regular file.

## Suggested CLI Library and Layout

- CLI framework: `argparse` (simple, zero extra dependency) or `typer`.
- Recommended structure:
  - `dictwalk_cli/main.py` (entrypoint)
  - `dictwalk_cli/io.py` (load/dump logic)
  - `dictwalk_cli/commands.py` (command handlers)
  - `dictwalk_cli/errors.py` (mapped exit codes)

## Acceptance Test Matrix

Core:

1. `get` JSON from file, scalar match.
2. `get` YAML from stdin with `--input-format yaml`.
3. `get` multi-match returns array.
4. `get --default` returns default when non-strict miss.
5. `get --strict` on missing path exits code `4`.
6. `set` typed numeric value updates output type as number.
7. `set --value-type string` stores raw string.
8. `set --in-place` updates file atomically, no stdout.
9. `set --in-place` with stdin fails code `2`.
10. `unset` removes target and prints mutated doc.
11. `unset --in-place` updates file.
12. Invalid JSON input exits code `3`.
13. Invalid YAML input exits code `3`.
14. Unsupported format flag exits code `2`.
15. Output write failure exits code `5`.

## Reference UX Examples

```bash
# Read from file
dictwalk --input data.json get "a.users[].name"

# Read from stdin YAML
cat data.yaml | dictwalk --input-format yaml get "a.items[0]"

# Set typed number
dictwalk --input data.json set "a.count" --value 42

# Set raw string
dictwalk --input data.json set "a.code" --value "0012" --value-type string

# In-place mutation
dictwalk --input data.yaml unset "a.debug" --in-place
```

## Defaults Locked

- Subcommand CLI (`get`, `set`, `unset`)
- Read + mutate in MVP
- Formats: JSON + YAML
- Default output format: preserve input format
- `get` multi-match output: array
- `set` value mode: typed literal by default, raw-string option
- Strict mode default: off
- In-place default: off, explicit only
