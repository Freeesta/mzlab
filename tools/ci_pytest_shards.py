"""Consistency check of the two 3.11 pytest shards of test.yml: `rest` runs `tests` minus every entry of tools/pytest_solo_311.txt (`--deselect`), `solo311` runs exactly
those entries. Together they must run every collected test once: no entry may match nothing (a renamed test would silently fall out of `solo311` and stay
deselected from `rest`), the two sets must not overlap and their union must be the full collection. Used by tests/test_pytest_shards.py; `python3 tools/ci_pytest_shards.py`
prints the problems and exits 1."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIST = Path(__file__).with_name("pytest_solo_311.txt")


def voci() -> list[str]:
    return [r.strip() for r in LIST.read_text(encoding="utf-8").splitlines() if r.strip()]


def collect(args: list[str]) -> list[str]:
    """Node ids pytest collects for these arguments (no test is run)."""
    out = subprocess.run([sys.executable, "-m", "pytest", "--collect-only", "-q", "-p", "no:cacheprovider", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    if out.returncode != 0:
        raise RuntimeError(f"collection of {args} failed:\n{out.stdout[-2000:]}{out.stderr[-2000:]}")
    return [r for r in out.stdout.splitlines() if "::" in r and not r.startswith(" ")]


def confronta(full: list[str], rest: list[str], solo: list[str], without_test: list[str], doppie: list[str]) -> list[str]:
    """Problems found between the full collection and the two shards (empty list = consistent)."""
    guai = [f"entry listed twice in the solo list: {v}" for v in doppie]
    guai += [f"solo entry matches no test: {v}" for v in without_test]
    guai += [f"run by both shards: {t}" for t in sorted(set(rest) & set(solo))[:20]]
    guai += [f"run by no shard: {t}" for t in sorted(set(full) - set(rest) - set(solo))[:20]]
    guai += [f"run by a shard but not collected: {t}" for t in sorted((set(rest) | set(solo)) - set(full))[:20]]
    if len(rest) != len(set(rest)) or len(solo) != len(set(solo)):
        guai.append("a shard collects the same test twice")
    return guai


def controlla() -> list[str]:
    listing = voci()
    doppie = sorted({v for v in listing if listing.count(v) > 1})
    full = collect(["tests"])
    rest = collect(["tests", *[a for v in listing for a in ("--deselect", v)]])
    solo = collect(listing)
    without_test = [v for v in listing if not any(t == v or t.startswith(v + "::") for t in full)]
    return confronta(full, rest, solo, without_test, doppie)


if __name__ == "__main__":
    problems = controlla()
    print("\n".join(problems) if problems else "shards consistent")
    sys.exit(1 if problems else 0)
