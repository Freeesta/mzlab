"""Disegno, evidence colours (`hilite/1` of the molkit .wasm, `disegno-evidenza.js`): `TPDraw.highlight(layers)` on the synthetic 4-chlorophenol of
tests/dati/hilite_disegno.json gives the `hilite/1` of the fixture (the bytes `mzlab-cli mol-hilite` and the app give), in the LR world only on the
selected atoms and with nothing selected nowhere; in the HR world every layer is painted with Ketcher's highlights in the colours of the CSS
variables, listed in a legend with words and in a data table (TDATI); an edit of the drawing and «Remove colours» (keyboard, 24 px) take them off.
Run from the repo root after tools/build_site.py and tools/build_feat_wasm.sh site/static (molkit); SKIP without site/static/wasm-feat."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
HERE = os.path.dirname(os.path.abspath(__file__))
if not os.path.exists(os.path.join(HERE, "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8996, wd="/tmp/wd_disegno_evidenza")
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
FX = json.load(open(os.path.join(HERE, "..", "tests", "dati", "hilite_disegno.json"), encoding="utf-8"))
MOL, LAYERS = FX["request"]["molfile"], FX["request"]["layers"]
# draws the fixture, selects `sel` (Ketcher ids = molfile order), asks the colours; resolves with the hilite/1 answer
ASK = """async ([mol, layers, sel]) => {
  const K = %s; await K.setMolecule(mol); await new Promise(r => setTimeout(r, 300));
  const ids = [...K.editor.struct().atoms.keys()];
  K.editor.selection(sel.length ? { atoms: sel.map(i => ids[i]) } : null);
  return await TPDraw.highlight(layers);
}""" % KJ
PAINTED = f"(() => {{ const h = {KJ}.editor.highlights; return h && h.getAll ? h.getAll().map(x => x.highlight || x).map(x => [x.color, (x.atoms || []).length, (x.bonds || []).length]) : null; }})()"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)

        def lr_nothing_selected():
            got = pg.evaluate(ASK, [MOL, LAYERS, []])
            assert [l.get("skip") for l in got["layers"]] == ["lr", "lr", "lr", "noMatch"], got
            assert pg.evaluate(PAINTED) == [], pg.evaluate(PAINTED)
            assert pg.locator("#ev-card").is_visible()
            assert pg.inner_text("#ev-status") == pg.evaluate("I18N.t('draw.ev.done.lr')")
            assert pg.get_attribute("#ev-status", "role") == "status"
        step("LR world, nothing selected: no colour, the legend says why", lr_nothing_selected)

        def lr_selected():
            got = pg.evaluate(ASK, [MOL, LAYERS, [0, 1]])
            assert got == json.loads(FX["hilite_lr"]), got
            assert len(pg.evaluate(PAINTED)) == 3, pg.evaluate(PAINTED)
        step("LR world, Cl and C1 selected: the hilite/1 of the fixture (= mol-hilite and the app), three layers painted", lr_selected)

        def hr_all():
            pg.evaluate("window.HR = true; window.__on = BANCO.on; BANCO.on = () => true")
            try: got = pg.evaluate(ASK, [MOL, LAYERS, []])
            finally: pg.evaluate("BANCO.on = window.__on; window.HR = false")
            assert got == json.loads(FX["hilite"]), got
            want = pg.evaluate("['moiety', 'sites', 'cuts'].map(k => getComputedStyle(document.documentElement).getPropertyValue('--ev-' + k).trim())")
            assert all(want), want
            assert pg.evaluate(PAINTED) == [[want[0], 7, 7], [want[1], 2, 0], [want[2], 0, 1]], (pg.evaluate(PAINTED), want)
            frames(pg, 6)                                                                       # painting fires no "change" that would clear it
            assert len(pg.evaluate(PAINTED)) == 3, pg.evaluate(PAINTED)
            assert pg.inner_text("#ev-status") == pg.evaluate("I18N.t('draw.ev.painted', {n: 3, total: 4})")
            ev = "getComputedStyle(document.documentElement).getPropertyValue('--ev-moiety').trim()"
            theme = pg.evaluate("document.documentElement.getAttribute('data-theme')")
            pg.evaluate("document.documentElement.setAttribute('data-theme', 'dark')")
            try: assert pg.evaluate(ev).lower() == "#56b4e9", pg.evaluate(ev)                  # lighter on a dark panel
            finally: pg.evaluate(f"(t => t === null ? document.documentElement.removeAttribute('data-theme') : document.documentElement.setAttribute('data-theme', t))({json.dumps(theme)})")
            assert pg.get_attribute("#ev-legend", "aria-label") == pg.evaluate("I18N.t('draw.ev.table.caption')")
            rows = pg.evaluate("[...document.querySelectorAll('#ev-legend li')].map(li => li.innerText)")
            assert len(rows) == 4 and rows[0].startswith(pg.evaluate("I18N.t('draw.ev.kind.moiety')")), rows
            assert pg.evaluate("I18N.t('draw.ev.skip.noMatch')") in rows[3], rows
        step("HR world: every layer painted in the colour of its CSS variable, legend in words", hr_all)

        def table():
            pg.click("#ev-table")
            pg.wait_for_selector("dialog[open] table, .modal table", timeout=10000)
            assert pg.locator("table caption").filter(has_text=pg.evaluate("I18N.t('draw.ev.table.caption')")).count() >= 1
            assert pg.locator("dialog[open] tbody tr, .modal tbody tr").count() == 7 + 7 + 2 + 1 + 1
            pg.keyboard.press("Escape")
        step("the data table lists every coloured atom and bond (TDATI)", table)

        def clear_keyboard():
            b = pg.locator("#ev-clear"); bb = b.bounding_box(); assert bb["height"] >= 24 and bb["width"] >= 24, bb
            b.focus(); pg.keyboard.press("Enter")
            pg.wait_for_function(f"{PAINTED}.length === 0", timeout=10000)
            assert pg.inner_text("#ev-status") == pg.evaluate("I18N.t('draw.ev.cleared')")
        step("«Remove colours» by keyboard takes them off", clear_keyboard)

        def edit_clears():
            pg.evaluate(ASK, [MOL, LAYERS, [0, 1]])
            assert len(pg.evaluate(PAINTED)) == 3
            pg.evaluate(f"{KJ}.setMolecule('CCO')")
            pg.wait_for_function(f"{PAINTED}.length === 0", timeout=10000)
        step("an edit of the drawing takes the colours off (they belong to the old drawing)", edit_clears)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if not steps or any(s[1] != "ok" for s in steps) else 0)
