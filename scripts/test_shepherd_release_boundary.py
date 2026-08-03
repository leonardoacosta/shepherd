import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ReleaseBoundaryTests(unittest.TestCase):
    def test_unpublished_channels_are_explicitly_source_only(self) -> None:
        stable = json.loads((ROOT / "website/shepherd-latest.json").read_text())
        preview = json.loads((ROOT / "website/shepherd-preview.json").read_text())

        self.assertEqual(stable["assets"], {})
        self.assertIn("source-built", stable["notes"])
        self.assertEqual(preview["assets"], {})
        self.assertIn("source-built", preview["notes"])

    def test_current_docs_do_not_advertise_unpublished_installers(self) -> None:
        readme = (ROOT / "README.md").read_text()
        self.assertIn("shepherd-install --update", readme)
        self.assertNotIn("curl -fsSL https://shepherd.dev/install.sh", readme)
        self.assertNotIn("brew install shepherd", readme)
        self.assertNotIn("mise use -g shepherd", readme)

        for relative in (
            "docs/next/website/src/content/docs/install.mdx",
            "docs/next/website/src/content/docs/ja/install.mdx",
            "docs/next/website/src/content/docs/zh-cn/install.mdx",
        ):
            content = (ROOT / relative).read_text()
            self.assertIn("shepherd-install --update", content, relative)

    def test_staged_installers_explain_the_source_build_boundary(self) -> None:
        shell_installer = (ROOT / "website/install.sh").read_text()
        windows_installer = (ROOT / "website/install.ps1").read_text()

        self.assertIn("no Shepherd binary release exists", shell_installer)
        self.assertIn("No Shepherd binary release exists", windows_installer)
        self.assertIn("shepherd-install", shell_installer)
        self.assertIn("shepherd-install", windows_installer)


if __name__ == "__main__":
    unittest.main()
