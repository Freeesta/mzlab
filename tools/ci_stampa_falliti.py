#!/usr/bin/env python3
"""Prints, in the job log, the report of the run and the last 60 lines of the log of every FAIL (notte.yml): the cause is readable without downloading the artifact.
A FAIL that is not an e2e (the site build `prepara_sito`, `precompila`, pytest, prova_hr, the syntax checks) prints the tail of the logs of those stages."""
import re
import sys
from pathlib import Path

OUT = Path(".verifica")
STAGES = ("prepara_sito", "precompila", "pytest-cargo", "pytest", "prova_hr")


def tail(name: str, n: int = 60) -> None:
    f = OUT / "log" / f"{name}.log"
    print(f"===== {name}")
    print("\n".join(f.read_text(encoding="utf-8", errors="replace").splitlines()[-n:]) if f.exists() else "(no log)")


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")      # the Windows console is cp1252
    reports = [OUT / "ultimo-parte.md", OUT / "ultimo.md"]
    if not (OUT / "ultimo.md").exists():
        print("No .verifica/ultimo.md: verifica.py stopped before writing its report (see the log of the failed step above)")
        return 0
    seen: set[str] = set()
    for rep in (r for r in reports if r.exists()):
        text = rep.read_text(encoding="utf-8", errors="replace")
        print(f"--- {rep.name}")
        print("\n".join(l[:2000] for l in text.splitlines()))
        fails = [m.group(1) for m in re.finditer(r"^- FAIL +(.+)$", text, re.M)]
        print(f"{len(fails)} FAIL")
        for line in fails:
            name = line.split()[0]
            if name.startswith("e2e") and name not in seen:
                seen.add(name); tail(name)
        if any(not l.startswith("e2e") for l in fails):
            for st in STAGES:
                if st not in seen and (OUT / "log" / f"{st}.log").exists():
                    seen.add(st); tail(st)
    return 0


if __name__ == "__main__":
    sys.exit(main())
