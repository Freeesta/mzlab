"""Chromium of the cloud container: Playwright may look for a build that is not installed while /opt/pw-browsers/chromium (a link to the installed one) works.
Used by tests_e2e/lib.py and tools/verifica.py; never runs `playwright install`."""
from __future__ import annotations

import os
from pathlib import Path

CLOUD = Path(os.environ.get("PLAYWRIGHT_BROWSERS_PATH") or "/opt/pw-browsers") / "chromium"


def abilita() -> None:
    """Make `chromium.launch()` fall back to the cloud executable when Playwright's own build is missing (no effect elsewhere)."""
    if not CLOUD.exists():
        return
    from playwright.sync_api import BrowserType
    if getattr(BrowserType.launch, "_cloud", False):
        return
    orig = BrowserType.launch

    def launch(self, *a, **k):
        if self.name == "chromium" and "executable_path" not in k:
            try:
                if not Path(self.executable_path).exists():
                    k["executable_path"] = str(CLOUD)
            except Exception:
                k["executable_path"] = str(CLOUD)
        return orig(self, *a, **k)

    launch._cloud = True
    BrowserType.launch = launch
