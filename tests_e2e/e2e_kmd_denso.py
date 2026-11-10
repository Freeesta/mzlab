"""Statistica, «Kendrick mass defect» with a feature table of thousands of ions (synthetic): the plot draws its points as one path per series instead of one
button per point, the click and the hover find the nearest ion, a new selection keeps the plot (and its «Dati» button), a click on a point does not paint the
whole panel again, the «Dati» table writes its rows a chunk at a time while the count and Copy cover all of them."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_kmd_denso.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))

D = HERE / "shots" / "kmd_denso_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = ["HR_t0", "HR_t30"]
for i, n in enumerate(NAMES):
    if not (D / f"{n}.mzML").exists(): ds.hr_series(D / f"{n}.mzML", np.random.default_rng(i), [(300.1 + k, 1 + k, 0.05, 1e6) for k in range(4)])
N = 3000
rng = np.random.default_rng(7)
mz, rt = rng.uniform(100, 900, N), rng.uniform(0.5, 12, N)
CF2 = 49.99681
mz[:6] = 212.97918 + CF2 * np.arange(6); rt[:6] = 3 + np.arange(6)          # one CF2 homologous series among random ions
CSV = "row ID,row m/z,row retention time," + ",".join(n + " Peak area" for n in NAMES) + "\n" + "\n".join(f"{i + 1},{mz[i]:.5f},{rt[i]:.3f},1000,2000" for i in range(N))
SCREEN = "([x, y]) => { const p = new DOMPoint(x, y).matrixTransform(document.querySelector('#kmd-plot').getScreenCTM()); return [p.x, p.y]; }"       # viewBox units to the page, read when used: the panel may have scrolled
SEL = "() => (document.querySelector('#kmd-plot g[data-i]') || {dataset: {}}).dataset.i"

r = Run(port=8949, wd="/tmp/wdkmdd")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / f"{n}.mzML") for n in NAMES])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=2", timeout=120000)
        open_first(pg); pg.wait_for_function("typeof E!=='undefined' && E.files.length>=2", timeout=60000); ready(pg)
        drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec[data-s=kmd]")
        pg.evaluate("t => TABELLA.fromText(t)", CSV); pg.wait_for_selector("#kmd-plot", timeout=60000)
        def dense():
            n = pg.evaluate("[document.querySelectorAll('#kmd-plot g[data-i]').length, document.querySelectorAll('#kmd-plot path.kmd-cloud').length, document.querySelector('#kmd-plot').getElementsByTagName('*').length]")
            assert pg.get_attribute("#kmd-plot", "data-cloud") == "1" and n[0] == 0 and n[1] >= 2 and n[2] < 400, n
            assert pg.get_attribute("#kmd-plot", "role") == "img" and str(N) in pg.get_attribute("#kmd-plot", "aria-label")
            assert pg.locator("#kmd-res .tdbtn").count() == 1, "the plot has its «Dati» button"
        step("thousands of ions: one path per series, no element per point, the plot keeps its role, label and «Dati» button", dense)
        def keep():
            pg.evaluate("document.querySelector('#kmd-plot')._probe = 1; document.querySelector('#stat-pca')._probe = 1")
            pg.locator("#kmd-tab tbody tr").nth(3).dispatch_event("click"); wait_for_js(pg, "E.panels.some(p => p.kmdIon) && document.querySelectorAll('#kmd-plot g[data-i]').length === 1", 30000)
            assert pg.evaluate("document.querySelector('#kmd-plot')._probe === 1"), "the plot is kept across the selection"
            assert pg.evaluate("document.querySelector('#stat-pca')._probe === 1"), "a click inside the panel does not paint it again"
            assert pg.locator("#kmd-res .tdbtn").count() == 1 and pg.get_attribute("#kmd-plot g[data-i]", "tabindex") == "0", "the selected ion is a button, the «Dati» button stays"
        step("a selection keeps the plot and its «Dati» button, the selected ion is a button, the panel is not painted again", keep)
        def click_near():
            # an ion with no other within 3 units: a click rounded to whole pixels still lands nearest to it
            k = pg.evaluate("""() => { const r = KMD.result, P = r.rows.map((_, k) => KMDCLOUD.at(r, k));
              return P.findIndex(([x, y], k) => x > 70 && P.every(([u, v], j) => j === k || (u - x) ** 2 + (v - y) ** 2 > 9)); }""")
            assert k >= 0
            want = pg.evaluate(f"String(KMD.result.rows[{k}].i)"); at = pg.evaluate(f"KMDCLOUD.at(KMD.result, {k})")
            pg.locator("#kmd-plot").scroll_into_view_if_needed(); x, y = pg.evaluate(SCREEN, at)
            pg.mouse.move(x, y); wait_for_js(pg, "document.querySelector('#kmd-hov').getAttribute('visibility') === 'visible'", 10000)
            tip = pg.evaluate("document.querySelector('#kmd-hov title').textContent"); assert "m/z" in tip, tip
            pg.mouse.click(x, y); wait_for_js(pg, f"({SEL})() === '{want}'", 10000)
        step("the hover and the click on the plot find the nearest ion", click_near)
        def dim():
            pg.locator("#kmd-series tbody tr").first.dispatch_event("click"); wait_for_js(pg, "document.querySelector('#kmd-series tr.sel')", 10000)
            op = pg.evaluate("[...document.querySelectorAll('#kmd-plot path.kmd-cloud')].map(p => [p.dataset.s, p.getAttribute('opacity')])")
            s = pg.evaluate("document.querySelector('#kmd-series tr.sel').dataset.s")
            assert all((o == "1") == (k == s) for k, o in op), op
        step("a chosen series stays bright, the others are dimmed", dim)
        def table():
            pg.evaluate("TDATI.open(document.querySelector('#kmd-plot'))"); pg.wait_for_selector("#tdati tbody tr")
            assert pg.locator("#tdati tbody tr:not(.td-more)").count() == 200 and str(N) in pg.inner_text("#tdati-n"), pg.inner_text("#tdati-n")
            for k in range(1, 4):
                pg.evaluate("const s = document.querySelector('#tdati .td-sc'); s.scrollTop = s.scrollHeight")
                wait_for_js(pg, f"document.querySelectorAll('#tdati tbody tr:not(.td-more)').length >= {min(N, 200 + k * 1000)}", 10000)
            assert pg.locator("#tdati tbody tr.td-more").count() == 0 and pg.locator("#tdati tbody tr").count() == N
            assert len(pg.evaluate("KMD.plotTable().rows")) == N, "Copy covers every row"
            pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached")
        step("the «Dati» table writes 200 rows, then 1000 at a time on scrolling; the count and Copy cover all", table)
finally:
    r.close()
for s in steps: print(s)
sys.exit(0 if steps and all(s[1] == "ok" for s in steps) else 1)
