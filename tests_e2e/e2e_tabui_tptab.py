"""Tabui, the table component TPTAB (TP_Mine/js/42_tp_table.js): fixed columns and header, the "Columns" menu kept after a reload, filter chips with «N of M rows», the pinned row,
virtual rows (1 000 rows, few <tr>), one open row at a time with tabs, the clicked row stays on the screen, keyboard (tabs, Esc), «Export view» (only the filtered rows and the visible
columns), targets of at least 24 px and axe in the light and dark theme. The component is mounted by hand with fake rows in #tp-right of the built site (administrator mode).
MZLAB_PIPE_SITE reuses a built site. Run from the repo root: python3 tests_e2e/e2e_tabui_tptab.py"""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import axe_path  # noqa: E402

steps = []
settle = lambda: pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")      # two frames: the scroll handler and the redraw have run
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tabui_tptab.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:900]))

tmp = Path(tempfile.mkdtemp(prefix="tabui_tptab_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True, capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
for _ in range(50):                                        # the server answers
    try: __import__("urllib.request").request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
SHOTS = Path(os.environ.get("TMPDIR", "/tmp")) / "tabui_tptab_shots"; SHOTS.mkdir(exist_ok=True)

# 1 000 fake rows: 12 states each; row 0 is the parent (pinned); the third column is the Identity (fixed with the level)
MOUNT = """() => {
  const S = ["detected", "filled", "targeted", "absent", "detected", "detected", "filled", "detected", "absent", "detected", "detected", "targeted"];
  const rows = Array.from({ length: 1000 }, (_, i) => ({ id: i, name: i === 0 ? "Parent" : "+O " + i, lv: i % 5 === 0 ? "3" : "5", mz: 150 + i * 0.1234, rt: 3 + (i % 70) / 10, area: 1e5 * (1 + (i * 37) % 101), blank: i % 4 === 0,
    st: S.map((s, k) => (i + k) % 7 === 0 ? "absent" : s), extra: "x" + i }));
  window.__rows = rows;
  const host = document.querySelector("#tp-right"); host.innerHTML = '<div id="tt-host"></div><div id="tt-leg"></div>';
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const T = TPTAB.mount(document.querySelector("#tt-host"), { id: "tt-t", label: "Test table", esc, rowId: r => r.id, name: r => r.name, height: 700, columns: true, search: r => r.name, exportName: "tabui-test",
    cols: [
      { key: "lvl", label: "Lvl", sticky: true, width: 110, sort: r => r.lv, cell: r => TPTAB.level(r.lv), text: r => "L" + r.lv },
      { key: "identity", label: "Identity", sticky: true, width: 150, sort: r => r.name, cell: r => esc(r.name) },
      { key: "mz", label: "<i>m/z</i>", num: true, sort: r => r.mz, cell: r => r.mz.toFixed(4) },
      { key: "rt", label: "RT", num: true, sort: r => r.rt, cell: r => r.rt.toFixed(2) },
      { key: "cells", label: "Cells", width: 700, cell: r => TPTAB.cells(r.st), text: r => r.st.join(" ") },
      { key: "area", label: "Max area", num: true, width: 500, sort: r => r.area, cell: r => r.area.toExponential(1) },
      { key: "extra", off: true, label: "Extra", cell: r => r.extra },
    ],
    chips: [{ key: "noblank", label: "Not in blank", test: r => !r.blank }, { key: "l3", label: "L3 or better", test: r => r.lv === "3" }],
    pin: r => r.id === 0,
    tabs: [{ key: "ov", label: "Overview" }, { key: "sm", label: "Samples" }, { key: "id", label: "Identity" }, { key: "q", label: "Quality" }],
    head: (r, box) => { box.innerHTML = TPTAB.level(r.lv) + " " + TPTAB.pred(2) + " <span>" + esc(r.name) + " in the head</span>"; },
    onOpen: (r, p, tab) => { p.innerHTML = '<div class="tpt-scroll"><p id="tt-body">' + esc(r.name) + " · " + tab + "</p>" + "<p>more</p>".repeat(8) + "</div>"; },
  });
  document.querySelector("#tt-leg").innerHTML = TPTAB.legend(["detected", "filled", "targeted", "absent"]);
  window.__T = T; T.setRows(rows);
}"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", accept_downloads=True); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        def load():
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav button[data-v=\"tpmine\"]", state="attached", timeout=120000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-right", timeout=60000)
            pg.wait_for_function("typeof window.TPTAB === 'object'", timeout=120000)
            pg.evaluate(MOUNT); pg.wait_for_selector("#tt-t tr.clk")
        load()
        W = "#tt-host .tpt-wrap"

        def fixed():
            r0 = pg.evaluate("() => { const w = document.querySelector('%s'); w.scrollLeft = 0; return [w.getBoundingClientRect().left, document.querySelector('#tt-t tbody tr.clk td.tpt-stk').getBoundingClientRect().left, w.scrollWidth - w.clientWidth]; }" % W)
            assert r0[2] > 100, ("the table must be wider than the box", r0)
            pg.evaluate("() => { document.querySelector('%s').scrollLeft = 300; }" % W); settle()
            r1 = pg.evaluate("() => { const w = document.querySelector('%s'), c = [...document.querySelectorAll('#tt-t tbody tr.clk')][1].querySelectorAll('td.tpt-stk'); return [w.getBoundingClientRect().left, c[0].getBoundingClientRect().left, c[1].getBoundingClientRect().left, c[0].getBoundingClientRect().width]; }" % W)
            assert abs(r1[1] - r1[0]) <= 2 and abs(r1[2] - (r1[0] + r1[3])) <= 3, ("the first two columns stay", r1)
            pg.evaluate("() => { document.querySelector('%s').scrollLeft = 0; document.querySelector('%s').scrollTop = 200; }" % (W, W)); settle()
            t = pg.evaluate("() => [document.querySelector('%s').getBoundingClientRect().top, document.querySelector('#tt-t thead th').getBoundingClientRect().top]" % W)
            assert abs(t[1] - t[0]) <= 2, ("the header stays on top", t)
            pg.evaluate("() => { document.querySelector('%s').scrollTop = 0; }" % W)
        step("columns fixed after a horizontal scroll, header fixed after a vertical one", fixed)

        def columns_menu():
            assert "Extra" not in pg.locator("#tt-t thead").inner_text()
            pg.locator("#tt-host .tpt-colm summary").click(); pg.check('#tt-host input[data-vc="extra"]')
            assert "Extra" in pg.locator("#tt-t thead").inner_text()
            assert pg.locator('#tt-host input[data-vc="lvl"]').count() == 0, "fixed columns are not in the menu"
            load()
            assert "Extra" in pg.locator("#tt-t thead").inner_text(), "the choice is kept after a reload"
            pg.evaluate("localStorage.removeItem('qqq.tabui.cols.tt-t')"); load()
        step("«Columns» menu: kept after a reload", columns_menu)

        def chips():
            assert pg.locator("#tt-host .tpt-n").inner_text() == "1000 of 1000 rows", pg.locator("#tt-host .tpt-n").inner_text()
            pg.click('#tt-host [data-chip="noblank"]')
            assert pg.get_attribute('#tt-host [data-chip="noblank"]', "aria-pressed") == "true"
            n = pg.locator("#tt-host .tpt-n").inner_text(); assert n == "750 of 1000 rows" or n == "751 of 1000 rows", n      # the parent (row 0) is in the blank group and is pinned anyway
            pg.fill("#tt-host .tpt-q", "+O 17"); n = pg.locator("#tt-host .tpt-n").inner_text(); assert n.endswith("of 1000 rows") and not n.startswith("750"), n
            pg.fill("#tt-host .tpt-q", ""); pg.click('#tt-host [data-chip="noblank"]')
            assert pg.locator("#tt-host .tpt-n").inner_text() == "1000 of 1000 rows"
        step("filter chips and search: «N of M rows»", chips)

        def pinned():
            pg.evaluate("window.__T.sortBy('mz', -1)")
            first = pg.locator("#tt-t tbody tr.clk").first
            assert first.get_attribute("data-id") == "0" and "★" in first.inner_text(), first.inner_text()
            assert pg.evaluate("document.querySelectorAll('#tt-t tbody tr.clk')[1].dataset.id") == "999", "the largest m/z comes after the pinned row"
            pg.evaluate("window.__T.sortBy('mz', 1)"); assert pg.get_attribute("#tt-t tbody tr.clk", "data-id") == "0"
        step("the pinned row stays first, also when sorting", pinned)

        def virtual():
            n = pg.locator("#tt-t tbody tr").count(); assert n < 100, n
            assert pg.get_attribute("#tt-t", "aria-rowcount") == "1001"
            pg.evaluate("() => { document.querySelector('%s').scrollTop = 900 * 30; }" % W); settle()
            assert pg.locator('#tt-t tr.clk[data-id="900"]').count() == 1 and pg.locator("#tt-t tbody tr").count() < 100
            pg.click('#tt-t [data-tg="900"]'); pg.wait_for_selector("#tt-t tr.tpt-exp #tp-det #tt-body")
            assert "+O 900" in pg.inner_text("#tt-body")
            pg.evaluate("() => { document.querySelector('%s').scrollTop = 0; }" % W); settle()
            assert pg.locator("#tt-t tr.tpt-exp").count() == 1, "the open row stays in the DOM outside the window"
        step("virtual rows: few <tr>, aria-rowcount 1001, the row 900 opens", virtual)

        def one_open():
            pg.evaluate("window.__T.close()"); pg.evaluate("document.querySelector('%s').scrollTop = 0" % W); settle()
            pg.click('#tt-t [data-tg="3"]'); pg.click("#tt-t [data-tab=sm]"); pg.evaluate("document.querySelector('%s').scrollTop = 400" % W); settle(); assert "Samples" not in pg.inner_text("#tt-body") and "sm" in pg.inner_text("#tt-body")
            before = pg.evaluate("document.querySelector('#tt-t tr.clk[data-id=\"6\"]').getBoundingClientRect().top")
            pg.click('#tt-t [data-tg="6"]'); settle()
            assert pg.locator("#tt-t tr.tpt-exp").count() == 1 and pg.locator('#tt-t [data-tg="3"]').get_attribute("aria-expanded") == "false"
            assert pg.get_attribute("#tt-t [data-tab=sm]", "aria-selected") == "true" and "sm" in pg.inner_text("#tt-body"), "the same tab"
            after = pg.evaluate("document.querySelector('#tt-t tr.clk[data-id=\"6\"]').getBoundingClientRect().top")
            assert abs(after - before) <= 2, ("the clicked row stays on the screen", before, after)
            assert pg.get_attribute("#tp-det", "role") == "region" and pg.get_attribute("#tp-det", "aria-labelledby") == "tt-t-i-6"
        step("one open row at a time; the tab stays; the clicked row does not move", one_open)

        def keys():
            pg.focus("#tt-t [data-tab=sm]"); pg.keyboard.press("ArrowRight")
            assert pg.get_attribute("#tt-t [data-tab=id]", "aria-selected") == "true" and pg.evaluate("document.activeElement.dataset.tab") == "id"
            pg.keyboard.press("End"); assert pg.evaluate("document.activeElement.dataset.tab") == "q"
            pg.keyboard.press("Home"); assert pg.evaluate("document.activeElement.dataset.tab") == "ov"
            pg.keyboard.press("ArrowLeft"); assert pg.evaluate("document.activeElement.dataset.tab") == "q"
            pg.keyboard.press("Escape")
            assert pg.locator("#tt-t tr.tpt-exp").count() == 0 and pg.evaluate("document.activeElement.dataset.tg") == "6", pg.evaluate("document.activeElement.outerHTML.slice(0,120)")
            pg.keyboard.press("ArrowDown"); assert pg.evaluate("document.activeElement.dataset.tg") == "7"
            pg.keyboard.press("End"); assert pg.evaluate("document.activeElement.dataset.tg") == "999"
            pg.keyboard.press("Home"); assert pg.evaluate("document.activeElement.dataset.tg") == "0"
            pg.keyboard.press("Enter"); pg.wait_for_selector("#tt-t tr.tpt-exp"); pg.keyboard.press("Escape")
        step("keyboard: arrows between tabs, Esc gives the focus back, arrows/Home/End between rows", keys)

        def export():
            pg.click('#tt-host [data-chip="l3"]'); pg.locator("#tt-host .tpt-xm summary").click()
            with pg.expect_download() as d: pg.click('#tt-host [data-x="csv"]')
            txt = Path(d.value.path()).read_bytes().decode("utf-8")
            assert txt.startswith("﻿# filters: L3 or better"), txt[:80]
            lines = [l for l in txt.split("\r\n") if l]
            assert lines[1] == "Lvl;Identity;m/z;RT;Cells;Max area", lines[1]        # the visible columns (Extra is off again)
            assert len(lines) == 2 + 200, len(lines)                                  # the header, the 200 rows with level 3 (the pinned parent among them)
            assert all(l.startswith("L3;") for l in lines[2:]), lines[2:4]
            pg.click('#tt-host [data-chip="l3"]')
        step("«Export view»: only the filtered rows and the visible columns", export)

        def targets():
            bad = pg.evaluate("""() => [...document.querySelectorAll('#tt-host button, #tt-host summary, #tt-host label, #tt-host input.tpt-q')].filter(e => { const r = e.getBoundingClientRect(); return r.width && (r.width < 24 || r.height < 24); }).map(e => e.outerHTML.slice(0, 80))""")
            assert not bad, bad
        step("targets of at least 24 px", targets)

        def axe(theme):
            def f():
                if not AXE: print("SKIP axe-core not available"); return
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                pg.evaluate('window.__T.open(3)'); settle()
                if not pg.evaluate("typeof axe") == "object": pg.add_script_tag(path=AXE)
                v = pg.evaluate(RUN % "'#tt-host'"); assert not v, v
                pg.locator("#tt-host").screenshot(path=str(SHOTS / f"tptab-{theme}-1280.png"))
            return f
        step("axe: no serious violation, light theme", axe("light"))
        step("axe: no serious violation, dark theme", axe("dark"))

        def narrow():
            pg.set_viewport_size({"width": 390, "height": 800}); settle()
            w = pg.evaluate("[document.querySelector('#tp-det').getBoundingClientRect().width, document.querySelector('%s').clientWidth]" % W)
            assert w[0] <= w[1] + 2, w
            pg.locator("#tt-host").screenshot(path=str(SHOTS / "tptab-dark-390.png")); pg.set_viewport_size({"width": 1280, "height": 900})
        step("narrow window: the panel fits the visible width", narrow)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
if errs: print("PAGE ERRORS", errs[:5])
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
