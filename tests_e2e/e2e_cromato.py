"""Block H1/H2: optional S/N (noise stretch chosen by the student) and chromatographic figures (w1/2, N, tailing) in the integration table."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_cromato.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
def tick(pg, sel):      # a checkbox of the dialog that is drawn again after each change: a DOM click (Playwright's check() fights with the new element in WebKit and Firefox)
    pg.wait_for_selector(sel, timeout=30000); pg.evaluate("s=>{const e=document.querySelector(s); if(!e.checked) e.click()}", sel)
r = Run(port=8881, wd="/tmp/wd81")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60"), mz("B_FullMass-t10")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=4", timeout=60000)      # all 4 uploaded: with a fixed pause, on a loaded machine «Carica dati» loaded only the files staged so far
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_function(f"(()=>{{const p={XP};return !!(p && p._a && p._a.sr && p._a.sr.length)}})()", timeout=60000)
        pg.evaluate(f"(()=>{{const p={XP};p.sr=p._a.sr;addInt(p,p._a.sr[0],13.9,14.9)}})()"); pg.wait_for_timeout(800)
        def default_off():
            pg.evaluate("showInts()"); pg.wait_for_timeout(400)
            t = pg.inner_text("#bigbody"); assert "S/N" in t and "N (piatti)" not in t and pg.locator("#ig-par").is_checked() is False, t[:300]
        step("columns are optional and off by default", default_off)
        def params():
            tick(pg, "#ig-par"); pg.wait_for_timeout(400)
            rows = pg.evaluate("[...document.querySelectorAll('#bigbody table tr')].map(r=>[...r.children].map(c=>c.textContent))"); h, d = rows[0], rows[1]
            i = [j for j, x in enumerate(h) if x.startswith("w")][0]
            w, N, T = float(d[i]), float(d[i + 1]), float(d[i + 2]); print(w, N, T)
            assert 0.05 < w < 0.6 and N > 100 and 0.5 < T < 6, (w, N, T)
        step("w1/2, N and tailing appear with the switch", params)
        def sn():
            tick(pg, "#ig-sn"); pg.wait_for_timeout(400)
            assert pg.locator("button[data-nz]").count() == 1, "no noise stretch yet: the cell asks for it"
            pg.click("button[data-nz]"); pg.wait_for_timeout(300); assert not pg.evaluate("document.querySelector('#bigdlg').open")
            c = pg.evaluate(f"(()=>{{const p={XP},r=p.cv.getBoundingClientRect();p.el.scrollIntoView({{block:'center'}});const q=p.cv.getBoundingClientRect();return {{x0:q.left+p._a.X(2),x1:q.left+p._a.X(5),y:q.top+q.height/2}}}})()")
            pg.mouse.move(c["x0"], c["y"]); pg.mouse.down(); pg.mouse.move(c["x1"], c["y"], steps=6); pg.mouse.up(); pg.wait_for_timeout(800)
            assert pg.evaluate(f"{XP}.ints[0].noise") is not None and pg.evaluate("document.querySelector('#bigdlg').open")
            rows = pg.evaluate("[...document.querySelectorAll('#bigbody table tr')].map(r=>[...r.children].map(c=>c.textContent))"); h = rows[0]; v = rows[1][h.index("S/N")]
            assert float(v) > 1, rows[1]
        step("S/N: the student drags a noise stretch, the number appears", sn)
        def xl():
            with pg.expect_download() as d: pg.click("#ig-xlsx")
            hd = [c[1] for c in xlsx_rows(d.value.path())[0] if c]; assert any(x.startswith("S/N") for x in hd) and "N (piatti teorici)" in hd, hd
            pg.click("#bigx")
        step("Excel has the extra columns", xl)
        def cascade():
            pg.evaluate("E.files.forEach(f=>f.vis=true); syncBrowse(); redrawAll()")
            pg.evaluate("E.panels.filter(p=>p.type==='xic').forEach(p=>p.el.querySelector('.x').click())"); pg.wait_for_timeout(300)
            ch = pg.evaluate("E.panels.findIndex(p=>p.type==='chrom'&&p.tab==='full')")
            assert "a cascata" in pg.evaluate(f"E.panels[{ch}].el.querySelector('[data-o=mode]').innerText")
            pg.select_option(f".pnl.chrom [data-o=mode]", "cas")
            try: pg.wait_for_function(f"(()=>{{const a=E.panels[{ch}]._a;return !!(a && a.cas && a.sr && a.sr.length===4)}})()", timeout=60000)
            except Exception: raise AssertionError("state: " + str(pg.evaluate(f"(()=>{{const p=E.panels[{ch}],a=p._a;return {{mode:p.mode,cas:a&&a.cas,n:a&&a.sr&&a.sr.length,files:E.files.map(f=>[f.label,f.vis,f.kind]),tab:E.tab}}}})()")))
            a = pg.evaluate(f"(()=>{{const a=E.panels[{ch}]._a;return {{cas:a.cas,n:a.sr.length}}}})()"); assert a["cas"] and a["n"] == 4, a
            pg.evaluate(f"E.panels[{ch}].el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            box = pg.locator(".pnl.chrom canvas").first.bounding_box(); pg.mouse.move(box["x"] + 300, box["y"] + 150); pg.wait_for_timeout(2000)
            assert pg.locator(".pnl.chrom .tip").first.is_hidden(), "no reading of values in the waterfall view"
            pg.screenshot(path=SH + "cascata.png", clip={"x": box["x"], "y": box["y"] - 40, "width": box["width"], "height": box["height"] + 60})
            pg.select_option(".pnl.chrom [data-o=mode]", "ovl")
        step("waterfall view: 4 files in time order, no value readout", cascade)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
