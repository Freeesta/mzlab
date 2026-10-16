"""Level decided by the chemist (TP_Mine/js/45_tabui_decisioni.js) on the HR table of the products: L3 on a row gives the double-border badge and survives a reload and a second run; L2b is refused
where LIVELLI.decide refuses it and the programme never writes a 2b; "not a TP" with "Hide not a TP" takes the row away; "Export view" has the columns "Decided level" and "Note"; keyboard,
targets and axe. Builds the site like e2e_tpmine_tabella.py (MZLAB_PIPE_SITE reuses one). Run from the repo root: python3 tests_e2e/e2e_tabui_decisioni.py
"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent; SHOTS = HERE / "shots"; SHOTS.mkdir(exist_ok=True)
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

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

tmp = Path(tempfile.mkdtemp(prefix="tabui_dec_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")), capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
idx = sorted(ser.glob("*.mzidx"))
data = site / "static" / "dati_finder"; data.mkdir(exist_ok=True)
for f in idx:
    shutil.copy2(f, data / (f.stem + ".mzML.mzidx"))      # the name the Data tab gives the index of «x.mzML»
    shutil.copy2(f.with_suffix(".mzML"), data / (f.stem + ".mzML"))
names = [f.stem + ".mzML" for f in idx]
PARENT = {"name": "parent", "neutral": "C21H32N2O2", "adduct": "[M+H]+"}
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(100):                                   # the server is ready when its port accepts a connection
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError:
        try: srv.wait(timeout=0.2); sys.exit("server did not start")        # still running: try again
        except subprocess.TimeoutExpired: pass
errs = []; reqs = []
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_finder/" + n + ".mzidx")).body.pipeTo(w); }
}"""
PUTF = PUT.replace('"idx"]', '"files"]').replace('n + ".mzidx", { create', 'n, { create').replace('n + ".mzidx")).body', 'n)).body')
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", accept_downloads=True).new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("console", lambda m: print("CONSOLE", m.text[:400]) if "tp_row" in m.text else None)
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
        pg.evaluate(PUT, names); pg.evaluate(PUTF, names)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.click("#tp-page"); pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 7", timeout=120000)
        pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
        rows = pg.locator("#hr-t tbody tr.clk")
        n0 = rows.count()

        from lib import axe_path, attendi
        AXE = axe_path()
        RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))"

        def dig():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.click("#tp-page"); pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 7", timeout=120000)
            pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
        rowsL = pg.locator("#hr-t tbody tr.clk")
        def open_row(i):
            pg.locator("#hr-t button.tpt-tg").nth(i).click(); pg.wait_for_selector("#tp-det .tpd-sel", timeout=60000)
        def level_of(i): return rowsL.nth(i).locator(".lvb").first.inner_text().strip()

        def none_before():
            assert "L2b" not in [level_of(i) for i in range(rowsL.count())], "the programme wrote a 2b"
            assert pg.locator("#hr-t .tpt-dec, #hr-t .tpt-nottp").count() == 0, "a decision before any choice"
        step("no decision and no 2b before the chemist chooses", none_before)

        def chips_there():
            for c in ("decided", "hidenottp"): assert pg.locator(f'#hr-tbl [data-chip="{c}"]').count() == 1, c
        step("chips «Decided» and «Hide not a TP»", chips_there)

        def decide_l3():
            open_row(0)
            sel = pg.locator("#tp-det .tpd-sel").first
            assert sel.get_attribute("aria-label").startswith("Decided level for"), sel.get_attribute("aria-label")
            assert sel.bounding_box()["height"] >= 24
            sel.select_option("3a"); pg.fill("#tp-det .tpd-note", "checked by hand"); pg.locator("#tp-det .tpd-note").press("Tab")
            attendi(pg, "!!document.querySelector('#hr-t tbody tr.clk .lvb.tpt-dec')")
            assert pg.locator("#hr-t tbody tr.clk").first.locator(".lvb.tpt-dec").count() == 1, "double border on the closed row"
            assert "L3a" in level_of(0)
            assert pg.locator("#tp-det .tpr-head .lvb.tpt-dec").count() == 1, "badge of the head"
            assert pg.evaluate("document.querySelector('#hr-t tbody tr.clk .lvb.tpt-dec').getAttribute('aria-label')").startswith("L3a, decided by the chemist; proposed"), "label"
        step("L3a on a row: double-border badge in the row and in the head", decide_l3)

        def identity_tab():
            pg.locator("#tp-det [role=tab]", has_text="Identity").click(); pg.wait_for_selector("#tp-det [role=tabpanel] .tpd-sel")
            assert pg.locator("#tp-det [role=tabpanel] .tpd-sel").input_value() == "3a"
            pg.locator("#tp-det [role=tabpanel] .tpd-sel").select_option("4")
            attendi(pg, "document.querySelector('#tp-det .tpr-head .tpd-sel').value === '4'")
            assert pg.locator("#tp-det .tpr-head .tpd-sel").input_value() == "4", "the head follows the Identity tab"
            pg.locator("#tp-det [role=tabpanel] .tpd-sel").select_option("3a"); pg.locator("#tp-det [role=tab]", has_text="Overview").click()
        step("the Identity tab has the same control, kept in step with the head", identity_tab)

        def refused_2b():
            assert pg.evaluate("TPDEC.check(5, '2b').ok") is False and pg.evaluate("TPDEC.check(0, '2b').ok") is False, "2b from L5 or L0"
            assert pg.evaluate("TPDEC.check(4, '2b').ok") is True and pg.evaluate("TPDEC.check(5, 'nottp').ok") is True
            open_row(2)
            sel = pg.locator("#tp-det .tpd-sel").first
            sel.select_option("1"); attendi(pg, "!!document.querySelector('#tp-det .tpd-msg').textContent.trim()")                 # level 1 needs a standard marked by the user: LIVELLI.decide refuses it
            msg = pg.locator("#tp-det .tpd-msg").first.inner_text(); assert msg.strip(), "the message of LIVELLI.decide"
            assert sel.input_value() == "", "the choice was not kept"
            assert pg.locator("#hr-t tbody tr.clk .lvb").filter(has_text="L1").count() == 0
            assert '"l":"1"' not in (pg.evaluate("localStorage.getItem('qqq.tabui.decisions')") or ""), "a refused level was stored"
        step("a level that LIVELLI.decide refuses is refused (2b from L5 and L0, level 1 without standard)", refused_2b)

        def remembered():
            raw = pg.evaluate("localStorage.getItem('qqq.tabui.decisions')"); assert raw and "checked by hand" in raw, raw
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
            dig()
            assert any("L3a" in level_of(i) for i in range(rowsL.count())), [level_of(i) for i in range(rowsL.count())]
            assert pg.locator("#hr-t tbody tr.clk .lvb.tpt-dec").count() == 1
        step("after a reload and a second run the decision is there", remembered)

        def nottp():
            n = rowsL.count(); open_row(1)
            pg.locator("#tp-det .tpd-sel").first.select_option("nottp"); attendi(pg, "!!document.querySelector('#hr-t tbody tr.clk .lvb.tpt-nottp')")
            b = rowsL.nth(1).locator(".lvb.tpt-nottp"); assert b.count() == 1 and "not a TP" in b.inner_text(), "badge struck through with the words"
            assert pg.locator("#hr-t tbody tr.clk s").count() >= 1
            pg.click('#hr-tbl [data-chip="hidenottp"]'); attendi(pg, f"document.querySelectorAll('#hr-t tbody tr.clk').length === {n - 1}")
            assert rowsL.count() == n - 1, (n, rowsL.count())
            pg.click('#hr-tbl [data-chip="decided"]'); attendi(pg, "document.querySelectorAll('#hr-t tbody tr.clk').length === 1")
            assert rowsL.count() == 1, rowsL.count()
            pg.click('#hr-tbl [data-chip="decided"]'); attendi(pg, f"document.querySelectorAll('#hr-t tbody tr.clk').length === {n - 1}")
        step("«not a TP» + «Hide not a TP» takes the row away; «Decided» keeps the decided ones", nottp)

        def export_view():
            pg.locator("#hr-tbl .tpt-xm summary").click()
            with pg.expect_download(timeout=30000) as dl: pg.click('#hr-tbl [data-x="csv"]')
            lines = Path(dl.value.path()).read_text(encoding="utf-8-sig").splitlines()
            h = lines[1].split(";"); assert h[-2:] == ["Decided level", "Note"], h
            assert any("L3a" in l and "checked by hand" in l for l in lines[2:]), lines[:5]
        step("Export view: «Decided level» and «Note»", export_view)

        def keyboard():
            pg.click('#hr-tbl [data-chip="hidenottp"]'); attendi(pg, "document.querySelector('#hr-tbl [data-chip=hidenottp]').getAttribute('aria-pressed') === 'false'")
            tgs = pg.locator("#hr-t button.tpt-tg"); tgs.nth(0).focus(); pg.keyboard.press("Enter"); pg.wait_for_selector("#tp-det .tpd-sel")
            pg.locator("#tp-det .tpd-sel").first.focus()
            assert pg.evaluate("document.activeElement.classList.contains('tpd-sel')")
            pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.classList.contains('tpd-note')"), "Tab goes from the select to the note"
            pg.keyboard.press("Escape")
        step("keyboard: the select and the note are reached with Tab", keyboard)

        def axe(theme):
            def f():
                if not AXE: print("SKIP axe-core not available"); return
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                if pg.locator("#tp-det .tpd-sel").count() == 0: pg.locator("#hr-t button.tpt-tg").first.click(); pg.wait_for_selector("#tp-det .tpd-sel")
                if not pg.evaluate("typeof axe") == "object": pg.add_script_tag(path=AXE)
                v = pg.evaluate(RUN % "'#hr-tbl'"); assert not v, v
                pg.screenshot(path=str(SHOTS / f"tabui_decisioni_{theme}.png"))
            return f
        step("axe: no serious violation, light theme", axe("light"))
        step("axe: no serious violation, dark theme", axe("dark"))

        def narrow():
            pg.set_viewport_size({"width": 390, "height": 800}); attendi(pg, "window.innerWidth === 390")
            if pg.locator("#tp-det .tpd-sel").count() == 0: open_row(0)
            w = pg.evaluate("[document.querySelector('#tp-det').getBoundingClientRect().width, document.querySelector('#hr-t').closest('.tpt-wrap').clientWidth]"); assert w[0] <= w[1] + 2, w
            pg.screenshot(path=str(SHOTS / "tabui_decisioni_390.png")); pg.set_viewport_size({"width": 1280, "height": 900})
        step("390 px: the panel fits", narrow)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
