"""Tabui, «Draw» with an uncertain position: a drawing with brackets and a delta («+O») comes back to mzFinder with the SMILES, the formula and the m/z computed with the
delta and the note «position uncertain» (a candidate, not an identification); a plain drawing comes back exactly as before (no note, the formula of the SMILES).
Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it.
Run from the repo root: python3 tests_e2e/e2e_tabui_incerto.py"""
import os, shutil, subprocess, sys, tempfile, time, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))

WF = ROOT / "site" / "static" / "wasm-feat"
if not (WF / "manifest.json").exists():
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site); shutil.copytree(WF, site / "static" / "wasm-feat", dirs_exist_ok=True)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):                                                  # the server answers: no fixed pause
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
errs = []
TAB = '#nav button[data-v="tpmine"]'
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh():
            c = b.new_context(viewport={"width": 1400, "height": 1000}, reduced_motion="reduce"); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang=en"); pg.wait_for_selector(TAB, timeout=60000)
            pg.click(TAB); pg.wait_for_selector("#tp-draw")
            return pg
        KET = "document.querySelector('#kframe').contentWindow.ketcher"
        pg = fresh(); pg.click("#tp-draw")
        pg.wait_for_function("TPDraw.ready() && !document.querySelector('#tp-draw-use').disabled", timeout=90000)
        def menu_on_atom(i):
            pt = pg.evaluate(f"""(()=>{{const K={KET},a=K.editor.struct().atoms.get({i}),R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}});
              return [(a.pp.x-o1.x)*100/(o2.x-o1.x),(a.pp.y-o1.y)*100/(o2.y-o1.y)]}})()""")
            pg.evaluate("""([x,y])=>{const d=document.querySelector('#kframe').contentDocument;const e=d.elementFromPoint(x,y)||d.body;e.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:x,clientY:y}))}""", pt)
        def plain():
            pg.evaluate(f"{KET}.setMolecule('c1ccccc1CCO')"); pg.wait_for_function(f"{KET}.editor.struct().atoms.size === 9", timeout=10000)
            pg.click("#tp-draw-use"); pg.wait_for_function("document.querySelector('#tp-smi').value.length > 0", timeout=10000)
            pg.wait_for_function("document.querySelector('#tp-prev').innerText.includes('C8H10O')", timeout=30000)
            t = pg.inner_text("#tp-prev"); assert "uncertain" not in t and pg.input_value("#tp-mol") == "C8H10O", (t, pg.input_value("#tp-mol"))
        step("a plain drawing: as before (formula of the SMILES), no note", plain)
        def uncertain():
            pg.click("#tp-draw"); pg.wait_for_function("TPDraw.ready() && !document.querySelector('#tp-draw-use').disabled", timeout=90000)
            pg.evaluate(f"{KET}.setMolecule('c1ccccc1CCO')"); pg.wait_for_function(f"{KET}.editor.struct().atoms.size === 9", timeout=10000)
            pg.evaluate(f"{KET}.editor.selection({{atoms:[0,1,2,3,4,5]}})")
            menu_on_atom(0); pg.locator("#ctx div", has_text="Brackets with").first.click()
            pg.wait_for_selector("#askdlg[open]", timeout=5000); pg.fill("#askin", "+O"); pg.click("#askok")
            pg.evaluate(f"{KET}.editor.selection(null)")
            pg.wait_for_function("TPDraw.partial().then(p => p.delta === '+O' && p.info && p.info.formula === 'C8H10O2')", timeout=20000)
            pg.click("#tp-draw-use")
            pg.wait_for_function("document.querySelector('#tp-mol').value === 'C8H10O2'", timeout=20000)
            pg.wait_for_function("document.querySelector('#tp-prev').innerText.includes('position uncertain')", timeout=30000)
            t = pg.inner_text("#tp-prev"); assert "139.0" in t and "Δ +O" in t, t                       # [M+H]+ of C8H10O2 at unit decimals
            assert pg.inner_text("#tp-warn") == "", pg.inner_text("#tp-warn")
        step("brackets with «+O»: formula and m/z with the delta and the note «position uncertain»", uncertain)
        def edited():
            pg.click("#tp-draw-no") if pg.is_visible("#tp-draw-no") else None
            pg.fill("#tp-mol", "C8H10O"); pg.wait_for_function("!document.querySelector('#tp-prev').innerText.includes('position uncertain')", timeout=20000)
        step("editing the formula drops the note", edited)
        b.close()
finally:
    srv.terminate()
    if not reuse: shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
