"""e2e_menu_coerenti: the same graph offers the same actions from the right click, from the keyboard (Shift+F10 / Menu key) and from its bar.
Map: the ids of the menu (data-azione) are the actions of the registry actions/1 for the view, the buttons of the bar are registered actions.
Spectrum, chromatogram and MRM: Shift+F10 on the focused graph opens the menu of the right click at the same point, with the focus inside; Esc gives it back.
Every kind (spectrum, chromatogram, XIC, MRM, map): a long press (finger) opens the same menu as the right click; the point actions come first, a separator, then the panel actions (scope "panel") with
their key; the panel actions of the bar are in it; never more than 12 rows at the top level (the rest in one submenu)."""
# covers: mzlab/web/arrange.js, mzlab/web/azioni-icone.js, mzlab/web/azioni.js, mzlab/web/azioni-tasti.js, mzlab/web/azioni-esc.js
# covers: mzlab/web/mappa.js, mzlab/web/mappa3d.js, mzlab/web/eventi.js, mzlab/web/azioni-barra.js, mzlab/web/menu-sotto.js, mzlab/web/explore.js
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_menu_coerenti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
MAP = "E.panels.find(q=>q.type==='map')"
OPEN = "!document.querySelector('#ctx').hidden"
IDS = "[...document.querySelectorAll('#ctx [data-azione]')].map(d=>d.dataset.azione)"
LABELS = "[...document.querySelectorAll('#ctx div[role^=menuitem]')].slice(1).map(d=>d.textContent)"     # without the first line (where the menu acts)
r = Run(port=8871, wd="/tmp/wd_menu_coerenti")
try:
    with sync_playwright() as p:
        pg = r.page(p, touch=True); pg.set_viewport_size({"width": 1440, "height": 1000})
        stage(pg, [mz(f) for f in FILES]); pg.click("text=Load data"); ready(pg)
        def close():
            pg.keyboard.press("Escape"); wait_for_js(pg, "document.querySelector('#ctx').hidden")
        def keyboard(P):
            pg.evaluate(f"(()=>{{const p={P};p.cv.scrollIntoView({{block:'center'}});p.cv.focus()}})()")
            pg.keyboard.press("Shift+F10"); wait_for_js(pg, OPEN)
            assert pg.evaluate("document.activeElement.closest('#ctx')!==null"), "the focus is in the menu"
        pg.evaluate("window.__lt=0;document.addEventListener('pointerdown',e=>{if(e.pointerType!=='mouse')window.__lt=Date.now()},true)")      # touch.js drops a mouse right click for 1.5 s after a finger went down (the browser's own menu of a touch)
        def right_at(P, x, y):
            wait_for_js(pg, "Date.now()-window.__lt>1600")
            b = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.left,r.top]}})()")
            pg.mouse.click(b[0] + x, b[1] + y, button="right"); wait_for_js(pg, OPEN)
        def map2d():
            open_kind(pg, "full"); ready(pg); pg.click("#np-map"); ready(pg); wait_for_js(pg, MAP + "&&" + MAP + "._a")
            want = pg.evaluate(f"AZIONI.forContext(MAPPA.ad2({MAP},null)).filter(i=>i!=='-').map(i=>i.id)")
            assert "mapxic" in want and "maphelp" in want and "map3top" not in want, want
            keyboard(MAP); kb = pg.evaluate(IDS)
            noic = pg.evaluate("[...document.querySelectorAll('#ctx [data-azione]')].filter(d=>!d.querySelector('.ic[aria-hidden=true] svg')).map(d=>d.dataset.azione)"); close()
            assert not noic, ("map menu entries without icon", noic)
            assert pg.evaluate(f"document.activeElement==={MAP}.cv"), "Esc gives the focus back to the map"
            w = pg.evaluate(f"(()=>{{const r={MAP}.cv.getBoundingClientRect();return [r.width,r.height]}})()")
            right_at(MAP, w[0] * 0.5, w[1] * 0.5); ms = pg.evaluate(IDS); close()
            assert kb == ms == want, (kb, ms, want)
            bar = pg.evaluate(f"[...{MAP}.el.querySelectorAll('.mznav [data-nav]')].map(b=>b.dataset.nav)")
            assert bar and all(pg.evaluate(f"(AZIONI.get('{i}')||{{views:[]}}).views.includes('map')") for i in bar), bar
            pal = pg.evaluate("MAPPA.CMDS.filter(c=>MAPPA.paletteOk(c)).map(c=>c.id)")
            assert {i for i in want if not i.startswith("panel.")} <= set(pal), (want, pal)      # the bar tools repeated in the menu (panel.*) are registry actions, not map commands
        step("map 2D: right click = Shift+F10 = the registry; the bar and the palette use registered actions", map2d)
        def same(P, kind):
            open_kind(pg, kind); ready(pg); wait_for_js(pg, P + "&&" + P + "._a")
            pg.evaluate("(()=>{if(window.__menuAt!==undefined)return;window.__menuAt=null;const m=window.menu;window.menu=(ev,items)=>{window.__menuAt=[ev.clientX,ev.clientY];return m(ev,items)}})()")     # where the menu was asked for
            keyboard(P); kb = pg.evaluate(LABELS); at = pg.evaluate("window.__menuAt")
            close(); assert pg.evaluate(f"document.activeElement==={P}.cv"), "Esc gives the focus back to the graph"
            b = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.left,r.top]}})()")
            right_at(P, at[0] - b[0], at[1] - b[1]); ms = pg.evaluate(LABELS); close()
            assert kb and kb == ms, (kb, ms)
        step("chromatogram (Full Scan): Shift+F10 = right click at the same point", lambda: same("E.panels.find(q=>q.type==='chrom'&&q.tab==='full')", "full"))
        SP = "E.panels.find(q=>q.type==='spec'&&q.el&&q.el.offsetParent)"
        step("spectrum: Shift+F10 = right click at the same peak", lambda: same(SP, "full"))
        def spec_ids():
            pg.evaluate(f"(()=>{{const p={SP};p.zoom=null;p.zoomY=null;p.anns=[];draw(p)}})()")
            keyboard(SP); r = pg.evaluate("""(()=>{const its=[...document.querySelectorAll('#ctx div[role^=menuitem]')].slice(1);
              return {n:its.length,free:its.filter(d=>!d.dataset.azione).map(d=>d.textContent),ids:[...new Set(its.map(d=>d.dataset.azione))],
                noic:its.filter(d=>!d.querySelector('.ic[aria-hidden=true] svg')).map(d=>d.dataset.azione),
                bad:its.map(d=>d.dataset.azione).filter(i=>i&&i!=='menu.more'&&!(AZIONI.get(i)||{views:[]}).views.some(x=>x==='spec'||x==='*'))}})()"""); close()
            assert r["n"] >= 5 and not r["free"] and not r["bad"] and not r["noic"], r
            assert "spec.copy" in r["ids"] and "spec.xic" in r["ids"] and "spec.measure" in r["ids"], r["ids"]
        step("spectrum: every entry of the menu is an action of the registry (data-azione)", spec_ids)
        step("MRM: Shift+F10 = right click at the same point", lambda: same("E.panels.find(q=>q.type==='mrm')", "mrm"))
        def trace_ids():
            for sel, view in (("E.panels.find(q=>q.type==='chrom'&&q.tab==='full')", "chrom"), ("E.panels.find(q=>q.type==='mrm')", "mrm")):
                pg.evaluate(f"(()=>{{const p={sel};p.zoom=null;p.zoomY=null;p.anns=[];draw(p)}})()")
                keyboard(sel); r = pg.evaluate("""(v=>{const its=[...document.querySelectorAll('#ctx div[role^=menuitem]')].slice(1);
                  return {n:its.length,free:its.filter(d=>!d.dataset.azione).map(d=>d.textContent),ids:[...new Set(its.map(d=>d.dataset.azione))],
                    noic:its.filter(d=>!d.querySelector('.ic[aria-hidden=true] svg')).map(d=>d.dataset.azione),
                    bad:its.map(d=>d.dataset.azione).filter(i=>i&&i!=='menu.more'&&!(AZIONI.get(i)||{views:[]}).views.some(x=>x===v||x==='*'))}})""" + f"('{view}')"); close()
                assert r["n"] >= 2 and not r["free"] and not r["bad"] and not r["noic"], (view, r)
                assert "chrom.annotate" in r["ids"] and "chrom.int" in r["ids"], r["ids"]
                assert "ann.edit" not in r["ids"], r["ids"]      # the annotation entries need an annotation under the pointer (none here: p.anns is empty)
                assert "chrom.specAtRt" not in r["ids"] and "chrom.avgSel" not in r["ids"], r["ids"]      # the spectrum comes from a click, a drag, the double click and Enter
        step("chromatogram and MRM: every entry of the menu is an action of the registry (data-azione)", trace_ids)
        def tic_vs_xic():
            T = "I18N.t('int.menu.auto')"
            def labels(sel):
                pg.evaluate(f"(()=>{{const p={sel};p.zoom=null;p.zoomY=null;p.anns=[];draw(p)}})()")
                keyboard(sel); l = pg.evaluate(LABELS); close(); return l
            tic = labels("E.panels.find(q=>q.type==='chrom'&&q.tab==='full')"); auto = pg.evaluate(T)
            assert auto not in tic, ("a total chromatogram has no single peak to integrate", tic)
            assert not any(t in " ".join(tic) for t in (pg.evaluate("I18N.t('ctx.spec.avgSel')"), pg.evaluate("I18N.t('ctx.spec.atRt')"))), tic
            pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})")
            XP = "E.panels.find(q=>q.type==='xic')"; wait_for_js(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
            assert auto in labels(XP), "an XIC keeps «Integrate peak»"
        step("TIC: no «Integrate peak», no spectrum entries; XIC keeps the integration", tic_vs_xic)
        # ---- scope: point actions, a separator, panel actions with their key; the bar's commands in the menu; at most 12 rows; long press = right click
        SHAPE = """(()=>{const m=document.querySelector('#ctx'),kids=[...m.children],rows=kids.filter(k=>k.tagName!=='HR');
          const idOf=k=>k.dataset.azione||(k.querySelector('[data-azione]')||{dataset:{}}).dataset.azione;
          let panel=false,bad=[],sep=false,prevHr=false,sawPoint=false,keyless=[],badKey=[];
          for(const k of kids){ if(k.tagName==='HR'){prevHr=true;continue}
            const id=idOf(k),a=id&&AZIONI.get(id),sc=id==='menu.more'?'panel':a?a.scope:null;
            if(sc==='panel'){ if(!panel&&sawPoint)sep=prevHr; panel=true } else if(sc==='point'){ sawPoint=true; if(panel)bad.push(id) }
            if(a&&a.shortcut&&!k.classList.contains('mrow')){const kb=k.dataset.key;if(!kb)keyless.push(id);else if(kb!==AZIONI.keyText(a.shortcut))badKey.push(id)}
            prevHr=false }
          return {top:rows.length,bad,sep,panel,keyless,badKey,labels:rows.map(k=>k.getAttribute('aria-label')||k.textContent),more:!!m.querySelector('[data-azione="menu.more"]')}})()"""
        def shape(P, x=None, y=None):
            w = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.width,r.height]}})()")
            right_at(P, w[0] * 0.5 if x is None else x, w[1] * 0.5 if y is None else y); r_ = pg.evaluate(SHAPE)
            if r_["more"]:        # the rows of the submenu too
                pg.evaluate("document.querySelector('#ctx [data-azione=\"menu.more\"]').click()"); r_["sub"] = pg.evaluate("[...document.querySelectorAll('#ctx .ctxsub > div')].map(d=>d.getAttribute('aria-label')||d.textContent)")
            if r_["more"]:
                pg.keyboard.press("Escape"); wait_for_js(pg, "document.querySelector('#ctx .ctxsub').hidden")      # the first Esc closes the submenu only, the second one the menu
                assert pg.evaluate("document.activeElement.dataset.azione==='menu.more'"), "Esc gives the focus back to the «More» entry"
            r_["bar"] = pg.evaluate(f"(()=>{{const p={P},c={{view:p.type,p}};return AZIONI.list(c,'barra').filter(a=>a.menu!==false&&AZIONI.ready(a,c)).map(a=>(a.menuLabel||a.name)(c))}})()")
            close(); return r_
        def check_scope(P, kind):
            r_ = shape(P)
            assert r_["top"] <= 12, (kind, "more than 12 rows at the top level", r_["labels"])
            assert not r_["bad"], (kind, "a point action after a panel action", r_["bad"])
            assert r_["panel"] and r_["sep"], (kind, "the panel block is separated from the point block", r_)
            assert not r_["keyless"] and not r_["badKey"], (kind, "the key of an action is shown as AZIONI.keyText writes it", r_)
            have = set(r_["labels"]) | set(r_.get("sub", []))
            assert all(b in have for b in r_["bar"]), (kind, "a command of the bar is missing from the menu", [b for b in r_["bar"] if b not in have])
            return r_
        def long_at(P, x, y):
            cdp = pg.context.new_cdp_session(pg)
            b = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.left,r.top]}})()")
            cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": [{"x": b[0] + x, "y": b[1] + y, "id": 0}]})
            wait_for_js(pg, OPEN); labs = pg.evaluate(LABELS)
            cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []}); close(); return labs
        def kind_step(P, kind):
            open_kind(pg, "mrm" if kind == "mrm" else "full"); ready(pg); wait_for_js(pg, P + "&&" + P + "._a")
            pg.evaluate(f"(()=>{{const p={P};p.cv.scrollIntoView({{block:'center'}});p.zoom=null;p.zoomY=null;p.anns=[];draw(p)}})()")
            check_scope(P, kind)
            w = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.width,r.height]}})()")
            right_at(P, w[0] * 0.5, w[1] * 0.5); rc = pg.evaluate(LABELS); close()
            lp = long_at(P, w[0] * 0.5, w[1] * 0.5)
            assert rc and lp == rc, (kind, "a long press opens the menu of the right click", rc, lp)
        for kind, P in (("spectrum", SP), ("chromatogram", "E.panels.find(q=>q.type==='chrom'&&q.tab==='full')"), ("XIC", "E.panels.find(q=>q.type==='xic')"), ("MRM", "E.panels.find(q=>q.type==='mrm')")):
            step(f"{kind}: point actions, separator, panel actions with their key, the bar's commands, at most 12 rows; a long press = right click", lambda P=P, kind=kind: kind_step(P, kind))
        def map_scope():
            open_kind(pg, "full"); ready(pg); pg.click("#np-map"); ready(pg); wait_for_js(pg, MAP + "&&" + MAP + "._a"); pg.evaluate(f"{MAP}.cv.scrollIntoView({{block:'center'}})")
            r_ = shape(MAP); assert r_["top"] <= 12 and not r_["bad"] and r_["panel"] and r_["sep"] and not r_["keyless"] and not r_["badKey"], r_
            w = pg.evaluate(f"(()=>{{const r={MAP}.cv.getBoundingClientRect();return [r.width,r.height]}})()")
            right_at(MAP, w[0] * 0.5, w[1] * 0.5); rc = pg.evaluate(LABELS); close()
            lp = long_at(MAP, w[0] * 0.5, w[1] * 0.5); assert rc and lp == rc, ("map: a long press opens the menu of the right click", rc, lp)
        step("map: point actions, separator, panel actions; a long press = right click", map_scope)
        def early_click():
            ctx = r.b.new_context(viewport={"width": 1500, "height": 2200}, reduced_motion="reduce"); pg2 = ctx.new_page(); held = []
            pg2.route("**/panel-menu.js", lambda route: held.append(route))      # the lazy file is held: the right click comes before it has loaded
            pg2.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');}catch(_){}")
            pg2.goto(f"http://127.0.0.1:{r.port}/"); stage(pg2, [mz(f) for f in FILES]); pg2.click("text=Load data"); ready(pg2)
            open_kind(pg2, "full"); ready(pg2); wait_for_js(pg2, SP + "&&" + SP + "._a"); wait_for_js(pg2, "!window.PANELMENU")
            b = pg2.evaluate(f"(()=>{{const c={SP}.cv;c.scrollIntoView({{block:'center'}});const r=c.getBoundingClientRect();return [r.left+r.width/2,r.top+r.height/2]}})()")
            pg2.mouse.click(b[0], b[1], button="right"); wait_for_js(pg2, "document.querySelector('#ctx').hidden && !window.PANELMENU")
            assert len(held) == 1, "the menu asked for the lazy file"
            held[0].continue_(); wait_for_js(pg2, OPEN)
            r_ = pg2.evaluate(SHAPE); ctx.close()
            assert r_["panel"] and r_["sep"] and not r_["bad"] and r_["top"] <= 12 and not r_["keyless"], ("the full menu one tick later", r_)
        step("a right click before the lazy menu file has loaded waits for it and still gets the full menu", early_click)
        def every_action():
            r = pg.evaluate("""(()=>{const none=AZIONI.actions.filter(a=>!a.svg&&!(a.iconKey&&AZIONI_ICONE.DISEGNI[a.iconKey])).map(a=>a.id);
              const bad=AZIONI.actions.filter(a=>a.iconKey&&!AZIONI_ICONE.DISEGNI[a.iconKey]).map(a=>a.id);
              const col=getComputedStyle(document.querySelector('#ctx')).color;return {n:AZIONI.actions.length,none,bad,col}})()""")
            assert r["n"] >= 40 and not r["none"] and not r["bad"], r
        step("every action of the registry has an existing icon", every_action)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
