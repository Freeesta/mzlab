"""One XIC window (da/a with automatic "a", "oppure", neutral formula), right-click menu with "Ripristina zoom", header button groups, xlsx exports."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e18.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
def pt(pg, idx, x, fy=0.5):
    return pg.evaluate("""([i,x,fy])=>{const p=E.panels[i],r=p.cv.getBoundingClientRect(),a=p._a;return {px:r.left+a.X(x),py:r.top+(r.height-60)*fy+10}}""", [idx, x, fy])
r = Run(port=8818, wd="/tmp/wd18")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES])
        pg.click("text=Carica dati"); ready(pg)
        open_kind(pg, "full"); ready(pg)      # the tab opened at load can differ on a loaded machine: the chromatogram panels live in Full Scan
        dlg = lambda: pg.evaluate("document.querySelector('#xicdlg').open")
        def window_():
            pg.locator("#dpanels .pnl.chrom [data-a=xic]").first.click(); assert dlg()
            assert pg.locator("#xic-rows .xrw").count() == 2 and pg.locator("#xic-add").is_enabled(), "two rows to start with"
            t = pg.inner_text("#xicdlg"); assert "oppure" in t and "neutra" in t  and "cifra decimale" not in t and "m/z o formula" not in t, t
            pg.fill("#xic-mz", "194.04"); assert "193.8 - 194.8" in pg.inner_text("#xic-sum") and pg.input_value("#xic-mz") == "194.04", pg.inner_text("#xic-sum")
            pg.fill("#xic-mz", "C14H13F4N3O2S"); hold(pg, 1300)
            assert "363.8 - 364.8" in pg.inner_text("#xic-sum") and pg.input_value("#xic-mz") == "C14H13F4N3O2S", pg.inner_text("#xic-sum")
            pg.select_option("#xic-ad", "[M+Na]+"); pg.wait_for_timeout(600)
            assert "385.8 - 386.8" in pg.inner_text("#xic-sum"), pg.inner_text("#xic-sum")
            pg.fill("#xic-mz", "364"); pg.click("#xic-go"); ready(pg)
            assert not dlg() and pg.evaluate("E.panels.some(p=>p.type==='xic' && p.traces[0].w===0.5 && p._a && p._a.sr.length>0)")
            assert "cifra" not in pg.evaluate("document.querySelector('.pnl.chrom [data-o=mz0]').closest('label').title")
        step("XIC window: one m/z value, oppure, neutral formula, unit window around the nominal mass", window_)
        def enter_():
            n = pg.evaluate("E.panels.filter(p=>p.type==='xic').length")
            pg.locator("#dpanels .pnl.chrom [data-a=xic]").first.click(); assert dlg()
            pg.fill("#xic-mz", ""); pg.press("#xic-mz", "Enter"); assert dlg() and pg.inner_text("#xic-err").strip(), "empty field: no extraction, a message"
            pg.fill("#xic-mz", "364"); pg.press("#xic-mz", "Shift+Enter"); assert dlg(), "Shift+Enter only moves to the next field"
            pg.press("#xic-mz", "Enter"); ready(pg)
            assert not dlg() and pg.evaluate("E.panels.filter(p=>p.type==='xic').length") == n + 1, "Enter extracts at once"
        step("XIC window: Enter in an Ion field extracts, Shift+Enter moves on, empty field gives a message", enter_)
        def chrom_menu():
            ci = pg.evaluate("E.panels.findIndex(p=>p.type==='chrom')"); c = pt(pg, ci, 12.0)
            pg.mouse.click(c["px"], c["py"], button="right"); pg.wait_for_function("document.querySelectorAll('#ctx div').length>0", timeout=5000)
            items = pg.evaluate("[...document.querySelectorAll('#ctx div')].map(d=>[d.textContent,d.className])"); print([i[0] for i in items])
            assert not any(i[0] == "Ripristina zoom" for i in items) and any(i[0] == "Estrai uno ione (XIC)…" for i in items), items          # no zoom yet: no such entry
            pg.locator("#ctx div", has_text="Estrai uno ione").first.click(); pg.wait_for_timeout(300)
            assert dlg() and not pg.evaluate("document.querySelector('#askdlg').open"); pg.click("#xic-no")      # the same window, not a separate question
            pg.locator("#dpanels .pnl.chrom [data-a=izoom]").first.click(); box = pg.locator("#dpanels .pnl.chrom canvas").first.bounding_box()
            pg.mouse.move(box["x"] + pg.evaluate(f"E.panels[{ci}]._a.X(13)"), box["y"] + 100); pg.mouse.down(); pg.mouse.move(box["x"] + pg.evaluate(f"E.panels[{ci}]._a.X(16)"), box["y"] + 100, steps=5); pg.mouse.up(); pg.wait_for_timeout(500)
            assert pg.evaluate(f"E.panels[{ci}].zoom") and pg.locator("#dpanels .pnl.chrom [data-a=fit]").first.is_enabled()
            c = pt(pg, ci, 14.3); pg.mouse.click(c["px"], c["py"], button="right"); pg.wait_for_timeout(300)
            items = pg.evaluate("[...document.querySelectorAll('#ctx div')].map(d=>[d.textContent,d.className])"); assert items[0] == ["Ripristina zoom", ""], items          # with a zoom it is the first entry
            pg.locator("#ctx div", has_text="Ripristina zoom").first.click(); pg.wait_for_timeout(500)
            assert pg.evaluate(f"E.panels[{ci}].zoom") is None and pg.locator("#dpanels .pnl.chrom [data-a=fit]").first.is_disabled()
            pg.locator("#dpanels .pnl.chrom [data-a=izoom]").first.click()
        step("right click on the chromatogram: Estrai uno ione (same window), Ripristina zoom", chrom_menu)
        def guard():
            c = pg.locator("#dpanels .pnl.chrom [data-a=iauto]").first; c.click()
            ci = pg.evaluate("E.panels.findIndex(p=>p.type==='chrom')"); pt_ = pt(pg, ci, 14.3); pg.mouse.click(pt_["px"], pt_["py"]); pg.wait_for_timeout(400)
            pg.screenshot(path=SH + "181_guard.png"); pg.click("#askok"); pg.wait_for_timeout(400)
            assert dlg(); pg.click("#xic-no"); c.click()
        step("integrating from the TIC offers the same XIC window", guard)
        def spec_menu():
            si = pg.evaluate("E.panels.findIndex(p=>p.type==='spec')"); a = pg.evaluate(f"""()=>{{const p=E.panels[{si}],r=p.cv.getBoundingClientRect(),d=p._a.data[0].d;let j=0;d.y.forEach((v,i)=>{{if(v>d.y[j])j=i}});return {{px:r.left+p._a.X(d.mz[j]),py:r.top+p._a.Y(d.y[j])+6}}}}""")
            n0 = pg.evaluate("E.panels.filter(p=>p.type==='xic').length")
            pg.mouse.click(a["px"], a["py"], button="right"); pg.wait_for_timeout(300)
            pg.locator("#ctx div", has_text="Estrai l'XIC").first.click(); pg.wait_for_timeout(600)
            assert not dlg(), "no dialog: the XIC appears at once"
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').length") == n0 + 1
            t = pg.evaluate("(()=>{const x=E.panels.filter(p=>p.type==='xic').pop().traces[0];return [x.mz,x.w,x.label]})()"); print(t)
            assert t[2].startswith("m/z ") and abs(t[1] - 0.5) < 1e-9, t          # default unit window [n-0.2, n+0.8]
        step("right click on a peak of the spectrum extracts the XIC at once, no window", spec_menu)
        def groups():
            for sel, expect in [(".pnl.chrom", ["izoom", "fit", "tlink", "iauto", "iman", "idel", "smooth", "log", "xic", "tab", "dl", "max"]), (".pnl.xic", ["izoom", "fit", "tlink", "iauto", "iman", "idel", "smooth", "log", "tab", "dl", "max"]), (".pnl.spec", ["fit", "rul", "par", "rel", "tab", "dl", "max"])]:
                got = pg.evaluate(f"[...document.querySelector('{sel}').querySelectorAll('.tbs [data-a]')].filter(b=>b.tagName==='BUTTON'&&!b.hidden).map(b=>b.dataset.a)"); assert got == expect, (sel, got)
            # hidden until needed
            assert pg.evaluate("[...document.querySelectorAll('.pnl.xic [data-a=intf], .pnl.xic [data-a=iclr], .pnl.xic [data-a=itab]')].every(b=>b.hidden)")
            # one row for the buttons, and the groups are told apart by a vertical bar
            for w in (1500, 1100, 700):
                pg.set_viewport_size({"width": w, "height": 2200}); pg.wait_for_timeout(700)
                tops = pg.evaluate("[...document.querySelectorAll('.pnl')].filter(p=>p.offsetParent).map(p=>[p.className, Math.round(p.querySelector('.tbs').getBoundingClientRect().height)])")
                assert all(h <= 30 for _, h in tops), (w, tops)
            pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(800)      # (narrow: some groups are in the «⋯» menu)
            bars = pg.evaluate("[...document.querySelectorAll('.pnl.chrom .tbg')].map(g=>getComputedStyle(g).borderLeftWidth)"); print("bars:", bars); assert bars[1] != "0px" and bars[-1] != "0px"
            pg.wait_for_timeout(100); pg.screenshot(path=SH + "182_header.png", clip={"x": 250, "y": 150, "width": 1250, "height": 330})
        step("button groups in order, zoom next to reset, one row at 1500/1100/700 px", groups)
        def xlsx():
            xp = pg.locator(".pnl.xic").first
            with pg.expect_download() as d: (xp.locator("[data-a=dl]").click(), pg.locator("#ctx div", has_text="Excel").first.click())
            f = d.value.path(); rows = xlsx_rows(f); assert xlsx_sheets(f) and rows[0][0][0] == "s" and "RT (min)" in rows[0][0][1]
            nums = [c[1] for r_ in rows[1:] for c in r_ if c and c[0] == "n"]; assert len(nums) > 400 and max(nums) > 1000, len(nums)
            xi = pg.evaluate("E.panels.findIndex(p=>p.type==='xic')"); xp.locator("[data-a=iauto]").click()
            xbox = xp.locator("canvas").bounding_box(); xx = pg.evaluate(f"E.panels[{xi}]._a.X(14.33)")
            pg.mouse.click(xbox["x"] + xx, xbox["y"] + 120); pg.wait_for_timeout(600)
            xp.locator("[data-a=itab]").click(); pg.wait_for_timeout(500)
            with pg.expect_download() as d2: pg.click("#ig-xlsx")
            g = d2.value.path(); assert d2.value.suggested_filename == "integrazioni.xlsx"; rr = xlsx_rows(g)
            heads = [c[1] for c in rr[0]]; assert "Area (conteggi*s)" in heads and "RT inizio (min)" in heads, heads
            area = rr[1][heads.index("Area (conteggi*s)")]; assert area[0] == "n" and area[1] > 0, area
            wb = xlsx_openpyxl(g)
            if wb: assert wb.active.cell(2, heads.index("Area (conteggi*s)") + 1).value > 0
            pg.click("#bigx")
        step("xlsx of an XIC and of the integrations table", xlsx)
finally:
    r.close(); r.report()
print("\nSTEPS")
for s in steps: print("  ", s)
