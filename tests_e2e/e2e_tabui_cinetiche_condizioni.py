"""Tabui, the Kinetics tab with several conditions (TPGRUPPI_VIEW.kinetics in TP_Mine/lazy/gruppi-vista.js): one curve per condition on the same time axis, overlaid or side by side, area or % of max or % of each own
max, the dark control as a dashed curve, the comparison of conditions that share a name part (TW_PMS against MQ_PMS), the table of the data and axe. Synthetic fixture (3 conditions renamed TW_PMS, MQ_PMS, TW_H2O2 from
TP_Mine/tests/gruppi_fixture_kin.json, with a dark course added). MZLAB_PIPE_SITE reuses a built site. Run from the repo root: python3 tests_e2e/e2e_tabui_cinetiche_condizioni.py"""
import json, os, re, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import axe_path  # noqa: E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tabui_cinetiche_condizioni.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:900]))

FX = json.loads((ROOT / "TP_Mine" / "tests" / "gruppi_fixture.json").read_text(encoding="utf-8"))
FK = json.loads((ROOT / "TP_Mine" / "tests" / "gruppi_fixture_kin.json").read_text(encoding="utf-8"))
tmp = Path(tempfile.mkdtemp(prefix="tabui_cinetiche_cond_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True, capture_output=True)
shutil.copy2(ROOT / "TP_Mine" / "lazy" / "gruppi-vista.js", site / "static" / "mzfinder" / "lazy" / "gruppi-vista.js")      # fresh source over the site that was built
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
for _ in range(50):
    try: __import__("urllib.request").request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"

def fixture():
    fx = json.loads(json.dumps(FK)); new = {c["id"]: n for c, n in zip(fx["conditions"], ["TW_PMS", "MQ_PMS", "TW_H2O2"])}
    fx["reference"] = new.get(fx["reference"], fx["reference"])
    for c in fx["conditions"]: c["id"] = new[c["id"]]
    for s in fx["sets"]: s["conditions"] = [new[x] for x in s["conditions"]]
    for r in fx["rows"]:
        r["present_in"] = [new[x] for x in r["present_in"]]; r["only_in"] = new.get(r["only_in"])
        for c in r["cond"]:
            c["id"] = new[c["id"]]
            if c["id"] == "TW_PMS": c["dark"] = [{"t": k["t"], "area": (k["area"] or 0) * 0.1, "sd": None, "n": 1} for k in c["kin"]]
    return fx
FXD = fixture()
ROW = next(r["row"] for r in FXD["rows"] if all(c["present"] and len(c["kin"]) > 1 for c in r["cond"]))

try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce"); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector('#nav button[data-v="tpmine"]', state="attached", timeout=120000)
        pg.evaluate("() => { const l = document.getElementById('loading'); if (l && /motore|engine/i.test(l.innerText)) l.remove(); }")
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-right", timeout=60000)
        pg.wait_for_function("typeof window.TDATI === 'object'", timeout=120000)
        pg.evaluate("() => window.TPLAZY.load('gruppi-vista.js')")
        pg.wait_for_function("typeof window.TPGRUPPI_VIEW === 'object'", timeout=20000)
        pg.evaluate("([fx, row]) => { document.querySelector('#tp-right').innerHTML = '<div id=\"gv-host\"></div>'; window.__V = window.TPGRUPPI_VIEW.kinetics(document.querySelector('#gv-host'), fx, {parentName: 'Demo'}); window.__V.selectRow(row); }", [FXD, ROW])
        N = lambda sel: pg.locator(sel).count()

        def overlay():
            assert N("#gv-tp svg.gv-svg:not(.gv-leg svg)") == 1 and N("#gv-tp svg.gv-svg path.ln") == 3, "three curves on one axis"
            assert N("#gv-tp svg.gv-svg path.drk") == 1 and N("#gv-tp .gv-leg li") >= 4, "dark curve dashed + legend"
        step("overlaid by default: one curve per condition and the dark control dashed", overlay)

        def side():
            pg.click('[data-layout="side"]'); assert N("#gv-tp .gv-multi figure") == 3 and N("#gv-tp svg.gv-svg path.drk") == 1
            assert pg.get_attribute('[data-layout="side"]', "aria-pressed") == "true"
            pg.click('[data-layout="overlay"]'); assert N("#gv-tp .gv-multi") == 0
        step("side by side: one small chart per condition, back to overlaid", side)

        def modes():
            for m in ("area", "pct", "own"):
                pg.click(f'[data-mode="{m}"]'); assert pg.get_attribute(f'[data-mode="{m}"]', "aria-pressed") == "true"
            pg.click('[data-mode="own"]')
            tops = pg.evaluate("() => [...document.querySelectorAll('#gv-tp svg.gv-svg:not(.gv-leg svg) text.mut')].map(t => t.textContent)")
            assert "100" in tops, tops
        step("area, % of max and % of each own max", modes)

        def compare():
            opts = pg.evaluate("() => [...document.querySelectorAll('#gv-cmp option')].map(o => o.textContent)")
            assert any("Same ending" in o and "TW_PMS" in o and "MQ_PMS" in o for o in opts), opts
            pg.select_option("#gv-cmp", index=[i for i, o in enumerate(opts) if "Same ending" in o][0])
            assert N("#gv-tp svg.gv-svg path.ln") == 2, "TW_PMS against MQ_PMS only"
            pg.select_option("#gv-cmp", "all"); assert N("#gv-tp svg.gv-svg path.ln") == 3
        step("compare the same treatment in two matrices", compare)

        def data():
            pg.focus("#gv-tp svg.gv-svg"); pg.keyboard.press("t")
            pg.wait_for_selector("table", timeout=3000)
            txt = pg.evaluate("() => document.body.innerText"); assert "Dark" in txt and "% of own max" in txt
            pg.keyboard.press("Escape")
        step("the table of the data has the dark series and both percentages", data)

        def axe():
            if not AXE: return
            pg.add_script_tag(path=AXE)
            for theme in ("light", "dark"):
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                v = pg.evaluate(RUN % "'#gv-host'"); assert not v, v
        step("axe: no serious violations in light and dark", axe)

        def small():
            pg.set_viewport_size({"width": 360, "height": 800}); w = pg.evaluate("[document.documentElement.scrollWidth, document.documentElement.clientWidth]")
            pg.click('[data-layout="side"]'); w2 = pg.evaluate("[document.documentElement.scrollWidth, document.documentElement.clientWidth]")
            assert w2[0] <= max(w2[1], w[0]), (w, w2)
        step("no horizontal scroll of the page at 360 px", small)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
if errs: print("PAGE ERRORS", errs[:5])
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
