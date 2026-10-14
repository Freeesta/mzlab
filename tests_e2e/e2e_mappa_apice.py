"""Marked points land on the apex of the peak: a click a few pixels from the most intense point of the example file puts the marker on that point (RT and m/z
of the true data), with the option «snap» off the marker stays where it was clicked, and the 3D viewer draws it on the top cell of the peak.
Points of one group of RT share a shape in low resolution too."""
import base64, struct, sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_apice.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8898, wd="/tmp/wd_mappa_apice")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); attendi(pg, f"{M}&&{M}._a")
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        raw = pg.evaluate(f"""(async () => {{ const a = {M}._a; return await fetch(`api/mappunti3d?k=${{a.f.k}}&level=1&rt0=${{a.full[0]}}&rt1=${{a.full[1]}}&mz0=${{a.y0}}&mz1=${{a.y1}}&max=5`).then(r => r.json()); }})()""")
        top = {"rt": struct.unpack("<f", base64.b64decode(raw["rt"])[:4])[0], "mz": struct.unpack("<d", base64.b64decode(raw["mz"])[:8])[0]}     # the most intense true point of the file (the points come most intense first)
        pg.evaluate(f"(() => {{ const p = {M}, a = p._a; p.zoom = [{top['rt']} - 0.5, {top['rt']} + 0.5]; p.zoomY = [{top['mz']} - 3, {top['mz']} + 3]; draw(p); }})()")
        attendi(pg, f"{M}._a && Math.abs({M}._a.x0 - ({top['rt']} - 0.5)) < 1e-6")
        def px():
            c = pg.evaluate(f"(() => {{ const r = {M}.cv.getBoundingClientRect(), a = {M}._a; return [r.left + a.X({top['rt']}), r.top + a.Y({top['mz']})]; }})()")
            return c
        def click_near(dx, dy):
            c = px(); pg.mouse.click(c[0] + dx, c[1] + dy)
        def apex():
            pg.click(".pnl.map [data-mz=mark]"); click_near(3, 2)
            attendi(pg, f"MAPPA.pts({M}).length === 1")
            q = pg.evaluate(f"MAPPA.pts({M})[0]")
            assert abs(q["rt"] - top["rt"]) < 1e-5 and abs(q["mz"] - top["mz"]) < 1e-9, (q, top)
        step("a click 3 px from the most intense point puts the marker on its RT and m/z", apex)
        def free():
            pg.click(".pnl.map [data-mz=snap]"); assert pg.get_attribute(".pnl.map [data-mz=snap]", "aria-pressed") == "false"
            click_near(30, 20); attendi(pg, f"MAPPA.pts({M}).length === 2")
            q = pg.evaluate(f"MAPPA.pts({M})[1]"); assert abs(q["rt"] - top["rt"]) > 1e-4, (q, top)       # snap off: free marker, where it was clicked
            pg.click(".pnl.map [data-mz=snap]"); assert pg.get_attribute(".pnl.map [data-mz=snap]", "aria-pressed") == "true"
        step("with «snap» off the marker stays where it was clicked", free)
        def d3():
            pg.locator(".pnl.map [data-o=view][data-v='3d']").click()
            pg.wait_for_function("document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g && window.__m3d.V.Z", timeout=60000)
            m = pg.evaluate(f"""(() => {{ const w = window.__m3d, g = w.V.g, q = MAPPA.pts({M})[0], c = w.markCell(q); if (!c) return null; let mx = 0;
              for (let i = Math.max(0, c.i - 2); i <= Math.min(g.nrt - 1, c.i + 2); i++) for (let j = Math.max(0, c.j - 2); j <= Math.min(g.nmz - 1, c.j + 2); j++) mx = Math.max(mx, w.V.Z[i * g.nmz + j]); return [c.z, mx]; }})()""")
            assert m and m[0] > 0 and m[0] == m[1], m                                           # the marker stands on the top cell of the peak
        step("3D: the marker stands on the top cell of the peak", d3)
        def groups():
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('dialog.m3d[open]')")
            pg.wait_for_function(f"({M}._mz.rows || []).length >= 1", timeout=15000)
            pg.evaluate(f"MAPPA.addPointAt({M}, {top['rt']} + 0.01, {top['mz']}, 1, 0.2)"); attendi(pg, f"({M}._mz.rows || []).length >= 3")
            g = pg.evaluate(f"{M}._mz.rows.map(r => r.g)"); assert g[0] == g[2] and g[1] != g[0], g          # 1 and 3 are within the window of RT, 2 is 30 px away
            sh = pg.evaluate(f"[0, 1, 2].map(i => MAPPA.grpShape({M}, i))"); assert sh[0] == sh[2], sh
            assert pg.locator(".pnl.map .mztab tr.mzgr").count() == 1
        step("low resolution too: the points of one group of RT share a shape and a group row", groups)
    r.close()
finally:
    r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
