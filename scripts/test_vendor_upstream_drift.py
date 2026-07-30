from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent
SCRIPT_PATH = PROJECT_ROOT / "scripts" / "vendor_upstream_drift.py"


def load_module():
    if not SCRIPT_PATH.exists():
        raise AssertionError(f"missing drift check script at {SCRIPT_PATH}")
    spec = importlib.util.spec_from_file_location("vendor_upstream_drift", SCRIPT_PATH)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class VendorUpstreamDriftTests(unittest.TestCase):
    def test_loads_libghostty_source_commit(self) -> None:
        module = load_module()
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "libghostty-vt.vendor.json"
            path.write_text(json.dumps({"source_commit": "abc123"}))
            self.assertEqual(module.load_libghostty_source_commit(path), "abc123")

    def test_loads_portable_pty_base_version(self) -> None:
        module = load_module()
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "portable-pty.patches.md"
            path.write_text("vendored base: `portable-pty 0.9.0`\n")
            self.assertEqual(module.load_portable_pty_base_version(path), "0.9.0")

    def test_collects_drift_from_upstream_sources(self) -> None:
        module = load_module()
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / "vendor").mkdir()
            (root / "vendor" / "libghostty-vt.vendor.json").write_text(
                json.dumps({"source_commit": "abc123"})
            )
            (root / "vendor" / "portable-pty.patches.md").write_text(
                "vendored base: `portable-pty 0.9.0`\n"
            )

            responses = {
                module.GHOSTTY_REPO_API_URL: {"default_branch": "main"},
                module.GHOSTTY_DEFAULT_BRANCH_API_URL.format(branch="main"): {"sha": "def456"},
                module.PORTABLE_PTY_CRATE_API_URL: {
                    "crate": {"max_stable_version": "0.10.0"}
                },
            }

            checks = module.collect_vendor_drift(
                root, fetch_json=lambda url: responses[url]
            )

            self.assertEqual(len(checks), 2)
            self.assertEqual(checks[0].name, "libghostty-vt")
            self.assertEqual(checks[0].current, "abc123")
            self.assertEqual(checks[0].upstream, "def456")
            self.assertTrue(checks[0].drifted)
            self.assertEqual(checks[1].name, "portable-pty")
            self.assertEqual(checks[1].current, "0.9.0")
            self.assertEqual(checks[1].upstream, "0.10.0")
            self.assertTrue(checks[1].drifted)

    def test_collect_vendor_drift_reports_up_to_date_sources(self) -> None:
        module = load_module()
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / "vendor").mkdir()
            (root / "vendor" / "libghostty-vt.vendor.json").write_text(
                json.dumps({"source_commit": "abc123"})
            )
            (root / "vendor" / "portable-pty.patches.md").write_text(
                "vendored base: `portable-pty 0.9.0`\n"
            )

            responses = {
                module.GHOSTTY_REPO_API_URL: {"default_branch": "main"},
                module.GHOSTTY_DEFAULT_BRANCH_API_URL.format(branch="main"): {"sha": "abc123"},
                module.PORTABLE_PTY_CRATE_API_URL: {
                    "crate": {"max_stable_version": "0.9.0"}
                },
            }

            checks = module.collect_vendor_drift(
                root, fetch_json=lambda url: responses[url]
            )

            self.assertEqual(len(checks), 2)
            self.assertFalse(any(check.drifted for check in checks))


if __name__ == "__main__":
    unittest.main()
