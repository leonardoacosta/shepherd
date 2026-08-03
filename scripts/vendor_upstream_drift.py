from __future__ import annotations

import json
import re
import sys
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


PROJECT_ROOT = Path(__file__).resolve().parent.parent
GHOSTTY_REPO_API_URL = "https://api.github.com/repos/ghostty-org/ghostty"
GHOSTTY_DEFAULT_BRANCH_API_URL = (
    "https://api.github.com/repos/ghostty-org/ghostty/commits/{branch}"
)
PORTABLE_PTY_CRATE_API_URL = "https://crates.io/api/v1/crates/portable-pty"
REQUEST_TIMEOUT_SECONDS = 30
REQUEST_HEADERS = {
    "Accept": "application/json",
    "User-Agent": "shepherd-vendor-upstream-drift-check",
}


@dataclass(frozen=True)
class VendorDriftCheck:
    name: str
    current: str
    upstream: str
    source: str
    drifted: bool


def fetch_json(url: str) -> dict:
    request = urllib.request.Request(url, headers=REQUEST_HEADERS)
    with urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
        return json.load(response)


def load_libghostty_source_commit(path: Path) -> str:
    return json.loads(path.read_text())["source_commit"]


def load_portable_pty_base_version(path: Path) -> str:
    match = re.search(r"vendored base:\s*`portable-pty ([^`]+)`", path.read_text())
    if match is None:
        raise ValueError(f"could not find portable-pty vendored base in {path}")
    return match.group(1)


def collect_vendor_drift(
    project_root: Path = PROJECT_ROOT,
    fetch_json: Callable[[str], dict] = fetch_json,
) -> list[VendorDriftCheck]:
    vendor_root = project_root / "vendor"
    libghostty_current = load_libghostty_source_commit(vendor_root / "libghostty-vt.vendor.json")
    portable_pty_current = load_portable_pty_base_version(
        vendor_root / "portable-pty.patches.md"
    )

    repo = fetch_json(GHOSTTY_REPO_API_URL)
    default_branch = repo["default_branch"]
    libghostty_upstream = fetch_json(
        GHOSTTY_DEFAULT_BRANCH_API_URL.format(branch=default_branch)
    )["sha"]
    portable_pty_upstream = fetch_json(PORTABLE_PTY_CRATE_API_URL)["crate"][
        "max_stable_version"
    ]

    return [
        VendorDriftCheck(
            name="libghostty-vt",
            current=libghostty_current,
            upstream=libghostty_upstream,
            source=f"ghostty-org/ghostty {default_branch}",
            drifted=libghostty_current != libghostty_upstream,
        ),
        VendorDriftCheck(
            name="portable-pty",
            current=portable_pty_current,
            upstream=portable_pty_upstream,
            source="crates.io portable-pty max_stable_version",
            drifted=portable_pty_current != portable_pty_upstream,
        ),
    ]


def main() -> int:
    checks = collect_vendor_drift()
    drifted = [check for check in checks if check.drifted]
    if not drifted:
        print("vendored upstream bases are current")
        return 0

    print("vendored upstream drift detected:")
    for check in drifted:
        print(
            f"- {check.name}: vendored {check.current} lags {check.source} at {check.upstream}"
        )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
