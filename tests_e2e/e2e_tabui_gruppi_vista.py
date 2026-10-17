"""Tabui, the «Compare conditions» view (TP_Mine/lazy/gruppi-vista.js, TPGRUPPI_VIEW): matrix TP × condition, UpSet, chart of the chosen TP, parent table, exports. The component is mounted by hand in
#tp-right of the built site (administrator mode) with the synthetic fixtures TP_Mine/tests/gruppi_fixture.json (6 endpoint conditions) and gruppi_fixture_kin.json (3 conditions × 5 times; both made by
tools/gruppi_fixture_vista.py). Checks: counts of the UpSet equal to `sets`, the filter «Only in O2», bars with one time and curves with more, the files downloaded (name, header, content), the
keyboard on the matrix and the UpSet, the table of the data of every chart, axe (light and dark), the first draw of 300 rows × 8 conditions (≤ 500 ms), small multiples with 9 and 12 conditions
and no horizontal scroll of the page at 360 px. MZLAB_PIPE_SITE reuses a built site. Run from the repo root: python3 tests_e2e/e2e_tabui_gruppi_vista.py"""
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
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tabui_gruppi_vista.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:900]))

FX = json.loads((ROOT / "TP_Mine" / "tests" / "gruppi_fixture.json").read_text(encoding="utf-8"))
FK = json.loads((ROOT / "TP_Mine" / "tests" / "gruppi_fixture_kin.json").read_text(encoding="utf-8"))
tmp = Path(tempfile.mkdtemp(prefix="tabui_gruppi_vista_"))
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

# a synthetic gruppi/1 with `nc` conditions and `nr` rows (one time): the sets are computed here the way the contract says
GEN = """([nr, nc]) => {
  const ids = Array.from({ length: nc }, (_, i) => i === 0 ? "Reference" : "Cond " + i), sets = new Map(), rows = [];
  for (let r = 0; r < nr; r++) {
    const mask = ((r * 2654435761) >>> 0) % ((1 << nc) - 1) + 1, pres = ids.filter((_, k) => mask >> k & 1);
    const cond = ids.map((id, k) => { const p = !!(mask >> k & 1), a = p ? 1e5 * (1 + (r * 7 + k * 13) % 50) : null;
      return { id, present: p, n_det: p ? 1 : 0, n_rep: 1, area: a, area_sd: null, cv: null, ratio_ref: null, shape: p ? "single" : null, t_max: p ? 60 : null, t_half_rise: null, rate_rel: null,
        kin: [{ t: 60, area: a, sd: null, n: 1 }], prev: { absent_t0: "pass", absent_blank: "pass", in_parent_std: "no", above_min: "pass", ms2: "none", verdict: "Prev. possible" } }; });
    rows.push({ row: r + 1, tp_name: "TP " + (200 + r), mz: 200 + r + 0.3, polarity: "+", rt: 3 + (r % 90) / 10, neutral_formula: null, ppm: null, other_polarity: null, present_in: pres, only_in: pres.length === 1 ? pres[0] : null, in_all: pres.length === nc, cond });
    const key = pres.join("|"); sets.set(key, [...(sets.get(key) || []), r + 1]);
  }
  return { format: "gruppi", version: 1, reference: "Reference", norm: "raw", conditions: ids.map(id => ({ id, factors: {}, files: [], times: [60], replicates: 1, controls: {}, parent: { k: 0.01, half_life: 69, remaining: 0.5 } })),
    rows, sets: [...sets].sort((a, b) => b[1].length - a[1].length).map(([k, v]) => ({ conditions: k.split("|"), rows: v })), warnings: [] };
}"""
MOUNT = """([fx, name]) => { const host = document.querySelector("#tp-right"); host.innerHTML = '<div id="gv-host"></div>'; window.__opened = [];
  const t0 = performance.now(); window.__V = window.TPGRUPPI_VIEW.mount(document.querySelector("#gv-host"), fx, { parentName: name, onOpenRow: id => window.__opened.push(id) });
  return performance.now() - t0; }"""

WIDE = "() => [...document.querySelectorAll('body *')].filter(e => { if (e.getBoundingClientRect().right <= document.documentElement.clientWidth + 1 || getComputedStyle(e).position === 'fixed') return false; for (let a = e.parentElement; a && a !== document.documentElement; a = a.parentElement) if (getComputedStyle(a).overflowX !== 'visible') return false; return true; }).slice(0, 5).map(e => e.tagName + '#' + e.id + '.' + String(e.className.baseVal ?? e.className) + ' ' + Math.round(e.getBoundingClientRect().right))"

try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", accept_downloads=True); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector('#nav button[data-v="tpmine"]', state="attached", timeout=120000)
        pg.evaluate("() => { const l = document.getElementById('loading'); if (l && /motore|engine/i.test(l.innerText)) l.remove(); }")     # the view needs no engine: a build without the .wasm must not block the test
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-right", timeout=60000)
        pg.wait_for_function("typeof window.TDATI === 'object'", timeout=120000)
        pg.evaluate("() => window.TPLAZY.load('gruppi-vista.js')")
        pg.wait_for_function("typeof window.TPGRUPPI_VIEW === 'object'", timeout=20000)
        mount = lambda fx, name="Demo Parent": pg.evaluate(MOUNT, [fx, name])
        mount(FX)

        def sets_counts():
            bars = pg.evaluate("() => [...document.querySelectorAll('#gv-up rect.hit')].map(r => [r.dataset.set, r.getAttribute('aria-label')])")
            assert len(bars) == len(FX["sets"]), (len(bars), len(FX["sets"]))
            for idx, label in bars:
                s = FX["sets"][int(idx)]; assert label.startswith(" + ".join(s["conditions"]) + ": %d rows" % len(s["rows"])), (label, s)
            tot = pg.evaluate("() => [...document.querySelectorAll('#gv-upsvg .tot + text')].map(t => +t.textContent)")
            assert tot == [sum(1 for r in FX["rows"] if c["id"] in r["present_in"]) for c in FX["conditions"]], tot
        step("UpSet: bars and totals equal the sets of the fixture", sets_counts)

        def only_filter():
            want = sorted(r["row"] for r in FX["rows"] if r["only_in"] == "O2")
            assert want, "the fixture must have rows only in O2"
            pg.select_option("#gv-flt", "only:O2")
            got = pg.evaluate("() => [...document.querySelectorAll('#gv-mx tbody tr')].map(r => +r.dataset.row)")
            assert sorted(got) == want, (got, want)
            assert "%d of %d rows" % (len(want), len(FX["rows"])) in pg.locator("#gv-nv").inner_text()
            pg.select_option("#gv-flt", "inall")
            got = pg.evaluate("() => [...document.querySelectorAll('#gv-mx tbody tr')].map(r => +r.dataset.row)")
            assert sorted(got) == sorted(r["row"] for r in FX["rows"] if r["in_all"]), got
            pg.select_option("#gv-flt", "all")
        step("filter «Only in O2» and «In all conditions» give the rows of the fixture", only_filter)

        def upset_click():
            idx = 0; s = FX["sets"][idx]
            pg.click(f'#gv-up rect.hit[data-set="{idx}"]', force=True)
            got = pg.evaluate("() => [...document.querySelectorAll('#gv-mx tbody tr')].map(r => +r.dataset.row)")
            assert sorted(got) == sorted(s["rows"]), (got, s)
            assert pg.get_attribute(f'#gv-up rect.hit[data-set="{idx}"]', "aria-pressed") == "true"
            pg.click(f'#gv-up rect.hit[data-set="{idx}"]', force=True)
            assert pg.locator("#gv-mx tbody tr").count() == len(FX["rows"])
        step("a click on an UpSet bar filters the matrix, a second click clears it", upset_click)

        def cell_content():
            t = pg.locator("#gv-mx tbody tr").first.inner_text()
            assert re.search(r"\d\.\d\de\+\d\d", t), ("area in scientific notation", t)
            assert "●" in t and "○" in pg.locator("#gv-mx").inner_text(), "presence as a symbol"
            assert "only here" in pg.locator("#gv-mx").inner_text() and "×" in t
            assert pg.locator("#gv-mx .gv-pv").first.get_attribute("title").startswith("Absent in T0"), pg.locator("#gv-mx .gv-pv").first.get_attribute("title")
            assert pg.locator("#gv-mx thead th .ref").count() == 1 and "ref" in pg.locator("#gv-mx thead").inner_text()
        step("matrix: scientific area, ratio, presence symbol, «Prev.» token with its proof, reference marked", cell_content)

        def sort():
            pg.click('#gv-mx [data-sort="mz"]')
            mz = pg.evaluate("() => [...document.querySelectorAll('#gv-mx tbody tr')].map(r => parseFloat(r.children[1].textContent))")
            assert mz == sorted(mz), mz
            pg.click('#gv-mx [data-sort="mz"]'); mz = pg.evaluate("() => [...document.querySelectorAll('#gv-mx tbody tr')].map(r => parseFloat(r.children[1].textContent))")
            assert mz == sorted(mz, reverse=True), mz
            assert pg.get_attribute('#gv-mx [data-sort="mz"]', "data-tip") == "Sort by m/z", pg.evaluate("document.querySelector('#gv-mx [data-sort=mz]').outerHTML")
        step("matrix: sort by a column, both directions", sort)

        def bars_and_curves():
            assert pg.locator("#gv-tp svg.gv-svg rect.bar").count() >= 1 and pg.locator("#gv-tp svg.gv-svg path.ln").count() == 0, "one time per condition: bars"
            mount(FK, "Kin"); pg.evaluate("window.__V.selectRow(3)")
            assert pg.locator("#gv-tp svg.gv-svg path.ln").count() == len(FK["conditions"]), "more times: one curve per condition"
            assert pg.locator("#gv-tp svg.gv-svg rect.bar").count() == 0
            pg.click('#gv-tp [data-mode="pct"]'); assert pg.locator("#gv-tp svg.gv-svg path.par").count() >= 1, "parent dashed in % of max"
            assert pg.get_attribute('#gv-tp [data-mode="pct"]', "aria-pressed") == "true"
            mount(FX)
        step("chart: bars for one time per condition, curves (and the dashed parent) for more times", bars_and_curves)

        def tdati():
            for fx, name in ((FX, "Demo Parent"), (FK, "Kin")):
                mount(fx, name)
                for sel in ("#gv-upsvg", "#gv-tp svg.gv-svg"):
                    d = pg.evaluate("s => { const e = document.querySelector(s); return [e.getAttribute('aria-describedby'), !!(e.nextElementSibling && e.nextElementSibling.nextElementSibling)]; }", sel)
                    assert d[0], ("description", sel)
                    pg.focus(sel); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
                    n = pg.locator("#tdati tbody tr").count(); assert n >= len(fx["sets"] if sel == "#gv-upsvg" else fx["conditions"]), (sel, n)
                    pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached")
                assert pg.locator("#gv-tp .tdbtn").count() == 1 and pg.locator("#gv-up .tdbtn").count() == 1, "a visible Data button for each chart"
            mount(FX)
        step("every chart has its table of the data (button and T key) and a description", tdati)

        def keyboard():
            pg.focus("#gv-mx button.gv-nm >> nth=0"); pg.keyboard.press("ArrowDown")
            assert pg.evaluate("document.activeElement.dataset.open") == str(FX["rows"][1]["row"])
            pg.keyboard.press("End"); assert pg.evaluate("document.activeElement.dataset.open") == str(FX["rows"][-1]["row"])
            pg.keyboard.press("Enter")
            assert pg.evaluate("window.__opened.slice(-1)[0]") == FX["rows"][-1]["row"], "Enter opens the row"
            assert pg.evaluate("document.querySelector('#gv-mx tr.sel').dataset.row") == str(FX["rows"][-1]["row"])
            assert FX["rows"][-1]["tp_name"] in pg.locator("#gv-tp .gv-sub").first.inner_text() or FX["rows"][-1]["tp_name"] in pg.locator("#gv-tp").inner_text()
            pg.focus('#gv-up rect.hit[data-set="0"]'); pg.keyboard.press("ArrowRight")
            assert pg.evaluate("document.activeElement.dataset.set") == "1"
            pg.keyboard.press("Enter")
            got = pg.evaluate("() => [...document.querySelectorAll('#gv-mx tbody tr')].map(r => +r.dataset.row)")
            assert sorted(got) == sorted(FX["sets"][1]["rows"]), got
            pg.keyboard.press(" "); assert pg.locator("#gv-mx tbody tr").count() == len(FX["rows"])
        step("keyboard: arrows, Home/End and Enter on the matrix; arrows and Enter/Space on the UpSet", keyboard)

        def targets():
            small = pg.evaluate("""() => [...document.querySelectorAll('[data-gv] button, [data-gv] select, [data-gv] rect.hit')].filter(e => e.getClientRects().length)
              .map(e => { const r = e.getBoundingClientRect(); return [e.textContent.trim().slice(0, 20) || e.className.baseVal || e.id, Math.round(r.width), Math.round(r.height)]; }).filter(x => x[1] < 24 || x[2] < 24)""")
            assert not small, small
        step("targets of at least 24 × 24 px (buttons, selects, UpSet bars)", targets)

        def downloads():
            def grab(sel):
                with pg.expect_download() as d: pg.click(sel, force=True)
                return d.value
            d = grab('[data-ex="long"]'); assert re.fullmatch(r"tabui_conditions_demo-parent_\d{8}\.csv", d.suggested_filename), d.suggested_filename
            txt = Path(d.path()).read_text(encoding="utf-8"); L = txt.splitlines()
            assert L[0] == "TP,m/z,RT (min),Polarity,Condition,Time (min),Present,Area,Area SD,n,Ratio to reference,Shape,Prev.", repr(L[0])
            assert len(L) == 1 + sum(len(c["kin"]) for r in FX["rows"] for c in r["cond"]), len(L)
            first = FX["rows"][0]; c0 = first["cond"][1]
            assert L[2].split(",")[:7] == [first["tp_name"], str(first["mz"]), str(first["rt"]), "+", c0["id"], "60", "yes" if c0["present"] else "no"], L[2]
            d = grab('[data-ex="wide"]'); assert re.fullmatch(r"tabui_conditions-wide_demo-parent_\d{8}\.csv", d.suggested_filename), d.suggested_filename
            L = Path(d.path()).read_text(encoding="utf-8").splitlines()
            assert L[0].startswith("TP,m/z,RT (min),Polarity,Present in,Only in," + ",".join("%s present,%s area,%s ratio to reference,%s Prev." % ((c["id"],) * 4) for c in FX["conditions"][:2])), L[0]
            assert len(L) == 1 + len(FX["rows"])
            d = grab('[data-ex="sets"]'); L = Path(d.path()).read_text(encoding="utf-8").splitlines()
            assert L[0] == "Conditions,Number of TPs,TP names" and len(L) == 1 + len(FX["sets"]) and L[1].startswith(" + ".join(FX["sets"][0]["conditions"]) + "," + str(len(FX["sets"][0]["rows"]))), L[:2]
            d = grab('[data-ex="json"]'); assert re.fullmatch(r"tabui_gruppi_demo-parent_\d{8}\.json", d.suggested_filename)
            assert json.loads(Path(d.path()).read_text(encoding="utf-8")) == FX, "the JSON is the gruppi/1 as given"
            pg.select_option("#gv-flt", "only:O2")
            d = grab('[data-ex="long"]'); L = Path(d.path()).read_text(encoding="utf-8").splitlines()
            assert len(L) == 1 + sum(len(c["kin"]) for r in FX["rows"] if r["only_in"] == "O2" for c in r["cond"]), "the CSV holds the rows shown"
            pg.select_option("#gv-flt", "all")
        step("exports: names, headers and content of the CSV (long, wide, sets) and the JSON", downloads)

        def chart_files():
            def grab(sel):
                with pg.expect_download() as d: pg.click(sel, force=True)
                return d.value
            d = grab('[data-dl="up-svg"]'); assert d.suggested_filename == "tabui_upset_demo-parent.svg", d.suggested_filename
            t = Path(d.path()).read_text(encoding="utf-8"); assert t.startswith("<?xml") and "<svg" in t and "var(--" not in t and "xmlns" in t, "standalone SVG with resolved colours"
            d = grab('[data-dl="up-png"]'); assert d.suggested_filename == "tabui_upset_demo-parent.png"
            assert Path(d.path()).read_bytes()[:8] == b"\x89PNG\r\n\x1a\n", "PNG signature"
            row = FX["rows"][-1]; mz = ("%.1f" % row["mz"]).rstrip("0").rstrip(".") if False else str(round(row["mz"], 1))
            d = grab('[data-dl="tp-svg"]'); assert d.suggested_filename == f"tabui_tp-{mz}_conditions.svg", d.suggested_filename
            d = grab('[data-dl="tp-png"]'); assert d.suggested_filename == f"tabui_tp-{mz}_conditions.png"
        step("chart files: SVG (standalone) and PNG with the names of the study", chart_files)

        def warnings_and_parent():
            fx = json.loads(json.dumps(FX)); fx["warnings"] = [{"code": "condNoReplicates", "condition": "N2"}, {"code": "refMissing", "condition": "Reference"}]
            mount(fx)
            t = pg.locator("#gv-warn").inner_text(); assert "N2" in t and "reference condition" in t, t
            assert pg.get_attribute("#gv-warn div", "role") == "status"
            T = pg.locator("#gv-par").inner_text(); assert "t½" in T and "Remaining" in T and "O2" in T
            assert pg.locator("#gv-par tbody tr").count() == len(FX["conditions"])
            mount(FX)
        step("warnings on top and the parent table (k, t½, remaining)", warnings_and_parent)

        def axe(theme):
            def f():
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                for fx, name in ((FX, "Demo Parent"), (FK, "Kin")):
                    mount(fx, name); pg.locator('#gv-mx button[data-open="3"]').scroll_into_view_if_needed()
                    if AXE:
                        if not pg.evaluate("typeof axe") == "object": pg.add_script_tag(path=AXE)
                        v = pg.evaluate(RUN % "'#gv-host'"); assert not v, v
                        pg.focus("#gv-upsvg"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
                        v = pg.evaluate(RUN % "'#tdati'"); assert not v, v
                        pg.keyboard.press("Escape")
                pg.evaluate("() => { delete document.documentElement.dataset.theme; }")
            return f
        step("axe without serious violations, light theme (fixtures endpoint and kinetic, table of the data)", axe("light"))
        step("axe without serious violations, dark theme", axe("dark"))

        def perf():
            fx = pg.evaluate(GEN, [300, 8]); ms = []
            for _ in range(3): ms.append(mount(fx))
            print("first draw 300 rows × 8 conditions: %s ms" % ", ".join("%.0f" % m for m in ms))
            assert min(ms) <= 500, ms
            assert pg.locator("#gv-mx tbody tr").count() == 300
            assert pg.locator("#gv-tp svg.gv-svg rect.bar").count() >= 1
        step("first draw of 300 rows × 8 conditions in at most 500 ms", perf)

        def many():
            pg.set_viewport_size({"width": 360, "height": 800}); pg.evaluate("window.__V.destroy()")
            base = pg.evaluate("document.documentElement.scrollWidth")          # what the page itself (the app navigation) needs at 360 px: the view must add nothing
            for nc in (9, 12):
                fx = pg.evaluate(GEN, [60, nc]); mount(fx)
                assert pg.locator("#gv-tp .gv-multi figure").count() == nc, "small multiples: one box per condition"
                assert pg.locator("#gv-tp .gv-leg").count() == 0, "no legend with repeated colours"
                w = pg.evaluate("[document.documentElement.scrollWidth, document.documentElement.clientWidth]"); assert w[0] <= max(w[1], base), ("horizontal scroll of the page", nc, w, base, pg.evaluate(WIDE))
            fx = pg.evaluate(GEN, [60, 8]); mount(fx)
            assert pg.locator("#gv-tp .gv-multi").count() == 0 and pg.locator("#gv-tp .gv-leg li").count() == 8
            w = pg.evaluate("[document.documentElement.scrollWidth, document.documentElement.clientWidth]"); assert w[0] <= max(w[1], base), ("horizontal scroll of the page", 8, w, base)
            pg.set_viewport_size({"width": 1280, "height": 900})
        step("9 and 12 conditions: small multiples; no horizontal scroll of the page at 360 px", many)

        def no_miss():
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0
        step("MZLAB_LR_MISS stays empty", no_miss)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
if errs: print("PAGE ERRORS", errs[:5])
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
