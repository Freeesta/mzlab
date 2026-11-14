"""Which browser tests `verifica.py --cambiati` (and the CI job e2e-cambiati) runs for a set of changed files: each tests_e2e/e2e*.py declares what it covers
in a header line `# covers: <prefix>, <prefix>`; this module reads those lines. verifica.py re-exports `changed_tests`."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
E2E = ROOT / "tests_e2e"

# --cambiati: which e2e cover which files. Each browser test declares its own areas in a header line `# covers: <prefix>, <prefix>` of
# tests_e2e/e2e*.py (a prefix is a file, a folder or a name stem); a new test edits only its own file. A changed file that matches no
# prefix (the core: explore.js, index.html, app.py, ...) means "all of them". Documents and the NO_E2E prefixes: no e2e.
NO_E2E = ("tools/build_site.py", "tools/controlla_i18n.py", "tools/genera_", "tools/metriche", "tools/prova_hr.py", "tests/")
COVERS = re.compile(r"(?:^|\s)# covers: *(.+?) *$", re.M)


def covers() -> list[tuple[str, str]]:
    """(prefix, e2e name) for every `# covers:` line in the first 40 lines of the tests_e2e/e2e*.py scripts."""
    out = []
    for f in sorted(E2E.glob("e2e*.py")):
        head = "\n".join(f.read_text(encoding="utf-8").split("\n", 40)[:40])
        out += [(p.strip(), f.stem) for m in COVERS.finditer(head) for p in m.group(1).split(",") if p.strip()]
    return out

DOCS = (".md", ".txt", "LICENSE", ".github/", ".gitignore", ".gitattributes", "pyproject.toml")


def changed_tests(files: set[str] | None = None) -> tuple[set[str] | None, str]:
    """E2E to run for the files changed against origin/main (committed or not): (names, explanation); None = all; {"-"} = none."""
    def git(*a):
        p = subprocess.run(["git", *a], cwd=ROOT, capture_output=True, text=True)
        return p.stdout.split("\n") if p.returncode == 0 else []
    if files is None:
        files = {f for f in git("diff", "--name-only", "origin/main...HEAD") + [l[3:] for l in git("status", "--porcelain")] if f.strip()}
    if not files:
        return {"-"}, "nessun file cambiato rispetto a origin/main: nessun e2e"
    tests: set[str] = set()
    table = covers()
    for f in sorted(files):
        if f.endswith(DOCS) or any(f.startswith(d) for d in DOCS if d.endswith("/")):
            continue
        if f.startswith("tests_e2e/"):
            n = Path(f).stem
            if n in ("lib", "lib_hr", "synth"):
                return None, f"{f} è un aiuto comune: tutti gli e2e"
            if n.startswith("e2e"):
                tests.add(n)
            continue
        hit = {n for pre, n in table if f.startswith(pre)}
        if not hit and not f.startswith(NO_E2E):
            return None, f"{f} è del nucleo: tutti gli e2e"
        tests |= hit
    if not tests:
        return {"-"}, "solo documenti o file senza e2e: nessun e2e"
    return tests | ({"e2e3"} if any(f.startswith("mzlab/") for f in files) else set()), "e2e: " + ", ".join(sorted(tests))
