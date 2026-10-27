#!/usr/bin/env python3
"""Merge origin/main into the current branch; conflicts only in generated files are resolved by regenerating them.

Used before pushing a PR branch (AGENTS.md §2). Standard library only (the regenerators are the repository's own tools).

    python3 tools/fondi_main.py [--senza-controlli]
Exit 0: merged and `pre_pr.py --veloce` green. 1: conflicts in human files (merge left open, files printed). 2: checks red.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PY = sys.executable

# generated file -> regeneration command (several files may share one command: it runs once)
GENERATI: dict[str, list[str]] = {
    "docs/schemi/strumenti-1.json": ["env", "MZLAB_UPDATE_STEPS=1", "cargo", "test", "-p", "mzlab-cli", "--lib", "llm_tools"],
    "docs/schemi/steps-cli-1.json": ["env", "MZLAB_UPDATE_STEPS=1", "cargo", "test", "-p", "mzlab-cli", "--lib", "flow_ms2"],
    "docs/schemi/steps-1.json": ["env", "MZLAB_UPDATE_STEPS=1", "cargo", "test", "-p", "mzlab-feat", "--test", "steps"],
    "docs/schemi/steps-attributio-1.json": ["env", "MZLAB_UPDATE_STEPS=1", "cargo", "test", "-p", "mzlab-feat", "--test", "steps_attributio"],
    "docs/schemi/records-1.json": ["env", "MZLAB_UPDATE_STEPS=1", "cargo", "test", "-p", "mzlab-feat", "--test", "steps_attributio"],
    "docs/schemi/steps-tabui-1.json": ["env", "MZLAB_UPDATE_STEPS=1", "cargo", "test", "-p", "mzlab-chem", "--test", "steps_tabui"],
    "tools/tipi/globali.d.ts": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/globali-nomi.d.ts": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/stretto.json": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/stretto-worker.json": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/stretto2.json": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/stretto2-worker.json": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/stretto3.json": [PY, "tools/tipi/genera_globali.py"],
    "tools/tipi/stretto3-worker.json": [PY, "tools/tipi/genera_globali.py"],
    "docs/agenti/mappa.md": [PY, "tools/indice.py"],
}


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=check)


def fondi(root: Path = ROOT, generati: dict[str, list[str]] = GENERATI, out=print) -> int:
    """0 merged (conflicts in generated files resolved), 1 conflicts left in human files."""
    git(root, "fetch", "-q", "origin", "main")
    # a runner has no git identity: use a placeholder one only when none is configured
    ident = [] if git(root, "config", "user.email", check=False).stdout.strip() else ["-c", "user.name=fondi_main", "-c", "user.email=fondi_main@invalid"]
    m = git(root, *ident, "merge", "--no-edit", "origin/main", check=False)
    if m.returncode == 0:
        return 0
    conflitti = git(root, "diff", "--name-only", "--diff-filter=U").stdout.split()
    if not conflitti:
        out("merge failed without conflicts: " + (m.stderr or m.stdout).strip()[:300])
        return 1
    umani = [f for f in conflitti if f not in generati]
    if umani:
        out("conflicts in files to resolve by hand (merge left open):")
        for f in umani:
            out("  " + f)
        return 1
    for f in conflitti:
        git(root, "checkout", "--theirs", "--", f)
    for cmd in {tuple(generati[f]) for f in conflitti}:
        out("regenerating: " + " ".join(cmd))
        subprocess.run(cmd, cwd=root, check=True)
    git(root, "add", "--", *conflitti)
    git(root, *ident, "-c", "core.editor=true", "commit", "--no-edit")
    out("generated files taken from main and regenerated: " + ", ".join(conflitti))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--senza-controlli", action="store_true", help="skip pre_pr.py --veloce")
    a = ap.parse_args()
    if fondi():
        return 1
    if a.senza_controlli:
        return 0
    return 2 if subprocess.run([PY, "tools/pre_pr.py", "--veloce"], cwd=ROOT).returncode else 0


if __name__ == "__main__":
    sys.exit(main())
