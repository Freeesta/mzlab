"""Intestazioni dei pannelli (HR): una riga bassa, mai a capo né tagliata a 1280/1440/1680 px, a 700/900/1200 px, con il cassetto aperto o fissato e su iPad
(1024×1366 e 1366×1024, anche ruotando); sotto la larghezza utile i gruppi vanno nel menu «⋯» (e tornano a posto con lo spazio); il grafico resta alto almeno 100 px."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_intestazioni.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))

D = synth("intestazioni")
r = Run(port=8958, wd="/tmp/wd_intestazioni")

# every panel header: one row, every control inside it, no scrolling content, a chart at least 100 px high
CHECK = """(() => E.panels.filter(p => p.el && p.el.offsetParent).map(p => {
  const hd = p.el.querySelector('.hd'), hr = hd.getBoundingClientRect();
  const items = [...hd.querySelectorAll(':scope > *, .ctl > *, .tbs > *')].filter(e => e.offsetParent && !e.closest('.hmore') && !e.classList.contains('rd') && !e.classList.contains('hmore') && e.getBoundingClientRect().width);
  const mid = items.map(e => { const q = e.getBoundingClientRect(); return q.top + q.height / 2; });
  const out = items.filter(e => { const q = e.getBoundingClientRect(); return q.right > hr.right + 1 || q.left < hr.left - 1; }).map(e => e.className || e.tagName);
  return {id: p.id, type: p.type, rows: Math.round(Math.max(...mid) - Math.min(...mid)), hh: Math.round(hr.height), over: hd.scrollWidth - hd.clientWidth, out,
          ch: Math.round(p.cv.getBoundingClientRect().height), more: !!(p.el.querySelector('[data-a=more]') && !p.el.querySelector('[data-a=more]').hidden), pw: p.el.offsetWidth};
}))()"""

def check(q, tag):
    res = q.evaluate(CHECK); assert res, (tag, "no panels")
    for x in res:
        assert x["rows"] <= x["hh"] * 0.5 and x["over"] <= 1 and not x["out"], (tag, x)
        assert x["ch"] >= 100, (tag, "the chart is too low", x)
    return res

def setup(q):
    ready(q); q.wait_for_function("typeof E !== 'undefined' && E.files.length>=1", timeout=60000); q.wait_for_timeout(800)
    q.evaluate("(()=>{addPanel('xic',{tab:E.tab,traces:[HR.ionTrace(305.0702,{tab:E.tab})],x:0,y:700,w:hostWidth(),h:300,full:true});relayout();fitHost()})()"); q.wait_for_timeout(1500)
    q.evaluate("(()=>{const f=E.files.find(x=>x.kind==='full'); if(f){addPanel('map',{tab:'full',k:f.k,x:0,y:1020,w:hostWidth(),h:380,full:true});relayout();fitHost()}})()"); q.wait_for_timeout(1500)

try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML", D / "HR_DDA-Fusion-t30.mzML"], 4000, 4)
        setup(pg)
        assert pg.evaluate("E.panels.some(p=>p.type==='chrom') && E.panels.some(p=>p.type==='spec') && E.panels.some(p=>p.type==='xic')")

        def larghe():
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(700)
                res = check(pg, w); print(w, [(x["type"], x["rows"], x["more"]) for x in res])
        step("header on one row at 1280, 1440, 1680 px", larghe)

        def strette():
            seen = False
            for w in (1200, 900, 700):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(700)
                res = check(pg, w); print(w, [(x["type"], x["rows"], x["more"]) for x in res]); seen = seen or any(x["more"] for x in res)
            assert seen, "at 700 px something goes into the «⋯» menu"
            # the «⋯» menu opens, holds the moved controls and closes with Esc; a moved control still works
            cid = pg.evaluate("E.panels.find(p=>p.type==='chrom').id")
            sel = f"E.panels.find(p=>p.id=={cid}).el"
            assert pg.evaluate(f"!{sel}.querySelector('[data-a=more]').hidden"), "chromatogram header has the «⋯» button at 700 px"
            pg.evaluate(f"{sel}.querySelector('[data-a=more]').click()"); pg.wait_for_timeout(200)
            assert pg.evaluate(f"!{sel}.querySelector('.hmore').hidden && {sel}.querySelector('.hmore').children.length>0 && {sel}.querySelector('[data-a=more]').getAttribute('aria-expanded')==='true'")
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.evaluate(f"{sel}.querySelector('.hmore').hidden")
            pg.set_viewport_size({"width": 1680, "height": 900}); pg.wait_for_timeout(700)
            assert pg.evaluate(f"{sel}.querySelector('[data-a=more]').hidden && {sel}.querySelector('.hmore').children.length===0"), "with room again everything is back in the header"
        step("narrow: groups go into «⋯» (opens, closes with Esc, comes back with room); 1200, 900, 700 px", strette)

        def smoothing():
            cid = pg.evaluate("E.panels.find(p=>p.type==='chrom').id")
            b = f"E.panels.find(p=>p.id=={cid}).el.querySelector('[data-o=smooth]')"
            assert pg.evaluate(f"{b}.getAttribute('aria-pressed')") == "true" and pg.evaluate(f"{b}.tagName") == "BUTTON"
            pg.evaluate(f"E.active=E.panels.find(p=>p.id=={cid}); BARRA.commands.find(c=>c.id==='smooth').run()"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"E.panels.find(p=>p.id=={cid}).smooth") is False and pg.evaluate(f"{b}.getAttribute('aria-pressed')") == "false"
            pg.evaluate(f"{b}.click()"); pg.wait_for_timeout(300)
            assert pg.evaluate(f"E.panels.find(p=>p.id=={cid}).smooth") is True and pg.evaluate(f"{b}.getAttribute('aria-pressed')") == "true"
        step("Smoothing: a button, also reachable from the palette command", smoothing)

        def cassetto():
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(500)
                pg.evaluate("BARRA.setTab('file',{noFocus:true})"); pg.wait_for_timeout(500)
                check(pg, ("drawer", w))
                pg.evaluate("BARRA.setPinned(true)"); pg.wait_for_timeout(900)
                check(pg, ("pinned", w))
                pg.evaluate("BARRA.setPinned(false); BARRA.close && BARRA.close()"); pg.wait_for_timeout(500)
        step("drawer open or pinned: headers stay on one row, nothing cut (1280, 1440, 1680)", cassetto)

        def ipad():
            for w, h in ((1024, 1366), (1366, 1024)):
                ctx = r.b.new_context(viewport={"width": w, "height": h}, has_touch=True, is_mobile=True, locale="it-IT", reduced_motion="reduce")
                q = ctx.new_page(); q.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
                q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_timeout(1500)
                try:
                    setup(q)                       # the files are already in the session of the server
                    check(q, ("ipad", w))
                    q.evaluate("BARRA.setTab('file',{noFocus:true})"); q.wait_for_timeout(500); check(q, ("ipad drawer", w))
                    q.set_viewport_size({"width": h, "height": w}); q.wait_for_timeout(800); check(q, ("ipad rotated", h))
                finally:
                    ctx.close()
        step("iPad 1024×1366 and 1366×1024 (touch), drawer open, rotated: one row, nothing cut", ipad)

    print("\n".join(f"{n}: {s}" for n, s in steps))
    bad = [s for s in steps if s[1] != "ok"]
    for n, s in steps: print(("OK " if s == "ok" else "FAIL ") + n + (" " + s if s != "ok" else ""))
finally:
    r.close()
