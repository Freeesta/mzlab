"""Decides whether a pull request touches only the Teoria chapters, so test.yml can skip the Rust/wasm toolchain and the full smoke round for it.
`python3 tools/ci_solo_teoria.py <base sha>` writes `solo=true|false` to $GITHUB_OUTPUT (and prints it); `solo` is true only when EVERY file changed between
<base sha> and HEAD is a chapter source (teoria/src/NN-*.md), a chapter page (teoria/NN-*.html) or teoria/indice-ricerca.js; one other file, or none, gives false.
`--pytest` / `--e2e` print the Teoria-only test selection that test.yml runs instead (space-separated)."""
import os
import re
import subprocess
import sys

ALLOWED = [re.compile(p) for p in (
    r"mzlab/web/teoria/src/\d\d-[a-z0-9-]+\.md",
    r"mzlab/web/teoria/\d\d-[a-z0-9-]+\.html",
    r"mzlab/web/teoria/indice-ricerca\.js",
)]
# the unit tests that guard the chapters, the Libro converter and the names that must never appear in the public repository
PYTEST = ["test_controlla_teoria", "test_esempi_anonimi", "test_composti", "test_indice_teoria", "test_libro", "test_libro_generato", "test_libro_importa",
          "test_nuovo_capitolo", "test_teoria_entry", "test_server_teoria", "test_strumenti_teoria"]
E2E = ["e2e_teoria_nav", "e2e_teoria_cerca", "e2e_teoria_barra", "e2e_teoria_scura"]


def solo(files: list[str]) -> bool:
    """True if there is at least one file and every file is on the Teoria list."""
    return bool(files) and all(any(rx.fullmatch(f) for rx in ALLOWED) for f in files)


def changed(base: str) -> list[str]:
    """Paths changed between `base` and HEAD; renames are reported as a deletion plus an addition, so the old path counts too."""
    out = subprocess.run(["git", "diff", "--name-only", "--no-renames", "-z", base, "HEAD"], check=True, capture_output=True, encoding="utf-8").stdout
    return [f for f in out.split("\0") if f]


def main(argv: list[str]) -> int:
    if argv[:1] == ["--pytest"]:
        print(" ".join(f"tests/{t}.py" for t in PYTEST))
        return 0
    if argv[:1] == ["--e2e"]:
        print(",".join(E2E))
        return 0
    # no base (a push to main) or a git failure: never reduce the checks
    try:
        result = bool(argv) and solo(changed(argv[0]))
    except (subprocess.CalledProcessError, OSError):
        result = False
    line = f"solo={'true' if result else 'false'}"
    print(line)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write(line + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
