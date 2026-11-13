"""Disegno, structure check (`check/1` of the molkit .wasm): a pentavalent carbon is a valence mistake, a neutral four-bonded nitrogen asks for a + charge
and the same nitrogen drawn as N+ is fine, and common groups (sulfone, nitro, phosphate, DMSO, pyridine, carboxylic acid, azide) raise no alarm.
A remark selects its atoms on the canvas and «Perché?» opens its three parts; draw -> Check -> remark -> Why? is three clicks; auto-run is off by default.
Run from the repo root after tools/build_site.py and tools/build_feat_wasm.sh site/static (molkit); SKIP without site/static/wasm-feat."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
if not os.path.exists(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8996, wd="/tmp/wd_disegno_verifica")
KJ = "document.querySelector('#kframe').contentWindow.ketcher"


def molfile(els, bonds, charges=()):
    """A V2000 molfile: elements on a line, bonds as (a, b, type) 1-based, charges as (atom, q)."""
    s = "\n  test\n\n%3d%3d  0  0  0  0  0  0  0  0999 V2000\n" % (len(els), len(bonds))
    s += "".join("%10.4f%10.4f%10.4f %-3s 0  0  0  0  0  0  0  0  0  0  0  0\n" % (x, y, 0, e) for e, (x, y) in zip(els, [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1), (0.8, 0.8)]))
    s += "".join("%3d%3d%3d  0  0  0  0\n" % b for b in bonds)
    s += "".join("M  CHG  1 %3d %3d\n" % c for c in charges)
    return s + "M  END"


PENTA = molfile(["C"] * 6, [(1, k, 1) for k in range(2, 7)])
AMMONIUM = molfile(["N", "C", "C", "C", "C"], [(1, k, 1) for k in range(2, 6)])
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready()) && !!document.querySelector('#chk-run')", timeout=60000)

        def check(mol):
            """draw `mol`, press the button, wait for a final status; returns the remark codes"""
            pg.evaluate(f"{KJ}.setMolecule({json.dumps(mol)})"); frames(pg, 4)
            busy = pg.evaluate("I18N.t('draw.check.busy')")
            pg.click("#chk-run")
            pg.wait_for_function(f"(() => {{ const s = document.querySelector('#chk-status').textContent; return s && s !== {json.dumps(busy)}; }})()", timeout=30000)
            return pg.evaluate("[...document.querySelectorAll('#chk-list li')].map(li => li.dataset.code)")

        step("auto-run is off by default", lambda: (lambda c: None if not c else (_ for _ in ()).throw(AssertionError("checked")))(pg.is_checked("#chk-auto")))

        def penta():
            codes = check(PENTA)
            assert codes == ["valence"], (codes, pg.inner_text("#chk-card"))
            assert pg.get_attribute("#chk-status", "role") == "status"
            li = pg.locator("#chk-list li").first
            assert "Errore" in li.inner_text() and "C1" in li.inner_text(), li.inner_text()       # the level and the atom in words, not only a colour
            li.locator(".chk-sel").click()                                                       # click 2 (after Check): the remark selects its atom
            pg.wait_for_function(f"(({KJ}.editor.selection() || {{}}).atoms || []).length === 1", timeout=10000)
            why = li.locator(".chk-why")
            assert why.get_attribute("aria-expanded") == "false"
            why.click()                                                                          # click 3: Why?
            assert why.get_attribute("aria-expanded") == "true"
            box = li.locator(".chk-x"); assert box.is_visible()
            text = box.inner_text()
            for part in ("draw.check.part.what", "draw.check.part.why", "draw.check.part.next", "draw.check.valence.why"):
                want = pg.evaluate(f"I18N.t({json.dumps(part)})")
                assert want.split("{")[0][:20] in text, (part, text)
            bb = why.bounding_box(); assert bb["width"] >= 24 and bb["height"] >= 24, bb
        step("pentavalent carbon: one valence remark, selects C1, Why? opens problem / why / next step (3 clicks)", penta)

        def menu_why():
            box = pg.locator("#chk-list li .chk-x").first
            pg.locator("#chk-list li .chk-why").first.click(); assert box.is_hidden()    # closed again; C1 is still selected
            OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
            for _ in range(20):                                          # one right click at a time: a newer one replaces the menu of an older one still filling
                pg.evaluate(OPEN)
                try: pg.wait_for_selector("#ctx:not([hidden]) [data-row=check-why]", timeout=3000); break
                except Exception: pass
            row = pg.locator("#ctx [data-row=check-why]")
            assert row.count() == 1 and row.inner_text().startswith(pg.evaluate("I18N.t('draw.check.ctx', {what: ''})").strip()), pg.inner_text("#ctx")
            row.click()
            assert box.is_visible() and pg.locator("#chk-list li .chk-why").first.get_attribute("aria-expanded") == "true"
            assert pg.evaluate("document.activeElement && document.activeElement.classList.contains('chk-why')")
        step("right click on the selected atom: a «Why?» row opens the same three parts in the card", menu_why)

        def ammonium():
            assert check(AMMONIUM) == ["valenceCharge"], pg.inner_text("#chk-card")
            pg.locator("#chk-list li .chk-why").first.click()
            assert "+1" in pg.locator("#chk-list li .chk-x").first.inner_text()
            assert check("C[N+](C)(C)C") == [], pg.inner_text("#chk-card")
            assert pg.inner_text("#chk-status") == pg.evaluate("I18N.t('draw.check.ok')")
        step("N with four bonds and no charge asks for +1; drawn as N+ it is fine", ammonium)

        def groups():
            for smi in ("CS(=O)(=O)C", "C[N+](=O)[O-]", "COP(=O)(OC)OC", "CS(=O)C", "c1ccncc1", "CC(=O)O", "CN=[N+]=[N-]", "c1ccc2[nH]ccc2c1"):
                codes = check(smi)
                assert codes == [], (smi, codes, pg.inner_text("#chk-card"))
        step("sulfone, nitro, phosphate, DMSO, pyridine, acetic acid, azide, indole: no remark", groups)

        def pieces_and_empty():
            assert check("CC(=O)[O-].[Na+]") == ["pieces"], pg.inner_text("#chk-card")
            assert "Nota" in pg.inner_text("#chk-list li")
            pg.evaluate(f"{KJ}.setMolecule('')"); frames(pg, 4)
            pg.click("#chk-run")
            pg.wait_for_function("document.querySelector('#chk-status').textContent === I18N.t('draw.check.empty')", timeout=10000)
        step("a salt drawn as two pieces is a note; an empty canvas says so", pieces_and_empty)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if not steps or any(s[1] != "ok" for s in steps) else 0)
