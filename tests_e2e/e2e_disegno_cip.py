"""Disegno, stereodescriptors (`cip/1` of the molkit .wasm): the toggle is off by default and the card says that mass spectrometry cannot tell
stereoisomers apart; on, (R,E)-pent-3-en-2-ol drawn with its wedge lists «C2: R» and «C4=C5: E» as text rows (role=status) that select their
atoms; the mirror image reads S; a molecule without defined stereo says so; off again clears the list. Run from the repo root after
tools/build_site.py and tools/build_feat_wasm.sh site/static (molkit); SKIP without site/static/wasm-feat."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
HERE = os.path.dirname(os.path.abspath(__file__))
if not os.path.exists(os.path.join(HERE, "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8997, wd="/tmp/wd_disegno_cip")
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
# the fixture of the parity tests: hashed wedge from C2 to C1, trans double bond C4=C5; MIRROR makes it a solid wedge
MOL = json.load(open(os.path.join(HERE, "..", "tests", "dati", "cip_disegno.json"), encoding="utf-8"))["request"]["molfile"]
MIRROR = MOL.replace("  2  1  1  6", "  2  1  1  1")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready()) && !!document.querySelector('#cip-show')", timeout=60000)

        def rows(mol):
            """draw `mol`, wait for a final status of the card; returns the texts of its rows"""
            busy = pg.evaluate("I18N.t('draw.cip.busy')")
            pg.evaluate("document.querySelector('#cip-status').textContent = ''")
            pg.evaluate(f"{KJ}.setMolecule({json.dumps(mol)})")
            pg.wait_for_function(f"(() => {{ const s = document.querySelector('#cip-status').textContent; return s && s !== {json.dumps(busy)}; }})()", timeout=30000)
            return pg.evaluate("[...document.querySelectorAll('#cip-list li')].map(li => li.innerText)")

        def off_and_note():
            assert not pg.is_checked("#cip-show")
            assert pg.inner_text("#cip-note") == pg.evaluate("I18N.t('draw.cip.note')")
            assert "spettrometria di massa" in pg.inner_text("#cip-note")
            assert pg.get_attribute("#cip-status", "role") == "status"
        step("off by default; the note says mass spectrometry cannot tell stereoisomers apart", off_and_note)

        def labels():
            pg.evaluate(f"{KJ}.setMolecule({json.dumps(MOL)})"); frames(pg, 4)
            pg.focus("#cip-show"); pg.keyboard.press("Space")                                   # keyboard turns it on
            pg.wait_for_function("document.querySelectorAll('#cip-list li').length === 2", timeout=30000)
            got = pg.evaluate("[...document.querySelectorAll('#cip-list li')].map(li => li.innerText)")
            assert got == ["C2: R", "C4=C5: E"], (got, pg.inner_text("#cip-card"))
            assert pg.inner_text("#cip-status") == pg.evaluate("I18N.t('draw.cip.found', {n: 2})")
            b = pg.locator("#cip-list .cip-sel").nth(1)
            bb = b.bounding_box(); assert bb["height"] >= 24 and bb["width"] >= 24, bb
            b.click()
            pg.wait_for_function(f"(() => {{ const s = {KJ}.editor.selection() || {{}}; return (s.atoms || []).length === 2 && (s.bonds || []).length === 1; }})()", timeout=10000)
        step("on: C2 is R and C4=C5 is E, as text rows; a row selects its atoms and bond", labels)

        def mirror():
            got = rows(MIRROR)
            assert got == ["C2: S", "C4=C5: E"], (got, pg.inner_text("#cip-card"))
        step("the solid wedge (mirror image) reads S, the canvas is followed while on", mirror)

        def none():
            assert rows("CCO") == [], pg.inner_text("#cip-card")
            assert pg.inner_text("#cip-status") == pg.evaluate("I18N.t('draw.cip.none')")
        step("ethanol: no defined stereo, said in words", none)

        def off():
            pg.evaluate(f"{KJ}.setMolecule({json.dumps(MOL)})")
            pg.wait_for_function("document.querySelectorAll('#cip-list li').length === 2", timeout=30000)
            pg.click("#cip-show")
            assert pg.locator("#cip-list li").count() == 0 and pg.inner_text("#cip-status") == ""
        step("off again clears the list", off)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if not steps or any(s[1] != "ok" for s in steps) else 0)
