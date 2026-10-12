"""Panel «Pipeline» of mzFinder (TP_Mine/js/32_pipeline.js, 33_pipeline_view.js, 34_pipeline_run.js). Part 1: parameters with their proof, preview on one file, gate of quality (cancello.json).
Part 2: run on the series, list of identities (gate open) or of rows (gate absent: no candidate in the DOM), detail with XIC and MS2, 10 000 rows virtualized, send to the feature tables.
Part 3: the recipe in the log and in the .mzflow, «Re-apply» (same SHA-256 of the table), unknown operation.
Builds the site (python3 tools/build_site.py) and the two .wasm of the feature engine (tools/build_feat_wasm.sh), writes a synthetic .mzidx to OPFS, turns mzFinder on and
opens the panel. Needs cargo, the wasm32 target, wasm-bindgen and numpy (SKIP without). MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site (the TP_Mine
files are copied over it); MZLAB_PIPE_MZIDX=<file .mzidx> adds the time of the preview on a real file (its time is printed).
Run from the repo root: python3 tests_e2e/e2e_pipeline.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import axe_path

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="pipeline_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=env, capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for sub in ("js", "files"):
        for f in (ROOT / "TP_Mine" / sub).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    ix = site / "static" / "mzfinder" / "indice.json"; d = json.loads(ix.read_text())
    d["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); ix.write_text(json.dumps(d))
data = site / "static" / "dati_pipeline"; data.mkdir(exist_ok=True)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
for f in sorted(ser.glob("*.mzidx")): shutil.copy2(f, data / f.name)
real = os.environ.get("MZLAB_PIPE_MZIDX")
if real: shutil.copy2(real, data / "real.mzidx")
names = sorted(p.name for p in data.glob("*.mzidx"))

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
URL = f"http://127.0.0.1:{PORT}/"
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_pipeline/" + n)).body.pipeTo(w); }
}"""
GATE = {"format": "cancello/1", "crate_version": "", "data": "2026-10-12", "vista_candidati": True, "concordanza_mzmine": "passa", "criteri": [{"id": "c1", "valore": 1, "soglia": 1, "ok": True}]}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.PIPE && window.UNTUI", timeout=120000)
        pg.evaluate(PUT, names)

        def gate_absent():
            pg.evaluate("PIPE.open()"); pg.wait_for_selector("#pipe[open]")
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('not validated')", timeout=60000)
            assert pg.locator("#pipe-file option").count() >= 1
        step("without cancello.json the panel says «not validated»", gate_absent)

        def gate_states():
            crate = pg.evaluate("PIPE.state.crate"); assert crate, crate
            g = lambda c, cr: pg.evaluate("([c, cr]) => PIPE.gateState(c, cr)", [c, cr])
            ok = dict(GATE, crate_version=crate)
            assert g(ok, crate)["ok"] is True
            assert g(dict(ok, crate_version="0.0.0"), crate)["why"] == "stale"
            assert g(dict(ok, vista_candidati=False), crate)["why"] == "closed"
            assert g(dict(ok, format="x/9"), crate)["why"] == "format"
            assert g(None, crate)["why"] == "absent"
        step("gate: open only with vista_candidati and the crate version of the .wasm", gate_states)

        def gate_page():
            crate = pg.evaluate("PIPE.state.crate")
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version=crate))
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('candidate view is available')", timeout=60000)
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version="0.0.0"))
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('this version of the engine')", timeout=60000)
            pg.evaluate("delete QTOOLS.ctx.files['cancello.json']; PIPE.open();")
        step("gate in the panel: valid, other version of the engine", gate_page)

        def preview():
            pg.select_option("#pipe-file", names[0]); t0 = time.time()
            pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.includes('features in')", timeout=90000)
            n = pg.inner_text("#pipe-n"); assert "0 features" not in n, n
            assert pg.locator("#pipe-hist svg").count() == 2 and pg.locator("#pipe-ex svg").count() == 3, "charts"
            floor = pg.evaluate("PIPE.values().floor"); assert 1e4 < floor < 1e6, floor
            proof = pg.inner_text("#pipe-proof"); assert "Noise floor" in proof and "Mass tolerance" in proof and "Median peak width" in proof, proof
            print("  preview of", names[0], round(time.time() - t0, 2), "s;", n)
        step("preview: features, estimated floor, tolerance and width with their proof, 2 histograms and 3 XIC", preview)

        def tables():
            svgs = pg.locator("#pipe svg[role=img]"); assert svgs.count() == 5
            for i in range(5):
                el = svgs.nth(i); el.focus(); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
                assert pg.locator("#tdati tbody tr").count() >= 3, i
                pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached")
            assert pg.locator("#pipe .tdbtn").count() == 5
        step("every chart has its table of data (T key and Data button)", tables)

        def edit_and_again():
            pg.fill("#pipe-floor", "5e4"); assert "chosen" in pg.inner_text("#pipe-floor-why")
            assert json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))["floor"] == 5e4
            pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-msg').textContent === ''", timeout=60000)
            pg.wait_for_timeout(500)
            assert float(pg.evaluate("PIPE.values().floor")) == 5e4 and pg.evaluate("PIPE.state.prev.n") > 0
        step("a chosen floor is kept and sent to the engine", edit_and_again)

        def keyboard_and_targets():
            small = pg.evaluate("""[...document.querySelectorAll('#pipe button, #pipe select, #pipe input, #pipe svg[tabindex]')].filter(e => e.offsetParent)
                .map(e => [e, (e.closest('label') && e.matches('input[type=checkbox]') ? e.closest('label') : e).getBoundingClientRect()]).filter(([e, r]) => r.width < 24 || r.height < 24)
                .map(([e, r]) => (e.id || e.tagName) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))""")
            assert not small, small
        step("WCAG 2.5.8: the controls of the panel are at least 24 x 24 px", keyboard_and_targets)

        def run_plain():
            pg.evaluate("delete QTOOLS.ctx.files['cancello.json']; PIPE.open();")
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('not validated')", timeout=60000)
            pg.wait_for_selector("#pipe-run-go"); pg.click("#pipe-run-go")
            pg.wait_for_function("document.querySelectorAll('#pv-list .pv-row').length > 0", timeout=120000)
            assert pg.locator("#pv-list [data-cand]").count() == 0 and pg.locator("#pv-dis").count() == 0, "candidates in the DOM without the gate"
            assert "no candidates" in pg.inner_text("#pipe-res")
            assert not pg.evaluate("/C21H32N2O2|against|1\\/\\d/.test(document.querySelector('#pipe-res').textContent)")
        step("without the gate the run lists rows (traces and families) and no candidate is in the DOM", run_plain)

        def run_gated():
            crate = pg.evaluate("PIPE.state.crate")
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version=crate))
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('candidate view is available')", timeout=60000)
            pg.fill("#pipe-pf", "C21H32N2O2"); pg.fill("#pipe-pmz", "345.253655"); pg.fill("#pipe-prt", "1.0")
            assert pg.input_value("#pipe-times") == ",,0,5,10,20", pg.input_value("#pipe-times")
            pg.click("#pipe-run-go")
            pg.wait_for_function("document.querySelectorAll('#pv-list [data-cand]').length > 0", timeout=180000)
            assert pg.locator("#pv-list .pv-chip").count() > 0 and pg.locator("#pv-list .pv-cells").count() > 0
            head = pg.inner_text("#pv-list .pv-head"); assert "Contributions" in head and "1/N" in head and "Cells" in head, head
            pts = pg.evaluate("[...document.querySelectorAll('#pv-list [data-cand]')].map(r => +r.children[3].textContent)")
            assert pts == sorted(pts, reverse=True), pts
        step("with the gate open the identities are listed by points with the column of the contributions, 1/N and the cells", run_gated)

        def filter_discarded():
            n = int(pg.inner_text("#pv-nd")); tot = pg.evaluate("PIPER.state.all.length")
            assert pg.evaluate("PIPER.state.list.every(r => !r.discarded)")
            pg.check("#pv-dis")
            assert pg.evaluate("PIPER.state.list.length") == n and pg.evaluate("PIPER.state.list.every(r => r.discarded && r.reasons.length)")
            pg.uncheck("#pv-dis"); assert pg.evaluate("PIPER.state.list.length") == tot - n
        step("the filter «discarded» lists the identities without a proof in favour, each with its reason code", filter_discarded)

        def against_red():
            pg.evaluate("""(() => { const r = PIPER.state.all.find(r => r.chips.some(c => c.against)); window.__against = r ? r.id : null; })()""")
            ident = pg.evaluate("window.__against")
            if ident is None:                                   # the synthetic series may have none: draw one with the same function
                html = pg.evaluate("PIPEV.chips({chips: [{name: 'kinetics', against: true, points: 0}]})")
                assert "pv-against" in html and "against" in html
            else:
                pg.evaluate("(id) => { const i = PIPER.state.list.findIndex(r => r.id === id); if (i < 0) document.querySelector('#pv-dis').click(); }", ident)
            color = pg.evaluate("(() => { const e = document.createElement('span'); e.className = 'pv-chip pv-against'; document.querySelector('#pv-list').appendChild(e); const c = getComputedStyle(e).color; e.remove(); return c; })()")
            assert color != pg.evaluate("getComputedStyle(document.querySelector('#pipe')).color"), color
        step("a proof against is drawn in red and flagged with a cross", against_red)

        def detail_charts():
            pg.evaluate("""(names) => { window.__fetch = window.fetch; window.fetch = (u, o) => {
              const s = String(u), j = x => Promise.resolve(new Response(JSON.stringify(x), { headers: { "Content-Type": "application/json" } }));
              if (/api\\/state/.test(s)) return j({ files: names.map((n, i) => ({ k: i, name: n.replace(".mzidx", ".mzML") })) });
              if (/api\\/xic/.test(s)) return j({ traces: [{ k: 0, rt: Array.from({ length: 40 }, (_, i) => 0.5 + i * 0.05), y: Array.from({ length: 40 }, (_, i) => 1000 * (1 + i % 7)) }] });
              if (/api\\/spectrum/.test(s)) return j({ mz: [100.1, 150.2, 200.3], y: [10, 500, 80] });
              return window.__fetch(u, o); }; }""", names)
            pg.click("#pv-list .pv-row >> nth=0")
            try: pg.wait_for_selector("#pv-det:not([hidden]) #pv-ch .ch.a svg", timeout=8000)
            except Exception: raise AssertionError(pg.inner_text("#pv-det")[-600:] + " | " + str(errs[-2:]))
            pg.wait_for_selector("#pv-ch .ch.b svg")
            assert pg.locator("#pv-det table").count() == 2 and "against" not in pg.inner_text("#pv-det caption >> nth=1")
            for sel in (".a", ".b"):
                el = pg.locator("#pv-ch " + sel + " svg"); el.focus(); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
                assert pg.locator("#tdati tbody tr").count() >= 3; pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached")
            pg.evaluate(r'''window.fetch = (u, o) => /api\/state/.test(String(u)) ? Promise.resolve(new Response('{"files":[]}')) : window.__fetch(u, o); null''')
            pg.select_option("#pv-file", "0")
            pg.wait_for_function("document.querySelector('#pv-ch .a').textContent.includes('Open ')", timeout=20000)
            pg.evaluate("window.fetch = window.__fetch; null")
        step("the click on a row shows proofs, cells, XIC and MS2 (each with its table of data)", detail_charts)

        def unt():
            pg.click("#pv-unt")
            t = pg.evaluate("UNTUI.state.table"); assert t and t["program"] == "mzlab" and len(t["features"]) > 0 and len(t["samples"]) == len(names), t and t["program"]
        step("the table enters the feature tables (UNT, program «mzlab»)", unt)

        def virtual():
            r = pg.evaluate("""() => {
              const box = document.createElement('div'); box.id = 'pv-big'; document.querySelector('#pipe-res').appendChild(box);
              const rows = Array.from({ length: 10000 }, (_, i) => ({ id: i, mass: 200 + i / 100, formula: 'C1H' + i, total: 5, against: 0, n: 1, chips: [{ name: 'mass', points: 2, against: false }], reasons: [], discarded: false, cells: [{ state: 0, area: 1 }, { state: 2, area: 2 }], mz: 200, rt: 1 }));
              const v = PIPEV.mount(box, { gated: true }); v.set(rows);
              const sc = v.scroller, t0 = performance.now();
              for (let k = 1; k <= 20; k++) { sc.scrollTop = k * 9973; sc.dispatchEvent(new Event('scroll')); }
              return new Promise(res => requestAnimationFrame(() => requestAnimationFrame(() => res({ dom: box.querySelectorAll('.pv-row').length, ms: performance.now() - t0, h: sc.scrollHeight }))));
            }""")
            assert r["dom"] < 80 and r["h"] >= 300000, r
            pg.evaluate("document.querySelector('#pv-big').remove()")
        step("10 000 rows are virtualized: few nodes in the DOM", virtual)

        def recipe_logged():
            ops = pg.evaluate("NB.ops.filter(o => o.op === 'feature_pipeline')"); assert len(ops) >= 1, ops
            o = ops[-1]; assert not pg.evaluate("WORKFLOW.checkFeatureOp(NB.ops.filter(o => o.op === 'feature_pipeline').pop())")
            assert o["crate_version"] and len(o["tabella_sha256"]) == 64 and o["regole"]["versione"] and o["giri"] == 1 and len(o["ingressi"]) == len(names), o
            assert o["genitore"]["formula"] == "C21H32N2O2" and json.loads(o["recipe"]) == o["parametri"]
            wf = pg.evaluate("WORKFLOW.buildWorkflow()"); assert any(x["op"] == "feature_pipeline" for x in wf["operazioni"])
            pg.evaluate("window.__sha = NB.ops.filter(o => o.op === 'feature_pipeline').pop().tabella_sha256; window.__wf = WORKFLOW.buildWorkflow(); null")
        step("the run is in the log of operations and in the .mzflow with parameters, proofs, inputs, versions and the SHA-256 of the table", recipe_logged)

        def reopen_and_again():
            pg.evaluate("document.querySelector('#unt') && document.querySelector('#unt').close(); NB.ops = []; PIPER.state.last = null; null"); pg.evaluate("PIPE.open()")
            pg.wait_for_function("document.querySelector('#pipe-again').disabled === true", timeout=20000)
            pg.evaluate("WORKFLOW.applyWorkflow(window.__wf)")
            pg.evaluate("PIPE.open()"); pg.wait_for_function("document.querySelector('#pipe-again') && !document.querySelector('#pipe-again').disabled", timeout=20000)
            pg.fill("#pipe-floor", "1"); pg.fill("#pipe-pf", "")                       # the recipe must bring them back
            pg.click("#pipe-again")
            pg.wait_for_function("/Re-applied/.test(document.querySelector('#pipe-run-msg').textContent)", timeout=180000)
            m = pg.inner_text("#pipe-run-msg"); assert "identical" in m and pg.evaluate("window.__sha").startswith(m.split("SHA-256 ")[1][:12]), m
            assert pg.input_value("#pipe-pf") == "C21H32N2O2" and float(pg.evaluate("PIPE.values().floor")) == 5e4
        step("saved, reopened and «Re-apply» give the same table (same SHA-256)", reopen_and_again)

        def unknown_op():
            pg.evaluate("window.__msgs = []; const i = window.info; window.info = m => { window.__msgs.push(String(m)); }; null")
            ok = pg.evaluate("WORKFLOW.applyWorkflow(Object.assign({}, window.__wf, { operazioni: window.__wf.operazioni.concat([{ op: 'dal_futuro' }]) }))")
            ops = pg.evaluate("NB.ops.map(o => o.op)"); assert "dal_futuro" not in ops and "feature_pipeline" in ops, ops
            assert any("dal_futuro" in m for m in pg.evaluate("window.__msgs")), pg.evaluate("window.__msgs")
        step("a .mzflow with an unknown operation opens with a warning", unknown_op)

        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""axe.run('#pipe', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length, v.nodes.map(n => n.target.join(' ')).join(' | ').slice(0, 200)]))""")
            assert not v, v
        step("axe: the panel has no violation", axe) if AXE else print("  SKIP axe-core not available")

        def shots():
            out = os.environ.get("MZLAB_PIPE_SHOTS")
            if not out: return
            for w in (1280, 390):
                pg.set_viewport_size({"width": w, "height": 1400}); pg.wait_for_timeout(300)
                pg.locator("#pipe").screenshot(path=str(Path(out) / f"pipeline_{w}.png"))
        shots()

        if real:
            def real_file():
                pg.select_option("#pipe-file", "real.mzidx"); t0 = time.time()
                pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.includes('real.mzidx')", timeout=120000)
                print("  preview of the real file:", round(time.time() - t0, 2), "s")
            step("preview of a real file of ~80 MB", real_file)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
