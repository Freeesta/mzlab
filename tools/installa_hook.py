#!/usr/bin/env python3
"""Installs tools/hook_pre_push.sh as .git/hooks/pre-push (run by hand once, and by the SessionStart hook of .claude/settings.json).

    python3 tools/installa_hook.py          # install; never overwrites a pre-push hook that is not ours
    python3 tools/installa_hook.py --togli  # remove ours
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SORGENTE = ROOT / "tools" / "hook_pre_push.sh"
MARCA = "tools/installa_hook.py"


def percorso_hook(root: Path = ROOT) -> Path | None:
    r = subprocess.run(["git", "rev-parse", "--git-path", "hooks/pre-push"], cwd=root, capture_output=True, text=True)
    return (root / r.stdout.strip()) if r.returncode == 0 and r.stdout.strip() else None


def nostro(p: Path) -> bool:
    return p.is_file() and MARCA in p.read_text(encoding="utf-8", errors="replace")


def installa(root: Path = ROOT) -> str:
    dest = percorso_hook(root)
    if dest is None:
        return "not a git repository: nothing installed"
    if dest.exists() and not nostro(dest):
        return f"{dest} exists and is not ours: left alone"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(SORGENTE.read_text(encoding="utf-8"), encoding="utf-8")
    dest.chmod(0o755)
    return f"installed {dest}"


def togli(root: Path = ROOT) -> str:
    dest = percorso_hook(root)
    if dest is not None and nostro(dest):
        dest.unlink()
        return f"removed {dest}"
    return "no hook of ours to remove"


if __name__ == "__main__":
    print(togli() if "--togli" in sys.argv else installa())
