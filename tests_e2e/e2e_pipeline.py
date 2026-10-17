"""Panel «Pipeline» of mzFinder (TP_Mine/js/32_pipeline.js, 33_pipeline_view.js, 34_pipeline_run.js). Part 1: parameters with their proof, preview on one file, gate of quality (cancello.json).
Part 2: run on the series, list of identities (gate open) or of rows (gate absent: no candidate in the DOM), detail with XIC and MS2, 10 000 rows virtualized, send to the feature tables.
Part 3: the recipe in the log and in the .mzflow, «Re-apply» (same SHA-256 of the table), unknown operation.
Builds the site (python3 tools/build_site.py) and the two .wasm of the feature engine (tools/build_feat_wasm.sh), writes a synthetic .mzidx to OPFS, turns mzFinder on and
opens the panel. Needs cargo, the wasm32 target, wasm-bindgen and numpy (SKIP without). MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site (the TP_Mine
files are copied over it); MZLAB_PIPE_MZIDX=<file .mzidx> adds the time of the preview on a real file (its time is printed).
Run from the repo root: python3 tests_e2e/e2e_pipeline.py"""
import re
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

SMILES = "CC12CCC3C(C1CCC2(C)O)CCC4C3(CC5=C(C4)NN=C5O)C"     # a structure of the formula C21H32N2O2 of the synthetic parent (its grafo/1 goes to the hypotheses)
tmp = Path(tempfile.mkdtemp(prefix="pipeline_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", env)
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
CEN = names[0][:-len(".mzidx")] + ".cen.mzidx"  # second index of the same sample (centroids for mzFinder): not a file of its own for the Pipeline
shutil.copy2(data / names[0], data / CEN)

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
URL = f"http://127.0.0.1:{PORT}/"
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_pipeline/" + n)).body.pipeTo(w); }
}"""
GATE = {"format": "cancello/2", "crate_version": "", "data": "2026-10-12", "vista_candidati": True, "vista_tabella": True, "concordanza_mzmine": "passa", "criteri": [{"id": "c1", "valore": 1, "soglia": 1, "ok": True}]}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.PIPE && window.UNTUI", timeout=120000)
        pg.evaluate(PUT, names + [CEN])

        def gate_absent():
            pg.evaluate("delete QTOOLS.ctx.files['cancello.json']; PIPE.open()"); pg.wait_for_selector("#pipe[open]")
            pg.wait_for_function("PIPE.state.gate && PIPE.state.gate.why === 'absent'", timeout=60000)
            assert pg.locator("#pipe-file option").count() >= 1 and pg.locator("#pipe-gate").count() == 0 and "validated" not in pg.inner_text("#pipe")      # the gate is internal: no text, no closed view
        step("without cancello.json the panel shows no gate text", gate_absent)

        def cen_not_listed():
            opts = pg.eval_on_selector_all("#pipe-file option", "o => o.map(x => x.value)")
            assert CEN not in opts and not any(".cen." in o for o in opts), opts
            assert sorted(opts) == names, (opts, names)
        step("a .cen.mzidx in OPFS is not listed as a file of the Pipeline", cen_not_listed)

        def gate_states():
            crate = pg.evaluate("PIPE.state.crate"); assert crate, crate
            g = lambda c, cr: pg.evaluate("([c, cr]) => PIPE.gateState(c, cr)", [c, cr])
            ok = dict(GATE, crate_version=crate)
            assert g(ok, crate)["validated"] is True
            assert g(dict(ok, crate_version="0.0.0"), crate)["why"] == "stale"
            assert g(dict(ok, vista_candidati=False), crate)["why"] == "closed"
            assert g(ok, crate)["validatedTable"] is True
            r = g(dict(ok, vista_candidati=False), crate); assert r["validatedTable"] is True and r["validated"] is False        # table datum independent of the candidates
            assert g(dict(ok, vista_tabella=False, vista_candidati=False), crate)["validatedTable"] is False
            assert g(dict(ok, format="cancello/1"), crate)["validated"] is True and g(dict(ok, format="cancello/1", vista_candidati=False), crate)["validatedTable"] is False
            assert g(dict(ok, format="x/9"), crate)["why"] == "format"
            assert g(None, crate)["why"] == "absent"
            for c in (ok, dict(ok, crate_version="0.0.0"), dict(ok, vista_candidati=False, vista_tabella=False), None):      # the measured state closes no view
                r = g(c, crate); assert r["ok"] is True and r["table"] is True, r
        step("gate: an internal datum (validated, validatedTable) bound to vista_* and the crate version; it closes no view", gate_states)

        def gate_page():
            crate = pg.evaluate("PIPE.state.crate")
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version=crate))
            pg.wait_for_function("PIPE.state.gate && PIPE.state.gate.why === 'open'", timeout=60000)
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version="0.0.0"))
            pg.wait_for_function("PIPE.state.gate && PIPE.state.gate.why === 'stale'", timeout=60000)
            assert "validated" not in pg.inner_text("#pipe")
            pg.evaluate("delete QTOOLS.ctx.files['cancello.json']; PIPE.open();")
        step("gate in the panel: internal state follows the file and the engine version, no text", gate_page)

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

        def roles_proposed():
            pg.evaluate("PIPE.open()"); pg.wait_for_function("document.querySelectorAll('#pr-tab .pr-role').length === %d" % len(names), timeout=60000)
            roles = pg.evaluate("[...document.querySelectorAll('#pr-tab .pr-role')].map(e => e.value)")
            assert roles == ["blank", "dark", "t0", "sample", "sample", "sample"], roles
            assert pg.evaluate("[...document.querySelectorAll('#pr-tab .pr-time')].map(e => e.value)") == ["", "", "0", "5", "10", "20"]
            assert pg.locator("#pr-tab tr.pr-prop").count() == 4, "the times proposed from the names are yellow"
            assert pg.is_disabled("#pipe-run-go") and "not confirmed" in pg.get_attribute("#pipe-run-go", "title"), pg.get_attribute("#pipe-run-go", "title")
            assert "Times not confirmed" in pg.inner_text("#pr-warn")
        step("roles and times are proposed from the names, Run is off until the times are confirmed", roles_proposed)

        def roles_confirm():
            pg.click("#pr-all"); pg.wait_for_function("document.querySelectorAll('#pr-tab tr.pr-prop').length === 0")
            assert not pg.is_disabled("#pipe-run-go") and "Times not confirmed" not in pg.inner_text("#pr-warn")
            assert pg.evaluate("PIPER.state.tp.minuti") == [None, None, 0, 5, 10, 20], pg.evaluate("PIPER.state.tp.minuti")
            pg.fill("#pr-t4", "12"); pg.press("#pr-t4", "Enter"); pg.wait_for_function("document.querySelector('#pr-t4').value === '12'")
            assert pg.evaluate("document.activeElement.id") == "pr-t4" and pg.evaluate("PIPER.state.tp.minuti[4]") == 12
            pg.fill("#pr-t4", "10"); pg.press("#pr-t4", "Enter"); pg.wait_for_function("PIPER.state.tp.minuti[4] === 10")
        step("«Confirm times» and Enter accept the times; the minutes for the pipeline come from the Rust", roles_confirm)

        def roles_warn():
            pg.select_option("#pr-r2", "qc"); pg.wait_for_function("document.querySelector('#pr-warn').textContent.includes('No t0')")
            assert not pg.is_disabled("#pipe-run-go")
            pg.select_option("#pr-r0", "sample"); pg.select_option("#pr-r1", "sample")
            pg.wait_for_function("document.querySelector('#pipe-run-go').disabled")
            t = pg.get_attribute("#pipe-run-go", "title"); assert "No t0, blank or dark" in t or "Time missing" in t, t
            pg.select_option("#pr-r0", "blank"); pg.select_option("#pr-r1", "dark"); pg.select_option("#pr-r2", "t0")
            pg.wait_for_function("!document.querySelector('#pipe-run-go').disabled && !document.querySelector('#pr-warn').textContent.includes('No t0')")
        step("without the t0 a short warning; without any control Run is off with the reason in its title", roles_warn)

        def roles_kept():
            pg.select_option("#pr-r3", "infusion"); pg.wait_for_function("document.querySelector('#pr-t3').disabled")
            nb = pg.evaluate("NB.campioni"); assert nb["schema"] == "campioni/2" and nb["campioni"][3]["tipo"] == "infusion" and nb["campioni"][3]["tempo"] is None
            pg.select_option("#pr-r3", "sample"); pg.fill("#pr-t3", "5"); pg.press("#pr-t3", "Enter")
            pg.wait_for_function("PIPER.state.tp.minuti[3] === 5")
            pg.evaluate("PIPE.open()"); pg.wait_for_function("document.querySelectorAll('#pr-tab .pr-role').length > 0")
            assert pg.evaluate("[...document.querySelectorAll('#pr-tab .pr-role')].map(e => e.value)") == ["blank", "dark", "t0", "sample", "sample", "sample"]
            assert pg.locator("#pr-tab tr.pr-prop").count() == 0, "the confirmed times stay confirmed"
        step("a role set in the panel is kept in the notebook (NB.campioni) and found again", roles_kept)

        def roles_from_data_view():
            pg.evaluate("""() => { window.__c = NB.campioni; delete NB.campioni;
              NB.session = [{ file: "SIN_t05min.mzML", label: "a", time: 5, type: "blank", conc: null }, { file: "SIN_t10min.mzML", label: "b", time: 10, type: "sample", conc: null }]; return null; }""")
            pg.evaluate("PIPE.open()"); pg.wait_for_function("document.querySelector('#pr-r3') && document.querySelector('#pr-r3').value === 'blank'", timeout=20000)
            assert pg.evaluate("[...document.querySelectorAll('#pr-tab .pr-time')].map(e => e.value)")[4] == "10" and pg.locator("#pr-tab tr.pr-prop").count() == 2
            pg.evaluate("() => { NB.session = null; NB.campioni = window.__c; return null; }"); pg.evaluate("PIPE.open()")
            pg.wait_for_function("document.querySelector('#pr-r3').value === 'sample'")
        step("the roles and times of the Data view are taken over (confirmed), the rest is proposed", roles_from_data_view)

        def roles_names():
            r = pg.evaluate("""async () => {
              const url = URL.createObjectURL(new Blob([QTOOLS.ctx.files["finder-ruoli.js"]], { type: "text/javascript" })), R = await import(url);
              const m = await PIPEQC.engine(QTOOLS.ctx.base), call = (op, j) => m.campioni(op, j);
              const cs = await R.build(call, ["S-t5.mzidx", "S-t30 (2).mzidx", "S_blank.mzidx", "S_dark.mzidx", "S_standard_direct-infusion_MSn.mzidx"], null, null);
              return { roles: cs.map(c => c.tipo), min: cs.map(c => c.tempo_min), conf: cs.map(c => c.tempo_confermato), tp: await R.forTp(call, cs), sha: (await R.canonical(call, cs)).sha256 };
            }""")
            assert r["roles"] == ["sample", "sample", "blank", "dark", "infusion"] and r["min"][:2] == [5, 30] and not any(r["conf"]), r
            assert r["tp"]["infusion"] == [4] and r["tp"]["blank"] == [2] and r["tp"]["minuti"][4] is None and len(r["sha"]) == 64, r
        step("the names S-t5, S-t30 (2), blank, dark and direct infusion give the right roles and minutes (infusion is not a control)", roles_names)

        def run_plain():
            pg.evaluate("delete QTOOLS.ctx.files['cancello.json']; PIPE.open();")
            pg.wait_for_function("PIPE.state.gate && PIPE.state.gate.why === 'absent'", timeout=60000)
            pg.wait_for_selector("#pipe-run-go"); pg.click("#pipe-run-go")
            pg.wait_for_function("document.querySelectorAll('#pv-list .pv-row').length > 0", timeout=120000)
            assert pg.locator("#pv-list [data-cand]").count() == 0 and pg.locator("#pv-dis").count() == 0, "candidates in the DOM without a parent"
            assert "validated" not in pg.inner_text("#pipe-res")
            assert not pg.evaluate("/C21H32N2O2|against|1\\/\\d/.test(document.querySelector('#pipe-res').textContent)")
            pg.wait_for_selector("#pipe-qc .qc #qc-m svg", timeout=60000)
            pg.wait_for_selector("#pipe-tab #ft-t", timeout=60000)
            assert "validated" not in pg.inner_text("#pipe-tab"), "the table view is always open, with no gate text"
        step("without cancello.json the run lists rows (traces and families), the table view is open and no candidate is in the DOM without a parent", run_plain)

        def run_gated():
            crate = pg.evaluate("PIPE.state.crate")
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version=crate))
            pg.wait_for_function("PIPE.state.gate && PIPE.state.gate.why === 'open'", timeout=60000)
            pg.fill("#pipe-pf", "C21H32N2O2"); pg.fill("#pipe-pmz", "345.253655"); pg.fill("#pipe-prt", "1.0"); pg.fill("#pipe-psmi", SMILES)
            assert pg.evaluate("PIPER.state.tp.minuti") == [None, None, 0, 5, 10, 20]
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

        def qc_step():
            pg.wait_for_selector("#pipe-qc .qc #qc-m svg", timeout=60000)
            n = len(names)
            got = pg.locator("#pipe-qc table >> nth=0 >> tr").count(); assert got == n + 1, f"one row per file: {got} rows for {n} files: " + pg.inner_text("#pipe-qc")[:500]
            pg.locator("#qc-m svg").focus(); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            assert pg.locator("#tdati table tr").count() >= n, "the chart has its table of data"
            pg.keyboard.press("Escape")
            assert "validated" not in pg.inner_text("#pipe-qc")      # no gate text in the quality control
        step("quality control: matrix D/T/F/A per file with its table, numbers of the engine", qc_step)

        def table_step():
            pg.wait_for_function("document.querySelector('#ft-n').textContent.includes(' of ')", timeout=60000)
            count = lambda: [int(x.replace(',', '')) for x in re.findall(r"[\d,]+", pg.inner_text("#ft-n"))]
            kept, total = count(); assert kept == total > 0, pg.inner_text("#ft-n")
            pg.wait_for_selector("#ft-t tbody tr.clk")
            assert pg.evaluate("document.querySelector('#ft-t').tagName") == "TABLE" and int(pg.get_attribute("#ft-t", "aria-rowcount")) == kept + 1
            assert 0 < pg.locator("#ft-t tbody tr").count() < 100 and pg.locator("#ft-t thead th").count() <= 8, "virtualised and at most 8 columns"
            pg.evaluate("document.querySelector('#pv-det').hidden = true")        # the detail of the list of identities, opened by the step before
            # the first two columns stay while the table scrolls sideways
            x0 = pg.evaluate("document.querySelectorAll('#ft-t thead th')[1].getBoundingClientRect().left")
            pg.evaluate("(() => { const w = document.querySelector('#ft-host .tpt-wrap'); w.style.maxWidth = '380px'; w.scrollLeft = 150; })()"); lib.attendi(pg, "document.querySelector('#ft-host .tpt-wrap').scrollLeft > 0 || document.querySelector('#ft-host .tpt-wrap').scrollWidth <= document.querySelector('#ft-host .tpt-wrap').clientWidth")
            assert abs(pg.evaluate("document.querySelectorAll('#ft-t thead th')[1].getBoundingClientRect().left") - x0) < 2, "Identity stays in place"
            pg.evaluate("(() => { const w = document.querySelector('#ft-host .tpt-wrap'); w.scrollLeft = 0; w.style.maxWidth = ''; })()")
            # a chip is the same view as the numeric field
            def changed(act):                                   # runs `act` and waits for the engine to give the new count
                prev = pg.inner_text("#ft-n"); act()
                pg.wait_for_function("p => document.querySelector('#ft-n').textContent !== p", arg=prev, timeout=30000); return count()[0]
            pg.click('[data-fc="det"]')
            pg.wait_for_function("document.querySelector('#ft-why').textContent.includes('too few')", timeout=30000); k1 = count()[0]
            pg.click('[data-fc="det"]'); pg.wait_for_function("document.querySelector('#ft-why').textContent === ''", timeout=30000)
            pg.click("#ft-moreb"); pg.fill("#ft-det", "2")
            pg.wait_for_function("document.querySelector('#ft-why').textContent.includes('too few')", timeout=30000)
            assert count()[0] == k1 < kept, (k1, count(), kept)                                              # the field gives the count of the chip
            # normalisation and filters of the numeric fields (the counts are the engine's)
            pg.select_option("#ft-norm", "somma"); kept2 = changed(lambda: pg.fill("#ft-det", str(len(names) // 2)))
            assert kept2 < total, (kept2, total, pg.inner_text("#ft-why"), pg.input_value("#ft-det"), pg.input_value("#ft-norm"))
            assert pg.inner_text("#ft-why").startswith("Rejected by each filter"), pg.inner_text("#ft-why")
            changed(lambda: pg.fill("#ft-det", "")); pg.select_option("#ft-norm", "nessuna")
            # sort by m/z
            pg.click("#ft-t th button[data-sh=mz]"); pg.wait_for_selector("#ft-t th[aria-sort=descending]")
            mzs = pg.evaluate("[...document.querySelectorAll('#ft-t tbody tr.clk')].map(r => +r.children[2].textContent)")
            assert len(mzs) > 3 and mzs == sorted(mzs, reverse=True) or mzs == sorted(mzs), mzs[:6]
            # the click expands the row on the spot, with the XIC of every file and no file open in Data
            pg.click("#ft-t tbody tr.clk >> nth=1")
            pg.wait_for_selector("#ft-t tr.tpt-exp #tp-det .tpr-ov svg", timeout=60000)
            assert pg.locator("#ft-t tr.tpt-exp").count() == 1 and pg.locator("#pv-det:not([hidden])").count() == 0
            assert "Open " not in pg.inner_text("#tp-det"), pg.inner_text("#tp-det")[:300]
            pg.wait_for_function("document.querySelectorAll('#tp-det .tpr-ov svg path.tr').length >= %d" % len(names), timeout=60000)
            if re.search(r"RT [\d.]+ min \(", pg.get_attribute("#tp-det .tpr-ov .tpr-ch:first-child svg", "data-tt-text") or ""):         # the engine gave the borders of this row
                # peak borders are shaded when they fall inside the drawn window, and the description says so only then
                shaded = pg.locator("#tp-det .tpr-ov svg .band").count() > 0
                desc = pg.evaluate("[...document.querySelectorAll('#tp-det .tpr-ov')].map(e => e.getAttribute('aria-label') || e.textContent).join(' ')")
                assert shaded == ("(shaded)" in desc) or not desc, (shaded, desc[:200])
            pg.locator("#tp-det .tpr-ov svg").first.focus(); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            assert pg.locator("#tdati tbody tr").count() >= 3; pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached")
            out = os.environ.get("MZLAB_PIPE_SHOTS")
            if out:
                pg.wait_for_timeout(500)
                for sel, nm in (("#pipe-qc", "qc"), ("#pipe-tab", "tabella")): pg.locator(sel).screenshot(path=str(Path(out) / f"pipeline_{nm}.png"))
            pg.click("#tp-det [data-tab=samples]")
            assert pg.locator("#tp-det table.tpr tbody tr").count() == len(names), pg.inner_text("#tp-det")[:300]
            pg.click("#tp-det [data-tab=quality]"); qt = pg.inner_text("#tp-det"); assert "S/N" in qt and "FWHM" in qt, qt[:300]
            pg.click("#tp-det [data-tab=overview]")
            pg.click("#ft-t tbody tr.clk >> nth=3")                      # another row closes the first one
            pg.wait_for_function("document.querySelectorAll('#ft-t tr.tpt-exp').length === 1")
            pg.locator("#ft-t tbody tr.tpt-exp").evaluate("e => e.previousElementSibling.querySelector('[data-tg]').focus()"); pg.keyboard.press("Escape")
            assert pg.locator("#ft-t tr.tpt-exp").count() == 0 and pg.evaluate("!!document.activeElement.dataset.tg"), "Esc closes and gives back the focus"
        step("table: engine view with chips and «N of M rows», virtual TPTAB, the click expands the row (XIC of every file, Samples, Quality)", table_step)

        def export_step():
            def get(i):
                with pg.expect_download(timeout=60000) as d: pg.click("#px-" + i)
                return Path(d.value.path()).read_bytes()
            assert get("mzmine").startswith(b"id,mz,rt"), "CSV for mzmine"
            assert get("full").startswith(b"id,mz,rt")
            assert get("xlsx")[:2] == b"PK", "xlsx is a zip"
            assert get("semi").startswith(b"\xef\xbb\xbf"), "CSV with ; has a BOM"
            z = get("fbmn"); assert z[:2] == b"PK" and b"quant.csv" in z and b"ms2.mgf" in z, "FBMN zip"
            pg.wait_for_function("document.querySelector('#px-msg').textContent.includes('done')", timeout=20000)
            get("sirius")
            html = get("report").decode(); assert "Tabui feature table" in html and "Quality control" in html and "rows kept" in html or "whole table" in html, html[:300]
            assert "error" not in pg.inner_text("#px-msg").lower(), pg.inner_text("#px-msg")
        step("export and report: mzmine, cells, xlsx, CSV with BOM, FBMN zip, SIRIUS and the report come from the engine", export_step)

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
            assert o["genitore"]["formula"] == "C21H32N2O2" and o["genitore"]["smiles"] == SMILES and json.loads(o["recipe"]) == o["parametri"]
            wf = pg.evaluate("WORKFLOW.buildWorkflow()"); assert any(x["op"] == "feature_pipeline" for x in wf["operazioni"])
            c = [x for x in wf["operazioni"] if x["op"] == "campioni"]; assert c and c[-1]["schema"] == "campioni/2" and len(c[-1]["sha256"]) == 64 and len(c[-1]["campioni"]) == len(names), c
            pg.evaluate("window.__sha = NB.ops.filter(o => o.op === 'feature_pipeline').pop().tabella_sha256; window.__wf = WORKFLOW.buildWorkflow(); null")
        step("the run is in the log of operations and in the .mzflow with parameters, proofs, inputs, versions and the SHA-256 of the table", recipe_logged)

        def reopen_and_again():
            pg.evaluate("document.querySelector('#unt') && document.querySelector('#unt').close(); NB.ops = []; PIPER.state.last = null; null"); pg.evaluate("PIPE.open()")
            pg.wait_for_function("document.querySelector('#pipe-again').disabled === true", timeout=20000)
            pg.evaluate("WORKFLOW.applyWorkflow(window.__wf)")
            pg.evaluate("PIPE.open()"); pg.wait_for_function("document.querySelector('#pipe-again') && !document.querySelector('#pipe-again').disabled", timeout=20000)
            pg.fill("#pipe-floor", "1"); pg.fill("#pipe-pf", ""); pg.fill("#pipe-psmi", "")   # the recipe must bring them back
            pg.click("#pipe-again")
            pg.wait_for_function("/Re-applied/.test(document.querySelector('#pipe-run-msg').textContent)", timeout=180000)
            m = pg.inner_text("#pipe-run-msg"); assert "identical" in m and pg.evaluate("window.__sha").startswith(m.split("SHA-256 ")[1][:12]), m
            assert pg.input_value("#pipe-pf") == "C21H32N2O2" and pg.input_value("#pipe-psmi") == SMILES and float(pg.evaluate("PIPE.values().floor")) == 5e4
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
