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
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site), "--no-pyodide"]       # the component needs neither Pyodide nor the engine
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
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');localStorage.setItem('qqq.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        def load():
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav button[data-v=\"tpmine\"]", state="attached", timeout=120000)
            pg.evaluate("() => { const l = document.getElementById('loading'); if (l && /motore|engine/i.test(l.innerText)) l.remove(); }")     # the component needs no engine: a build without the .wasm must not block the test
            pg.wait_for_function("typeof window.TPTAB === 'object' && typeof window.TPHELP === 'object' && typeof window.TPRIGA === 'object'", timeout=120000)       # before the click: the view attaches the help to what exists when it is built
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-right", timeout=60000)
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
            if not pg.locator("#tt-t tr.tpt-exp").count(): pg.click('#tt-t [data-tg="3"]'); pg.wait_for_selector("#tt-t tr.tpt-exp #tp-det")      # the keyboard step closed the row
            pg.set_viewport_size({"width": 390, "height": 800}); settle()
            try:
                w = pg.evaluate("[document.querySelector('#tp-det').getBoundingClientRect().width, document.querySelector('%s').clientWidth]" % W)
                assert w[0] <= w[1] + 2, w
                pg.locator("#tt-host").screenshot(path=str(SHOTS / "tptab-dark-390.png"))
            finally: pg.set_viewport_size({"width": 1280, "height": 900}); settle()
        step("narrow window: the panel fits the visible width", narrow)
        # ---- TPRIGA: the panel of the open row, from a model (row 0 complete, row 1 only xic and samples, row 2 with 12 samples and 5 candidates)
        MOUNT2 = """() => {
          const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
          const G = (c, a, w) => Array.from({ length: 41 }, (_, i) => 6.5 + i * 0.05).map(t => a * Math.exp(-((t - c) ** 2) / (2 * w * w)));
          const rt = Array.from({ length: 41 }, (_, i) => 6.5 + i * 0.05);
          const mk = (n) => ["blank", "t0", "t15", "t30", "t60", "t120"].slice(0, n).map((g, i) => ({ file: g + "_r1", group: g, time: g === "blank" ? null : g === "t0" ? 0 : +g.slice(1), rt, y: G(7.1, g === "blank" || g === "t0" ? 1e3 : 1e5 * (i + 1), 0.12) }));
          const samp = n => Array.from({ length: n }, (_, i) => ({ file: "s" + i + ".mzML", type: i === 0 ? "blank" : i === 1 ? "t0" : "time", time: i < 2 ? (i === 1 ? 0 : null) : i * 15, area: i === 0 ? 0 : 1e4 * Math.pow(2, i), norm: 0.1 * i,
            state: i === 0 ? "absent" : i % 5 === 3 ? "filled" : "detected", apexRt: 7.1, lo: 6.95, hi: 7.31 }));
          const acts = [{ key: "copy", label: "Copy", help: "ft.riga.copy", run: () => { window.__act = "copy"; } }];
          const full = { title: "+O", level: "3", pred: 2, summary: [{ label: "Net change", value: "+O", help: "ft.col.net" }, { label: "m/z", value: "257.0591" }, { label: "RT", value: "7.10 min" }], actions: acts, open: { xic: () => { window.__go = "xic"; } }, xic: mk(6), apexRt: 7.1, lo: 6.95, hi: 7.31,
            spectrum: { kind: "ms1", mz: [257.059, 258.062, 259.057], y: [100, 11, 5], iso: { exp: { mz: [257.059, 258.062, 259.057], y: [100, 12, 4.5] }, obs: { mz: [257.059, 258.062, 259.057], y: [100, 11, 5] } } },
            xicTip: "RT 7.10 min (6.95-7.31) · FWHM 0.08 min.", samples: samp(6),
            identity: { neutral: 256.0518, adduct: "[M+H]+", candidates: [
              { name: "Sulfoxide", formula: "C10H12N2O4S", ppm: 1.2, adduct: "[M+H]+", source: "hypothesis", level: "3", pred: 2, proofs: [{ key: "persist", ok: true, text: "found in 4 of 4 treated samples" }, { key: "ms2", ok: false, text: "MS2 not measured" }] },
              { formula: "C11H16N2O3S", ppm: 3.4, adduct: "[M+H]+", source: "library", level: "4", pred: 3 }, { formula: "C9H8N4O4S", ppm: -4.1, adduct: "[M+H]+", source: "formula", level: "4" }, { formula: "C12H20N2O2S", ppm: 4.9, adduct: "[M+H]+", source: "formula", level: "4" }, { formula: "C13H22N2O2S", ppm: -5.0, adduct: "[M+H]+", source: "formula" }, { formula: "C14H10N2O2S", ppm: 5.1, adduct: "[M+H]+", source: "formula" }] },
            quality: { shape: 0.93, snr: 142, fwhm: 4, flags: ["points"], isoM1: 0.11, blank: { present: false, ratio: 0.01 }, t0: false, rejected: [] } };
          const part = { title: "partial", level: "5", xic: mk(3), apexRt: 7.1, samples: samp(3) };
          const many = Object.assign({}, full, { samples: samp(12), spectrum: { kind: "mirror", mz: [91.05, 119.08, 147.04], y: [30, 100, 55], ref: { mz: [91.05, 119.08, 150], y: [20, 100, 40] } } });
          const models = { 0: full, 1: part, 2: many };
          const rows = [0, 1, 2].map(i => ({ id: i, name: "row " + i }));
          document.querySelector("#tp-right").innerHTML = '<div id="tr-host" style="width:100%"></div>';
          const T = TPTAB.mount(document.querySelector("#tr-host"), Object.assign({ id: "tr-t", label: "Panel test", esc, rowId: r => r.id, name: r => r.name, height: 700,
            cols: [{ key: "lvl", label: "Lvl", sticky: true, width: 90, cell: r => TPTAB.level(models[r.id].level) }, { key: "identity", label: "Identity", sticky: true, width: 150, cell: r => esc(r.name) }] }, TPRIGA.hook(r => models[r.id])));
          window.__T2 = T; T.setRows(rows);
        }"""
        def riga_mount():
            pg.evaluate(MOUNT2); pg.wait_for_selector("#tr-t tr.clk"); pg.click('#tr-t [data-tg="0"]'); pg.wait_for_selector("#tp-det .tpr-ov svg")
        step("TPRIGA: a model opens in the row", riga_mount)

        def riga_head():
            h = pg.inner_text("#tp-det .tpt-head")
            assert "Prev. 2" in h and "Decided" not in h and "Level" not in h and "+O" in h and "257.0591" in h, h
            assert pg.locator("#tp-det .tpt-head .lvb.pd-2").count() == 1 and pg.locator("#tp-det .tpt-head .lvb.lv-3").count() == 0, "the level only in the closed row"
            assert pg.locator("#tp-det .tpt-head .tpr-sum").count() == 3 and pg.locator('#tp-det .tpt-head .tpr-sum[data-tt="ft.col.net"]').count() == 1, "the summary of the closed row, with its help"
            pg.click('#tp-det [data-act="copy"]'); assert pg.evaluate("window.__act") == "copy"
            assert [t for t in pg.locator("#tp-det [role=tab]").all_inner_texts()] == ["Overview", "Samples", "Identity", "Quality"]
            small = pg.evaluate("[...document.querySelectorAll('#tp-det .tpr-act')].filter(e => e.getBoundingClientRect().height < 24).length"); assert small == 0
        step("TPRIGA: head with Prev. 2, the summary of the row, actions; four tabs", riga_head)

        def riga_overview():
            r = pg.evaluate("[...document.querySelectorAll('#tp-det .tpr-ov > .tpr-ch svg')].map(s => { const b = s.getBoundingClientRect(); return [b.left, b.top, b.width] })"); assert len(r) == 2, r
            assert abs(r[0][1] - r[1][1]) <= 2 and r[1][0] > r[0][0] + 100, ("side by side at 1280 px", r)
            band = pg.evaluate("(() => { const a = document.querySelector('#tp-det svg .ax').getBoundingClientRect(), b = document.querySelector('#tp-det svg .band').getBoundingClientRect(); return [a.left, a.right, b.left, b.right] })()")
            assert band[2] >= band[0] - 1 and band[3] <= band[1] + 1 and band[3] > band[2], ("the shaded edges are inside the axis", band)
            assert pg.locator("#tp-det .tpr-ov svg .apex").count() == 1 and pg.locator("#tp-det svg .tr").count() == 6
            assert pg.locator("#tp-det svg .tr-blank").count() == 1 and pg.locator("#tp-det svg .tr-t0").count() == 1
            assert pg.locator("#tp-det .tpr-leg > span").count() == 6, "legend per group"
            assert pg.locator("#tp-det svg .iso-e").count() == 3 and pg.locator("#tp-det svg .iso-o").count() == 3, "expected hollow, observed filled"
            assert pg.locator("#tp-det .tdbtn").count() == 0 and pg.locator("#tp-det .tpr-line").count() == 0, "no Data button, no line under the charts"
            assert "FWHM 0.08 min" in (pg.get_attribute("#tp-det .tpr-ch:first-child svg", "data-tt-text") or ""), "the window of the peak goes in the help of the chart"
            assert pg.locator("#tp-det .tpr-go").count() == 1, "XIC in Data (an MS1 spectrum has no MS2 button)"
            pg.click("#tp-det .tpr-go"); assert pg.evaluate("window.__go") == "xic"
            h = pg.evaluate("document.querySelector('#tp-det').getBoundingClientRect().height"); assert h <= 420, ("panel height", h)
            pg.focus("#tp-det .tpr-ch:first-child svg"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]"); assert "6.500" in pg.inner_text("#tdati"); pg.keyboard.press("Escape")
        step("TPRIGA overview: two charts side by side, shaded edges inside the axis, legend, tables of the data, height <= 420 px", riga_overview)

        def riga_samples():
            pg.click("#tr-t [data-tab=samples]"); settle()
            assert pg.locator("#tp-det .tpr-cell").count() == 6
            cells = pg.evaluate("[...document.querySelectorAll('#tp-det .tpr-cell')].map(c => [c.getBoundingClientRect().width, c.getBoundingClientRect().height, c.textContent.trim(), c.getAttribute('aria-label')])")
            assert all(c[0] >= 24 and c[1] >= 24 for c in cells), cells
            assert cells[0][2] == "○" and "absent" in cells[0][3] and "s0.mzML" in cells[0][3], cells[0]
            assert cells[3][2] == "◐" and "filled" in cells[3][3], cells[3]
            assert pg.locator("#tp-det .tpr-cell[tabindex='0'][data-tt][data-tt-text]").count() == 6, "every cell is focusable and has its tooltip"
            assert pg.locator("#tp-det .tpr-grp b[tabindex='0'][data-tt]").count() == pg.locator("#tp-det .tpr-grp").count() > 0, "every group title has a tooltip"
            leg = pg.inner_text("#tp-det .tpr-mxleg"); assert "White = none, darker green = larger area" in leg and "symbol = peak state" in leg and "absent" in leg and "filled" in leg, leg
            pg.locator("#tp-det .tpr-cell").nth(3).focus(); pg.wait_for_selector("#tph-pop:not([hidden])")
            tip = pg.inner_text("#tph-pop"); assert "area" in tip and "peak filled" in tip, tip
            pg.locator("#tp-det .tpr-grp b").first.focus(); pg.wait_for_selector("#tph-pop:not([hidden])"); assert pg.inner_text("#tph-pop").strip() != ""
            assert pg.locator("#tp-det table.tpr tbody tr").count() == 6 and "Normalized" in pg.inner_text("#tp-det table.tpr thead")
            pg.click('#tr-t [data-tg="2"]'); settle(); assert pg.get_attribute("#tr-t [data-tab=samples]", "aria-selected") == "true"
            assert pg.locator("#tp-det .tpr-cell").count() == 12 and pg.locator("#tp-det .tpr-tw.sc").count() == 1, "inner scroll beyond 10 rows"
            assert pg.locator("#tp-det .tpr-cell .on-w, #tp-det .tpr-cell.on-w").count() > 0
            h = pg.evaluate("document.querySelector('#tp-det').getBoundingClientRect().height"); assert h <= 420, ("panel height with 12 samples", h)
        step("TPRIGA samples: one 24 px cell per sample with symbol and label, table, scroll beyond 10 rows, height", riga_samples)

        def cells_column():
            pg.evaluate("""() => { const d = document.createElement('div'); d.id = 'cc-t'; d.className = 'tp'; document.querySelector('#tp-right').appendChild(d);
              d.innerHTML = TPTAB.cells(['detected', 'absent', 'filled', 'detected'], 12, [{ time: 30, area: 1e6 }, { time: 0, area: 0 }, { time: 10, area: 1e3 }, { time: 60, area: 5e4 }]); }""")
            c = pg.evaluate("[...document.querySelectorAll('#cc-t .tpt-cell')].map(e => [e.textContent, e.getAttribute('data-tt-text'), e.getAttribute('tabindex'), e.getBoundingClientRect().width, e.style.background])")
            assert [x[0] for x in c] == ["○", "◐", "●", "●"], ("order: t0 first, then growing times", c)
            assert all(x[1] and x[2] == "0" and x[3] >= 24 for x in c), c
            assert "time 10" in c[1][1] and "area 1000" in c[1][1] and "peak filled" in c[1][1], c[1]
            assert c[0][4] == "" and c[1][4] != c[2][4] != c[3][4] and "color-mix" in c[2][4] and "--tpr-g" in c[2][4], ("green gradient by area, white when there is none", c)
            cls = pg.evaluate("[...document.querySelectorAll('#cc-t .tpt-cell')].map(e => e.className)")
            assert "on-w" in cls[2] and "on-k" in cls[3] and "on-k" in cls[1] and "on-" not in cls[0] and "tpt-gs" in cls[1], ("symbol contrast class follows the area; a separator between groups", cls)
            pg.locator("#cc-t .tpt-cell").nth(2).focus(); pg.wait_for_selector("#tph-pop:not([hidden])"); assert "time 30" in pg.inner_text("#tph-pop")
            pg.evaluate("document.getElementById('cc-t').remove()")
        step("Cells column: sample order, focusable symbols with help, area gradient", cells_column)

        def riga_identity_quality():
            pg.click('#tr-t [data-tg="0"]'); settle()
            pg.click("#tr-t [data-tab=identity]"); settle()
            t = pg.inner_text("#tp-det"); assert "Neutral formula" in t and "[M+H]+ formula" not in t and "✗" in t and "✓" in t and "Persistent" in t, t
            assert pg.locator("#tp-det .tpr-cand tbody tr").count() == 5 and "C14H10N2O2S" not in t and "found in 4 of 4" in (pg.get_attribute("#tp-det .tpr-pk.ok", "data-tt-text") or "")
            assert pg.get_attribute("#tp-det details.tpr-det", "open") is None and "256.0518" not in t, "the neutral mass only on request"
            pg.click("#tp-det details.tpr-det > summary"); assert "256.0518" in pg.inner_text("#tp-det details.tpr-det")
            pg.click("#tp-det .tpr-more"); assert pg.get_attribute("#tp-det .tpr-more", "aria-expanded") == "true" and "C14H10N2O2S" in pg.inner_text("#tp-det")
            pg.click("#tr-t [data-tab=quality]"); settle()
            q = pg.inner_text("#tp-det dl.tpr-q"); assert "few points" in q and "0.11" in q and "142" in q and "absent, ratio 0.01" in q and "Left out of the view by" in q, q
            h = pg.evaluate("document.querySelector('#tp-det').getBoundingClientRect().height"); assert h <= 420, h
        step("TPRIGA identity (candidates, proofs, +N) and quality (definition list)", riga_identity_quality)

        def riga_partial():
            pg.click("#tr-t [data-tab=overview]"); pg.click('#tr-t [data-tg="1"]'); settle()
            t = pg.inner_text("#tp-det .tpr-ov"); assert "Spectrum: not available for this row." in t, t
            assert pg.locator("#tp-det .tpr-ov svg").count() == 1 and pg.locator("#tp-det .tpr-line").count() == 0
            pg.click("#tr-t [data-tab=identity]"); assert "Identity: not available for this row." in pg.inner_text("#tp-det"), pg.inner_text("#tp-det")
            pg.click("#tr-t [data-tab=quality]"); assert "Quality: not available" in pg.inner_text("#tp-det")
            assert pg.locator("#tp-det .tpt-head .lvb.pd-1, #tp-det .tpt-head [class*=pd-]").count() == 0, "no prediction without a value"
        step("TPRIGA: a model with only xic and samples says what is missing and draws no empty box", riga_partial)

        def riga_narrow():
            pg.click("#tr-t [data-tab=overview]"); pg.click('#tr-t [data-tg="2"]'); settle()
            pg.set_viewport_size({"width": 600, "height": 900}); settle()
            r = pg.evaluate("[...document.querySelectorAll('#tp-det .tpr-ov > .tpr-ch svg')].map(s => { const b = s.getBoundingClientRect(); return [b.left, b.top] })"); assert len(r) == 2, r
            assert r[1][1] > r[0][1] + 50 and abs(r[1][0] - r[0][0]) <= 2, ("one under the other at 600 px", r)
            pg.set_viewport_size({"width": 1280, "height": 900}); settle()
        step("TPRIGA: the charts stack under 900 px", riga_narrow)

        def riga_axe(theme):
            def f():
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                for tab in ("overview", "samples", "identity", "quality"):
                    pg.click(f"#tr-t [data-tab={tab}]"); settle()
                    if AXE:
                        if not pg.evaluate("typeof axe") == "object": pg.add_script_tag(path=AXE)
                        v = pg.evaluate(RUN % "'#tr-host'"); assert not v, (tab, v)
                pg.click("#tr-t [data-tab=overview]"); settle(); pg.locator("#tr-host").screenshot(path=str(SHOTS / f"riga-{theme}-1280.png"))
                pg.set_viewport_size({"width": 390, "height": 900}); settle(); pg.locator("#tr-host").screenshot(path=str(SHOTS / f"riga-{theme}-390.png"))
                w = pg.evaluate("[document.querySelector('#tp-det').getBoundingClientRect().width, document.querySelector('#tr-host .tpt-wrap').clientWidth]"); assert w[0] <= w[1] + 2, w
                pg.set_viewport_size({"width": 1280, "height": 900})
            return f
        step("TPRIGA: axe without serious violations in the four tabs, light theme (+ shots 1280 and 390 px)", riga_axe("light"))
        step("TPRIGA: axe without serious violations in the four tabs, dark theme (+ shots 1280 and 390 px)", riga_axe("dark"))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
if errs: print("PAGE ERRORS", errs[:5])
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
