"""molecules/1 in the page: the routes api/molecules/* answer from the Rust engine. The catalogue (static/molecole/catalogo.csv) is loaded with `load`; the search finds a
name, a formula and a mass; an invented molecule of the user (to_json -> load) is found and replaces nothing; from_csv/to_csv give the same CSV back; a wrong row gives its error;
MZLAB_LR_MISS stays empty. Needs the built site (tools/prepara_sito.py). Run from the repo root."""
# covers: mzlab/web/molecole, mzlab/web/molecules-
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8851, wd="/tmp/wd_molecules")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#drop", timeout=120000)
        post = """async ([op, body]) => (await fetch('api/molecules/' + op, { method: 'POST', body: JSON.stringify(body) })).text()"""
        call = lambda op, body: pg.evaluate(post, [op, body])
        cat = pg.evaluate("fetch('static/molecole/catalogo.csv').then(r => r.text())")
        loaded = json.loads(call("load", {"catalog": cat}))
        assert loaded["errors"] == [] and loaded["total"] == 1228, loaded
        hrms = json.loads(call("list", {"catalog": "hrms", "adducts": ["[M+H]+", "[M+Na]+"]}))
        assert len(hrms) >= 645 and all(len(m["mz"]) == 2 for m in hrms)
        hit = json.loads(call("search", {"q": "paracetamol"}))
        assert hit and hit[0]["name"] == "acetaminophen" and hit[0]["formula"] == "C8H9NO2", hit[:1]
        assert any(h["name"] == "acetaminophen" for h in json.loads(call("search", {"q": "C8H9NO2"})))
        by_mass = json.loads(call("search", {"q": "151.0633", "ppm": 10}))
        assert by_mass and by_mass[0]["name"] == "acetaminophen" and by_mass[0]["score"] < 10, by_mass[:1]
        print("OK  catalogue loaded (1228), found by name, formula and mass")

        csv = "name;molecular_formula;synonyms\nEsempina;C10H15NO;Ex-1\n"
        doc = call("from_csv", {"csv": csv, "file": "mine.csv"})
        assert json.loads(doc)["molecules"][0]["origin"]["type"] == "imported", doc
        assert call("load", {"user": doc}).count('"others":1') == 1
        mine = json.loads(call("search", {"q": "esempina"}))
        assert mine and mine[0]["name"] == "Esempina" and mine[0]["id"].startswith("u:"), mine[:1]
        again = call("to_csv", json.loads(doc))
        assert call("to_csv", json.loads(call("from_csv", {"csv": again, "file": "mine.csv"}))) == again
        bad = json.loads(call("validate", {"csv": "name;molecular_formula\nOk;CH4\nBad;Xx9\n"}))
        assert [(e["line"], e["key"]) for e in bad["errors"]] == [(3, "error.molecules.badFormula")], bad
        print("OK  a molecule of the user, the CSV round trip and the error of a wrong row")
        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        assert not [e for e in r.errs if e[0] in ("pageerror", "console.error")], r.errs[:3]
        print("OK  MZLAB_LR_MISS empty")
finally:
    r.close()
