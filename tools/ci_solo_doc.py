"""Decides whether a pull request changes only documents, so test.yml can skip the Rust/wasm toolchain, most pytest files and the e2e smoke round for it.
`python3 tools/ci_solo_doc.py <base sha>` runs after ci_solo_teoria.py in the `solo` step: when EVERY file changed between <base sha> and HEAD is a document it appends
`solo=true` and `kind=doc` to $GITHUB_OUTPUT; otherwise it writes nothing, so the output of ci_solo_teoria.py stands. A file outside the list, no file or no base means everything runs.
`--pytest [kind]` prints the pytest selection (the documents' tests for kind `doc`, the Teoria ones otherwise), space-separated."""
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ci_solo_teoria  # noqa: E402

# markdown at the root (AGENTS.md, CHANGELOG.md, ...), markdown under docs/, anything under .claude/; a .md elsewhere may be a test input (fixtures, crate READMEs)
ALLOWED = [re.compile(p) for p in (
    r"[^/]+\.md",
    r"docs/.+\.md",
    r"\.claude/.+",
)]
# the tests that read documents: forbidden names, version/CHANGELOG consistency, release gate, i18n catalogs, rules guards, and the selection tests themselves
PYTEST = ["test_esempi_anonimi", "test_versione", "test_cancello_rilascio", "test_i18n", "test_guardie_regole", "test_ci_solo_doc", "test_ci_solo_teoria"]


def solo(files: list[str]) -> bool:
    """True if there is at least one file and every file is on the document list."""
    return bool(files) and all(any(rx.fullmatch(f) for rx in ALLOWED) and ".." not in f.split("/") for f in files)


def main(argv: list[str]) -> int:
    if argv[:1] == ["--pytest"]:
        names = PYTEST if argv[1:2] == ["doc"] else ci_solo_teoria.PYTEST
        print(" ".join(f"tests/{t}.py" for t in names))
        return 0
    # no base (a push to main) or a git failure: never reduce the checks
    try:
        result = bool(argv) and solo(ci_solo_teoria.changed(argv[0]))
    except (subprocess.CalledProcessError, OSError):
        result = False
    print(f"doc={'true' if result else 'false'}")
    if result and os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write("solo=true\nkind=doc\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
