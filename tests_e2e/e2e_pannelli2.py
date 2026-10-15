"""Panels of the Data view (9/10): Correzione outside the popover and with any file, RT interval, ruler anchored to labels and cancelled by zoom, isotope simulation off with the zoom, no Mirino/Ridge."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_pannelli2.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
r = Run(port=8957, wd="/tmp/wd_pan2")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 1000})
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")])
        pg.click("text=Carica dati"); ready(pg)
        def corr_out():
            assert pg.is_visible(".pnl.chrom [data-o=corr]"), "Correzione is in the header, not hidden in the popover"
            assert pg.evaluate(f"!{CH}.el.querySelector('.cpop').contains({CH}.el.querySelector('.corrb'))")
            pg.click(".pnl.chrom [data-o=corr]"); pg.wait_for_timeout(200)
            t = pg.inner_text(".pnl.chrom .corrw >> nth=0 >> .corrm"); print(t)
            assert t.count("Sottrai file:") == 1 and pg.locator(".pnl.chrom .corrm option").count() == 3, t      # one entry, any of the 3 files in its chooser, not only the blank
            pg.keyboard.press("Escape")
        step("Correzione: in the header, one entry and a chooser with the 3 files", corr_out)
        def corr_icons():
            b = pg.evaluate("[...document.querySelectorAll('.pnl.chrom .corrb')].map(b=>({svg:b.querySelector('svg').innerHTML,aria:b.getAttribute('aria-label'),on:b.classList.contains('on')}))")
            assert len(b) == 2 and b[0]["svg"] != b[1]["svg"], "«Subtract a file» and «Subtract the background» have different icons"
            assert b[0]["aria"] == "Sottrai un file" and b[1]["aria"] == "Sottrai il fondo", b
            assert "<circle" not in b[0]["svg"] and "<circle" not in b[1]["svg"], "neither idle icon is the «none» ⊘"
            pg.click(".pnl.chrom [data-o=corr]"); pg.locator(".pnl.chrom .corri", has_text="Sottrai file").first.click()
            attendi(pg, "document.querySelector('.pnl.chrom [data-o=corr]').classList.contains('on')")      # active state is marked
            pg.click(".pnl.chrom [data-o=corr]"); pg.locator(".pnl.chrom .corri", has_text="Nessuna").first.click()
            attendi(pg, "!document.querySelector('.pnl.chrom [data-o=corr]').classList.contains('on')")
        step("Correzione: the two subtract buttons have different icons and explicit names", corr_icons)
        def corr_info():
            B, M = ".pnl.chrom [data-o=cinfo]", ".pnl.chrom .corrbar"
            assert pg.is_visible(B) and not pg.is_visible(M) and pg.evaluate(f"document.querySelector('{M}').offsetHeight") == 0, "one icon, no bar taking height"
            assert pg.is_hidden(B + " .cidot"), "no dot without a correction"
            pg.click(".pnl.chrom [data-o=corr]"); pg.locator(".pnl.chrom .corri", has_text="Sottrai file").first.click()
            attendi(pg, f"!document.querySelector('{B} .cidot').hidden")
            assert pg.is_visible(B + " .cidot"), "the dot says that a correction is active"
            pg.click(B); assert pg.is_visible(M) and pg.get_attribute(B, "aria-expanded") == "true" and pg.inner_text(M).strip() not in ("", "Nessuna correzione attiva"), pg.inner_text(M)
            pg.keyboard.press("Escape"); assert pg.is_hidden(M), "Esc closes the summary"
            pg.click(B); assert pg.is_visible(M); pg.mouse.click(5, 5); assert pg.is_hidden(M), "a click outside closes the summary"
            pg.click(".pnl.chrom [data-o=corr]"); pg.locator(".pnl.chrom .corri", has_text="Nessuna").first.click()
            attendi(pg, f"document.querySelector('{B} .cidot').hidden")
        step("Correzioni: one icon opens the summary popover (Esc / click outside close it), the dot follows the state", corr_info)
        def subtract():
            pg.click(".pnl.chrom [data-o=corr]"); pg.wait_for_timeout(200)
            pg.locator(".pnl.chrom .corri", has_text="Sottrai file").first.click(); pg.wait_for_timeout(1500)
            assert pg.evaluate(f"{CH}.bk") not in ("", None), "a file was chosen"
            pg.click(".pnl.chrom [data-o=corr]"); pg.wait_for_timeout(200); pg.locator(".pnl.chrom .corri", has_text="Nessuna").first.click(); pg.wait_for_timeout(800)
        step("a non-blank file is subtracted from the chromatogram", subtract)
        def rt():
            pg.click(".pnl.chrom [data-a=cpar]"); pg.wait_for_timeout(200)
            pg.fill(".pnl.chrom [data-o=rt0]", "13"); pg.press(".pnl.chrom [data-o=rt0]", "Tab"); pg.fill(".pnl.chrom [data-o=rt1]", "16"); pg.press(".pnl.chrom [data-o=rt1]", "Tab"); pg.wait_for_timeout(800)
            z = pg.evaluate(f"{CH}.zoom"); assert z and abs(z[0] - 13) < 1e-6 and abs(z[1] - 16) < 1e-6, z
            pg.fill(".pnl.chrom [data-o=rt0]", ""); pg.fill(".pnl.chrom [data-o=rt1]", ""); pg.press(".pnl.chrom [data-o=rt1]", "Tab"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{CH}.zoom") is None
            pg.keyboard.press("Escape"); pg.mouse.click(5, 5)
        step("RT from ... to ... sets the visible interval; empty = whole run", rt)
        def popover_out():
            pg.evaluate(f"(()=>{{const p={CH};p._parOpen=true;ctl(p)}})()"); pg.wait_for_timeout(300)
            pg.evaluate("document.querySelector('.pnl.chrom .cpop').style.minHeight='700px'"); pg.wait_for_timeout(200)
            r = pg.evaluate("(()=>{const pn=document.querySelector('.pnl.chrom'),c=document.querySelector('.pnl.chrom .cpop');return [getComputedStyle(pn).overflow,c.getBoundingClientRect().bottom,pn.getBoundingClientRect().bottom]})()"); print(r)
            assert r[0] == "visible" and r[1] > r[2], r
            pg.evaluate("document.querySelector('.pnl.chrom .cpop').style.minHeight=''"); pg.evaluate(f"(()=>{{const p={CH};p._parOpen=false;ctl(p)}})()")
        step("a tall popover goes beyond the panel (not cut)", popover_out)
        def ruler():
            pg.evaluate(f"(()=>{{const p={SPEC};p.rul=true;measClick(p,152.2);measClick(p,364.4)}})()"); pg.wait_for_timeout(300)
            assert pg.evaluate(f"{SPEC}.meas.list.length") == 1
            pg.evaluate(f"(()=>{{const p={SPEC};p.zoom=[100,300];draw(p)}})()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}.meas") is None, "the zoom cancels the measure"
            pg.evaluate(f"(()=>{{const p={SPEC};p.zoom=null;p.rul=false;draw(p)}})()")
        step("ruler: a new zoom cancels the measure", ruler)
        def iso():
            pg.evaluate(f"(()=>{{const p={SPEC};p.iso={{formula:'C8H10NO2',ad:'[M]+'}};p.zoom=[145,165];p._isoKey=JSON.stringify(p.zoom);draw(p)}})()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"!!{SPEC}.iso")
            pg.evaluate(f"(()=>{{const p={SPEC};p.zoom=[140,170];draw(p)}})()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}.iso") is None, "zoom changed: the simulation is off"
            assert pg.evaluate("QADD.some(a=>a.n==='[M]-')")
        step("isotope simulation switches off when the zoom changes; [M]- exists", iso)
        def map_views():
            pg.click("#np-map"); pg.wait_for_timeout(1500)
            v = pg.evaluate("[...document.querySelectorAll('[data-o=view]')].map(b=>b.dataset.v)"); assert v == ["3d"], v
        step("map: only 2D and 3D (no Mirino, no Ridge)", map_views)
        def grips():
            pg.evaluate("(()=>{BARRA.setTab('file',{noFocus:true})})()")
            pg.click("#np-spec"); pg.wait_for_timeout(1200)
            ps = pg.evaluate("E.panels.filter(q=>q.full).map(q=>q.id)"); assert len(ps) >= 2, ps
            for w in (1100, 1440, 1920):
                for fold in (False, True):
                    pg.set_viewport_size({"width": w, "height": 1000}); pg.evaluate("(()=>{BARRA.setTab('file',{noFocus:true})})()" if not fold else "BARRA.close(false)"); pg.wait_for_timeout(500)
                    ov = pg.evaluate("[document.documentElement.scrollWidth - document.documentElement.clientWidth, E.panels.filter(q=>!q.el.classList.contains('max')).map(q=>q.x+q.w-document.getElementById('dpanels').clientWidth).reduce((m,v)=>Math.max(m,v),-9)]")
                    assert ov[0] <= 1 and ov[1] <= 2, (w, fold, ov)               # no horizontal scroll, no panel wider than the column
            pg.set_viewport_size({"width": 1280, "height": 1000}); pg.evaluate("(()=>{BARRA.setTab('file',{noFocus:true})})()"); pg.wait_for_timeout(400)
            assert pg.evaluate("getComputedStyle(E.panels[0].el).resize") == "none"
            assert pg.locator(".pnl .pgrip.t").count() >= 2 and pg.locator(".pnl .pgrip.b").count() >= 2
        step("panels: grips on the top and bottom edge, never wider than the column at 1100, 1440 and 1920 px (sidebar open and closed)", grips)
        def drag_top():
            pg.evaluate("window.scrollTo(0,0)")
            ids = pg.evaluate("E.panels.filter(q=>q.full&&q.el.offsetParent).sort((a,b)=>a.y-b.y).map(q=>q.id)")
            i = ids[1]; h = lambda: pg.evaluate("id=>E.panels.find(q=>q.id===id).h", i); hp = lambda: pg.evaluate("id=>E.panels.find(q=>q.id===id).h", ids[0])
            h0, p0 = h(), hp()
            pg.evaluate("id=>E.panels.find(q=>q.id===id).el.scrollIntoView({block:'center'})", i); pg.wait_for_timeout(300)
            bb = pg.evaluate("id=>{const r=E.panels.find(q=>q.id===id).el.querySelector('.pgrip.t').getBoundingClientRect();return [r.left+r.width/2,r.top+r.height/2]}", i)
            pg.mouse.move(bb[0], bb[1]); pg.mouse.down(); pg.mouse.move(bb[0], bb[1] - 60, steps=5); pg.mouse.up(); pg.wait_for_timeout(500)
            assert h() == h0 + 60 and hp() == p0 - 60, (h0, h(), p0, hp())     # splitter: the panel above gives what this one gains
            pg.evaluate("id=>E.panels.find(q=>q.id===id).el.querySelector('.pgrip.t').dispatchEvent(new MouseEvent('dblclick',{bubbles:true}))", i); pg.wait_for_timeout(400)
            assert h() == 300, h()                                              # double click = default height
            pg.evaluate("id=>{const g=E.panels.find(q=>q.id===id).el.querySelector('.pgrip.b');g.focus()}", i); pg.keyboard.press("ArrowDown"); pg.wait_for_timeout(300)
            assert h() == 320, h()
        step("panels: the top grip moves the boundary with the panel above, double click = default height, arrows on the focused grip", drag_top)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
