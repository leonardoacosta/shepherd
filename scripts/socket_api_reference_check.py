from __future__ import annotations

import json
import sys
from pathlib import Path

SCHEMA_PATH = Path("docs/next/api/shepherd-api.schema.json")
DOC_PATH = Path("docs/next/website/src/content/docs/socket-api.mdx")
SKIPPED_METHODS = {
    "pane.graphics.stream_open",
    "pane.graphics.stream_close",
}


def load_schema_methods(schema_path: Path = SCHEMA_PATH) -> list[str]:
    schema = json.loads(schema_path.read_text())
    variants = schema["schemas"]["request"]["oneOf"]
    methods: list[str] = []

    for variant in variants:
        const = variant.get("properties", {}).get("method", {}).get("const")
        if isinstance(const, str):
            methods.append(const)

    return methods


def load_documented_methods(
    schema_methods: list[str],
    doc_path: Path = DOC_PATH,
) -> set[str]:
    text = doc_path.read_text()
    return {
        method
        for method in schema_methods
        if f"`{method}`" in text or f'"{method}"' in text
    }


def collect_errors(
    schema_path: Path = SCHEMA_PATH,
    doc_path: Path = DOC_PATH,
) -> list[str]:
    schema_methods = load_schema_methods(schema_path)
    documented_methods = load_documented_methods(schema_methods, doc_path)

    missing = [
        method
        for method in schema_methods
        if method not in SKIPPED_METHODS and method not in documented_methods
    ]

    return [
        f"{method} missing from {doc_path}"
        for method in missing
    ]


def main() -> int:
    errors = collect_errors()
    if errors:
        print("error: socket api docs are missing methods from the generated schema", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
