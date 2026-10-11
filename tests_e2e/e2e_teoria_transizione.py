"""Transition between chapters: cross-document view transitions (header and index stay still, the content fades), no flash of colour, no animation
with reduced motion, the CSS fade when view transitions do not exist, scroll at the top / index scroll kept / anchors, prefetch of the neighbours."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import lib
real_wait = lib._real_wft
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
# recorded by every page at pagereveal (sessionStorage survives the navigation): was there a view transition, which animations ran, the html background
SPY = """(()=>{addEventListener('pagereveal',e=>{try{sessionStorage.length}catch(x){return}const bg=[getComputedStyle(document.documentElement).backgroundColor];
 const o={vt:!!e.viewTransition,bg,anims:[],side:document.querySelectorAll('#side li').length,top:!!document.querySelector('#top')};sessionStorage.setItem('__rev',JSON.stringify(o));
 if(e.viewTransition)e.viewTransition.ready.then(()=>{o.anims=document.documentElement.getAnimations({subtree:true}).map(a=>(a.effect.pseudoElement||'')+':'+a.animationName+':'+a.effect.getTiming().duration);
  bg.push(getComputedStyle(document.documentElement).backgroundColor);sessionStorage.setItem('__rev',JSON.stringify(o))}).catch(()=>{})});
 window.__errs=[];addEventListener('error',e=>window.__errs.push(String(e.message)))})();"""
def rev(pg): return pg.evaluate("JSON.parse(sessionStorage.getItem('__rev')||'null')")
def ctxpage(b, init=SPY, **kw):
    ctx = b.new_context(viewport={"width": 1280, "height": 800}, **kw); ctx.add_init_script(init); pg = ctx.new_page(); errs = []
    pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
    return ctx, pg, errs
r = Run(port=8976, wd="/tmp/wd_tt")
try:
    with sync_playwright() as p:
        r.page(p); b = r.b; T = f"http://127.0.0.1:{r.port}/static/teoria/"
        def click_chapter(pg, href):
            pg.click(f'#side li a[href="{href}"]'); pg.wait_for_url("**/" + href); pg.wait_for_function("document.readyState==='complete'"); real_wait(pg, 500)
        def vt():
            ctx, pg, errs = ctxpage(b, color_scheme="dark"); pg.goto(T + "06-esi.html"); real_wait(pg, 400)
            click_chapter(pg, "08-vuoto.html"); o = rev(pg)
            assert o and o["vt"], o
            assert o["side"] > 20 and o["top"], ("index and header are already built when the new page is first revealed", o)
            names = " ".join(o["anims"]); assert "::view-transition-new(root):vt-in:180" in names, o["anims"]
            assert all(x == "rgb(20, 23, 27)" for x in o["bg"]), o["bg"]
            nm = pg.evaluate("[getComputedStyle(document.querySelector('#top')).viewTransitionName,getComputedStyle(document.querySelector('#side')).viewTransitionName]")
            assert nm == ["vt-top", "vt-side"], nm
            assert not errs, errs; ctx.close()
        step("click on another chapter: view transition with a 180 ms fade of the content, header and index named, dark background throughout, no console errors", vt)
        def slow():
            ctx, pg, errs = ctxpage(b, color_scheme="dark"); pg.goto(T + "06-esi.html"); real_wait(pg, 400)
            import time
            pg.route("**/teoria/teoria.js", lambda rt: (time.sleep(0.8), rt.continue_()))   # a loaded machine: the script arrives late
            click_chapter(pg, "08-vuoto.html"); o = rev(pg)
            assert o and (o["side"] > 20 or not o["anims"]), ("a transition never starts over a page without index", o)
            assert not errs, errs; ctx.close()
        step("slow script: no transition over a page that has no index yet, no errors", slow)
        def scroll():
            ctx, pg, errs = ctxpage(b); pg.goto(T + "06-esi.html"); real_wait(pg, 400)
            pg.evaluate("document.querySelector('#side').scrollTop=150"); pg.evaluate("scrollTo(0,900)"); real_wait(pg, 200)
            before = pg.evaluate("document.querySelector('#side').scrollTop"); assert before > 100, before
            href = pg.evaluate("(()=>{const s=document.querySelector('#side').getBoundingClientRect();return [...document.querySelectorAll('#side ol a')].find(a=>{const r=a.getBoundingClientRect();return r.top>s.top+80&&r.bottom<s.bottom-80&&!a.classList.contains('on')}).getAttribute('href')})()")
            click_chapter(pg, href)
            y = pg.evaluate("scrollY"); st = pg.evaluate("document.querySelector('#side').scrollTop")
            assert y == 0, y; assert abs(st - before) <= 2, (st, before); assert not errs, errs; ctx.close()
        step("new chapter starts at the top and the index keeps its scroll position", scroll)
        def anchor():
            ctx, pg, errs = ctxpage(b); pg.goto(T + "06-esi.html"); real_wait(pg, 400)
            pg.goto(T + "08-vuoto.html"); real_wait(pg, 300)
            hid = pg.evaluate("document.querySelectorAll('main h2')[2].id")
            pg.evaluate("location.href=location.pathname+'#" + hid + "'"); real_wait(pg, 700)
            y1 = pg.evaluate("scrollY"); top = pg.evaluate(f"document.getElementById('{hid}').getBoundingClientRect().top"); real_wait(pg, 600)
            y2 = pg.evaluate("scrollY"); assert y1 > 100 and y1 == y2 and 0 <= top < 140, (y1, y2, top)
            ctx.close()
        step("anchor in the page: scrolls to the heading and stays there", anchor)
        def anchor_nav():
            ctx, pg, errs = ctxpage(b); pg.goto(T + "08-vuoto.html"); real_wait(pg, 300)
            hid = pg.evaluate("document.querySelectorAll('main h2')[2].id")
            pg.goto(T + "06-esi.html"); real_wait(pg, 300)
            pg.evaluate(f"location.href='08-vuoto.html#{hid}'"); pg.wait_for_url("**#" + hid); real_wait(pg, 1200)
            y1 = pg.evaluate("scrollY"); top = pg.evaluate(f"document.getElementById('{hid}').getBoundingClientRect().top")
            assert y1 > 100 and 0 <= top < 140, (y1, top); assert not errs, errs; ctx.close()
        step("navigating to chapter#section lands on the section after the transition", anchor_nav)
        def reduced():
            ctx, pg, errs = ctxpage(b, reduced_motion="reduce"); pg.goto(T + "06-esi.html"); real_wait(pg, 400)
            click_chapter(pg, "08-vuoto.html"); o = rev(pg)
            assert o and not o["vt"], o
            assert pg.evaluate("document.documentElement.hasAttribute('data-a11y-reduce')")
            assert pg.evaluate("getComputedStyle(document.querySelector('main')).animationName") == "none"
            assert not errs, errs; ctx.close()
        step("prefers-reduced-motion: no view transition and no animation", reduced)
        def a11y_reduce():
            ctx, pg, errs = ctxpage(b, init=SPY + "try{localStorage.setItem('qqq.a11y',JSON.stringify({motion:'always'}))}catch(e){}"); pg.goto(T + "06-esi.html"); real_wait(pg, 400)
            click_chapter(pg, "08-vuoto.html"); o = rev(pg)
            assert not o["anims"], o
            ctx.close()
        step("data-a11y-reduce (Lettura facilitata): the fade does not run", a11y_reduce)
        def fallback():
            ctx, pg, errs = ctxpage(b, init="delete window.PageRevealEvent;"); pg.goto(T + "06-esi.html")
            assert pg.evaluate("document.documentElement.classList.contains('vt-no')")
            n = pg.evaluate("(()=>{const m=document.querySelector('main');return getComputedStyle(m).animationName+':'+getComputedStyle(m).animationDuration})()")
            assert n == "vt-fade:0.12s", n; assert not errs, errs; ctx.close()
        step("without view transitions: 120 ms CSS fade of the content (html.vt-no)", fallback)
        def fallback_reduced():
            ctx, pg, errs = ctxpage(b, init="delete window.PageRevealEvent;", reduced_motion="reduce"); pg.goto(T + "06-esi.html")
            assert pg.evaluate("getComputedStyle(document.querySelector('main')).animationName") == "none"; ctx.close()
        step("without view transitions and with reduced motion: no fade", fallback_reduced)
        def prefetch():
            ctx, pg, errs = ctxpage(b); pg.goto(T + "08-vuoto.html"); real_wait(pg, 400)
            urls = pg.evaluate("[...document.querySelectorAll('script[type=speculationrules]')].map(s=>JSON.parse(s.textContent).prefetch[0].urls)")
            assert len(urls) == 1 and len(urls[0]) == 2 and all(u.endswith(".html") for u in urls[0]), urls; ctx.close()
            ctx, pg, errs = ctxpage(b, init="Object.defineProperty(navigator,'connection',{get:()=>({saveData:true,effectiveType:'4g'})});")
            pg.goto(T + "08-vuoto.html"); real_wait(pg, 400)
            assert pg.evaluate("document.querySelectorAll('script[type=speculationrules]').length") == 0
            ctx.close()
            ctx, pg, errs = ctxpage(b, init="Object.defineProperty(navigator,'connection',{get:()=>({saveData:false,effectiveType:'3g'})});")
            pg.goto(T + "08-vuoto.html"); real_wait(pg, 400)
            assert pg.evaluate("document.querySelectorAll('script[type=speculationrules]').length") == 0; ctx.close()
        step("prefetch of the previous and next chapter; none with Save-Data or a slow connection", prefetch)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
