"""Registry of the actions that change state (mzlab/web/commands.js): well-formed table, handlers and call sites with registered ids, no assignment to a
registered field outside the files the action lists, the ratchet of the actions not undoable yet, and docs/actions.md up to date."""
import re
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
sys.path.insert(0, str(ROOT / "tools"))
pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node not available")
import doc_commands  # noqa: E402

TO_PORT_MAX = 0      # only goes down: porting an action lowers it
UNDO = {"snapshot", "command", "removed", "layout", "flow", "draw", "to-port", "none"}
STANDALONE_FIELDS = {"snapshot", "command"}
SOURCES = [f for f in WEB.glob("*.js") if f.name not in ("commands.js", "history.js")]


def rows():
    return doc_commands.tabella()


def test_table_well_formed():
    ids = [r["id"] for r in rows()]
    assert len(ids) == len(set(ids)), "duplicate id"
    en = doc_commands.etichette("en")
    for r in rows():
        assert re.fullmatch(r"[a-z]+(\.[a-zA-Z]+)+", r["id"]), r["id"]
        assert r["undo"] in UNDO, r["id"]
        assert r["name"] in en, f"{r['id']}: label {r['name']} missing in lang/"
        assert (WEB / r["module"]).exists() or (ROOT / "flow" / "lazy" / r["module"]).exists(), f"{r['id']}: module {r['module']}"
        assert r["module"] in r["where"], f"{r['id']}: module not in where"
        if r["undo"] == "none":
            assert r.get("reason"), f"{r['id']}: a non-undoable action says why"


def test_handlers_and_calls():
    ids = {r["id"] for r in rows()}
    commands = {r["id"]: r for r in rows() if r["undo"] == "command"}
    seen = set()
    for f in SOURCES:
        t = f.read_text(encoding="utf-8")
        for m in re.finditer(r'COMMANDS\.handler\("([^"]+)"', t):
            assert m.group(1) in commands, f"{f.name}: handler of {m.group(1)} which is not a command of the table"
            assert f.name == commands[m.group(1)]["module"], f"{f.name}: the handler of {m.group(1)} lives in {commands[m.group(1)]['module']}"
            seen.add(m.group(1))
        for m in re.finditer(r'COMMANDS\.(?:run|begin)\("([^"]+)"', t):
            assert m.group(1) in commands, f"{f.name}: COMMANDS.run of {m.group(1)} which is not a command of the table"
            assert f.name in commands[m.group(1)]["where"], f"{f.name} runs {m.group(1)} but is not in its `where`"
    assert seen == set(commands), f"commands without handler: {set(commands) - seen}"
    assert ids >= set(commands)


def test_no_field_registered_outside_its_files():
    """A state field of a registered action is assigned only in the files the action lists: a new place that changes it must be added to the table
    (and go through the undo) first."""
    bad = []
    for r in rows():
        if r["undo"] not in STANDALONE_FIELDS or not r["fields"]:
            continue
        pat = re.compile(r"\b(?:p|q|xp|s1|c\.p|panel)\.(" + "|".join(map(re.escape, r["fields"])) + r")\s*(?:=(?!=)|\.push\()")
        for f in SOURCES:
            if f.name in r["where"] or f.name in ("explore.js",):          # explore.js restores a saved session
                continue
            for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
                if pat.search(line) and not line.lstrip().startswith("//"):
                    bad.append(f"{f.name}:{n} changes a field of {r['id']}")
    assert not bad, "\n".join(bad[:20])


def test_to_port_count_never_grows():
    n = sum(r["undo"] == "to-port" for r in rows())
    assert n <= TO_PORT_MAX, f"{n} actions not undoable (max {TO_PORT_MAX}): register the new action with its undo"


def test_page_up_to_date():
    assert doc_commands.OUT.exists() and doc_commands.OUT.read_text(encoding="utf-8") == doc_commands.pagina(), "run python3 tools/doc_commands.py"
