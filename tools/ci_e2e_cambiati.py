"""Chooses the e2e of the job `e2e-cambiati` of test.yml: the tests that cover the files a pull request changes, minus what other jobs already run.
`python3 tools/ci_e2e_cambiati.py <base sha>` prints what it chose and why, and appends `e2e=<names, comma-separated>` to $GITHUB_OUTPUT (empty: run nothing).
The map from files to tests is `changed_tests` of tools/verifica.py (its AREE); the smoke tests (job `browser`) and the tests that need the CI .wasm build, a .dam
or the lab data (sito-e2e and notte.yml) are left out. A core file selects every e2e: that is the complete round of notte.yml, so this job runs none."""
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verifica  # noqa: E402

ELSEWHERE = {"sito", "dam", "veri"}


def pick(files: set[str]) -> tuple[list[str], str]:
    """(tests to run, sorted; the plain reason) for the changed files."""
    sel, why = verifica.changed_tests(files)
    if sel is None:
        return [], f"{why}: this job runs none, the complete round of notte.yml runs them all"
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
