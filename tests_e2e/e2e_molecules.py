"""molecules/1 in the page: the routes api/molecules/* answer from the Rust engine. The catalogue (static/molecole/catalogo.csv) is loaded with `carica`; the search finds a
name, a formula and a mass; an invented molecule of the user (a_json -> carica) is found and replaces nothing; da_csv/a_csv give the same CSV back; a wrong row gives its error;
MZLAB_LR_MISS stays empty. Needs the built site (tools/prepara_sito.py). Run from the repo root."""
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
        loaded = json.loads(call("carica", {"catalogo": cat}))
        assert loaded["errori"] == [] and loaded["totale"] == 1228, loaded
        hit = json.loads(call("cerca", {"q": "paracetamol"}))
        assert hit and hit[0]["nome"] == "acetaminophen" and hit[0]["formula"] == "C8H9NO2", hit[:1]
        assert any(h["nome"] == "acetaminophen" for h in json.loads(call("cerca", {"q": "C8H9NO2"})))
        by_mass = json.loads(call("cerca", {"q": "151.0633", "ppm": 10}))
        assert by_mass and by_mass[0]["nome"] == "acetaminophen" and by_mass[0]["punteggio"] < 10, by_mass[:1]
        print("OK  catalogue loaded (1228), found by name, formula and mass")

        csv = "name;molecular_formula;synonyms\nEsempina;C10H15NO;Ex-1\n"
        doc = call("da_csv", {"csv": csv, "file": "mine.csv"})
        assert json.loads(doc)["molecole"][0]["origine"]["tipo"] == "importata", doc
        assert call("carica", {"utente": doc}).count('"altri":1') == 1
        mine = json.loads(call("cerca", {"q": "esempina"}))
        assert mine and mine[0]["nome"] == "Esempina" and mine[0]["id"].startswith("u:"), mine[:1]
        again = call("a_csv", json.loads(doc))
        assert call("a_csv", json.loads(call("da_csv", {"csv": again, "file": "mine.csv"}))) == again
        bad = json.loads(call("valida", {"csv": "name;molecular_formula\nOk;CH4\nBad;Xx9\n"}))
        assert [(e["riga"], e["chiave"]) for e in bad["errori"]] == [(3, "error.molecules.badFormula")], bad
        print("OK  a molecule of the user, the CSV round trip and the error of a wrong row")
        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        assert not [e for e in r.errs if e[0] in ("pageerror", "console.error")], r.errs[:3]
        print("OK  MZLAB_LR_MISS empty")
finally:
    r.close()
