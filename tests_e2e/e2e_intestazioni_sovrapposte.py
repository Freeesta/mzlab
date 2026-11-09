"""Panel headers: no control paints over another at 1000/1280/1600/2000 px and on iPad (1024x768, 1366x1024, touch), for the MRM panel after an integration
(its integration controls appear later and the «⋯» menu must take them into account) and for the chromatogram, spectrum, XIC and map of a full-scan file."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_intestazioni_sovrapposte.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:400]))

r = Run(port=8961, wd="/tmp/wd_intestazioni_sovrapposte")
SKIP = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1')}catch(_){}"
# every visible control of every panel header, compared pair by pair (1 px of rounding allowed); the controls group (.ctl) must not hold more than its width,
# since what it holds would spill over the buttons beside it
OVERLAP = r"""(() => { const bad = [];
  for (const p of E.panels) { const hd = p.el && p.el.offsetParent && p.el.querySelector('.hd'); if (!hd) continue;
    const els = [...hd.querySelectorAll('button,select,input,.ttl,[role=button]')].filter(e => e.offsetWidth && e.offsetHeight && !e.closest('.hmore,[hidden]'));
    const R = els.map(e => [e, e.getBoundingClientRect()]), nm = e => (e.dataset.a || e.dataset.o || e.className || e.tagName) + '';
    for (let i = 0; i < R.length; i++) for (let j = i + 1; j < R.length; j++) { const [a, ra] = R[i], [b, rb] = R[j]; if (a.contains(b) || b.contains(a)) continue;
      const ox = Math.min(ra.right, rb.right) - Math.max(ra.left, rb.left), oy = Math.min(ra.bottom, rb.bottom) - Math.max(ra.top, rb.top);
      if (ox > 1 && oy > 1) bad.push(`${p.type}: ${nm(a)} over ${nm(b)} by ${Math.round(ox)} px`); }
    const c = hd.querySelector('.ctl'); if (c && c.scrollWidth > c.clientWidth + 1) bad.push(`${p.type}: the controls hold ${c.scrollWidth} px in ${c.clientWidth}`); }
  return bad; })()"""
WIDTHS = [((1000, 900), False), ((1280, 900), False), ((1600, 1000), False), ((2000, 1100), False), ((1024, 768), True), ((1366, 1024), True)]

def page(size, touch):
    k = dict(viewport={"width": size[0], "height": size[1]}, locale="it-IT", reduced_motion="reduce")
    if touch: k.update(has_touch=True, is_mobile=True)
    c = r.b.new_context(**k); c.add_init_script(SKIP)
    q = c.new_page(); q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_selector("#pick", state="attached"); ready(q)
    return q

def settled(q):
    """The headers are fitted again a frame and 300 ms after a change (fitHdSoon): read them once that has run."""
    frames(q, 3); hold(q, 400); frames(q, 2)
    return q.evaluate(OVERLAP)

def mrm(size, touch):
    q = page(size, touch)
    try:
        q.set_input_files("#pick", [mz("B_MRM-t0"), mz("B_MRM-t5")]); hold(q, 1000); q.click("text=Carica dati"); ready(q)
        wait_for_js(q, "!!document.querySelector('.pnl.mrm canvas') && E.panels.some(p => p.type === 'mrm' && p._a)", 60000)
        q.evaluate("E.files.forEach(f => f.vis = true); syncBrowse(); redrawAll()"); ready(q)
        bad = settled(q); assert not bad, (size, "before integrating", bad)
        # automatic integration: a click on the peak (14.3 of 20 min) with the tool on; its controls (target file, clear, table) then appear in the header
        q.evaluate("document.querySelector('.pnl.mrm [data-a=iauto]').click()"); frames(q, 2)
        b = q.locator(".pnl.mrm canvas").first.bounding_box()
        for f in (0.70, 0.71, 0.69, 0.72):
            q.mouse.click(b["x"] + b["width"] * f, b["y"] + b["height"] * 0.5); frames(q, 3)
            if q.evaluate("(s => !!s && !s.hidden)(document.querySelector('.pnl.mrm [data-a=iclr]'))"): break
        assert q.evaluate("(s => !!s && !s.hidden)(document.querySelector('.pnl.mrm [data-a=iclr]'))"), (size, "no integration")
        bad = settled(q); assert not bad, (size, "after integrating", bad)
    finally:
        q.context.close()

def full(size, touch):
    q = page(size, touch)
    try:
        q.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t5")]); hold(q, 1000); q.click("text=Carica dati"); ready(q)
        wait_for_js(q, "E.panels.some(p => p.type === 'chrom' && p._a) && E.panels.some(p => p.type === 'spec')", 60000)
        q.evaluate("(()=>{const f=E.files.find(x=>x.kind==='full');addPanel('xic',{tab:E.tab,traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}],x:0,y:700,w:hostWidth(),h:300,full:true});addPanel('map',{tab:'full',k:f.k,x:0,y:1020,w:hostWidth(),h:380,full:true});relayout();fitHost()})()")
        wait_for_js(q, "E.panels.some(p => p.type === 'xic') && E.panels.some(p => p.type === 'map')", 30000); ready(q)
        bad = settled(q); assert not bad, (size, bad)
    finally:
        q.context.close()

bad = ["not run"]
try:
    with sync_playwright() as p:
        r.page(p).context.close()      # the browser of the run (Chromium, or WebKit with --browser)
        step("MRM header, before and after an integration: no control over another (1000, 1280, 1600, 2000 px, iPad 1024x768 and 1366x1024)",
             lambda: [mrm(s, t) for s, t in WIDTHS])
        step("full scan: chromatogram, spectrum, XIC and map headers, no control over another (same widths)", lambda: [full(s, t) for s, t in WIDTHS])
    bad = [s for s in steps if s[1] != "ok"]
    for n, s in steps: print(("OK " if s == "ok" else "FAIL ") + n + (" " + s if s != "ok" else ""))
finally:
    r.close()
sys.exit(1 if bad else 0)
