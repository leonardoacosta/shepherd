from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.socket_api_reference_check import collect_errors, load_documented_methods, load_schema_methods


def sample_schema(methods: list[str]) -> dict:
    return {
        "schemas": {
            "request": {
                "oneOf": [
                    {
                        "properties": {
                            "method": {
                                "const": method,
                            }
                        }
                    }
                    for method in methods
                ]
            }
        }
    }


class SocketApiReferenceCheckTests(unittest.TestCase):
    def write_fixture(self, methods: list[str], doc_text: str) -> tuple[Path, Path]:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        schema_path = root / "schema.json"
        doc_path = root / "socket-api.mdx"
        schema_path.write_text(json.dumps(sample_schema(methods)))
        doc_path.write_text(doc_text)
        return schema_path, doc_path

    def test_load_schema_methods_reads_request_method_consts(self) -> None:
        schema_path, _ = self.write_fixture(
            ["workspace.list", "server.stop"],
            "",
        )
        self.assertEqual(load_schema_methods(schema_path), ["workspace.list", "server.stop"])

    def test_load_documented_methods_finds_backticked_methods(self) -> None:
        _, doc_path = self.write_fixture(
            [],
            '`workspace.list` and "server.stop"',
        )
        self.assertEqual(
            load_documented_methods(["workspace.list", "server.stop"], doc_path),
            {"workspace.list", "server.stop"},
        )

    def test_collect_errors_reports_missing_methods(self) -> None:
        schema_path, doc_path = self.write_fixture(
            ["workspace.list", "server.live_handoff"],
            "`workspace.list`",
        )
        errors = collect_errors(schema_path, doc_path)
        self.assertEqual(len(errors), 1)
        self.assertIn("server.live_handoff", errors[0])
        self.assertIn("socket-api.mdx", errors[0])

    def test_collect_errors_skips_internal_stream_methods(self) -> None:
        schema_path, doc_path = self.write_fixture(
            ["pane.graphics.stream_open", "pane.graphics.stream_close", "pane.read"],
            "`pane.read`",
        )
        self.assertEqual(collect_errors(schema_path, doc_path), [])


if __name__ == "__main__":
    unittest.main()
