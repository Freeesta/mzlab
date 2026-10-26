"""Disegno, Quick draw: the contaminant chooser (disegno-contaminanti.js). Two contaminants chosen with the keyboard fill the SMILES field with the structure of the molecules database,
Enter draws them next to each other; only contaminants with a KNOWN structure are listed (never an isomer series or a formula-only guess); an empty or unreadable list leaves a disabled chooser and no error.
Needs the built site (tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_disegno_contaminanti.py"""
import csv, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CAT = {r["name"]: r["smiles"] for r in csv.DictReader(open(os.path.join(ROOT, "mzlab/web/molecole/catalogo.csv"), encoding="utf-8"), delimiter=";")}
PICK = [("dibutyl phthalate", "dibutyl phthalate"), ("palmitic acid", "palmitic acid")]       # (typed to select it, name in the catalogue)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

r = Run(port=8936, wd="/tmp/wd_dcontam")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 900})
        errors = []
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.evaluate("() => { location.hash = '#/draw'; }")
        attendi(pg, "!!(window.TPDraw && TPDraw.ready()) && document.getElementById('kframe').classList.contains('kwait') === false", timeout=90000)
        attendi(pg, "!document.getElementById('ex-cont').disabled && document.getElementById('ex-cont').options.length > 1", timeout=90000)

        def listed():
            opts = pg.evaluate("() => [...document.getElementById('ex-cont').options].map(o => [o.textContent, o.value])")
            assert opts[0][1] == "", opts[0]
            assert len(opts) - 1 >= 5, "too few contaminants with a structure: %d" % (len(opts) - 1)
            assert all(v for _, v in opts[1:]), "an entry without SMILES is listed"
            assert not any("series" in t.lower() or "isomer" in t.lower() or " n=" in t for t, _ in opts[1:]), "a series or an isomer mix is listed"
            tot = pg.evaluate("async () => (await (await fetch('api/contaminants')).json()).lists[0].items.map(i => i.id).filter((x, i, a) => a.indexOf(x) === i).length")
            print("STRUCTURE %d/%d" % (len(opts) - 1, tot))
        step("only contaminants with a known structure are listed", listed)

        def named():
            assert pg.evaluate("() => document.querySelector('label[for=ex-cont]').textContent.trim().length > 0")
            assert pg.evaluate("() => document.getElementById('ex-cont').getBoundingClientRect().height") >= 24
        step("the chooser has a visible label and a 24 px target", named)

        def keyboard():
            for typed, cat_name in PICK:
                pg.focus("#ex-cont"); pg.keyboard.type(typed, delay=20)
                attendi(pg, "document.getElementById('ex-smi').value !== ''", timeout=10000)
                got = pg.input_value("#ex-smi")
                assert got == CAT[cat_name], (typed, got, CAT[cat_name])
                assert pg.evaluate("() => document.activeElement.id") == "ex-cont", "the focus stays on the chooser"
                pg.focus("#ex-smi"); pg.keyboard.press("Enter")
                attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
                assert pg.evaluate("() => document.getElementById('ex-cont').value") == "", "after drawing the chooser is back at its prompt"
            smi = pg.evaluate("async () => await TPDraw.smiles()")
            assert str(smi).count(".") == 1, "two structures drawn side by side: " + str(smi)
        step("two contaminants chosen from the keyboard fill the SMILES field and Enter draws them", keyboard)

        def edge():
            res = pg.evaluate("""async () => {
              const url = new URL("disegno-contaminanti.js", [...document.scripts].find(s => /draw\\.js/.test(s.src)).src).href;
              const t = k => k, out = [], orig = window.fetch;
              const run = async (stub) => {
                const mod = await import(url + "?case=" + out.length);
                const sel = document.createElement("select");
                window.fetch = (u, ...a) => String(u).includes("api/contaminants") ? stub() : orig.call(window, u, ...a);
                try { await mod.mountContaminants(sel, t, () => {}); } finally { window.fetch = orig; }
                return { disabled: sel.disabled, n: sel.options.length, text: sel.options[0].textContent };
              };
              out.push(await run(() => Promise.resolve(new Response(JSON.stringify({ lists: [] })))));
              out.push(await run(() => Promise.reject(new Error("offline"))));
              return out;
            }""")
            for x in res:
                assert x == {"disabled": True, "n": 1, "text": "draw.contam.none"}, res
        step("an empty or unreadable list leaves a disabled chooser", edge)
        step("no page error", lambda: (_ for _ in ()).throw(AssertionError(errors[:2])) if errors else None)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s[1][:4].ljust(5), s[0], "" if s[1] == "ok" else s[1])
print("FAIL" if bad else "OK")
sys.exit(1 if bad else 0)
