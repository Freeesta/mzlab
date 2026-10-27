"""Flow form texts: the label shown for a parameter (x-label or title) has at most 8 words and its tooltip (x-tip or description) is one sentence of at most 20
words; the help of the CLI (title and description) is the one of origin/main. Parameters that still break the first rule are listed in EXCEPTIONS, which can only shrink."""
import json
import re
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
FILES = sorted(p.relative_to(ROOT).as_posix() for pat in ("docs/schemi/steps*.json", "mzfinder/steps-*.json") for p in ROOT.glob(pat))
MAX_LABEL, MAX_TIP = 8, 20


def _params(step):
    for name, p in step.get("params", {}).get("properties", {}).items():
        yield f"{step['id']}.{name}", p
    for alg in step.get("algorithms", []):
        for name, p in alg.get("params", {}).get("properties", {}).items():
            yield f"{step['id']}.{alg['id']}.{name}", p


def _steps(text):
    # steps-1.json style files are one JSON document with a "steps" list or a list of steps
    d = json.loads(text)
    return d["steps"] if isinstance(d, dict) and "steps" in d else d if isinstance(d, list) else [d]


def _words(s):
    return len(s.split())


def _one_sentence(s):
    return not re.search(r"[.!?]\s+[A-Z]", re.sub(r"\b(e\.g|i\.e)\.", "", s.strip().rstrip(".!?")) + " ")


def _breaks(p):
    label, tip = p.get("x-label") or p.get("title", ""), p.get("x-tip") or p.get("description", "")
    return _words(label) > MAX_LABEL or (tip != "" and (_words(tip) > MAX_TIP or not _one_sentence(tip)))


def _all():
    for f in FILES:
        for step in _steps((ROOT / f).read_text(encoding="utf-8")):
            yield from ((f"{f}:{k}", p) for k, p in _params(step))


# only the parameters of the steps the form shows with a short text are exempt from being listed: the rest are listed here, and the list can only shrink
EXCEPTIONS = set((ROOT / "tests" / "flow_tooltip_exceptions.txt").read_text().split()) if (ROOT / "tests" / "flow_tooltip_exceptions.txt").exists() else set()


def test_labels_and_tips_are_short():
    bad = {k for k, p in _all() if _breaks(p)}
    new = sorted(bad - EXCEPTIONS)
    assert not new, f"{len(new)} parameters have a label over {MAX_LABEL} words or a tip that is not one sentence of {MAX_TIP} words (add x-label/x-tip): {new[:10]}"
    fixed = sorted(EXCEPTIONS - bad)
    assert not fixed, f"remove from tests/flow_tooltip_exceptions.txt: {fixed[:10]}"


def _main(path):
    try:
        out = subprocess.run(["git", "show", f"origin/main:{path}"], cwd=ROOT, capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        pytest.skip("origin/main not available")
    return out


@pytest.mark.parametrize("path", FILES)
def test_cli_help_is_unchanged(path):
    try:
        old = {k: p for step in _steps(_main(path)) for k, p in _params(step)}
    except json.JSONDecodeError:
        pytest.skip("not in origin/main")
    new = {k: p for step in _steps((ROOT / path).read_text(encoding="utf-8")) for k, p in _params(step)}
    for k, p in old.items():
        if k in new:
            assert (new[k].get("title"), new[k].get("description")) == (p.get("title"), p.get("description")), k
