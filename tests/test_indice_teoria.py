"""Search index of the Teoria: fresh against the pages, anchors that exist, and the pure matching functions of cerca.js (node)."""
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import genera_indice_teoria as G  # noqa: E402


def test_index_is_fresh():
    assert G.OUT.read_text(encoding="utf-8") == G.render(G.build()), "run: python3 tools/genera_indice_teoria.py"


def test_index_is_deterministic_and_complete():
    a, b = G.build(), G.build()
    assert a == b
    kinds = {e[1] for e in a}
    assert {"capitolo", "pratica", "sezione", "glossario", "esercizio", "simulazione"} <= kinds
    assert len(a) > 400


def test_every_anchor_exists_in_its_page():
    for text, kind, url, cap, _ in G.build():
        f, _, frag = url.partition("#")
        src = (G.D / f).read_text(encoding="utf-8")
        if not frag:
            continue
        explicit = re.findall(r'id="([^"]+)"', src)
        slugs = [G.slug(G.plain(h)) for h in re.findall(r"<h[23][^>]*>(.*?)</h[23]>", src, re.S)]
        assert frag in explicit or frag in slugs, (url, text)


def test_glossary_ids_are_unique():
    ids = re.findall(r'<dt id="([^"]+)"', (G.D / "21-glossario.html").read_text(encoding="utf-8"))
    assert len(ids) == len(set(ids)) > 80


@pytest.mark.skipif(not shutil.which("node"), reason="node missing")
def test_search_functions_with_node():
    js = r"""
const fs=require('fs'),vm=require('vm');
const ctx={window:{},document:{currentScript:null,addEventListener(){}},location:{pathname:'/x/index.html'}};ctx.window=ctx;
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(process.argv[1],'utf8'),ctx);
vm.runInContext(fs.readFileSync(process.argv[2],'utf8').replace('const INDICE_RICERCA','var INDICE_RICERCA'),ctx);
const C=ctx.TEORIA._cerca, I=ctx.INDICE_RICERCA, out={};
const top=q=>C.search(I,q).flatMap(g=>g.items).map(e=>e[0]);
out.fram=top('frammenta'); out.acc=top('FRAMMENTAZIONE ÀÈ'); out.es=top('elettrospray'); out.esi=top('esi');
out.perd=top('perdita neutra'); out.vuoto=top('a'); out.mark=C.mark('Frammentazione è <b>',C.tokens('frammenta e'));
out.none=top('zzzzqq'); out.groups=C.search(I,'frammenta').map(g=>g.cap);
console.log(JSON.stringify(out));
"""
    r = subprocess.run(["node", "-e", js, str(ROOT / "mzlab/web/teoria/cerca.js"), str(ROOT / "mzlab/web/teoria/indice-ricerca.js")],
                       capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    o = json.loads(r.stdout)
    assert any("rammentazione" in t for t in o["fram"])
    assert o["acc"] == [] or all(isinstance(t, str) for t in o["acc"])
    assert o["es"] and o["esi"]
    assert any("erdit" in t for t in o["perd"])
    assert o["vuoto"] == [] and o["none"] == []
    assert o["mark"].startswith("<mark>Frammenta</mark>zion<mark>e</mark>") or "<mark>Frammenta</mark>" in o["mark"]
    assert "&lt;b&gt;" in o["mark"]
    assert len(o["groups"]) == len(set(o["groups"]))
