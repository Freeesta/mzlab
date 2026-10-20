"""Tabui, the hooks of the Feature table for other pages (TP_Mine/js/37_feature_table.js `extraCols`, `extraTabs`, `redraw`; TP_Mine/js/44_tp_riga.js `hook`): the table is mounted with a stub session twice,
without the options (the columns and the tabs of the row are the built-in ones) and with them (an extra column after the built-in ones, an extra tab in the open row that gets the row, `redraw` draws the cells again).
Run from the repo root: python3 tests_e2e/e2e_tabui_extra.py"""
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
STUB = """(extra) => {
  const session = {
    view: async () => ({ conteggi: { tenute: 2, righe: 2 } }), qc: async () => ({ scartate: [] }),
    rows: async () => [["id"], [1, 301.1, 5.0, 4.9, 5.1, 1, 1000, "DETECTED"], [2, 302.1, 6.0, 5.9, 6.1, 1, 500, "DETECTED"]],
    row: async () => ({ rt: 5.0, rt_lo: 4.9, rt_hi: 5.1, cells: [{ type: "sample", state: "detected", area: 1000, time: 0, apex_rt: 5.0 }], quality: null, ms2: null }),
    traces: async () => ({ xic: [], ms1: null, ms2: null }),
  };
  const host = document.createElement("div"); host.id = "ex-host"; host.style.cssText = "position:fixed;left:0;top:0;width:100%;max-width:900px;height:100%;overflow:auto;background:var(--panel)"; document.body.appendChild(host);
  const o = { session, names: ["a"], gate: { ok: true, table: true }, say: e => String(e && e.message || e), cs: null, ppm: () => 10, pol: () => 1, formula: "", levels: null, identsOf: () => [] };
  if (extra) {
    window.__fam = { 1: "2 ions" };
    o.extraCols = [{ key: "fam", label: "Family", sort: r => (window.__fam[r.id] || ""), cell: r => window.__fam[r.id] || "–", text: r => window.__fam[r.id] || "" }];
    o.extraTabs = [{ key: "fam", label: "Family", fill: (panel, r) => { panel.textContent = "family of row " + r.id; } }];
  }
  window.__mounted = window.PIPET.mount(host, o);
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(extra):
            c = b.new_context(viewport={"width": 1100, "height": 900}, reduced_motion="reduce"); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang=en"); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_function("window.PIPET && window.TPRIGA", timeout=60000)
            pg.evaluate(STUB, extra); pg.wait_for_selector("#ex-host #ft-t tbody tr", timeout=20000)
            return pg
        pg = fresh(False)
        def plain():
            heads = pg.evaluate("[...document.querySelectorAll('#ex-host #ft-t thead th')].map(t => t.textContent.trim())")
            assert "Family" not in heads, heads
            pg.click("#ex-host #ft-t tbody tr"); pg.wait_for_selector("#ex-host .tpt-tabs", timeout=20000)
            tabs = pg.evaluate("[...document.querySelectorAll('#ex-host .tpt-tabs button')].map(t => t.textContent.trim())")
            assert tabs == ["Overview", "Samples", "Identity", "Quality", "Structure"], tabs
        step("without the options the columns and the tabs of the row are the built-in ones", plain)
        pg.context.close()
        pg = fresh(True)
        def extra():
            heads = pg.evaluate("[...document.querySelectorAll('#ex-host #ft-t thead th')].map(t => t.textContent.trim())")
            assert heads[-1].startswith("Family") and heads.index("Max area") < len(heads) - 1, heads
            cells = pg.evaluate("[...document.querySelectorAll('#ex-host #ft-t tbody tr')].map(tr => tr.lastElementChild.textContent.trim())")
            assert cells == ["2 ions", "–"], cells
        step("an extra column comes after the built-in ones and shows the cell of the caller", extra)
        def redraw():
            pg.evaluate("() => { window.__fam[2] = '1 ion'; window.__mounted.redraw(); }")
            lib.attendi(pg, "[...document.querySelectorAll('#ex-host #ft-t tbody tr')].map(tr => tr.lastElementChild.textContent.trim()).join() === '2 ions,1 ion'")
        step("redraw() draws the rows again with the new values", redraw)
        def tab():
            pg.click("#ex-host #ft-t tbody tr"); pg.wait_for_selector("#ex-host .tpt-tabs", timeout=20000)
            tabs = pg.evaluate("[...document.querySelectorAll('#ex-host .tpt-tabs button')].map(t => t.textContent.trim())")
            assert tabs[-1] == "Family" and len(tabs) == 6, tabs
            pg.click("#ex-host .tpt-tabs button[data-tab=fam]")
            lib.attendi(pg, "/family of row \\d/.test(document.querySelector('#ex-host .tpt-body').textContent)")
            pg.click("#ex-host .tpt-tabs button[data-tab=samples]")
            lib.attendi(pg, "!/family of row/.test(document.querySelector('#ex-host .tpt-body').textContent)")
        step("an extra tab opens in the row with the row, the built-in tabs still work", tab)
        step("no page error", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        b.close()
finally:
    srv.terminate()
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("  ok " if r == "ok" else "  FAIL "), n, "" if r == "ok" else r)
sys.exit(1 if bad else 0)
