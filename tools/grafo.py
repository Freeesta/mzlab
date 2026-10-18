"""Builds the local code graph (graphify-out/, not committed: .gitignore) so agents can run `graphify query` instead of reading files.

    python3 tools/grafo.py   # install graphify if missing, then rebuild the graph (about 2 minutes the first time, seconds after)

Meant for a session-start hook. It never fails the session: on any problem it prints one line and exits 0."""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def installa() -> bool:
    """True when the `graphify` command is available (installed with uv or pipx if needed)."""
    if shutil.which("graphify"):
        return True
    for cmd in (["uv", "tool", "install", "graphifyy"], ["pipx", "install", "graphifyy"]):
        if shutil.which(cmd[0]) and subprocess.run(cmd, capture_output=True).returncode == 0:
            return bool(shutil.which("graphify"))
    return False


def main() -> int:
    if not installa():
        print("grafo: graphify not available (install uv or pipx), graph skipped")
        return 0
    r = subprocess.run(["graphify", "update", "."], cwd=ROOT, capture_output=True, text=True)
    print("grafo: graphify-out/ updated" if r.returncode == 0 else "grafo: graphify failed, graph skipped")
    return 0


if __name__ == "__main__":
    sys.exit(main())
