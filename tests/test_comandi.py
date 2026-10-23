"""Registry of the actions that change state (mzlab/web/comandi.js): well-formed table, handlers and call sites with registered ids, no assignment to a
registered field outside the files the action lists, the ratchet of the actions not undoable yet, and docs/azioni.md up to date."""
import re
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
sys.path.insert(0, str(ROOT / "tools"))
pytestmark = pytest.mark.skipif(not shutil.which("node"), reason="node not available")
import doc_comandi  # noqa: E402

DA_PORTARE_MAX = 2      # only goes down: porting an action lowers it
ANNULLA = {"istantanea", "comando", "tolto", "layout", "flow", "draw", "da-portare", "nessuna"}
CAMPI_SOLI = {"istantanea", "comando"}
FONTI = [f for f in WEB.glob("*.js") if f.name not in ("comandi.js", "storia.js")]


def righe():
    return doc_comandi.tabella()


def test_tabella_ben_formata():
    ids = [r["id"] for r in righe()]
    assert len(ids) == len(set(ids)), "duplicate id"
    it, en = doc_comandi.etichette("it"), doc_comandi.etichette("en")
    for r in righe():
        assert re.fullmatch(r"[a-z]+(\.[a-zA-Z]+)+", r["id"]), r["id"]
        assert r["annulla"] in ANNULLA, r["id"]
        assert r["nome"] in it and r["nome"] in en, f"{r['id']}: label {r['nome']} missing in lang/"
        assert (WEB / r["modulo"]).exists() or (ROOT / "flow" / "lazy" / r["modulo"]).exists(), f"{r['id']}: module {r['modulo']}"
        assert r["modulo"] in r["dove"], f"{r['id']}: modulo not in dove"
        if r["annulla"] == "nessuna":
            assert r.get("motivo"), f"{r['id']}: a non-undoable action says why"


def test_gestori_e_chiamate():
    ids = {r["id"] for r in righe()}
    comandi = {r["id"]: r for r in righe() if r["annulla"] == "comando"}
    visti = set()
    for f in FONTI:
        t = f.read_text(encoding="utf-8")
        for m in re.finditer(r'COMANDI\.gestore\("([^"]+)"', t):
            assert m.group(1) in comandi, f"{f.name}: handler of {m.group(1)} which is not a command of the table"
            assert f.name == comandi[m.group(1)]["modulo"], f"{f.name}: the handler of {m.group(1)} lives in {comandi[m.group(1)]['modulo']}"
            visti.add(m.group(1))
        for m in re.finditer(r'COMANDI\.(?:fai|inizia)\("([^"]+)"', t):
            assert m.group(1) in comandi, f"{f.name}: COMANDI.fai of {m.group(1)} which is not a command of the table"
            assert f.name in comandi[m.group(1)]["dove"], f"{f.name} runs {m.group(1)} but is not in its `dove`"
    assert visti == set(comandi), f"commands without handler: {set(comandi) - visti}"
    assert ids >= set(comandi)


def test_nessun_campo_registrato_fuori_dai_suoi_file():
    """A state field of a registered action is assigned only in the files the action lists: a new place that changes it must be added to the table
    (and go through the undo) first."""
    bad = []
    for r in righe():
        if r["annulla"] not in CAMPI_SOLI or not r["campi"]:
            continue
        pat = re.compile(r"\b(?:p|q|xp|s1|c\.p|panel)\.(" + "|".join(map(re.escape, r["campi"])) + r")\s*(?:=(?!=)|\.push\()")
        for f in FONTI:
            if f.name in r["dove"] or f.name in ("explore.js",):          # explore.js restores a saved session
                continue
            for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
                if pat.search(line) and not line.lstrip().startswith("//"):
                    bad.append(f"{f.name}:{n} changes a field of {r['id']}")
    assert not bad, "\n".join(bad[:20])


def test_da_portare_solo_in_calo():
    n = sum(r["annulla"] == "da-portare" for r in righe())
    assert n <= DA_PORTARE_MAX, f"{n} actions not undoable (max {DA_PORTARE_MAX}): register the new action with its undo"


def test_pagina_aggiornata():
    assert doc_comandi.OUT.exists() and doc_comandi.OUT.read_text(encoding="utf-8") == doc_comandi.pagina(), "run python3 tools/doc_comandi.py"
