"""«Liscia» / «Dettaglio» of the RT x m/z map (mappa.js, mappa-render.js): zoomed in, Liscia keeps the blob of a peak continuous (fewer horizontal stripes of one bin
per m/z / scan than Dettaglio) at three zoom levels, snapshots in shots/; the choice is remembered; the values the sight reads do not depend on it;
the cost of a splat and of a draw is printed and bounded."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8902, wd="/tmp/wd_maplis_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))
LEVELS = [("wide", 3.0, 60.0), ("middle", 1.0, 15.0), ("narrow", 0.4, 4.0)]
STRIPE = """(() => { const c = %s.cv, g = c.getContext('2d'), d = c.width / c.getBoundingClientRect().width, a = %s._a;
  const x0 = Math.round((a.X(a.x0) + 4) * d), x1 = Math.round((a.X(a.x1) - 4) * d), y0 = Math.round((a.Y(a.y1) + 4) * d), y1 = Math.round((a.Y(a.y0) - 4) * d);
  const w = x1 - x0, h = y1 - y0, im = g.getImageData(x0, y0, w, h).data, L = i => im[i] * .3 + im[i + 1] * .59 + im[i + 2] * .11; let s = 0, n = 0, mean = 0;
  for (let y = 0; y < h - 1; y++) for (let x = 0; x < w; x += 2) { const i = (y * w + x) * 4; s += Math.abs(L(i) - L(i + w * 4)); n++; mean += L(i); }
  return { stripe: s / n, mean: mean / n, w, h }; })()""" % (M, M)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); pg.wait_for_timeout(600)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        def zoom(rt, dr, mz, dm):
            pg.evaluate(f"(() => {{ const p = {M}; p.zoom = [{rt - dr}, {rt + dr}]; p.zoomY = [{mz - dm}, {mz + dm}]; draw(p); }})()")
            pg.wait_for_timeout(900); ready(pg); pg.wait_for_timeout(500)
        def mode(m):
            pg.evaluate(f"(() => {{ const s = {M}.el.querySelector('[data-mz=rend]'); s.value = '{m}'; s.dispatchEvent(new Event('change')); }})()")
            pg.wait_for_timeout(900); ready(pg); pg.wait_for_timeout(400)
        def clip(): return pg.evaluate(f"(()=>{{const r={M}.cv.getBoundingClientRect();return {{x:r.left,y:r.top,width:r.width,height:r.height}}}})()")
        res = {}
        def levels():
            for name, dr, dm in LEVELS:
                zoom(14.3, dr, 364.5, dm)
                for m in ("smooth", "detail"):
                    mode(m); res[(name, m)] = pg.evaluate(STRIPE); pg.screenshot(path=SH + f"liscia_{name}_{m}.png", clip=clip())
            print("stripe score:", {f"{k[0]}/{k[1]}": round(v["stripe"], 2) for k, v in res.items()})
            for name, _, _ in LEVELS:
                s, d = res[(name, "smooth")], res[(name, "detail")]
                assert s["mean"] > 0 and d["mean"] > 0, "nothing painted"
                assert s["stripe"] < d["stripe"] * 0.95, (name, s, d)                      # fewer abrupt steps between neighbouring rows
        step("Liscia has fewer stripes than Dettaglio at three zoom levels", levels)
        def values():
            zoom(14.3, 0.4, 364.5, 4.0); mode("smooth"); a = pg.evaluate(f"{M}._a.mapv(14.3, 364.5)"); mode("detail"); b = pg.evaluate(f"{M}._a.mapv(14.3, 364.5)")
            assert a == b and a is not None, (a, b)
        step("the values of the map do not depend on the look", values)
        def cost():
            t = pg.evaluate("""(() => { const n = 1000, v = new Float32Array(n * n); let s = 1; for (let k = 0; k < 20000; k++) { s = (s * 1103515245 + 12345) % 2147483648; v[s % (n * n)] = 1 + (s % 7); }
              const t0 = performance.now(); MAPREND.splat(v, n, n, 1, 3); return performance.now() - t0; })()""")
            d = {}
            for m in ("smooth", "detail"):
                mode(m); d[m] = pg.evaluate(f"(async () => {{ const p = {M}; const t0 = performance.now(); for (let i = 0; i < 20; i++) await drawMap(p); return (performance.now() - t0) / 20; }})()")
            print("splat 1000x1000 (20000 cells, sigma 1 x 3 bins):", round(t, 1), "ms; draw of the zoomed map:", {k: round(v, 2) for k, v in d.items()})
            assert t < 500 and d["smooth"] < 2.5 * d["detail"] + 20, (t, d)
        step("cost of the splat and of a draw", cost)
        def remembered():
            mode("detail"); assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.mappa')||'{}').rend") == "detail"
            pg.reload(); pg.wait_for_timeout(1200)
            mode_val = pg.evaluate("JSON.parse(localStorage.getItem('qqq.mappa')||'{}').rend"); assert mode_val == "detail"
        step("the choice is remembered", remembered)
        r.b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
