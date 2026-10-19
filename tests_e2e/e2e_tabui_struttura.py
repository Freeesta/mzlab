"""Tabui, the «Structure» tab of an open row (TP_Mine/js/47_tp_struttura.js, `TPSTRUCT`, drawn through `TPMOL`): fed with synthetic `interp/1` rows (no wasm needed), it shows the head sentence, the product on its parent
(a certain site as a structure, a ring as one bond from its centre, another zone or the whole parent in brackets), the table of the classes, «Open in Draw» (the V3000 loads in the Draw tab) and the Identity text
(«+O», «+O (ring A?)», «+O (?)»). Checked in the light and dark themes, at 390 px without horizontal scroll of the page, with reduced motion, and with axe. Run from the repo root: python3 tests_e2e/e2e_tabui_struttura.py"""
import os, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:700]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else: subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
errs = []
SMI = "c1ccccc1CCO"                                                   # ring atoms 0-5, chain 6, 7, 8 (O)
def cls(i, atoms, status, sup, group=None, kind="ring", n=2, a=0): return {"id": i, "kind": kind, "status": status, "atoms": atoms, "group": group, "n_support": n, "n_against": a, "support": sup}
def drawing(mode, **kw):
    d = {"mode": mode, "atoms": [], "ends": [], "lost": [], "sub": None, "highlight": [], "kept": [], "zones": [], "added": "O", "kind": "oxidation", "delta": "+O", "label": "+O", "uncertainty": "none", "fallback": "bracket"}
    d.update(kw); return d
def row(conf, drw, classes, region=None, reasons=None):
    return {"schema": "interp/1", "id": 1, "parent": {"smiles": SMI, "ion_formula": "C8H11O+", "mz": 123.08}, "change": {"delta": "+O"}, "tree": {"n_same": 3, "n_shifted": 2},
            "site": {"transform": "+O", "confidence": conf, "reasons": reasons or [], "source": "cuts", "informative": 4, "margin": 3.0, "classes": classes, "region": region, "retained": None, "proposal": {"sublevel": "3a", "why": ""}}, "drawing": drw, "hidden": None}
ROWS = {
    "site": row("site", drawing("structure", atoms=[7], highlight=[7]), [cls(1, [7], "supported", 1.0, kind="chain")]),
    "region": row("region", drawing("multicenter", atoms=[0, 1, 2, 3, 4, 5], ends=[0, 1, 2, 3, 4, 5], uncertainty="zone"), [cls(1, [1], "supported", 0.5, "ring A"), cls(2, [2], "supported", 0.5, "ring A")], {"name": "ring A", "atoms": [0, 1, 2, 3, 4, 5]}),
    "uncertain": row("uncertain", drawing("bracket", atoms=list(range(9)), uncertainty="zone", zones=[{"atoms": [0, 1, 2, 3, 4, 5], "support": 0.6, "class": 1}, {"atoms": [6, 7], "support": 0.3, "class": 2}, {"atoms": [8], "support": 0.1, "class": 3}]),
                     [cls(1, [1], "supported", 0.6, "ring A"), cls(2, [7], "supported", 0.3, "chain A", "chain"), cls(3, [8], "supported", 0.1, None, "hetero"), cls(4, [6], "excluded", 0.0, None, "chain", 0, 2)]),
    "cannot": row("cannot_say", drawing("bracket", atoms=list(range(9)), uncertainty="whole"), [], None, ["no_ms2"]),
}
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(scheme="light", width=1100):
            c = b.new_context(viewport={"width": width, "height": 900}, reduced_motion="reduce", color_scheme=scheme); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang=en"); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_function("window.TPSTRUCT && window.TPMOL && window.TPRIGA", timeout=60000)
            pg.evaluate("() => { const d = document.createElement('div'); d.className = 'tp'; d.id = 'tps-host'; d.style.cssText = 'position:fixed;left:0;top:0;width:100%;max-width:560px;background:var(--panel);color:var(--ink);z-index:99999;padding:8px;box-sizing:border-box'; d.innerHTML = '<div id=\"tps-panel\"></div>'; document.body.appendChild(d); }")
            return pg
        pg = fresh()
        def show(pg, name, rows=ROWS):
            pg.evaluate("async r => { const p = document.getElementById('tps-panel'); p.innerHTML = ''; await TPRIGA.fill(p, 'structure', { interp: r, missing: {} }); await new Promise(f => setTimeout(f, 50)); }", rows[name])
            pg.wait_for_selector("#tps-panel .tps-head", timeout=20000)
        def head(pg): return pg.inner_text("#tps-panel .tps-head")
        def count(pg, sel): return pg.evaluate("s => document.querySelectorAll('#tps-panel ' + s).length", sel)
        step("the row has a «Structure» tab after Quality", lambda: pg.evaluate("TPRIGA.tabs.map(t => t.key).join()") == "overview,samples,identity,quality,structure" or (_ for _ in ()).throw(AssertionError(pg.evaluate("TPRIGA.tabs.map(t => t.key).join()"))))
        def site_():
            show(pg, "site"); assert head(pg) == "+O · Site: C8", head(pg)
            assert count(pg, ".mz-parentesi") == 0 and count(pg, ".mz-multicentro") == 0 and "OH" in pg.evaluate("[...document.querySelectorAll('#tps-panel svg text')].map(t => t.textContent)"), "not a plain structure"
            assert count(pg, "table.tpr tbody tr") == 1
        step("a certain site: «Site: C8», the full structure, one class", site_)
        def region_():
            show(pg, "region"); assert head(pg) == "+O · Region: ring A", head(pg)
            assert count(pg, ".mz-multicentro") == 1 and count(pg, ".mz-parentesi") == 0 and count(pg, "table.tpr tbody tr") == 2
        step("a ring: «Region: ring A» and one bond from the centre of the ring", region_)
        def unc():
            show(pg, "uncertain"); assert head(pg) == "+O · Position uncertain · 3 classes · relative support 0.6 / 0.3 / 0.1", head(pg)
            assert count(pg, ".mz-parentesi") == 1 and count(pg, "table.tpr tbody tr") == 4
            t = pg.inner_text("#tps-panel table.tpr"); assert "excluded" in t and "supported" in t, t
        step("uncertain: the head sentence with the support, brackets, the four classes with their status in words", unc)
        def cant():
            show(pg, "cannot"); assert head(pg) == "Cannot say: no MS2 in any file", head(pg)
            assert count(pg, ".mz-parentesi") == 1 and "Position not determined" in pg.inner_text("#tps-panel .tps-fig") and count(pg, "table.tpr") == 0
        step("cannot say: the reason in words, brackets on the whole parent", cant)
        def nothing():
            pg.evaluate("async () => { const p = document.getElementById('tps-panel'); p.innerHTML = ''; await TPRIGA.fill(p, 'structure', { interp: undefined, missing: { structure: 'the interpretation of the series is not built' } }); }")
            assert "the interpretation of the series is not built" in pg.inner_text("#tps-panel"), pg.inner_text("#tps-panel")
        step("without an interpretation the tab says why", nothing)
        def ident():
            r = pg.evaluate("rs => Object.fromEntries(Object.entries(rs).map(([k, v]) => [k, TPSTRUCT.identText(v)]))", ROWS)
            assert r == {"site": "+O", "region": "+O (ring A?)", "uncertain": "+O (?)", "cannot": "+O (?)"}, r
            assert pg.evaluate("TPSTRUCT.identText(null)") == ""
        step("Identity column: «+O», «+O (ring A?)», «+O (?)»", ident)
        def todraw():
            show(pg, "region"); pg.click("#tps-panel [data-tps=draw]")
            pg.wait_for_function("TPDraw.ready()", timeout=90000)
            K = "document.querySelector('#kframe').contentWindow.ketcher"
            pg.wait_for_function(f"{K}.editor.struct().atoms.size === 10", timeout=20000)
            assert pg.is_visible("#kframe")
        step("«Open in Draw» loads the drawing in the Draw tab", todraw)
        def figdata():
            pg2 = fresh(); show(pg2, "uncertain")
            assert pg2.evaluate("() => { const s = document.querySelector('#tps-panel .tps-fig svg'); return s.getAttribute('role') === 'img' && !!s.getAttribute('aria-label') && s.getAttribute('tabindex') === '0'; }")
        step("the figure is an img with a name, focusable, with its table of the data", figdata)
        def narrow():
            pn = fresh(width=390)
            for n in ROWS:
                show(pn, n); w = pn.evaluate("() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"); assert w[0] <= w[1], (n, w)
        step("390 px: no horizontal scroll of the page for any form", narrow)
        def dark():
            pd = fresh("dark"); show(pd, "uncertain")
            col = pd.evaluate("() => getComputedStyle(document.querySelector('#tps-panel .tps-head')).color")
            assert col and col != "rgb(0, 0, 0)", col
        step("dark theme draws the same tab", dark)
        AXE = lib.axe_path()
        def axe():
            for n in ROWS:
                show(pg, n); pg.add_script_tag(path=AXE) if not pg.evaluate("!!window.axe") else None
                v = pg.evaluate("axe.run(document.getElementById('tps-panel'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))")
                assert not v, (n, v)
        if AXE: step("axe: no serious or critical violation in any form", axe)
        else: print("SKIP axe-core not available")
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
if errs: print("pageerrors:", errs[:3])
sys.exit(1 if any(r != "ok" for _, r in steps) or errs else 0)
