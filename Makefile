.PHONY: rust-build rust-build-release rust-fmt rust-clippy rust-check rust-test rust-ci test lint type deptry precommit build release ci

rust-build:
	uv run --with maturin maturin develop --manifest-path rust/Cargo.toml --release

rust-build-release:
	uv run --with maturin maturin build --manifest-path rust/Cargo.toml --release -o rust/target/wheels

rust-fmt:
	cargo fmt --manifest-path rust/Cargo.toml --all -- --check

rust-clippy:
	cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::needless_borrow -A clippy::manual_strip

rust-check:
	cargo check --manifest-path rust/Cargo.toml

rust-test:
	cargo test --manifest-path rust/Cargo.toml

rust-ci: rust-fmt rust-clippy rust-check rust-test

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

release:
	@test -n "$$PYPI_TOKEN" || (echo "PYPI_TOKEN is not set"; exit 1)
	uv publish --token "$$PYPI_TOKEN"

ci: rust-ci
	uv run tox -e py310,py311,py312,py313,py314,lint,type,deptry
