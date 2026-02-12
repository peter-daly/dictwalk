.PHONY: rust-build rust-build-release test lint type deptry precommit build benchmark release ci

BENCHMARK_ARGS ?=

rust-build:
	uv run --with maturin maturin develop --manifest-path rust/Cargo.toml --release

rust-build-release:
	uv run --with maturin maturin build --manifest-path rust/Cargo.toml --release -o rust/target/wheels

test: rust-build
	uv run tox -e py310,py311,py312,py313,py314

lint:
	uv run tox -e lint

type:
	uv run tox -e type

deptry:
	uv run tox -e deptry

precommit:
	uv run pre-commit run --all-files

build:
	uv build

benchmark:
	$(MAKE) rust-build-release
	uv run python scripts/benchmark_backends.py $(BENCHMARK_ARGS)

release:
	@test -n "$$PYPI_TOKEN" || (echo "PYPI_TOKEN is not set"; exit 1)
	uv publish --token "$$PYPI_TOKEN"

ci:
	uv run tox -e py310,py311,py312,py313,py314,lint,type,deptry
