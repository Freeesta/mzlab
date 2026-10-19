"""Tabui, the drawing of a transformation product on its parent (TP_Mine/js/46_tp_mol.js, `TPMOL`): a certain site is a full structure (the new atom on the outside bisector), a substitution on
an aromatic ring is one dashed bond from the centre of the ring to «OH» (no circle on a whole ring), any other zone is in square brackets with the change («+O», «−2H») and the zone the fragments
point to is hatched; an undetermined position has the brackets on the whole parent and the words «Position not determined». Every drawing has role «img» and an aria-label; «Open in Draw» loads the V3000
(S-groups «MZLAB» / «MZLAB_ENDPTS») in the Draw tab, which reads them back. No wasm needed (drawings are fed by hand). Run from the repo root: python3 tests_e2e/e2e_tabui_tp_disegno.py"""
import json, os, subprocess, sys, tempfile, urllib.request
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
SMI = "c1ccccc1CCO"                                                   # ring atoms 0-5, chain 6 (CH2), 7 (CH2), 8 (O)
CASES = {
    "structure": {"mode": "structure", "delta": "+O", "label": "+O", "uncertainty": "none", "atoms": [7]},
    "multicenter": {"mode": "multicenter", "delta": "+O", "label": "+O", "uncertainty": "zone", "ends": [0, 1, 2, 3, 4, 5], "highlight": [], "region": "ring A"},
    "bracket": {"mode": "bracket", "delta": "+O", "label": "+O", "uncertainty": "zone", "atoms": [6, 7, 8], "highlight": [7]},
    "minus": {"mode": "bracket", "delta": "−2H", "label": "−2H", "uncertainty": "zone", "atoms": [6, 7]},
    "whole": {"mode": "bracket", "delta": "+O", "label": "+O", "uncertainty": "whole", "atoms": [0, 1, 2, 3, 4, 5, 6, 7, 8]},
    "uncertain": {"mode": "bracket", "delta": "+O", "label": "+O", "uncertainty": "zone", "atoms": [0, 1, 2, 3, 4, 5, 6, 7, 8],
                  "zones": [{"atoms": [0, 1, 2, 3, 4, 5], "support": 0.6, "class": 1}, {"atoms": [6, 7], "support": 0.3, "class": 2}, {"atoms": [8], "support": 0.1, "class": 3}]},
}
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(scheme="light"):
            c = b.new_context(viewport={"width": 1100, "height": 900}, reduced_motion="reduce", color_scheme=scheme); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang=en"); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_function("window.TPMOL", timeout=60000)
            return pg
        pg = fresh()
        def draw(name):
            h = pg.evaluate("async ([s, d]) => await TPMOL.svg(s, d)", [SMI, CASES[name]])
            assert h and h.startswith("<svg"), name
            pg.evaluate("h => { let d = document.getElementById('tpm-test'); if (!d) { d = document.createElement('div'); d.id = 'tpm-test'; d.style.cssText = 'position:fixed;left:20px;top:20px;width:340px;background:var(--panel);z-index:99999'; document.body.appendChild(d); } d.innerHTML = h; }", h)
            return h
        def count(sel): return pg.evaluate("s => document.querySelectorAll('#tpm-test ' + s).length", sel)
        def texts(): return pg.evaluate("() => [...document.querySelectorAll('#tpm-test text')].map(t => t.textContent)")
        def structure():
            draw("structure"); assert count(".mz-parentesi") == 0 and count(".mz-multicentro") == 0 and "OH" in texts(), texts()
        step("a certain site: the structure with the new atom, no brackets, no attachment", structure)
        def multi():
            draw("multicenter"); assert count(".mz-multicentro") == 1 and count(".mz-multicentro circle") == 0 and "OH" in texts() and "Position uncertain within ring A" in texts(), texts()
        step("a whole aromatic ring: one dashed bond from the centre to «OH», no circles on the ring", multi)
        def bracket():
            draw("bracket"); assert count(".mz-parentesi") == 1 and "+O" in texts() and count("pattern") == 1 and count("circle") >= 1 and "Position uncertain" in texts(), texts()
        step("another zone: brackets with «+O», the zone of the fragments hatched", bracket)
        def minus():
            draw("minus"); assert "−2H" in texts() and count(".mz-parentesi") == 1, texts()
        step("«−2H» keeps the true minus sign in brackets", minus)
        def whole():
            draw("whole"); assert "Position not determined" in texts() and count(".mz-parentesi") == 1, texts()
        step("position not determined: brackets on the whole parent and the words", whole)
        def zones():
            draw("uncertain"); t = texts(); assert all(n in t for n in ("1", "2", "3")), t
            assert count("circle") >= 6, count("circle")
        step("uncertain classes: numbered zones, one dash each", zones)
        def names():
            for n in CASES:
                draw(n); r = pg.evaluate("() => { const s = document.querySelector('#tpm-test svg'); return [s.getAttribute('role'), s.getAttribute('aria-label'), s.querySelector('title').textContent]; }")
                assert r[0] == "img" and r[1] and r[1] == r[2] and "+O" in r[1] or n == "minus", (n, r)
        step("every drawing is an img with its aria-label", names)
        def bad():
            h = pg.evaluate("async () => await TPMOL.svg('not a smiles ((', { mode: 'bracket', delta: '+O' })"); assert h is None, h
            h = pg.evaluate("async ([s]) => await TPMOL.svg(s, { mode: 'bracket', delta: '+O', atoms: [99] })", [SMI]); assert h and "mz-parentesi" in h
        step("an unreadable SMILES gives nothing; a stray atom index is dropped, never drawn elsewhere", bad)
        def shot():
            draw("multicenter"); pg.screenshot(path=str(tmp / "multi.png"), clip={"x": 0, "y": 0, "width": 400, "height": 300})
            for n in ("structure", "bracket", "uncertain"): draw(n); pg.screenshot(path=str(tmp / f"{n}.png"), clip={"x": 0, "y": 0, "width": 400, "height": 300})
        step("screenshots written", shot)
        def to_draw():
            h = pg.evaluate("async ([s, d]) => await TPMOL.molfile(s, d)", [SMI, CASES["multicenter"]])
            assert "V3000" in h and "MZLAB_ENDPTS" in h and "COUNTS 10 10 1" in h, h
            pg.evaluate("async ([s, d]) => { await TPMOL.openInDraw(s, d); }", [SMI, CASES["multicenter"]])
            pg.wait_for_function("TPDraw.ready()", timeout=90000)
            K = "document.querySelector('#kframe').contentWindow.ketcher"
            pg.wait_for_function(f"{K}.editor.struct().atoms.size === 10", timeout=20000)
            n = pg.evaluate(f"[...{K}.editor.struct().sgroups.values()].filter(s => s.type === 'DAT' && s.data.fieldName === 'MZLAB_ENDPTS').length")
            assert n == 1, n
            assert pg.is_visible("#kframe"), "the Draw tab is not the open view"
            pg.evaluate("async ([s, d]) => { await TPMOL.openInDraw(s, d); }", [SMI, CASES["bracket"]])
            pg.wait_for_function(f"[...{K}.editor.struct().sgroups.values()].some(s => s.type === 'DAT' && s.data.fieldName === 'MZLAB')", timeout=20000)
            pg.wait_for_function("TPDraw.partial().then(p => p.delta === '+O' && p.level === 3)", timeout=20000)
        step("«Open in Draw»: the V3000 loads, the attachment and the brackets are read back by the Draw tab", to_draw)
        def dark():
            pd = fresh("dark"); h = pd.evaluate("async ([s, d]) => await TPMOL.svg(s, d)", [SMI, CASES["bracket"]])
            assert "var(--ink)" in h and "#" not in h.replace("&#", "").split("<title>")[0], "colours only from CSS variables"
        step("colours come from CSS variables (works in the dark theme)", dark)
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
if errs: print("pageerrors:", errs[:3])
print("screenshots:", tmp)
sys.exit(1 if any(r != "ok" for _, r in steps) or errs else 0)
