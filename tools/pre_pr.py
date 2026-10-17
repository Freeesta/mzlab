#!/usr/bin/env python3
"""Checks to run before opening a PR, in the order of the CI, stopping at the first failure (about 1-2 minutes, plus clippy when `crates/` changed).

    python3 tools/pre_pr.py            # all the steps
    python3 tools/pre_pr.py --senza-e2e  # skip `verifica.py --cambiati` (the e2e of the changed files)

Steps: code map, size limits, i18n, Teoria, generated globals, tsc, Rust format and clippy (only when `crates/` differs from origin/main), e2e of the changed files.
Merge `origin/main` first (`git merge origin/main`): the checks compare with the files as the merge leaves them.
"""
from __future__ import annotations

import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PY = sys.executable


def cambiato(prefisso: str) -> bool:
    """True when something under `prefisso` differs from origin/main (committed, staged or not); True too when origin/main is unknown (better a check too many)."""
    r = subprocess.run(["git", "diff", "--name-only", "origin/main", "--", prefisso], cwd=ROOT, capture_output=True, text=True)
    return r.returncode != 0 or bool(r.stdout.strip())


def passi(con_e2e: bool) -> list[tuple[str, list[str]]]:
    p = [
        ("mappa del codice", [PY, "tools/indice.py", "--controlla"]),
        ("soglie di dimensione", [PY, "tools/controlla.py"]),
        ("lingue (i18n)", [PY, "tools/controlla_i18n.py"]),
        ("Teoria", [PY, "tools/controlla_teoria.py"]),
        ("globali dei tipi", [PY, "tools/tipi/genera_globali.py", "--controlla"]),
        ("tsc", [PY, "tools/tipi/controlla_tsc.py"]),
    ]
    if (ROOT / "tools" / "controlla_import.py").exists():
        p.append(("import degli e2e", [PY, "tools/controlla_import.py"]))
    if cambiato("crates/"):
        p += [("cargo fmt", ["cargo", "fmt", "--all", "--check"]), ("fmt e clippy contro la base", [PY, "tools/controlla_rust.py"])]
    if con_e2e:
        p.append(("e2e dei file cambiati", [PY, "tools/verifica.py", "--cambiati"]))
    return p


def main() -> int:
    for nome, cmd in passi("--senza-e2e" not in sys.argv):
        t0 = time.time()
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
        dt = time.time() - t0
        if r.returncode:
            print(f"FAIL {nome} ({dt:.0f} s): {' '.join(cmd)}")
            print((r.stdout + r.stderr)[-3000:])
            return 1
        print(f"ok   {nome} ({dt:.0f} s)")
    print("Tutto verde: ora git merge origin/main (se non l'hai appena fatto) e apri la PR.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
