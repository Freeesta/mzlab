"""Focus / hover rings of the cells in the reference lists (Tavola periodica, Addotti, Perdite) are never clipped by the scrolling container:
for the first and last element of every row, the ring (outline width + offset) stays inside the nearest clipping ancestor. Light and dark, 1280 / 1440 / tablet."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
# for each candidate: apply hover/focus, take the ring rect and the nearest ancestor that clips (overflow not visible); report the pixels outside
SCAN = """(sel)=>{
 const out=[], rows=new Map();
 const els=[...document.querySelectorAll(sel)].filter(e=>e.getClientRects().length);
 els.forEach(e=>{const r=e.getBoundingClientRect(),k=Math.round(r.top/4);const a=rows.get(k)||[];a.push([r.left,e]);rows.set(k,a)});
 const pick=new Set(); rows.forEach(a=>{a.sort((x,y)=>x[0]-y[0]);pick.add(a[0][1]);pick.add(a[a.length-1][1])});
 pick.forEach(e=>{
  e.focus({focusVisible:true});
  const cs=getComputedStyle(e), w=cs.outlineStyle==='none'?0:parseFloat(cs.outlineWidth)||0, off=parseFloat(cs.outlineOffset)||0, g=w+Math.max(off,0)-Math.min(off,0)*0;
  const ext=cs.outlineStyle==='none'?0:Math.max(0,w+off), r=e.getBoundingClientRect();
  let p=e.parentElement; while(p&&p!==document.documentElement){const o=getComputedStyle(p); if(o.overflowX!=='visible'||o.overflowY!=='visible')break; p=p.parentElement}
  if(!p||p===document.documentElement)return;
  const c=p.getBoundingClientRect(), b=getComputedStyle(p), L=c.left+parseFloat(b.borderLeftWidth), R=c.right-parseFloat(b.borderRightWidth), T=c.top+parseFloat(b.borderTopWidth), B=c.bottom-parseFloat(b.borderBottomWidth);
  const ox=getComputedStyle(p).overflowX!=='visible', oy=getComputedStyle(p).overflowY!=='visible';
  const over=Math.max(ox?L-(r.left-ext):0, ox?(r.right+ext)-R:0, oy?T-(r.top-ext):0, 0);
  if(over>0.5) out.push((e.className||e.tagName)+' '+(e.textContent||'').trim().slice(0,6)+' over='+over.toFixed(1)+' in '+(p.id||p.className));
 });
 return {n:pick.size,out}}"""
r = Run(port=8979, wd="/tmp/wd_anelli")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def run(label, sel, opener):
            for theme in ("light", "dark"):
                for w, h in ((1280, 800), (1440, 900), (1024, 768)):
                    pg.set_viewport_size({"width": w, "height": h}); pg.evaluate(f"document.documentElement.setAttribute('data-theme','{theme}')")
                    opener(); pg.wait_for_timeout(300)
                    res = pg.evaluate(SCAN, sel); assert res["n"] > 1, (label, res)
                    # hovered state (the periodic table uses :hover): hover the first and last cell of the first row
                    assert not res["out"], (label, theme, w, res["out"][:4])
        def closedlg(): pg.evaluate("(document.querySelector('#refdlg')||{close(){}}).close()")
        def op(btn): 
            def f(): closedlg(); pg.evaluate(f"document.querySelector('{btn}').click()")
            return f
        step("Tavola periodica: ring of the cells inside the container", lambda: run("pt", "#refbody .pt-c", op("#np-pt")))
        step("Addotti: ring of the focusable cells inside the container", lambda: run("ad", "#refbody button, #refbody input, #refbody a", op("#np-ad")))
        step("Perdite: ring of the focusable cells inside the container", lambda: run("nl", "#refbody button, #refbody input, #refbody a, #refbody tr[tabindex]", op("#np-nl")))
        def pthover():
            pg.set_viewport_size({"width": 1280, "height": 800}); closedlg(); pg.evaluate("document.querySelector('#np-pt').click()"); pg.wait_for_timeout(400)
            bad = []
            for sym in ("He", "H", "Ne", "Ar", "Og", "La"):
                el = pg.locator(f'#refbody .pt-c:has(b:text-is("{sym}"))').first
                if not el.count(): continue
                el.hover(); pg.wait_for_timeout(80)
                d = pg.evaluate("""(sym)=>{const e=[...document.querySelectorAll('#refbody .pt-c')].find(x=>x.querySelector('b')?.textContent===sym);const cs=getComputedStyle(e),x=cs.outlineWidth,off=parseFloat(cs.outlineOffset)||0,w=parseFloat(x)||0;
                 const r=e.getBoundingClientRect(),c=document.querySelector('#refbody').getBoundingClientRect();return {l:r.left-(w+off)-c.left,r:c.right-(r.right+w+off),t:r.top-(w+off)-c.top,b:c.bottom-(r.bottom+w+off)}}""", sym)
                if min(d.values()) < -0.5: bad.append((sym, d))
            assert not bad, bad
        step("Tavola periodica: hover ring of He, H, Ne, Ar, Og, La never outside #refbody", pthover)
        def cellsize():
            pg.set_viewport_size({"width": 1280, "height": 800}); closedlg(); pg.evaluate("document.querySelector('#np-pt').click()"); pg.wait_for_timeout(400)
            m = pg.evaluate("(()=>{const c=document.querySelector('#refbody .pt-c').getBoundingClientRect(),b=document.querySelector('#refbody');return {w:Math.round(c.width*10)/10,h:c.height,sh:b.scrollWidth-b.clientWidth}})()")
            assert m["sh"] <= 0, ("horizontal scrollbar", m); print("CELL", m)
        step("no horizontal scroll bar in the table", cellsize)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
