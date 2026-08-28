.PHONY: rust-build rust-build-release rust-fmt rust-clippy rust-check rust-test rust-ci test lint type deptry precommit build release ci benchmark library-skills-sync library-skills-check

library-skills-sync:
	uv run python scripts/sync_library_skills.py

library-skills-check:
	uv run python scripts/sync_library_skills.py --check

rust-build: library-skills-sync
	uv run --with maturin maturin develop --manifest-path rust/Cargo.toml --release

rust-build-release: library-skills-sync
	uv run --with maturin maturin build --manifest-path rust/Cargo.toml --release -o rust/target/wheels

rust-fmt:
	cargo fmt --manifest-path rust/Cargo.toml --all -- --check

rust-clippy:
	cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings -A clippy::too_many_arguments -A clippy::needless_borrow -A clippy::manual_strip -A clippy::useless_conversion

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

benchmark:
	uv run tox -e benchbro

build: library-skills-sync
	uv build

release:
	@test -n "$$PYPI_TOKEN" || (echo "PYPI_TOKEN is not set"; exit 1)
	uv publish --token "$$PYPI_TOKEN"

ci: library-skills-check rust-ci
	uv run tox -e py310,py311,py312,py313,py314,lint,type,deptry,benchbro
