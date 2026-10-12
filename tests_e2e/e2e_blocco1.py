"""Prompt 2, block 1: right click takes the SAME peak as the label, one live spectrum per chromatogram, XIC straight from the spectrum, overlay entries,
'Ripristina zoom' first, grey dashed annotations that disappear out of the zoom, RT with 2 decimals."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_blocco1.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
r = Run(port=8877, wd="/tmp/wd77")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(500)
        def ctx_items(x, y):
            pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(300)
            return pg.evaluate("[...document.querySelectorAll('#ctx div')].map(d=>d.textContent)")
        def label_box(i=0):
            return pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),l=p._a.lbls.filter(l=>l.m!=null)[{i}];return l&&{{x:q.left+l.x+l.w/2,y:q.top+l.y+l.h/2,m:l.m}}}})()")
        def pick_label():
            lb = label_box(0); assert lb, "no peak labels"
            its = ctx_items(lb["x"], lb["y"]); print(lb["m"], its[:3])
            assert its[0] == f"m/z {lb['m']:.1f}" and its[1] == "Copia valore" and its[2] == f"Estrai l'XIC di m/z {lb['m']:.1f}", (lb, its)
            pg.keyboard.press("Escape")
            # the bar of the same peak gives the same value
            bar = pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a;return {{x:q.left+a.X({lb['m']}),y:q.top+a.Y(a.ymax*0.02)}}}})()")
            its = ctx_items(bar["x"], bar["y"]); assert its[0] == f"m/z {lb['m']:.1f}", (lb, its); pg.keyboard.press("Escape")
        step("1.0: right click on the label / on the bar gives the label's peak", pick_label)
        def xic_direct():
            lb = label_box(0); n0 = pg.evaluate("E.panels.filter(p=>p.type==='xic').length")
            ctx_items(lb["x"], lb["y"]); pg.locator("#ctx div", has_text="Estrai l'XIC").first.click(); pg.wait_for_timeout(700)
            assert not pg.evaluate("document.querySelector('#xicdlg').open"), "no dialog"
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').length") == n0 + 1
            tr = pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().traces[0]"); n = round(lb["m"] - 0.25)
            assert abs(tr["mz"] - (n + 0.3)) < 0.06 and abs(tr["w"] - 0.5) < 1e-9, (tr, lb)
        step("1.3: the XIC from the spectrum appears at once", xic_direct)
        def overlay():
            pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            lb = label_box(1); its = ctx_items(lb["x"], lb["y"]); print(its)
            assert any(i.startswith("Sovrapponi all'XIC di m/z ") and "(pannello " in i for i in its) and not any("Aggiungi a" in i for i in its), its
            tip = pg.evaluate("[...document.querySelectorAll('#ctx div')].find(d=>d.textContent.startsWith('Sovrapponi'))?.title"); assert "stesso grafico" in tip, tip
            pg.locator("#ctx div", has_text="Sovrapponi all'XIC").first.click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().traces.length") == 2
        step("1.4: 'Sovrapponi all'XIC di m/z … (pannello n)' adds a trace", overlay)
        def live():
            c = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect();p.el.scrollIntoView({{block:'center'}});return q}})()")
            for fr in (0.4, 0.6):
                b = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect();return [q.left+p._a.X(p._a.full[0]+(p._a.full[1]-p._a.full[0])*{fr}),q.top+q.height*0.4]}})()")
                pg.mouse.dblclick(b[0], b[1]); pg.wait_for_timeout(1200)
            info = pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p.tab==='full').map(p=>[!!p.link,!!p.el.querySelector('.lk'),p.el.querySelector('.rtl').textContent])"); print(info)
            assert sum(1 for i in info if i[1]) == 1 and sum(1 for i in info if i[0]) == 1, info
            assert all(i[2].startswith("fermo a RT ") for i in info if not i[1]), info
            # a spectrum linked by hand to the same chromatogram (an old notebook): only the newest stays live
            pg.evaluate(f"(()=>{{const c={CH},s=E.panels.filter(p=>p.type==='spec'&&p.tab==='full');s.forEach(x=>x.link=c.id);s.forEach(ctl)}})()")
            info = pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p.tab==='full').map(p=>!!p.el.querySelector('.lk'))"); assert sum(info) == 1, info
        step("1.1: one live spectrum per chromatogram, the others 'fermo a RT'", live)
        def zoom_first():
            pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            its = ctx_items(*pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect();return [q.left+q.width/2,q.top+q.height*0.5]}})()")); assert "Ripristina zoom" not in its, its
            pg.keyboard.press("Escape")
            pg.evaluate(f"(()=>{{const p={SPEC},a=p._a;p.zoom=[a.x0+(a.x1-a.x0)*0.4,a.x0+(a.x1-a.x0)*0.6];draw(p)}})()"); pg.wait_for_timeout(500)
            its = ctx_items(*pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect();return [q.left+q.width/2,q.top+q.height*0.5]}})()")); assert its[0] == "Ripristina zoom", its
            pg.locator("#ctx div", has_text="Ripristina zoom").first.click(); pg.wait_for_timeout(400)
            assert pg.evaluate(f"{SPEC}.zoom") is None
        step("1.5: 'Ripristina zoom' is the first entry only with a zoom", zoom_first)
        def annot():
            pg.evaluate("""(()=>{const o=CanvasRenderingContext2D.prototype.stroke;window.__st=[];CanvasRenderingContext2D.prototype.stroke=function(){window.__st.push([String(this.strokeStyle),this.getLineDash().join()]);return o.apply(this,arguments)}})()""")
            lb = label_box(0); m = lb["m"]
            pg.evaluate(f"(()=>{{const p={SPEC};p.anns.push({{x:{m},text:'prova'}});window.__st=[];draw(p)}})()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}._a.lbls.filter(l=>l.ann).length") == 1
            col = pg.evaluate(f"E.files[{SPEC}.k].color").lower()
            dashed = pg.evaluate("window.__st.filter(s=>s[1]==='3,3').map(s=>s[0])"); print(dashed, col)
            assert dashed and all(d.lower() != col for d in dashed), (dashed, col)
            pg.evaluate(f"(()=>{{const p={SPEC};p.zoom=[{m}+30,{m}+60];draw(p)}})()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}._a.lbls.filter(l=>l.ann).length") == 0, "annotation out of the zoom must not be drawn"
            pg.evaluate(f"(()=>{{const p={SPEC};p.zoom=null;draw(p)}})()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}._a.lbls.filter(l=>l.ann).length") == 1
            pg.evaluate(f"{SPEC}.anns=[]"); 
        step("1.6: grey dashed line, hidden out of the zoom, back in view", annot)
        def rt2():
            c = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect();p.el.scrollIntoView({{block:'center'}});return {{x:q.left+p._a.X(14.3),y:q.top+q.height/2}}}})()")
            pg.mouse.click(c["x"], c["y"]); pg.wait_for_timeout(700); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500)
            t = pg.inner_text(".pnl.spec .rtl"); import re; assert re.search(r"RT \d+\.\d{2} min", t) and not re.search(r"RT \d+\.\d{3}", t), t
        step("1.7: RT with 2 decimals", rt2)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s_ in steps: print(" ", s_)
r.report()
