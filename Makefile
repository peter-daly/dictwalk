.PHONY: test lint type deptry build release ci

test:
	uv run tox -e py310,py311,py312,py313,py314

lint:
	uv run tox -e lint

type:
	uv run tox -e type

deptry:
	uv run tox -e deptry

build:
	uv build

release:
	@test -n "$$PYPI_TOKEN" || (echo "PYPI_TOKEN is not set"; exit 1)
	uv publish --token "$$PYPI_TOKEN"

ci:
	uv run tox -e py310,py311,py312,py313,py314,lint,type,deptry
