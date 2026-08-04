import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ReleaseBoundaryTests(unittest.TestCase):
    def test_authoritative_dev_pushes_run_validation_workflows(self) -> None:
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        website = (ROOT / ".github/workflows/website.yml").read_text()
        nix = (ROOT / ".github/workflows/nix.yml").read_text()

        self.assertIn("branches: [dev, master, windows]", ci)
        self.assertEqual(ci.count("github.ref_name == 'dev'"), 2)
        self.assertIn("scripts.test_shepherd_release_boundary", ci)
        self.assertIn("branches: [dev, master]", website)
        self.assertIn("branches: [dev, master]", nix)

    def test_release_phases_share_a_fail_fast_master_preflight(self) -> None:
        justfile = (ROOT / "justfile").read_text()
        preflight = justfile.split("release-branch-check:", 1)[1].split(
            "# Prepare the release commit", 1
        )[0]

        self.assertIn('branch="$(git branch --show-current)"', preflight)
        self.assertIn('if [ "$branch" != "master" ]', preflight)
        self.assertIn("release-prepare version: release-branch-check", justfile)
        self.assertIn("release-publish version: release-branch-check", justfile)

    def test_release_guidance_uses_fast_forward_only_dev_promotion(self) -> None:
        guidance = (ROOT / "AGENTS.md").read_text()

        self.assertIn("`dev` is the integration branch", guidance)
        self.assertIn("`master` is the stable publication branch", guidance)
        self.assertIn("git fetch origin dev master --tags", guidance)
        self.assertIn("git switch master", guidance)
        self.assertIn("git merge --ff-only origin/dev", guidance)
        self.assertIn("Do not force-push or create a merge commit during release", guidance)

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
