#!/usr/bin/env python3
"""Checks to run before opening a PR, in the order of the CI, stopping at the first failure (about 1-2 minutes, plus clippy when `crates/` changed).

    python3 tools/pre_pr.py            # locally: all the steps; in the cloud (no browser, CLAUDE_CODE_REMOTE or no Playwright): same as --veloce
    python3 tools/pre_pr.py --completo   # all the steps even in the cloud
    python3 tools/pre_pr.py --senza-e2e  # skip `verifica.py --cambiati` (the e2e of the changed files)
    python3 tools/pre_pr.py --veloce     # only the cheap checks (well under 2 minutes, never clippy or e2e): adds the CSP hash and prints which e2e would run

When the diff against origin/main touches only documents (the list of tools/ci_solo_doc.py) it runs just code map, size limits and, if the Teoria changed, the Teoria check, and prints «solo documenti».
Steps: code map, size limits, i18n, Teoria, JS syntax (only when `mzlab/web/` differs from origin/main), generated globals, tsc, Rust format, clippy and machete (only when the Rust paths of the CI differ from origin/main), e2e of the changed files.
Every step prints its name before it starts, so a long one is never silent. Exit 0 = green; with --veloce (also the cloud default) the e2e and clippy are not run and the output says the CI will run them. With --completo, e2e skipped for lack of a browser give exit 2.
Merge `origin/main` first (`git merge origin/main`): the checks compare with the files as the merge leaves them.
"""
from __future__ import annotations

import importlib.util
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PY = sys.executable
sys.path.insert(0, str(ROOT / "tools"))
import ci_solo_doc  # noqa: E402


# the paths that switch on the Rust jobs in .github/workflows/rust.yml
RUST = ["crates/", "vendor/", "Cargo.*", "rust-toolchain.toml", ".cargo/", "tools/controlla_rust.py", "tools/clippy_base.json"]
JS_SYNTAX = [PY, "-c", "import sys; sys.path.insert(0, 'tools'); import verifica as v; v.LOG.mkdir(parents=True, exist_ok=True); r = []; v.js_syntax(r); "
             "print(r[0][3]); sys.exit(r[0][1] == 'FAIL')"]


def cambiato(*prefissi: str) -> bool:
    """True when something under a `prefisso` differs from origin/main (committed, staged or not); True too when origin/main is unknown (better a check too many)."""
    r = subprocess.run(["git", "diff", "--name-only", "origin/main", "--", *prefissi], cwd=ROOT, capture_output=True, text=True)
    return r.returncode != 0 or bool(r.stdout.strip())


def in_cloud() -> bool:
    """True in a cloud container or wherever Playwright is missing: there the e2e cannot run and the CI does them."""
    return os.environ.get("CLAUDE_CODE_REMOTE") == "true" or importlib.util.find_spec("playwright") is None


def files_cambiati() -> list[str]:
    """Paths that differ from origin/main (committed, staged, unstaged or untracked); empty when origin/main is unknown."""
    d = subprocess.run(["git", "diff", "--name-only", "--no-renames", "origin/main"], cwd=ROOT, capture_output=True, text=True)
    u = subprocess.run(["git", "ls-files", "--others", "--exclude-standard"], cwd=ROOT, capture_output=True, text=True)
    return [f for f in (d.stdout + u.stdout).split("\n") if f] if d.returncode == 0 else []


def passi_solo_doc() -> list[tuple[str, list[str]]]:
    """The checks a documents-only diff needs."""
    p = [("mappa del codice", [PY, "tools/indice.py", "--controlla"]), ("soglie di dimensione", [PY, "tools/controlla.py"])]
    if cambiato("mzlab/web/teoria/"):
        p.append(("Teoria", [PY, "tools/controlla_teoria.py"]))
    return p


# prints the e2e that `verifica.py --cambiati` would run, without running them
SELEZIONE_E2E = [PY, "-c", "import sys; sys.path.insert(0, 'tools'); import verifica as v; sel, why = v.changed_tests(); print(why)"]


def passi(con_e2e: bool, veloce: bool = False) -> list[tuple[str, list[str]]]:
    p = [
        ("mappa del codice", [PY, "tools/indice.py", "--controlla"]),
        ("soglie di dimensione", [PY, "tools/controlla.py"]),
        ("lingue (i18n)", [PY, "tools/controlla_i18n.py"]),
        ("Teoria", [PY, "tools/controlla_teoria.py"]),
    ]
    if shutil.which("node") and cambiato("mzlab/web/"):
        p.append(("sintassi JS (job javascript)", JS_SYNTAX))
    p += [
        ("globali dei tipi", [PY, "tools/tipi/genera_globali.py", "--controlla"]),
        ("tsc", [PY, "tools/tipi/controlla_tsc.py"]),
    ]
    if (ROOT / "tools" / "controlla_import.py").exists():
        p.append(("import degli e2e", [PY, "tools/controlla_import.py"]))
    if veloce:
        p.append(("hash CSP", [PY, "tools/csp_hash.py", "--check"]))
        if cambiato(*RUST):
            p.append(("cargo fmt contro la base", [PY, "tools/controlla_rust.py", "--solo-fmt"]))
        p.append(("e2e che verrebbero eseguiti", SELEZIONE_E2E))
        return p
    if cambiato(*RUST):
        p += [("fmt e clippy contro la base", [PY, "tools/controlla_rust.py"])]
        if shutil.which("cargo-machete"):
            p.append(("dipendenze non usate (cargo machete)", ["cargo", "machete"]))
    if con_e2e:
        p.append(("e2e dei file cambiati", [PY, "tools/verifica.py", "--cambiati"]))
    return p


def main() -> int:
    saltati = False
    veloce = "--veloce" in sys.argv or (in_cloud() and "--completo" not in sys.argv)
    solo_doc = ci_solo_doc.solo(files_cambiati())
    if solo_doc:
        print("solo documenti: only the checks the documents need")
    elenco = passi_solo_doc() if solo_doc else passi("--senza-e2e" not in sys.argv and not veloce, veloce)
    for i, (nome, cmd) in enumerate(elenco, 1):
        print(f"...  [{i}/{len(elenco)}] {nome}", flush=True)
        t0 = time.time()
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
        dt = time.time() - t0
        if r.returncode:
            print(f"FAIL {nome} ({dt:.0f} s): {' '.join(cmd)}")
            print((r.stdout + r.stderr)[-3000:])
            return 1
        if "verifica.py" in " ".join(cmd) and "SKIP e2e" in r.stdout:
            saltati = True
            print(f"SKIP {nome} ({dt:.0f} s)")
        else:
            print(f"ok   {nome} ({dt:.0f} s)" + (f": {r.stdout.strip()[:300]}" if veloce and "e2e" in nome else ""), flush=True)
    if saltati:
        print("E2E SALTATI: Chromium non trovato, la CI li eseguirà (gli altri passi sono verdi, ma questo NON è «tutto verde»).")
        return 2
    if veloce and not solo_doc:
        print("Tutto verde (giro veloce): clippy ed e2e non girati, li esegue la CI.")
    print("Tutto verde: ora git merge origin/main (se non l'hai appena fatto) e apri la PR.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
