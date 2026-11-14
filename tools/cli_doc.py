"""Writes the command list of docs/agenti/cli.md (between `<!--cli-->` and `<!--/cli-->`) from the command table of mzlab-cli.
`python3 tools/cli_doc.py` rewrites it, `--check` only tells whether it is current (exit 1 if not). The binary is `$MZLAB_CLI`, else `cargo run -p mzlab-cli`."""
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs/agenti/cli.md"
BEGIN, END = "<!--cli-->\n", "<!--/cli-->"


def listing() -> str:
    cli = [os.environ["MZLAB_CLI"]] if os.environ.get("MZLAB_CLI") else ["cargo", "run", "-q", "-p", "mzlab-cli", "--locked", "--"]
    r = subprocess.run(cli + ["help-md"], cwd=ROOT, capture_output=True, text=True)
    if r.returncode:
        sys.exit(r.stderr[-1500:])
    return r.stdout


def replaced(text: str, block: str) -> str:
    head, rest = text.split(BEGIN, 1)
    return head + BEGIN + block + END + rest.split(END, 1)[1]


def main(argv: list[str]) -> int:
    old = DOC.read_text(encoding="utf-8")
    new = replaced(old, listing())
    if new == old:
        return 0
    if "--check" in argv:
        print("docs/agenti/cli.md: the command list is stale: run python3 tools/cli_doc.py")
        return 1
    DOC.write_text(new, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
