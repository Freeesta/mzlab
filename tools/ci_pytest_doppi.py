"""Test files that rust.yml (job rust-wasm) already runs whenever it runs, so the 3.11 pytest job of test.yml can leave them out.
`python3 tools/ci_pytest_doppi.py <base sha>` writes `ignore=--ignore=tests/a.py ...` (empty when the files must run here) to $GITHUB_OUTPUT.
rust-wasm runs when the diff matches the Rust path filter of rust.yml (read from there, so the two never drift); no base or a git failure keeps every test (3.11 job only)."""
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOPPI = ["tests/test_guardie_memoria.py", "tests/test_feat_wasm.py", "tests/test_flow_determinismo.py"]


def filtro_rust() -> re.Pattern:
    """The path filter of the `changes` job of rust.yml (the `grep -qE '^(...)'` on the diff)."""
    testo = (ROOT / ".github/workflows/rust.yml").read_text(encoding="utf-8")
    m = re.search(r"git diff --name-only .*grep -qE '([^']+)'", testo)
    if not m:
        raise ValueError("path filter not found in rust.yml")
    return re.compile(m.group(1))


def rust_gira(files: list[str], filtro: re.Pattern) -> bool:
    return any(filtro.match(f) for f in files)


def da_saltare(files: list[str], filtro: re.Pattern) -> list[str]:
    return DOPPI if rust_gira(files, filtro) else []


def main(argv: list[str]) -> int:
    saltare: list[str] = []
    try:
        if argv and argv[0] and not re.fullmatch(r"0+", argv[0]):
            out = subprocess.run(["git", "diff", "--name-only", "--no-renames", "-z", argv[0], "HEAD"], check=True, capture_output=True, encoding="utf-8").stdout
            saltare = da_saltare([f for f in out.split("\0") if f], filtro_rust())
    except (subprocess.CalledProcessError, OSError, ValueError):
        saltare = []
    line = "ignore=" + " ".join(f"--ignore={f}" for f in saltare)
    print(line)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write(line + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
