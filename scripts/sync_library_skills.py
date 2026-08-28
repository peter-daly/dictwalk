"""Keep packaged Library Skills synchronized with their authored sources."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
SOURCE = PROJECT_ROOT / ".library-skills"
DESTINATION = PROJECT_ROOT / "dictwalk" / ".agents" / "skills"


def _files(root: Path) -> dict[Path, bytes]:
    return {
        path.relative_to(root): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


def check() -> bool:
    return SOURCE.is_dir() and DESTINATION.is_dir() and _files(SOURCE) == _files(DESTINATION)


def sync() -> None:
    if DESTINATION.exists():
        shutil.rmtree(DESTINATION)
    shutil.copytree(SOURCE, DESTINATION)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail when the packaged copy differs from .library-skills",
    )
    args = parser.parse_args()

    if args.check:
        if not check():
            raise SystemExit(
                "Packaged Library Skills are stale. "
                "Run `uv run python scripts/sync_library_skills.py`."
            )
        print("Packaged Library Skills are synchronized.")
        return

    sync()
    print(f"Synchronized {SOURCE} to {DESTINATION}.")


if __name__ == "__main__":
    main()
