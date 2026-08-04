from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.docs_translation_parity import check_docs_translation_parity, heading_outline


class DocsTranslationParityTests(unittest.TestCase):
    def test_heading_outline_ignores_fenced_code_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "doc.mdx"
            path.write_text(
                "# Title\n\n```md\n## Not a heading\n```\n\n## Real section\n",
                encoding="utf-8",
            )

            self.assertEqual(heading_outline(path), [1, 2])

    def test_parity_accepts_translated_heading_text_with_same_shape(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "ja").mkdir()
            (root / "zh-cn").mkdir()
            (root / "guide.mdx").write_text("# Guide\n\n## Install\n\n### Verify\n", encoding="utf-8")
            (root / "ja" / "guide.mdx").write_text(
                "# ガイド\n\n## インストール\n\n### 確認\n",
                encoding="utf-8",
            )
            (root / "zh-cn" / "guide.mdx").write_text(
                "# 指南\n\n## 安装\n\n### 验证\n",
                encoding="utf-8",
            )

            self.assertEqual(check_docs_translation_parity(root), [])

    def test_parity_reports_missing_heading_sections(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "ja").mkdir()
            (root / "zh-cn").mkdir()
            (root / "cli-reference.mdx").write_text(
                "# CLI reference\n\n## Launch\n\n## Shell completions\n",
                encoding="utf-8",
            )
            (root / "ja" / "cli-reference.mdx").write_text(
                "# CLI リファレンス\n\n## 起動\n",
                encoding="utf-8",
            )
            (root / "zh-cn" / "cli-reference.mdx").write_text(
                "# CLI 参考\n\n## 启动\n\n## Shell 补全\n",
                encoding="utf-8",
            )

            errors = check_docs_translation_parity(root)

            self.assertEqual(len(errors), 1)
            self.assertIn("ja/cli-reference.mdx", errors[0])
            self.assertIn("heading outline differs", errors[0])

    def test_parity_reports_missing_and_stale_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "ja").mkdir()
            (root / "zh-cn").mkdir()
            (root / "guide.mdx").write_text("# Guide\n", encoding="utf-8")
            (root / "ja" / "old.mdx").write_text("# Old\n", encoding="utf-8")
            (root / "zh-cn" / "guide.mdx").write_text("# 指南\n", encoding="utf-8")

            errors = check_docs_translation_parity(root)

            self.assertIn(f"{root / 'ja' / 'guide.mdx'}: missing translation file", errors)
            self.assertIn(f"{root / 'ja' / 'old.mdx'}: no matching English doc", errors)


class HermesGuidanceParityTests(unittest.TestCase):
    """Guards task 3.1: every maintained locale must agree that Hermes reports
    session identity only, that state comes from screen-manifest detection, and
    that the documented integration version matches the bundled
    HERMES_INTEGRATION_VERSION constant. Checks concepts and version numerals
    rather than exact translated wording, per design.md's narrow-parity decision.
    """

    DOCS_ROOT = Path(__file__).parents[1] / "docs/next/website/src/content/docs"
    CURRENT_HERMES_VERSION = "4"
    LOCALE_LIFECYCLE_AUTHORITY_MARKERS = {
        "": "Lifecycle authority",
        "ja": "ライフサイクル権威",
        "zh-cn": "生命周期权威",
    }

    def test_agents_table_never_claims_hermes_lifecycle_hook_authority(self) -> None:
        for locale in self.LOCALE_LIFECYCLE_AUTHORITY_MARKERS:
            path = self.DOCS_ROOT / locale / "agents.mdx"
            hermes_row = next(
                (line for line in path.read_text(encoding="utf-8").splitlines() if "Hermes" in line),
                None,
            )
            self.assertIsNotNone(hermes_row, f"{path}: no Hermes row found in agents table")
            self.assertNotIn(
                "lifecycle",
                hermes_row.lower(),
                f"{path}: {hermes_row!r} still claims lifecycle hook authority for Hermes",
            )

    def test_integrations_lifecycle_authority_row_excludes_hermes(self) -> None:
        for locale, marker in self.LOCALE_LIFECYCLE_AUTHORITY_MARKERS.items():
            path = self.DOCS_ROOT / locale / "integrations.mdx"
            authority_row = next(
                (
                    line
                    for line in path.read_text(encoding="utf-8").splitlines()
                    if line.startswith("|") and marker in line
                ),
                None,
            )
            self.assertIsNotNone(authority_row, f"{path}: no {marker!r} table row found")
            self.assertNotIn(
                "Hermes",
                authority_row,
                f"{path}: lifecycle authority row still lists Hermes",
            )

    def test_integrations_doc_reports_current_hermes_version_in_every_locale(self) -> None:
        for locale in self.LOCALE_LIFECYCLE_AUTHORITY_MARKERS:
            path = self.DOCS_ROOT / locale / "integrations.mdx"
            text = path.read_text(encoding="utf-8")
            self.assertRegex(
                text,
                rf"Hermes Agent[^\n]*`{self.CURRENT_HERMES_VERSION}`",
                f"{path}: Hermes Agent integration version must read `{self.CURRENT_HERMES_VERSION}`",
            )


if __name__ == "__main__":
    unittest.main()
