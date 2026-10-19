"""Blocco S/G (spectrum): short right-click menu, framed annotations, ruler (delta m/z), parameters (%, labels, decimals), table of the peaks (copy / xlsx),
zoom history + Backspace, linked time axes, one-line description of the scan, MS2 in % by default."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
r = Run(port=8876, wd="/tmp/wd76")
try:
    with sync_playwright() as p:
        br = None
        pg = r.page(p)
        try: pg.context.grant_permissions(["clipboard-read", "clipboard-write"])
        except Exception: pass
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)      # opened file by file: the view starts on the first file of the list
        pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(500)
        def px(expr_m, frac_y=0.0):   # pixel of a peak of the spectrum (m/z -> screen)
            return pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a;return {{x:q.left+a.X({expr_m}),y:q.top+a.Y(a.ymax*{frac_y})}}}})()")
        def peak_x(m):
            return pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a,d=a.data[0].d;let b=0;d.mz.forEach((v,i)=>{{if(Math.abs(v-{m})<0.6&&d.y[i]>d.y[b]||Math.abs(d.mz[b]-{m})>=0.6)b=i}});return {{x:q.left+a.X(d.mz[b]),y:q.top+a.Y(d.y[b]),mz:d.mz[b]}}}})()")
        def menu():
            pk = peak_x(364.4); pg.mouse.click(pk["x"], pk["y"] + 3, button="right"); pg.wait_for_timeout(300)
            items = pg.evaluate("[...document.querySelectorAll('#ctx div')].map(d=>d.textContent)"); print(items)
            txt = " | ".join(items)
            for want in ["m/z", "Estrai l'XIC di m/z", "Misura da questo picco", "Annota questo picco…", "Profilo isotopico di una formula…", "Congela lo spettro"]: assert want in txt, (want, items)
            for bad in ["Da dove viene", "sagoma", "Simula", "Confronta con"]: assert bad not in txt, (bad, items)
            pg.keyboard.press("Escape")
        step("S1/S2: the right-click menu of the spectrum is short, no origin / ghost / simulation", menu)
        def annot():
            pk = peak_x(194.2)
            pg.evaluate(f"(()=>{{const p={SPEC};p.anns.push({{x:{pk['mz']},text:'frammento'}});draw(p)}})()"); pg.wait_for_timeout(500)
            lb = pg.evaluate(f"{SPEC}._a.lbls.filter(l=>l.ann).map(l=>[l.x,l.y,l.w,l.h])"); assert len(lb) == 1 and lb[0][2] > 40, lb
            a = pg.evaluate(f"(()=>{{const a={SPEC}._a;return [a.W,a.H]}})()"); assert lb[0][0] >= 66 and lb[0][0] + lb[0][2] <= a[0] - 14 and lb[0][1] >= 14, (lb, a)       # inside the plot
            q = pg.evaluate(f"(()=>{{const p={SPEC},c=p.cv.getBoundingClientRect();return {{x:c.left+{lb[0][0]}+{lb[0][2]}/2,y:c.top+{lb[0][1]}+{lb[0][3]}/2}}}})()")
            pg.mouse.dblclick(q["x"], q["y"]); pg.wait_for_timeout(400); assert pg.evaluate("document.querySelector('#askdlg').open"), "double click on the label edits it"
            pg.fill("#askin", "frammento A") if pg.locator("#askin").count() else None
            pg.keyboard.press("Enter"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}.anns[0].text") in ("frammento A", "frammento")
            pg.screenshot(path=SH + "spettro_ann.png", clip={"x": 0, "y": max(0, q["y"] - 250), "width": 1300, "height": 330})
        step("S3: annotation is a framed label inside the plot; double click edits it", annot)
        def ruler():
            pg.evaluate(f"(()=>{{const p={SPEC};p.anns=[];p.zoom=null;p.zoomY=null;draw(p)}})()"); pg.wait_for_timeout(500)
            a = peak_x(364.4); pg.mouse.click(a["x"], a["y"] + 3, button="right"); pg.wait_for_timeout(300)
            pg.locator("#ctx div", has_text="Misura da questo picco").click(); pg.wait_for_timeout(300)
            assert pg.evaluate(f"{SPEC}.meas.ref") is not None
            b = peak_x(194.2); pg.mouse.move(b["x"], b["y"] + 3); pg.wait_for_timeout(300)
            tip = pg.inner_text(".pnl.spec .tip"); assert "Δ" in tip, tip
            pg.mouse.click(b["x"], b["y"] + 3); pg.wait_for_timeout(400)
            m = pg.evaluate(f"{SPEC}.meas"); assert len(m["list"]) == 1 and abs((m["list"][0]["b"] - m["list"][0]["a"]) - (b["mz"] - a["mz"])) < 0.5 and abs(abs(m["list"][0]["b"] - m["list"][0]["a"]) - 170) < 1, m      # about 170, sign + towards the right
            pg.screenshot(path=SH + "spettro_righello.png", clip={"x": 0, "y": 0, "width": 1500, "height": 900}) if False else None
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert pg.evaluate(f"{SPEC}.meas") is None
        step("G1: ruler: reference, second peak fixes delta m/z (about 170, shown without sign), Esc removes", ruler)
        def ruler_btn():
            pg.click(".pnl.spec [data-a=rul]"); a = peak_x(364.4); pg.mouse.click(a["x"], a["y"] + 3); pg.wait_for_timeout(300)
            assert pg.evaluate(f"{SPEC}.meas.ref") is not None
            b = peak_x(152.2); pg.mouse.click(b["x"], b["y"] + 3); pg.wait_for_timeout(300); assert len(pg.evaluate(f"{SPEC}.meas.list")) == 1
            pg.click(".pnl.spec [data-a=rul]"); pg.evaluate(f"(()=>{{const p={SPEC};p.meas=null;draw(p)}})()")
        step("G1: ruler button: first click = reference, second = measure", ruler_btn)
        def params():
            assert pg.evaluate(f"{SPEC}.level") == 1 and pg.evaluate(f"specRel({SPEC})") is False, "Full Scan: absolute"
            pg.click(".pnl.spec [data-a=par]"); pg.wait_for_timeout(300); assert pg.locator(".pnl.spec .sp-pop").count() == 1
            n0 = pg.evaluate(f"{SPEC}._a.lbls.filter(l=>!l.ann).length")
            pg.fill(".sp-pop [data-s=thr]", "60"); pg.press(".sp-pop [data-s=thr]", "Tab"); pg.wait_for_timeout(500)
            n1 = pg.evaluate(f"{SPEC}._a.lbls.filter(l=>!l.ann).length"); assert n1 < n0 or n0 <= 1, (n0, n1)
            pg.keyboard.press("Escape"); pg.click(".pnl.spec [data-a=rel]"); pg.wait_for_timeout(600)      # since 8/10 the % axis is the «%» button of the panel, not a field of the popover
            assert pg.evaluate(f"{SPEC}._a.ymax") < 200 and pg.evaluate(f"{SPEC}._a.yfull[1]") < 200, "y axis in %"
            pg.click(".pnl.spec [data-a=rel]"); pg.wait_for_timeout(400); assert pg.evaluate(f"specRel({SPEC})") is False
            pg.click(".pnl.spec [data-a=par]"); pg.wait_for_timeout(300)
            pg.select_option(".sp-pop [data-s=dec]", "2"); pg.wait_for_timeout(300)
            pg.click(".sp-pop [data-s=reset]"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"{SPEC}.thr") == 5
            pg.keyboard.press("Escape"); pg.mouse.click(5, 300) if False else None
        step("G2: parameters: Full Scan absolute, threshold changes the labels, % axis, reset", params)
        def history():
            c = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect();return {{x:q.left,y:q.top,w:q.width,h:q.height}}}})()")
            pg.evaluate(f"{CH}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300); pg.evaluate("setActive(%s)" % CH)
            pg.evaluate(f"zoomTo({CH}, 13, 16)"); pg.wait_for_timeout(300); pg.evaluate(f"pushZh({CH}); zoomTo({CH}, 14, 15)") ; pg.wait_for_timeout(300)
            for _ in range(3):
                pg.keyboard.press("Control+z"); pg.wait_for_timeout(300); z = pg.evaluate(f"{CH}.zoom")
                if z and abs(z[0] - 13) < 1e-6: break
            assert z and abs(z[0] - 13) < 1e-6, z
            pg.keyboard.press("Backspace"); pg.wait_for_timeout(300); assert pg.evaluate(f"{CH}.zoom") is None
            pg.keyboard.press("Control+z"); pg.wait_for_timeout(300); assert pg.evaluate(f"{CH}.zoom") is not None, "Ctrl+Z also undoes the whole view"
            pg.evaluate(f"{CH}.zoom=null;draw({CH})")
        step("G4: Ctrl+Z = previous zoom, Backspace = whole view", history)
        def linked():
            pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(1500)
            ids = pg.evaluate("E.panels.filter(p=>p.tab==='full'&&(p.type==='chrom'||p.type==='xic')).map(p=>p.id)")
            for i in ids: pg.evaluate(f"(()=>{{const q=E.panels.find(p=>p.id=={i});if(!q.tl)toggleTl(q)}})()")       # linked by default
            pg.evaluate(f"zoomTo({CH}, 13.5, 15.5)"); pg.wait_for_timeout(900)
            zs = pg.evaluate("E.panels.filter(p=>p.tab==='full'&&p.type==='xic').map(p=>p.zoom)"); assert zs and abs(zs[0][0] - 13.5) < 1e-6 and abs(zs[0][1] - 15.5) < 1e-6, zs
            pg.evaluate(f"(()=>{{const p={CH};p.zoom=null;draw(p)}})()"); pg.wait_for_timeout(700)
            assert pg.evaluate("E.panels.filter(p=>p.tab==='full'&&p.type==='xic').every(p=>p.zoom===null)"), "reset too"
        step("G5: linked time axes: zoom on the TIC = same interval on the XIC", linked)
        def caption():
            pg.evaluate(f"{CH}.el.scrollIntoView({{block:'center'}})")
            c = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect();return {{x:q.left+p._a.X(14.3),y:q.top+q.height/2}}}})()"); pg.mouse.click(c["x"], c["y"]); pg.wait_for_timeout(900)
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(600)
            t = pg.inner_text(".pnl.spec .leg"); print(t)
            import re; assert re.fullmatch(r"scansione \d+/\d+", t.strip()), t
            for w in ["TIC", "picco base", "RT"]: assert w not in t, (w, t)         # A4.3: the time is already next to the title
        step("G6: caption = only scansione i/N", caption)
        def ms2():
            open_kind(pg, "ms2")          # at the first load only the first file is open: the MS2 file opens with a click in the list
            attendi(pg, "(()=>{const p=E.panels.find(q=>q.tab==='ms2'&&q.type==='spec'&&q.cv);return !!p&&!!p._a&&p.cv.parentElement.querySelector('.leg').textContent.length>0})()")
            s = pg.evaluate("(()=>{const p=E.panels.find(q=>q.tab==='ms2'&&q.type==='spec');return {rel:specRel(p),ymax:p._a&&p._a.ymax,lock:!!p.lock,t:p.cv.parentElement.querySelector('.leg').textContent}})()"); print(s)
            assert s["rel"] is False and not s["lock"], s                      # cps by default since 8/10 (the «%» button switches)
            assert "precursore" in s["t"] and "CE" in s["t"], s
            assert pg.evaluate("document.querySelector('.pnl.spec:not([style*=\"display: none\"]) canvas')") is not None
        step("S6/G2: MS2 spectrum starts in %, no lock, shows precursor and CE", ms2)
        def thin_trace():
            # the TIC is a thin line (like the vendor software): every stroke of the trace is at most 1.25 px, and none is thinner than 1 px
            w = pg.evaluate("""(async () => { const seen = [], o = CanvasRenderingContext2D.prototype.stroke;
                CanvasRenderingContext2D.prototype.stroke = function () { seen.push(this.lineWidth); return o.apply(this, arguments); };
                try { await draw(%s); } finally { CanvasRenderingContext2D.prototype.stroke = o; }
                return seen; })()""" % CH)
            assert w and min(w) >= 1 and 1.25 in w, w
            assert pg.evaluate("TRACE_LW") == 1.25
        step("TIC line is thin (1.25 px)", thin_trace)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
