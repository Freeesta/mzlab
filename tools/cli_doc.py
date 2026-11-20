"""Writes the blocks of the agent docs that come from the command table of mzlab-cli: the command list of docs/agenti/cli.md and the `--json` list and
exit-code table and the renamed-command list of docs/agenti/cli-agenti.md (each between `<!--NAME-->` and `<!--/NAME-->`).
`python3 tools/cli_doc.py` rewrites them, `--check` only tells whether they are current (exit 1 if not). The binary is `$MZLAB_CLI`, else `cargo run -p mzlab-cli`."""
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# (document, block name, `help-md` argument)
BLOCKS = [
    ("docs/agenti/cli.md", "cli", []),
    ("docs/agenti/cli-agenti.md", "json", ["json"]),
    ("docs/agenti/cli-agenti.md", "exits", ["exits"]),
    ("docs/agenti/cli-agenti.md", "renames", ["renames"]),
]


def listing(arg: list[str]) -> str:
    cli = [os.environ["MZLAB_CLI"]] if os.environ.get("MZLAB_CLI") else ["cargo", "run", "-q", "-p", "mzlab-cli", "--locked", "--"]
    r = subprocess.run(cli + ["help-md"] + arg, cwd=ROOT, capture_output=True, text=True)
    if r.returncode:
        sys.exit(r.stderr[-1500:])
    return r.stdout


def replaced(text: str, name: str, block: str) -> str:
    begin, end = f"<!--{name}-->\n", f"<!--/{name}-->"
    head, rest = text.split(begin, 1)
    return head + begin + block + end + rest.split(end, 1)[1]


def main(argv: list[str]) -> int:
    stale = []
    texts: dict[str, str] = {}
    for doc, name, arg in BLOCKS:
        path = ROOT / doc
        old = texts.setdefault(doc, path.read_text(encoding="utf-8"))
        texts[doc] = replaced(texts[doc], name, listing(arg))
        if texts[doc] != old and doc not in stale:
            stale.append(doc)
    if not stale:
        return 0
    if "--check" in argv:
        print(f"{', '.join(stale)}: generated blocks are stale: run python3 tools/cli_doc.py")
        return 1
    for doc in stale:
        (ROOT / doc).write_text(texts[doc], encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
