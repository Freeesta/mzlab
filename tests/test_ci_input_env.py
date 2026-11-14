"""No `run:` script of a workflow interpolates a dispatch input (`${{ inputs.* }}`, `${{ github.event.inputs.* }}`) straight into the shell: the value goes through `env:`."""
import re
from pathlib import Path

WF = Path(__file__).resolve().parent.parent / ".github" / "workflows"
BAD = re.compile(r"\$\{\{[^}]*\b(?:github\.event\.inputs|inputs)\.")
RUN = re.compile(r"^(\s*)(?:-\s+)?run:\s*(.*)$")


def run_scripts(text):
    """The text of every `run:` of a workflow: the rest of the line, plus the more indented lines of a `|` / `>` block."""
    lines = text.splitlines()
    for i, line in enumerate(lines):
        m = RUN.match(line)
        if not m:
            continue
        indent, out = len(m.group(1)), [m.group(2)]
        for nxt in lines[i + 1:]:
            if nxt.strip() and len(nxt) - len(nxt.lstrip()) <= indent:
                break
            out.append(nxt)
        yield "\n".join(out)


def test_no_run_block_interpolates_an_input():
    bad = [f"{f.name}: {m.group(0)}" for f in sorted(WF.glob("*.yml")) for s in run_scripts(f.read_text(encoding="utf-8")) for m in BAD.finditer(s)]
    assert not bad, bad


def test_the_scanner_sees_inline_and_block_runs():
    assert list(run_scripts("steps:\n  - run: a ${{ inputs.x }}\n  - name: n\n    run: |\n      b\n      c ${{ inputs.y }}\n    env:\n      Z: 1\n")) == ["a ${{ inputs.x }}", "|\n      b\n      c ${{ inputs.y }}"]
