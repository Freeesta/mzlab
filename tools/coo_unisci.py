#!/usr/bin/env python3
"""Merge readiness of a PR for the Coordinator: CI from the stat-ci events, local combine with main, risky paths.

Read-only except for a temporary worktree that is always removed. Standard library only, no network beyond `git fetch`.

    python3 tools/coo_unisci.py PR SHA [--repo DIR]     # last line: JSON {pr, sha, verde, combine, rischi, unire, motivo}
Exit 0 when `unire` is true, 1 otherwise.
"""
from __future__ import annotations

import argparse
import fnmatch
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import ci_metrics

RISCHI = ["Cargo.toml", "*/Cargo.toml", "Cargo.lock", ".github/workflows/*", "*.wasm", "crates/mzlab-desktop/*"]
CSP = "mzlab/web/index.html"
OK = ("success", "skipped", "neutral")


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=check)


def ci_verde(eventi: list[dict], sha: str) -> tuple[bool, str | None]:
    """(green, first failing job) from the `run` events of `sha`; `rust` may be absent (skipped by the path filter)."""
    ultimo: dict[str, dict] = {}
    for r in eventi:
        if r.get("tipo") == "run" and r.get("sha") == sha:
            v = ultimo.get(r["workflow"])
            if v is None or (r.get("run_attempt", 1), r["ts_utc"]) >= (v.get("run_attempt", 1), v["ts_utc"]):
                ultimo[r["workflow"]] = r
    if "test" not in ultimo:
        return False, "test (no run recorded)"
    for nome in ("test", "rust"):
        if nome in ultimo and ultimo[nome].get("esito") not in OK:
            return False, nome
    return True, None


def rischi(file: list[str], index_html_csp: bool) -> list[str]:
    out = [f"VIA richiesto: {f}" for f in file if any(fnmatch.fnmatch(f, p) for p in RISCHI)]
    if index_html_csp:
        out.append(f"VIA richiesto: CSP in {CSP}")
    return out


def csp_cambiato(repo: Path, sha: str) -> bool:
    d = git(repo, "diff", "-U0", f"origin/main...{sha}", "--", CSP, check=False).stdout
    return any("Content-Security-Policy" in r for r in d.splitlines() if r[:1] in "+-")


def combine(repo: Path, sha: str) -> tuple[bool, str]:
    """Merge `sha` into origin/main in a temporary worktree and run the size and index checks there."""
    base = Path(tempfile.mkdtemp(prefix="coo_unisci_"))
    wt = base / "wt"
    try:
        git(repo, "worktree", "add", "--detach", str(wt), "origin/main")
        m = git(wt, "-c", "user.name=coo", "-c", "user.email=coo@invalid", "merge", "--no-ff", "--no-edit", sha, check=False)
        if m.returncode:
            files = git(wt, "diff", "--name-only", "--diff-filter=U", check=False).stdout.split()
            return False, "conflict: " + ", ".join(files[:5])
        for cmd in (["tools/controlla.py"], ["tools/indice.py", "--controlla"]):
            p = subprocess.run([sys.executable, *cmd], cwd=wt, capture_output=True, text=True)
            if p.returncode:
                return False, f"{cmd[0]} failed: " + (p.stdout + p.stderr).strip().splitlines()[-1][:200]
        return True, ""
    finally:
        git(repo, "worktree", "remove", "--force", str(wt), check=False)
        shutil.rmtree(base, ignore_errors=True)


def valuta(repo: Path, pr: int, sha: str) -> dict:
    git(repo, "fetch", "-q", "origin", "main", "stat-ci")
    git(repo, "fetch", "-q", "origin", f"pull/{pr}/head")
    testa = git(repo, "rev-parse", "FETCH_HEAD").stdout.strip()
    res = {"pr": pr, "sha": sha, "verde": False, "combine": None, "rischi": [], "unire": False, "motivo": ""}
    if testa != sha:
        res["motivo"] = f"head SHA differs: PR is at {testa}"
        return res
    eventi = ci_metrics.eventi_ramo(repo=str(repo))
    res["verde"], job = ci_verde(eventi, sha)
    if not res["verde"]:
        res["motivo"] = f"CI non verde: {job}"
        return res
    res["rischi"] = rischi(git(repo, "diff", "--name-only", f"origin/main...{sha}").stdout.split(), csp_cambiato(repo, sha))
    res["combine"], motivo = combine(repo, sha)
    res["motivo"] = motivo or "; ".join(res["rischi"])
    res["unire"] = bool(res["combine"] and not res["rischi"])
    return res


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pr", type=int)
    ap.add_argument("sha")
    ap.add_argument("--repo", default=".")
    a = ap.parse_args()
    res = valuta(Path(a.repo).resolve(), a.pr, a.sha)
    print(json.dumps(res, ensure_ascii=False))
    return 0 if res["unire"] else 1


if __name__ == "__main__":
    sys.exit(main())
