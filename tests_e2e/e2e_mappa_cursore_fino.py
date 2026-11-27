"""RT x m/z map, the cursor marker is slim and refined (mappa-cursore.js, mappa.js): one style wherever the cursor is drawn. Chromium at 1400 px (side spectrum and XIC
visible) and 390 px (cross only): the cross over the map is 1 px, the guide 1 px, the bar of the peak under the cursor at most 1.5 px, all in the muted token (contrast >= 3:1
on the panel, light and dark theme), the end tick is small, the label is not bold and sits on no neighbouring label, and the XIC RT line has the same style."""
# covers: mzlab/web/mappa.js, mzlab/web/mappa-cursore.js, mzlab/web/mappa-dad-ms.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8938, wd="/tmp/wd_mappa_cursore_fino_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))
def lum(c):
    f = lambda v: (v / 255 / 12.92) if v / 255 <= .03928 else (((v / 255 + .055) / 1.055) ** 2.4)
    return .2126 * f(c[0]) + .7152 * f(c[1]) + .0722 * f(c[2])
def rgb(s):                                                                      # "#rrggbb" or "rgb(r, g, b)" as the canvas and the CSS report them
    s = s.strip()
    if s.startswith("#"): return [int(s[i:i + 2], 16) for i in (1, 3, 5)]
    return [int(float(v)) for v in s[s.index("(") + 1:s.index(")")].replace("/", ",").split(",")[:3]]
def contrast(a, b):
    la, lb = lum(rgb(a)), lum(rgb(b)); return (max(la, lb) + .05) / (min(la, lb) + .05)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        for width, wide in ((1400, True), (390, False)):
            pg = b.new_context(locale="en-US", viewport={"width": width, "height": 950}).new_page()
            pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');}catch(_){}")
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "!!document.querySelector('#pick')")
            pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")])
            pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); wait_for_js(pg, f"!!{M}._mz")
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); ready(pg)
            if wide: wait_for_js(pg, f"!{M}._mz.sp.hidden && !{M}._mz.xc.hidden")
            else: wait_for_js(pg, f"!!{M}._a")
            def hover(rt, mz):
                x, y = pg.evaluate(f"(()=>{{const a={M}._a,r={M}.cv.getBoundingClientRect();return [r.left+a.X({rt}),r.top+a.Y({mz})]}})()")
                pg.mouse.move(x, y)
                wait_for_js(pg, f"(()=>{{const s={M}._mz,h=s.hov;return !!h&&Math.abs(h.rt-{rt})<0.5&&!!s.cm&&!!s.cm.cross&&(!{str(wide).lower()}||(!!s.shown&&s.shown.sp&&s.shown.xi&&s.shown.pt===h&&!!s.cm.rt&&!!s.cm.guide))}})()", 20000)
            def colour_ok(theme):
                muted = pg.evaluate("getComputedStyle(document.documentElement).getPropertyValue('--muted').trim()")
                panel = pg.evaluate("getComputedStyle(document.documentElement).getPropertyValue('--panel').trim()")
                cm = pg.evaluate(f"{M}._mz.cm")
                for k in [k for k in ("cross", "guide", "bar", "rt") if cm.get(k)]:
                    assert rgb(cm[k]["c"]) == rgb(muted), (width, theme, k, cm[k], muted)
                    assert contrast(cm[k]["c"], panel) >= 3, (width, theme, k, "contrast", contrast(cm[k]["c"], panel))
                return cm
            def style():
                for theme in ("light", "dark"):
                    pg.evaluate(f"document.documentElement.dataset.theme='{theme}'")
                    hover(14.38, 364.3); hover(14.38, 364.4)
                    cm = colour_ok(theme)
                    assert cm["cross"]["w"] <= 1, (width, theme, cm)
                    if wide:
                        assert cm["guide"]["w"] <= 1 and cm["rt"]["w"] <= 1, (width, theme, cm)
                        assert cm.get("bar") and cm["bar"]["w"] <= 1.5, (width, theme, "bar", cm)
            step(f"{width} px: marker lines 1 px (bar at most 1.5 px), muted token, contrast >= 3:1 in both themes", style)
            if wide:
                def label():
                    hover(14.38, 364.3)
                    top = pg.evaluate(f"(()=>{{const s={M}._mz,a={M}._a,sp=s.spec,o=[];for(let i=0;i<sp.mz.length;i++)if(sp.mz[i]>=a.y0&&sp.mz[i]<=a.y1)o.push([sp.mz[i],sp.y[i]]);return o.sort((u,v)=>v[1]-u[1]).slice(0,6)}})()")
                    assert len(top) >= 3, top
                    for mz, _ in top:
                        hover(14.38, mz)
                        L = pg.evaluate(f"({M}._mz.lbls||[]).map(l=>({{...l}}))"); W = pg.evaluate(f"{M}._mz.sp.getBoundingClientRect().width")
                        cur = [l for l in L if l.get("cur")]; assert len(cur) == 1, (mz, L)
                        c = cur[0]
                        assert c["h"] <= 13 and c["y"] >= 0 and c["x"] >= 0 and c["x"] + c["w"] <= W, ("small and inside the panel", c, W)
                        for o in L:
                            if o is not c: assert o["y"] + o["h"] <= c["y"] or c["y"] + c["h"] <= o["y"] or o["x"] + o["w"] <= c["x"] or c["x"] + c["w"] <= o["x"], ("overlap", c, o)
                step(f"{width} px: the label of the peak under the cursor is small, inside the panel and on no neighbour", label)
            pg.close()
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
