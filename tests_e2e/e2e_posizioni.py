"""Tabui, panel «Substitution position» (TP_Mine/js/44_posizioni.js, docs/agenti/posizioni.md): administrator mode only.
Without «qqq.admin» the package of Tabui is not even loaded (no TPPOS, no button). With it the panel shows, for a case built here (4-methyldiphenylamine + O, the MS2 has the fragment
with the oxygen on ring A and the rest of the molecule), the classes the spectrum tells apart, the table of the data of the spectrum (TDATI), the proposal 3a, the box of the
oxidation type and the suggested checks; a spectrum with no useful peak gives 3b; a row whose change is not covered, or without SMILES, has no button. In the LR demo the detail of the
hydroxylation gets the button when it has a MS2. Run from the repo root: python3 tests_e2e/e2e_posizioni.py   (env: PYODIDE_DIR)"""
import json, os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
PORT = 8843
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:500]))

tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
site = tmp / "site"

def add_grafo(site):
    """the .wasm «grafo» and its manifest in the copy of the site: the one built by tools/build_feat_wasm.sh in site/static/wasm-feat (CI) or, without it, built here"""
    import hashlib
    src = ROOT / "site" / "static" / "wasm-feat"
    dst = site / "static" / "wasm-feat"
    if (src / "manifest.json").exists() and (src / "grafo").is_dir():
        shutil.copytree(src, dst, dirs_exist_ok=True); return
    subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-grafo-wasm"], cwd=ROOT, check=True)
    subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(dst / "grafo"), "target/wasm32-unknown-unknown/release/mzlab_feat_grafo_wasm.wasm"], cwd=ROOT, check=True)
    w = (dst / "grafo" / "mzlab_feat_grafo_wasm_bg.wasm").read_bytes()
    json.dump({"grafo": {"js": "grafo/mzlab_feat_grafo_wasm.js", "wasm": "grafo/mzlab_feat_grafo_wasm_bg.wasm", "sha256": hashlib.sha256(w).hexdigest()}}, open(dst / "manifest.json", "w"))
add_grafo(site)

DPA = "Cc1ccc(Nc2ccccc2)cc1"
LR = [[109.26, 100], [186.29, 50]]          # unit resolution (0.5 Da): C7H8O 109.0648 and C12H11NO 186.0913, both with the oxygen
def model(**k):
    m = {"smiles": DPA, "delta": "+O", "peaks": LR, "positive": True, "tol": 0.5, "ppm": False, "label": "Change"}; m.update(k); return m
MOUNT = "m => { document.querySelector('#tpp-host')?.remove(); const h = document.createElement('div'); h.id = 'tpp-host'; document.querySelector('.tp').appendChild(h); window.TPPOS.mount(h, m); return !!h.querySelector('.tpp-btn'); }"

srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
_t0 = time.time()
while time.time() - _t0 < 30:                          # the server is up when it answers
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(admin):
            c = b.new_context(viewport={"width": 1400, "height": 1500}); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 and "logo.svg" not in r.url and "/vendor/" not in r.url else None)
            if admin: pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000); lib.attendi(pg, "typeof window.QTOOLS !== 'undefined'", 60000)
            return pg
        pg = fresh(False)
        step("without qqq.admin there is no TPPOS and no button", lambda: (_ for _ in ()).throw(AssertionError("TPPOS exists")) if pg.evaluate("typeof window.TPPOS !== 'undefined' || !!document.querySelector('.tpp-btn')") else None)
        pg.context.close()

        pg = fresh(True)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000); pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.wait_for_function("typeof window.TPPOS !== 'undefined'", timeout=60000)
        def gates():
            ev = lambda m: pg.evaluate("m => window.TPPOS.eligible(m)", m)
            assert ev(model()) and ev(model(delta="+O -H2")) and ev(model(delta="−CH2")) and ev(model(delta="+Cl-H")) and ev(model(delta="+Br-H")) and ev(model(delta="+O-2H"))
            assert not ev(model(delta="+H2O")) and not ev(model(delta="")) and not ev(model(smiles="")) and not ev(model(peaks=[])), "a row that the panel does not cover has the button"
            assert pg.evaluate("window.TPPOS.negative('[M-H]-') && !window.TPPOS.negative('[M+H]+')")
            assert pg.evaluate("window.TPPOS.symbols('CC(=O)[NH]c1ccc(Cl)cc1')") == ["C", "C", "O", "N", "C", "C", "C", "C", "Cl", "C", "C"]
        step("the button belongs to the rows with SMILES, MS2 and a covered change", gates)
        def hidden():
            assert not pg.evaluate(MOUNT, model(delta="+H2O")) and not pg.evaluate(MOUNT, model(smiles="")), "button for an uncovered row"
            assert pg.evaluate(MOUNT, model())
        step("mount draws the button only when the row is eligible", hidden)
        def panel():
            pg.evaluate(MOUNT, model()); pg.click("#tpp-host .tpp-btn"); pg.wait_for_selector("#tpp-host .tpp table", timeout=60000)
            rows = pg.evaluate("[...document.querySelectorAll('#tpp-host .tpp tbody tr')].map(r => [...r.children].map(c => c.innerText.replace(/\\s+/g, ' ').trim()))")
            a = [r for r in rows if "✓" in r[3]]; assert len(a) == 1 and "supported" in a[0][3], rows
            assert a[0][1] == "C3, C4, C13, C14", a[0]                    # atoms 2, 3, 12, 13 of the SMILES (ring A, ortho to the methyl or to N), 1-based, with the symbol
            ex = [r for r in rows if "✗" in r[3]]; assert len(ex) == 2 and all("excluded" in r[3] for r in ex), rows
            assert any(r[1] == "C1" for r in ex) and any(r[1] == "C8, C9, C10, C11, C12" for r in ex), ex
            assert "109.2600 C7H8O" in a[0][4] and "186.2900 C12H11NO" in a[0][4], a[0]
            t = pg.inner_text("#tpp-host .tpp-prop"); assert t == "Proposal: Schymanski 3a — 1 position class remains; the chemist decides", t
            assert pg.evaluate("document.querySelectorAll('#tpp-host button').length") >= 6 and not pg.evaluate("[...document.querySelectorAll('#tpp-host button')].some(b => /save|level/i.test(b.textContent))"), "a button that saves a level"
            txt = pg.inner_text("#tpp-host .tpp")
            for need in ("Oxidation type", "Suggested checks", "Reference standard", "Loss of H2O from the precursor: not seen"): assert need in txt, need
            assert pg.evaluate("[...document.querySelectorAll('#tpp-host a')].map(a => a.href)") == ["https://doi.org/10.1021/ac9911692", "https://doi.org/10.1007/s11419-018-0410-4"]      # oxidation type and MS3 (no C-OH / N-oxide ambiguity here: no H/D)
            pg.screenshot(path=str(HERE / "shots" / "posizioni.png"), full_page=False)
        step("the classes, their atoms, the proposal 3a, the oxidation type and the checks", panel)
        def chart():
            assert pg.evaluate("!!document.querySelector('#tpp-host .tpp-ch svg[role=img][aria-label]')")
            assert pg.evaluate("document.querySelectorAll('#tpp-host .tpp-ch svg .pk').length") == 2
            pg.click("#tpp-host .tpp-ch .tdbtn"); pg.wait_for_selector("#tdati[open]", timeout=15000)
            t = pg.inner_text("#tdati"); assert "109.2600" in t and "with the change, class 1" in t, t[:400]
            pg.keyboard.press("Escape")
            before = pg.inner_html("#tpp-host .tpp-fig svg")
            pg.click('#tpp-host .tpp-zb[data-z="in"]'); after = pg.inner_html("#tpp-host .tpp-fig svg"); assert before != after, "zoom in does nothing"
            pg.click('#tpp-host .tpp-zb[data-z="right"]'); assert pg.inner_html("#tpp-host .tpp-fig svg") != after
            pg.click('#tpp-host .tpp-zb[data-z="reset"]'); assert pg.inner_html("#tpp-host .tpp-fig svg") == before
            pg.focus("#tpp-host .tpp-fig svg"); pg.keyboard.press("+"); assert pg.inner_html("#tpp-host .tpp-fig svg") != before, "the key + does not zoom"
        step("the spectrum has its table of the data and zoom without dragging", chart)
        def targets():
            small = pg.evaluate("[...document.querySelectorAll('#tpp-host button')].filter(b => b.offsetParent).map(b => b.getBoundingClientRect()).filter(r => r.width < 24 || r.height < 24).length")
            assert small == 0, f"{small} targets under 24x24 px: " + str(pg.evaluate("[...document.querySelectorAll('#tpp-host button')].filter(b => b.offsetParent).map(b => [b.className, b.textContent, b.getBoundingClientRect().width, b.getBoundingClientRect().height]).filter(r => r[2] < 24 || r[3] < 24)"))
        step("the buttons are at least 24x24 px", targets)
        def axe():
            ax = lib.axe_path()
            if not ax: print("SKIP axe-core not available"); return
            pg.add_script_tag(path=ax)
            v = pg.evaluate("axe.run('#tpp-host', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))")
            assert not v, v
        step("axe: no serious or critical violation in the panel", axe)
        def three_b():
            pg.evaluate(MOUNT, model(peaks=[[300.0, 100]])); pg.click("#tpp-host .tpp-btn"); pg.wait_for_selector("#tpp-host .tpp table", timeout=30000)
            t = pg.inner_text("#tpp-host .tpp-prop"); assert t.startswith("Proposal: Schymanski 3b — 1 position class remains"), t      # nothing tells the sites apart: they form ONE class with all of them
            assert pg.evaluate("[...document.querySelectorAll('#tpp-host tbody tr')].filter(r => /undistinguished/.test(r.innerText)).length") == 1
            assert "unexplained" in pg.inner_text("#tpp-host tbody")
        step("a spectrum with no useful peak gives 3b with one undistinguished class", three_b)
        def hr():
            pg.evaluate(MOUNT, model(peaks=[[109.0648, 100], [186.0913, 50]], tol=5, ppm=True)); pg.click("#tpp-host .tpp-btn"); pg.wait_for_selector("#tpp-host .tpp table", timeout=30000)
            t = pg.inner_text("#tpp-host .tpp-prop"); assert t.startswith("Proposal: Schymanski 3a"), t
            assert "5 ppm" in pg.inner_text("#tpp-host .tpp")
        step("the same case in high resolution (ppm) gives the same classes", hr)
        def err():
            pg.evaluate(MOUNT, model(smiles="C", delta="+Cl-H")); pg.click("#tpp-host .tpp-btn"); pg.wait_for_selector("#tpp-host .err", timeout=30000)
            assert "no atom" in pg.inner_text("#tpp-host .err").lower(), pg.inner_text("#tpp-host .err")
        step("an engine error is shown as a sentence", err)
        pg.context.close()
        pg = fresh(True)
        def demo():
            """the real flow: the LR demo (synthetic) of mzFinder, the detail of the hydroxylation of the parent has the button when the TP has a MS2"""
            sys.path[:0] = [str(ROOT / "TP_Mine" / "py"), str(ROOT)]
            from tpmine.demo import make_demo
            fl = make_demo(Path(tempfile.mkdtemp()))
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000); pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.set_input_files("#tp-in", [f["path"] for f in fl])
            pg.wait_for_function("document.querySelector('#tp-files table') && document.querySelectorAll('#tp-files tr').length >= 9", timeout=240000)
            pg.fill("#tp-smi", "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
            pg.click("#tp-go"); pg.wait_for_selector("#tp-view .gem", timeout=300000)
            pg.click('.vt[data-v="gems"]'); pg.click(".gem:has-text('hydroxylation')"); pg.wait_for_selector("#tp-t tr.tpt-exp #tp-det", timeout=60000)
            pg.click('#tp-det [data-tab="identity"]'); pg.wait_for_selector("#tp-det .lr-idx", timeout=60000)
            has_ms2 = pg.evaluate("!!document.querySelector('#tp-ms2-fr table')")
            btn = pg.evaluate("!!document.querySelector('#tp-det .tpp-btn')")
            assert btn == has_ms2, f"button {btn} but MS2 {has_ms2}"
            if btn:
                pg.click("#tp-det .tpp-btn"); pg.wait_for_selector("#tp-det .tpp table", timeout=60000)
                t = pg.inner_text("#tp-det .tpp-prop"); assert t.startswith("Proposal: Schymanski 3"), t
                assert pg.evaluate("document.querySelectorAll('#tp-det .tpp tbody tr').length") >= 1
            else: print("SKIP the hydroxylation of the demo has no MS2: no button expected, none shown")
        step("LR demo: the detail of the hydroxylation has the button exactly when it has a MS2", demo)
        step("window.MZLAB_LR_MISS is empty", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS || []).length") else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s_ in steps: print(s_)
print("page errors:", errs)
sys.exit(1 if any(s_[1] != "ok" and not s_[1].startswith("SKIP") for s_ in steps) or errs else 0)
