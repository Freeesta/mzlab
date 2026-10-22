"""Charts of the assistant in Flow (docs/agenti/llm.md, «Grafici»): a recording asks for the TIC of the open files and gets a figure with its table (same numbers) and a CSV; a box plot of the
areas; the answer «no view» is text; a view that does not exist is refused with the views that do; the charts without a model come from the menu; colours only from CSS variables; zoom and pan with keys;
axe and 24 px targets on the figure. No request leaves the site. Run from the repo root: python3 tests_e2e/e2e_llm_grafici.py"""
import json, socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi, axe_path; PORT = free_port()
TAB = '#nav button[data-v="flow"]'
AXE = axe_path()
steps = []; LAST = []; CON = []; errs = []; reqs = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
        if LAST: print("DEBUG", name, "::", LAST[0].evaluate("(document.querySelector('#flx-llm') || {innerText: 'no panel'}).innerText")[:800].replace("\n", " | "), "::", CON[-8:], errs[-3:])

def call(n, cid, name, args):
    return json.dumps({"schema": "llmreg/1", "turn": n, "events": [
        {"ev": "tool_call_start", "id": cid, "name": name}, {"ev": "tool_call_delta", "json": json.dumps(args)}, {"ev": "tool_call_end", "id": cid},
        {"ev": "usage", "in": 100, "out": 20, "cache_read": 0}, {"ev": "stop", "reason": "tool"}]})
def text(n, t): return json.dumps({"schema": "llmreg/1", "turn": n, "events": [{"ev": "text_delta", "text": t}, {"ev": "usage", "in": 90, "out": 10, "cache_read": 0}, {"ev": "stop", "reason": "end"}]})
def rec(*lines): return "\n".join(lines) + "\n"
def spec(ref, mark, enc, **kw): return {"spec": {"schema": "grafico/1", "title": "T " + mark, "data": {"ref": ref}, "mark": mark, "encoding": enc, "why": "Test chart.", **kw}}
def f(n): return {"field": n}

N_FILES, N_POINTS = 3, 60
FILES = [{"name": f"s{k}.mzML", "tic": {"rt": [round(i * 0.1, 3) for i in range(N_POINTS)], "i": [1000 + (i * 37 + k * 11) % 5003 for i in range(N_POINTS)]}, "widths": [4 + (i % 5) * 0.5 for i in range(10)]} for k in range(N_FILES)]
CSV = "id,mz,rt," + ",".join(x["name"] for x in FILES) + "\n" + "".join(f"r{i},{100 + i},{1 + i * 0.1}," + ",".join(str((i * 7919 + k * 13) % 100003) for k in range(N_FILES)) + "\n" for i in range(20))
SOURCES = {"files": FILES, "features_csv": CSV}
TIC_SUM = sum(sum(x["tic"]["i"]) for x in FILES)

TIC_REC = rec(call(0, "c1", "chart_make", spec("tic:*", "line", {"x": f("rt"), "y": f("intensity"), "color": f("file")})), text(1, "Here is the TIC."))
BOX_REC = rec(call(0, "c1", "chart_make", spec("features:area", "box", {"x": f("file"), "y": f("area")})), text(1, "Here is the box plot."))
NOVIEW_REC = rec(call(0, "c1", "chart_make", {"spec": {"schema": "grafico/1", "error": "no_view", "why": "The concentration is in no view."}}), text(1, "I cannot draw that."))
BAD_REC = rec(call(0, "c1", "chart_make", spec("nope:1", "line", {"x": f("rt"), "y": f("intensity")})), text(1, "That view does not exist."))
STYLE_REC = rec(call(0, "c1", "chart_make", spec("tic:*", "line", {"x": f("rt"), "y": f("intensity")}, palette=["red"])), text(1, "Understood, no styles."))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(recording=None, sources=SOURCES, reduced=False, coarse=False):
            c = b.new_context(viewport={"width": 1500, "height": 1000}, reduced_motion="reduce" if reduced else "no-preference", has_touch=coarse, is_mobile=coarse); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: CON.append(m.text[:200])); LAST[:] = [pg]; pg.on("request", lambda r: reqs.append(r.url))
            settings = json.dumps({"on": True, "provider": "anthropic", "model": "claude-sonnet-5-5", "level": "L2", "l2max": 5})
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.lang','en');localStorage.setItem('mzlab.profilo','ricerca');localStorage.setItem('mzlab.llm',%s);localStorage.setItem('mzlab.llm.consenso',%s);}catch(_){}"
                               % (json.dumps(settings), json.dumps(json.dumps({"anthropic": {"classes": ["plan", "summary", "names"]}}))))
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector(TAB, timeout=60000); pg.click(TAB); pg.wait_for_selector("#flx-root", timeout=30000)
            pg.wait_for_selector("#flx-assistant", timeout=30000); attendi(pg, "!!window.MZLAB_LLM_AGENT && !!FLOW.llmChart")
            pg.evaluate("s => { FLOW.chartSources = () => s; }", sources)
            if recording: pg.evaluate("t => window.MZLAB_LLM_AGENT.replay(t, false)", recording)
            return pg
        def ask(pg, msg):
            if pg.locator("#flx-llm").is_hidden(): pg.click("#flx-assistant")
            pg.fill("#flx-llm-input", msg); pg.click('#flx-llm button[aria-label="Send the message"]')
        def rows_of(pg, sel):
            return pg.evaluate("s => [...document.querySelectorAll(s + ' tbody tr')].map(r => [...r.children].map(c => c.textContent))", sel)

        def tic():
            pg = fresh(TIC_REC); ask(pg, "Show me the TIC of all the files.")
            pg.wait_for_selector("#flx-llm figure.flx-chart[data-mark=line]", timeout=30000)
            assert pg.locator("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr").count() == N_FILES, "one line per file"
            rows = rows_of(pg, "#flx-llm figure.flx-chart")
            assert 0 < len(rows) <= 5000 and len(rows) == N_FILES * N_POINTS, len(rows)
            assert sum(float(r[1]) for r in rows) == TIC_SUM, "the table has the numbers of the data"
            assert {r[2] for r in rows} == {x["name"] for x in FILES}
            svg_vals = pg.evaluate("[...document.querySelectorAll('#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr')].map(p => (p.getAttribute('d').match(/[ML]/g) || []).length)")
            assert svg_vals == [N_POINTS] * N_FILES, svg_vals
            lab = pg.get_attribute("#flx-llm figure.flx-chart", "aria-label"); assert "T line" in lab and "Test chart." in lab, lab
            with pg.expect_download() as d: pg.click('#flx-llm figure.flx-chart button[aria-label="Download CSV"]')
            lines = Path(d.value.path()).read_text().splitlines()
            assert lines[0] == "rt (min),intensity (cps),file" and len(lines) == 1 + N_FILES * N_POINTS, lines[:2]
            pg.context.close()
        step("TIC of all the files: one line per file, a table with the same numbers, a CSV", tic)

        def box():
            pg = fresh(BOX_REC); ask(pg, "Distribution of the areas per file.")
            pg.wait_for_selector("#flx-llm figure.flx-chart[data-mark=box]", timeout=30000)
            assert pg.locator("#flx-llm figure.flx-chart svg rect").count() >= 1 + N_FILES
            rows = rows_of(pg, "#flx-llm figure.flx-chart"); assert len(rows) == 20 * N_FILES, len(rows)
            pg.context.close()
        step("a box plot of the areas per file, with its table", box)

        def noview():
            pg = fresh(NOVIEW_REC); ask(pg, "Plot the concentration.")
            pg.wait_for_selector("#flx-llm .flx-chart-none", timeout=30000)
            assert "The concentration is in no view." in pg.inner_text("#flx-llm .flx-chart-none") and pg.locator("#flx-llm figure.flx-chart").count() == 0
            pg.context.close()
        step("«no view» is shown as text, never as a chart", noview)

        def refused():
            pg = fresh(BAD_REC); ask(pg, "Plot nonsense.")
            attendi(pg, "document.querySelector('#flx-llm-log').textContent.includes('That view does not exist.')")
            t = pg.inner_text("#flx-llm-log"); assert "There is no view nope:1" in t and pg.locator("#flx-llm figure.flx-chart").count() == 0, t
            pg.context.close(); pg = fresh(STYLE_REC); ask(pg, "Make it red.")
            attendi(pg, "document.querySelector('#flx-llm-log').textContent.includes('no styles')")
            assert "belong to the program" in pg.inner_text("#flx-llm-log") and pg.locator("#flx-llm figure.flx-chart").count() == 0
            pg.context.close()
        step("a view that does not exist and a style are refused with the reason", refused)

        def presets():
            pg = fresh(None); pg.click("#flx-assistant"); pg.click("#flx-llm-charts summary")
            pg.wait_for_selector('#flx-llm-charts button[aria-label="Draw: TIC of all files"]', timeout=30000)
            names = pg.evaluate("[...document.querySelectorAll('#flx-llm-charts button')].map(b => b.getAttribute('aria-label'))")
            assert "Draw: Peak widths" in names and not any("QC" in n for n in names), names
            pg.click('#flx-llm-charts button[aria-label="Draw: TIC of all files"]')
            pg.wait_for_selector("#flx-llm figure.flx-chart[data-ref='tic:*']"); assert len(rows_of(pg, "#flx-llm figure.flx-chart")) == N_FILES * N_POINTS
            pg.context.close()
        step("charts without a model: the menu offers what the open files can draw", presets)

        def look():
            pg = fresh(TIC_REC); ask(pg, "TIC please.")
            pg.wait_for_selector("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", timeout=30000)
            bad = pg.evaluate("""() => [...document.querySelectorAll('#flx-llm figure.flx-chart svg *')].flatMap(e => ['fill', 'stroke'].map(a => [e.tagName, a, e.getAttribute(a)]))
                .filter(x => x[2] && !/^(none|var\\(--[a-z0-9-]+\\)|currentColor)$/.test(x[2]))""")
            assert not bad, bad[:5]
            dash = pg.evaluate("[...document.querySelectorAll('#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr')].map(p => p.getAttribute('stroke-dasharray') || '')")
            assert len(set(dash)) == N_FILES, dash
            pg.context.close()
        step("colours only from CSS variables; each series also has its own dash", look)

        def zoom():
            pg = fresh(TIC_REC); ask(pg, "TIC please.")
            pg.wait_for_selector("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", timeout=30000)
            d0 = pg.get_attribute("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", "d")
            pg.focus("#flx-llm figure.flx-chart"); pg.keyboard.press("+")
            d1 = pg.get_attribute("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", "d"); assert d1 != d0 and len(d1) < len(d0), "zoom with a key"
            assert "Showing" in pg.inner_text("#flx-llm figure.flx-chart [role=status]")
            pg.keyboard.press("]"); d2 = pg.get_attribute("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", "d"); assert d2 != d1, "pan with a key"
            pg.click('#flx-llm figure.flx-chart button[aria-label="Reset zoom"]'); assert pg.get_attribute("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", "d") == d0
            assert len(rows_of(pg, "#flx-llm figure.flx-chart")) == N_FILES * N_POINTS, "the table keeps every row when zoomed"
            pg.context.close()
        step("zoom and pan with buttons and keys, no dragging", zoom)

        def a11y():
            for coarse in (False, True):
                pg = fresh(TIC_REC, reduced=True, coarse=coarse); ask(pg, "TIC please.")
                pg.wait_for_selector("#flx-llm figure.flx-chart svg:not([aria-hidden]) path.sr", timeout=30000)
                need = 40 if coarse else 24
                small = pg.evaluate("n => [...document.querySelectorAll('#flx-llm figure.flx-chart button, #flx-llm figure.flx-chart summary')].filter(e => e.offsetParent).map(e => [e.getAttribute('aria-label') || e.textContent, e.getBoundingClientRect()]).filter(x => x[1].width < n || x[1].height < n).map(x => x[0] + ' ' + Math.round(x[1].width) + 'x' + Math.round(x[1].height))", need)
                assert not small, (coarse, small)
                if AXE and not coarse:
                    pg.add_script_tag(path=AXE)
                    v = pg.evaluate("axe.run(document.querySelector('#flx-llm figure.flx-chart'), {resultTypes: ['violations']}).then(r => r.violations.filter(x => ['serious', 'critical'].includes(x.impact)).map(x => x.id + ': ' + x.nodes.length))")
                    assert not v, v
                pg.context.close()
        step("axe, targets of 24 px (40 on touch) on the figure", a11y)

        def outside():
            ext = [u for u in reqs if not u.startswith(f"http://127.0.0.1:{PORT}/") and not u.startswith(("data:", "blob:"))]
            assert not ext, ext[:5]
            assert not errs, errs[:3]
        step("no request left the site and no script error", outside)
        b.close()
finally:
    srv.terminate()
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("PASS " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
sys.exit(1 if bad else 0)
