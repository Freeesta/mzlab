"""Consistency check of the two 3.11 pytest shards of test.yml: `rest` runs `tests` minus every entry of tools/pytest_solo_311.txt (`--deselect`), `solo311` runs exactly
those entries. Together they must run every collected test once: no entry may match nothing (a renamed test would silently fall out of `solo311` and stay
deselected from `rest`), the two sets must not overlap and their union must be the full collection. Used by tests/test_pytest_shards.py; `python3 tools/ci_pytest_shards.py`
prints the problems and exits 1."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LISTA = Path(__file__).with_name("pytest_solo_311.txt")


def voci() -> list[str]:
    return [r.strip() for r in LISTA.read_text(encoding="utf-8").splitlines() if r.strip()]


def raccogli(args: list[str]) -> list[str]:
    """Node ids pytest collects for these arguments (no test is run)."""
    out = subprocess.run([sys.executable, "-m", "pytest", "--collect-only", "-q", "-p", "no:cacheprovider", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    if out.returncode != 0:
        raise RuntimeError(f"collection of {args} failed:\n{out.stdout[-2000:]}{out.stderr[-2000:]}")
    return [r for r in out.stdout.splitlines() if "::" in r and not r.startswith(" ")]


def confronta(pieno: list[str], rest: list[str], solo: list[str], senza_test: list[str], doppie: list[str]) -> list[str]:
    """Problems found between the full collection and the two shards (empty list = consistent)."""
    guai = [f"entry listed twice in the solo list: {v}" for v in doppie]
    guai += [f"solo entry matches no test: {v}" for v in senza_test]
    guai += [f"run by both shards: {t}" for t in sorted(set(rest) & set(solo))[:20]]
    guai += [f"run by no shard: {t}" for t in sorted(set(pieno) - set(rest) - set(solo))[:20]]
    guai += [f"run by a shard but not collected: {t}" for t in sorted((set(rest) | set(solo)) - set(pieno))[:20]]
    if len(rest) != len(set(rest)) or len(solo) != len(set(solo)):
        guai.append("a shard collects the same test twice")
    return guai


def controlla() -> list[str]:
    elenco = voci()
    doppie = sorted({v for v in elenco if elenco.count(v) > 1})
    pieno = raccogli(["tests"])
    rest = raccogli(["tests", *[a for v in elenco for a in ("--deselect", v)]])
    solo = raccogli(elenco)
    senza_test = [v for v in elenco if not any(t == v or t.startswith(v + "::") for t in pieno)]
    return confronta(pieno, rest, solo, senza_test, doppie)


if __name__ == "__main__":
    problemi = controlla()
    print("\n".join(problemi) if problemi else "shards consistent")
    sys.exit(1 if problemi else 0)
