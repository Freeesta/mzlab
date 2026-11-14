"""Chooses the e2e of the job `e2e-cambiati` of test.yml: the tests that cover the files a pull request changes, minus what other jobs already run.
`python3 tools/ci_e2e_cambiati.py <base sha>` prints what it chose and why, and appends `e2e=<names, comma-separated>` to $GITHUB_OUTPUT (empty: run nothing).
The map from files to tests is `changed_tests` of tools/verifica.py (the `# covers:` header lines of the tests); the smoke tests (job `browser`) and the tests that need the site build, the CI .wasm build (`wasm`), a .dam
or the lab data (sito-e2e and notte.yml) are left out. A core file selects every e2e: the complete round is too long for this job (30 min) and runs in notte.yml, so the job runs a fixed safety set
instead: tests that cover as many `# covers:` prefixes as the time budget allows (`core_set`, from the allow list tools/ci_e2e_verdi.json: the e2e that passed on the runner in the latest full nightly, with their seconds there; `--verdi <ultimo.md of each shard>` rewrites it from the artifacts `notte-completa-K`)."""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verifica  # noqa: E402

ELSEWHERE = {"sito", "dam", "veri", "wasm"}
GREEN = Path(__file__).resolve().parent / "ci_e2e_verdi.json"   # the e2e that passed on the runner in the latest full nightly, with their seconds there: the only ones the safety set may take
TEST_CAP_S = 60       # a test slower than this is not a representative: it would eat the budget of ten others
BUDGET_S = 800        # sum of the nightly seconds of the safety set; the job's step took about 1.25x that on the runner, plus ~1.5 min of setup: the whole job stays under 22 of its 30 min


def core_set() -> tuple[list[str], str]:
    """Safety set for a core file: greedy cover of the `# covers:` prefixes by the cheapest tests (most entries per second) among the allow-listed ones, within BUDGET_S."""
    cost = json.loads(GREEN.read_text(encoding="utf-8"))
    smoke = set(verifica.SMOKE) | set(verifica.SMOKE_CHROMIUM_LINUX)
    by_prefix: dict[str, set[str]] = {}
    for prefix, n in verifica.covers():
        by_prefix.setdefault(prefix, set()).add(n)
    entries = [{n for n in tests if n not in smoke and not verifica.NEEDS.get(n, set()) & ELSEWHERE and cost.get(n, TEST_CAP_S + 1) <= TEST_CAP_S and (verifica.E2E / f"{n}.py").exists()}
               for tests in by_prefix.values()]
    entries = [e for e in entries if e]
    left, used, chosen = list(entries), 0, []
    while left:
        score: dict[str, int] = {}
        for e in left:
            for n in e:
                score[n] = score.get(n, 0) + 1
        best = max(score, key=lambda n: (score[n] / (cost[n] + 1), n))
        if used + cost[best] > BUDGET_S:
            break
        chosen.append(best)
        used += cost[best]
        left = [e for e in left if best not in e]
    return sorted(chosen), f"{len(entries) - len(left)} of {len(entries)} areas covered, {used} s of {BUDGET_S} s"


def write_green(reports: list[Path]) -> int:
    """Rewrite the allow list from the reports (ultimo.md) of the shards of a full nightly: only the lines «- OK e2e_x (12 s)»."""
    green = {m[1]: int(m[2]) for r in reports for m in (re.match(r"- OK +(e2e\w*) \((\d+) s\)", l) for l in r.read_text(encoding="utf-8").splitlines()) if m}
    GREEN.write_text(json.dumps(dict(sorted(green.items())), indent=0) + "\n", encoding="utf-8")
    return len(green)


def pick(files: set[str]) -> tuple[list[str], str]:
    """(tests to run, sorted; the plain reason) for the changed files."""
    sel, why = verifica.changed_tests(files)
    if sel is None:
        tests, note = core_set()
        return tests, f"{why}: the complete round is notte.yml's; this job runs a safety set of {len(tests)} e2e ({note})"
    if sel == {"-"}:
        return [], why
    smoke = set(verifica.SMOKE) | set(verifica.SMOKE_CHROMIUM_LINUX)
    left = sorted(n for n in sel if n not in smoke and not verifica.NEEDS.get(n, set()) & ELSEWHERE)
    gone = len(sel) - len(left)
    note = f"{len(sel)} e2e cover the changed files, {gone} of them run in the smoke round, sito-e2e or the night"
    return left, (note + (f": running {len(left)}" if left else ": nothing left to run"))


def changed(base: str) -> set[str]:
    """Paths changed between `base` and HEAD (a rename counts both paths)."""
    out = subprocess.run(["git", "diff", "--name-only", "--no-renames", "-z", base, "HEAD"], check=True, capture_output=True, encoding="utf-8").stdout
    return {f for f in out.split("\0") if f}


def main(argv: list[str]) -> int:
    if argv[:1] == ["--verdi"]:
        print(f"{write_green([Path(a) for a in argv[1:]])} green tests written")
        return 0
    if not argv:
        tests, why = [], "no base commit (not a pull request): nothing to choose"
    else:
        tests, why = pick(changed(argv[0]))
    print(why)
    for n in tests:
        print(f"  {n}")
    out = os.environ.get("GITHUB_OUTPUT")
    if out:
        with open(out, "a", encoding="utf-8") as f:
            f.write(f"e2e={','.join(tests)}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
