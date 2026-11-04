"""Disegno, Quick draw: the SMILES field is also a compound search (disegno-cerca.js over the molecules database). A name prefix lists suggestions, Enter or a click draws the
structure of the chosen one; a pasted SMILES is drawn without a search and an invalid one shows the message; the field is a labelled combobox (keyboard, 24 px).
Needs the built site (tools/build_site.py). Run from the repo root: python3 tests_e2e/e2e_disegno_cerca.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

r = Run(port=8937, wd="/tmp/wd_dcerca")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 900})
        errors = []
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.evaluate("() => { location.hash = '#/draw'; }")
        attendi(pg, "!!(window.TPDraw && TPDraw.ready()) && document.getElementById('kframe').classList.contains('kwait') === false", timeout=90000)
        attendi(pg, "document.getElementById('ex-smi').getAttribute('role') === 'combobox'", timeout=30000)
        smiles = lambda: pg.evaluate("async () => String(await TPDraw.smiles())")
        options = lambda: pg.evaluate("() => [...document.querySelectorAll('.molc [role=option]')].map(o => o.textContent)")
        def type_into(text):
            pg.fill("#ex-smi", ""); pg.focus("#ex-smi"); pg.keyboard.type(text, delay=20)

        def named():
            assert pg.evaluate("() => document.getElementById('ex-smi').getAttribute('aria-label').length > 0")
            assert pg.evaluate("() => document.getElementById('ex-smi').getBoundingClientRect().height") >= 24
            assert pg.evaluate("() => !document.getElementById('ex-cont')"), "the contaminant selector is gone"
        step("the field is a labelled combobox of at least 24 px and the old selector is gone", named)

        def keyboard():
            type_into("atraz")
            attendi(pg, "document.querySelectorAll('.molc [role=option]').length > 0", timeout=60000)
            assert options()[0].lower().startswith("atrazine"), options()
            assert pg.get_attribute("#ex-smi", "aria-expanded") == "true"
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
            s = smiles()
            assert "Cl" in s and "N" in s, s
        step("a name prefix lists suggestions and Enter draws the chosen structure", keyboard)

        def click():
            type_into("imidac")
            attendi(pg, "document.querySelectorAll('.molc [role=option]').length > 0", timeout=30000)
            pg.click(".molc [role=option]")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
            assert smiles().count(".") == 1, "two structures side by side: " + smiles()
        step("a click on a suggestion draws it next to the first structure", click)

        def plain():
            type_into("CCO")
            assert pg.get_attribute("#ex-smi", "aria-expanded") == "false" and not options(), "a SMILES is not searched"
            pg.keyboard.press("Enter")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
            assert smiles().count(".") == 2, smiles()
            pg.fill("#ex-smi", "c1ccccc1"); pg.click("#ex-load")
            attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
            assert smiles().count(".") == 3, smiles()
        step("a pasted SMILES is drawn without a search (Enter and the button)", plain)

        def invalid():
            pg.fill("#ex-smi", "C1CC"); pg.click("#ex-load")
            attendi(pg, "!document.getElementById('smi-warn').hidden", timeout=15000)
            assert "SMILES" in pg.inner_text("#smi-warn")
            assert pg.input_value("#ex-smi") == "C1CC", "an invalid SMILES stays in the field"
        step("an invalid SMILES shows the message", invalid)

        def accents():
            type_into("caffeìna")
            attendi(pg, "document.querySelectorAll('.molc [role=option]').length > 0", timeout=30000)
            assert any(o.lower().startswith("caffeine") for o in options()), options()
        step("accents in the typed name are ignored", accents)

        def nothing():
            type_into("zzzqqqxx")
            attendi(pg, "document.querySelector('.molc [role=status]').textContent.length > 0", timeout=30000)
            assert not options() and pg.get_attribute("#ex-smi", "aria-expanded") == "false"
        step("an unknown name shows no list and a status message", nothing)
        step("no page error", lambda: (_ for _ in ()).throw(AssertionError(errors[:2])) if errors else None)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s[1][:4].ljust(5), s[0], "" if s[1] == "ok" else s[1])
print("FAIL" if bad else "OK")
sys.exit(1 if bad else 0)
