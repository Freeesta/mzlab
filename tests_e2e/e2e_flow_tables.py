"""Flow, the column «Feature tables» on the right of the scheme (docs/agenti/flow-graphics.md): the column is there before any run with its empty line and the button «Open .mzfeat…»;
the answer of the native route `flow/live/tables` (`tables/1`, stubbed here: the entries are written by Rust, proved by tests/test_feat_table_wasm.py and the CLI tests) becomes one
button per table with rows · files, polarity, state and time, reachable and opened with the keyboard; a file that is no table is named; a .mzfeat chosen from disk joins the list and opens; at phone width the list stays above the inspector and Space opens a row.
Screenshots (desktop and phone, with and without a step selected): branch claude/flow-table-list-shots. Run from the repo root: python3 tests_e2e/e2e_flow_tables.py"""
# covers: flow/
import json, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, wait_for_js
PORT = free_port(); steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
DOC = {"schema": "tables/1", "skipped": [{"name": "broken.mzfeat", "error_key": "error.badTable"}], "tables": [
    {"name": "series.mzfeat", "bytes": 4096, "rows": 412, "files": 24, "polarity": 1, "identity": False, "tp": False, "recipe_hash": "ab", "crate": "1.0.0", "instance": "series", "step": "features.gapfill@1", "status": "ok", "ms": 12340.0, "warnings": 0},
    {"name": "neg.mzfeat", "bytes": 2048, "rows": 1, "files": 1, "polarity": -1, "identity": False, "tp": False, "recipe_hash": "cd", "crate": "1.0.0", "instance": "neg", "step": "features.align@1", "status": "cached", "ms": 0.0, "warnings": 0}]}
STUB = """(doc) => {
  window.__opened = []; window.__asked = [];
  FLOW.runners.desktop = { ...(FLOW.runners.desktop || {}), available: () => true };
  FLOW.desktopPost = async op => { window.__asked.push(op); return JSON.stringify(doc); };
  FLOW.desktopFile = async name => new File([new Uint8Array(8)], name);
  FLOW.openResult = async item => { window.__opened.push({ label: item.label, size: item.file ? item.file.size : -1, type: item.type }); };
  document.dispatchEvent(new Event("flow:changed"));
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=60000); pg.click('#nav button[data-v="flow"]'); pg.wait_for_selector("#flx-root", timeout=30000)
        wait_for_js(pg, "window.FLOW && FLOW.tablesPanel && document.querySelector('.flx-tabs')")
        def empty():
            col = pg.locator(".flx-right .flx-tabs")
            assert col.count() == 1 and col.locator("h3").inner_text() == "Feature tables"
            assert "No feature table yet" in col.inner_text() and col.locator(".flx-tab").count() == 0
            btn = col.get_by_role("button", name="Choose a .mzfeat file")
            assert btn.count() == 1 and btn.inner_text() == "Open .mzfeat…"
            btn.focus(); assert pg.evaluate("document.activeElement.textContent") == "Open .mzfeat…", "the button takes the focus"
        step("before a run: the column, its empty line and the button to open a .mzfeat", empty)
        def listed():
            pg.evaluate(STUB, DOC); wait_for_js(pg, "document.querySelectorAll('.flx-tab').length === 2")
            first, second = pg.locator(".flx-tab").nth(0), pg.locator(".flx-tab").nth(1)
            t1, t2 = first.inner_text(), second.inner_text()
            assert "series ESI+" in t1 and "412 rows · 24 files" in t1 and "done · 12.3 s" in t1, t1
            assert "neg ESI−" in t2 and "1 row · 1 file" in t2 and "up to date" in t2, t2
            assert pg.evaluate("window.__asked[0]") == "flow/live/tables"
            assert "broken.mzfeat could not be read as a feature table" in pg.inner_text(".flx-tabs"), pg.inner_text(".flx-tabs")
            assert "Open series.mzfeat: 412 rows · 24 files" in first.get_attribute("aria-label")
        step("the answer of the native route becomes one button per table: rows · files, polarity, state, time; a file that is no table is named", listed)
        def keyboard():
            pg.locator(".flx-tab").nth(1).focus(); pg.keyboard.press("Shift+Tab"); assert pg.evaluate("document.activeElement.classList.contains('flx-tab')"), "the buttons are in the tab order"
            pg.keyboard.press("Enter"); wait_for_js(pg, "window.__opened.length === 1")
            got = pg.evaluate("window.__opened[0]"); assert got == {"label": "series.mzfeat", "size": 8, "type": "table"}, got
        step("Enter on a table opens it in the Feature table view (the file is read first)", keyboard)
        def disk():
            pg.set_input_files(".flx-tabs input[type=file]", {"name": "mine.mzfeat", "mimeType": "application/octet-stream", "buffer": b"MZFEAT01" + bytes(24)})
            wait_for_js(pg, "window.__opened.length === 2")
            assert pg.evaluate("window.__opened[1].label") == "mine.mzfeat" and pg.evaluate("window.__opened[1].size") == 32
            wait_for_js(pg, "document.querySelectorAll('.flx-tab').length === 3")
            assert "from disk" in pg.locator(".flx-tab").nth(2).inner_text()
        step("a .mzfeat chosen from disk joins the list and opens", disk)
        def browser():
            # the browser leg: a step the scheduler finished (done, or reused) whose table is in OPFS is described by the wasm `table_entry` (stubbed) with the step the page names
            pg.evaluate("""() => {
              const S = document.querySelector('.flx-list').__flow; window.__calls = [];
              const inst = FLOW.addStep(S.plan, S.reg, 'features.gapfill'); window.__inst = inst.id;
              FLOW.runners.desktop = { ...FLOW.runners.desktop, available: () => false };
              const pool = { fs: { exists: async p => p === 'mzlab/flow/live/k1.mzfeat', read: async () => new Uint8Array(8) },
                tableEntry: async o => { window.__calls.push(o); return JSON.stringify({ name: o.name, rows: 5, files: 2, ...JSON.parse(o.made) }); } };
              FLOW.runners.web = { available: () => true, kit: { poolIfAny: () => pool } };
              FLOW.live.state = () => ({ tableKey: i => (i.id === inst.id ? 'k1' : null) });
              FLOW.liveView.node = id => (id === inst.id ? { state: 'reused', ms: 0 } : null);
              document.dispatchEvent(new Event('flow:changed'));
            }""")
            wait_for_js(pg, "window.__calls.length >= 1")
            call = pg.evaluate("window.__calls[0]")
            made = json.loads(call["made"]); assert call["path"] == "mzlab/flow/live/k1.mzfeat" and call["name"].endswith(".mzfeat"), call
            assert made["step"].startswith("features.gapfill@") and made["status"] == "cached" and made["instance"] == call["name"][:-7], made
            wait_for_js(pg, "[...document.querySelectorAll('.flx-tab')].some(b => /5 rows · 2 files/.test(b.textContent) && /up to date/.test(b.textContent))")
        step("this browser: a step the scheduler finished is listed from OPFS through table_entry with the step that made it", browser)
        def phone():
            # a phone: the column is under the scheme, and the inspector (the whole column height there) must not push the list away; the list is first and Space opens a row
            ph = b.new_context(viewport={"width": 390, "height": 844}).new_page()
            ph.on("pageerror", lambda e: errs.append(str(e)))
            ph.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            ph.goto(f"http://127.0.0.1:{PORT}/"); ph.wait_for_selector("#drop", timeout=120000)
            ph.wait_for_selector('#nav button[data-v="flow"]', timeout=60000); ph.click('#nav button[data-v="flow"]'); ph.wait_for_selector("#flx-root", timeout=30000)
            wait_for_js(ph, "window.FLOW && FLOW.tablesPanel && document.querySelector('.flx-tabs')")
            where = "() => { const t = document.querySelector('.flx-tabs').getBoundingClientRect(), i = document.querySelector('.fl-insp'); return { top: Math.round(t.top + scrollY), bottom: Math.round(t.bottom + scrollY), insp: i ? Math.round(i.getBoundingClientRect().top + scrollY) : null, vh: innerHeight }; }"
            before = ph.evaluate(where)
            ph.click('.fln:has-text("Import data")'); wait_for_js(ph, "document.querySelector('.fl-insp h3')")
            after = ph.evaluate(where)
            assert abs(after["top"] - before["top"]) <= 4, (before, after)
            assert after["insp"] > after["bottom"], ("the inspector is under the list", after)
            assert after["bottom"] < 2 * after["vh"], ("the list is within one scroll of the top", after)
            ph.evaluate(STUB, DOC); wait_for_js(ph, "document.querySelectorAll('.flx-tab').length === 2")
            ph.locator(".flx-tab").nth(1).scroll_into_view_if_needed(); ph.locator(".flx-tab").nth(1).focus(); ph.keyboard.press("Space")
            wait_for_js(ph, "window.__opened.length === 1"); got = ph.evaluate("window.__opened[0]"); assert got["label"] == "neg.mzfeat" and got["type"] == "table", got
            assert ph.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), "no sideways scroll"
        step("phone width: the list is under the scheme, above the inspector, with or without a step selected; Space opens a row", phone)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
