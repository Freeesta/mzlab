"""Part D: the «Librerie» window (gear), a synthetic MSP library, search from the MS2 spectrum, results table and mirror plot, copy / Excel."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import json
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_libreria.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8847, wd="/tmp/wd_lib")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_MS2-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('ms2',true)"); pg.wait_for_timeout(2500)
        SPEC = "E.panels.find(p => p.type === 'spec' && p.level === 2)"
        q = pg.evaluate(f"(() => {{ const p = {SPEC}, d = p._a.data[0].d; return {{mz: d.mz, y: d.y0 || d.y, prec: p.prec, pol: p._a.data[0].f.polarity}}; }})()")
        assert len(q["mz"]) >= 2 and q["prec"], q
        pk = sorted(zip(q["mz"], q["y"]), key=lambda t: -t[1])[:30]; pk.sort()
        def rec(name, prec, peaks, extra="Ion_mode: Positive\nPrecursor_type: [M+H]+\nCollision_energy: 30\nInstrument_type: QTRAP\n"):
            return f"Name: {name}\nPrecursorMZ: {prec}\n{extra}Num Peaks: {len(peaks)}\n" + "\n".join(f"{m:.4f}\t{i:.1f}" for m, i in peaks) + "\n"
        msp = "\n".join([rec("Match", q["prec"], pk), rec("Spostato", q["prec"], [(m + 40, i) for m, i in pk]), rec("Parziale", q["prec"], pk[: max(1, len(pk) // 3)]),
                         rec("Lontano", q["prec"] + 100, pk), rec("Negativo", q["prec"], pk, "Ion_mode: Negative\nPrecursor_type: [M-H]-\n"),
                         "Name: Senza precursore\nNum Peaks: 1\n100 100\n"])
        def load():
            pg.click("#np-set"); pg.wait_for_timeout(200); assert pg.locator("#uipset button", has_text="Librerie").count() == 1
            pg.click("#uipset button >> text=Librerie"); pg.wait_for_timeout(500)
            assert pg.evaluate("document.querySelector('#libdlg').open") and "Nessuna libreria" in pg.inner_text("#lib-list")
            pg.set_input_files("#lib-file", {"name": "prova.msp", "mimeType": "text/plain", "buffer": msp.encode()}); pg.wait_for_timeout(2500)
            t = pg.inner_text("#lib-list"); assert "prova" in t and "5" in t, t
            assert pg.locator("#lib-list tr").count() == 2 and pg.inner_text("#lib-list td.num:nth-child(5)").strip() == "1", pg.inner_text("#lib-list")     # one discarded (no precursor)
            pg.click("#lib-x"); pg.wait_for_timeout(200)
        step("gear → Librerie: a synthetic MSP is read, one spectrum discarded", load)
        def search():
            c = pg.evaluate(f"(() => {{ const p = {SPEC}, q = p.cv.getBoundingClientRect(); p.el.scrollIntoView({{block:'center'}}); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
            c = pg.evaluate(f"(() => {{ const q = {SPEC}.cv.getBoundingClientRect(); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
            pg.mouse.click(c["x"], c["y"], button="right"); pg.wait_for_timeout(400)
            assert "Cerca nelle librerie" in pg.inner_text("#ctx"), pg.inner_text("#ctx")
            pg.click("#ctx >> text=Cerca nelle librerie"); pg.wait_for_timeout(2500)
            assert pg.evaluate("document.querySelector('#libres').open")
            names = pg.locator("#libres tr.row td:nth-child(2)").all_inner_texts(); print("   ", names)
            assert names[0] == "Match" and "Negativo" not in names and "Lontano" not in names and {"Spostato", "Parziale"} <= set(names), names
            ent = float(pg.inner_text("#libres tr.row >> nth=0 >> td:nth-child(3)")); cos = float(pg.inner_text("#libres tr.row >> nth=0 >> td:nth-child(4)"))
            assert ent > 0.999 and cos > 0.999, (ent, cos)
            e2 = [float(x) for x in pg.locator("#libres tr.row td:nth-child(3)").all_inner_texts()]; assert e2 == sorted(e2, reverse=True), e2     # ordered by entropy
        step("right click on the MS2 → results: Match first, other polarity and far precursor left out", search)
        def mirror():
            assert pg.is_visible("#lr-cv")
            def accent_pixels():
                return pg.evaluate("""(() => { const c = document.querySelector('#lr-cv'), g = c.getContext('2d'), d = g.getImageData(0, 0, c.width, c.height).data; const a = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim(); const t = document.createElement('i'); t.style.color = a; document.body.appendChild(t); const v = getComputedStyle(t).color.match(/\\d+/g).map(Number); t.remove(); let n = 0; for (let i = 0; i < d.length; i += 4) if (Math.abs(d[i] - v[0]) < 12 && Math.abs(d[i+1] - v[1]) < 12 && Math.abs(d[i+2] - v[2]) < 12) n++; return n; })()""")
            a0 = accent_pixels(); assert a0 > 50, a0                                  # the common peaks of Match are drawn in the accent colour
            pg.locator("#libres tr.row", has_text="Spostato").click(); pg.wait_for_timeout(300)
            assert pg.evaluate("document.querySelector('#libres tr.row.sel td:nth-child(2)').textContent") == "Spostato"
            a1 = accent_pixels(); assert a1 < a0, (a0, a1)                            # the shifted spectrum shares fewer peaks
        step("mirror plot: common peaks in the accent colour, a click on a row redraws it", mirror)
        def export():
            with pg.expect_download() as d: pg.click("#lr-xlsx")
            rows = xlsx_rows(d.value.path()); assert any("Entropia" in str(c) for c in rows[0]), rows[0]
            pg.click("#lr-copy"); pg.wait_for_timeout(400); assert "copiate" in pg.inner_text("#lr-msg") or "non permette" in pg.inner_text("#lr-msg")
        step("copy the table / Excel", export)
        def none():
            pg.click("#lr-x"); pg.click("#np-set"); pg.click("#uipset button >> text=Librerie"); pg.wait_for_timeout(500)
            pg.click("#lib-list [data-rm]"); pg.wait_for_timeout(800); assert "Nessuna libreria" in pg.inner_text("#lib-list"); pg.click("#lib-x")
            c = pg.evaluate(f"(() => {{ const q = {SPEC}.cv.getBoundingClientRect(); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
            pg.mouse.click(c["x"], c["y"], button="right"); pg.wait_for_timeout(300); pg.click("#ctx >> text=Cerca nelle librerie"); pg.wait_for_timeout(1500)
            assert "Carica una libreria" in pg.inner_text("#lr-body"), pg.inner_text("#libres")
        step("no library: «Carica una libreria (ingranaggio → Librerie)»", none)
        def unsupported():
            pg.click("#lr-x"); pg.click("#np-set"); pg.click("#uipset button >> text=Librerie"); pg.wait_for_timeout(400)
            pg.set_input_files("#lib-file", {"name": "x.lib", "mimeType": "application/octet-stream", "buffer": b"\x00\x01"}); pg.wait_for_timeout(800)
            assert "MSP" in pg.inner_text("#lib-st"), pg.inner_text("#lib-st")
            pg.click("#lib-x"); pg.wait_for_timeout(200)
        step("a NIST .lib is refused with the hint to export MSP", unsupported)
        def persists():
            big = "\n".join(rec(f"Fill{i}", 150 + i * 0.01, [(50 + j * 3.7, 100 + j) for j in range(60)]) for i in range(5000)) + "\n" + rec("Target", 777.1234, pk)
            assert len(big) > 3_000_000, len(big)
            pg.click("#np-set"); pg.click("#uipset button >> text=Librerie"); pg.wait_for_timeout(400)
            pg.set_input_files("#lib-file", {"name": "grande.msp", "mimeType": "text/plain", "buffer": big.encode()}); pg.wait_for_timeout(6000)
            t = pg.inner_text("#lib-list"); assert "grande" in t and "5001" in t.replace(".", "").replace("\u202f", ""), t
            assert "MB" in t, t
            pg.reload(); pg.wait_for_timeout(2500)
            pg.click("#np-set"); pg.click("#uipset button >> text=Librerie"); pg.wait_for_timeout(1000)
            assert "grande" in pg.inner_text("#lib-list"), pg.inner_text("#lib-list")                  # still there after the reload: no new import
            res = pg.evaluate("LIB.call('search', {peaks: %s, prec: 777.1234, pol: 1, tol: {v: 10, unit: 'ppm'}, frag: 0.02})" % json.dumps([[m, i] for m, i in pk]))
            assert res["results"] and res["results"][0]["name"] == "Target", res
            pg.click("#lib-list [data-rm]"); pg.wait_for_timeout(1000); assert "Nessuna libreria" in pg.inner_text("#lib-list")
            pg.reload(); pg.wait_for_timeout(2000); pg.click("#np-set"); pg.click("#uipset button >> text=Librerie"); pg.wait_for_timeout(800)
            assert "Nessuna libreria" in pg.inner_text("#lib-list")                                    # removed for good
        step("a library of some MB survives a reload, is searched without re-importing and «Togli» removes it", persists)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
