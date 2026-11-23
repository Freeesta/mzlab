"""RT x m/z map, three interface defects of the cursor (mappa.js, app.css), on a synthetic Full Scan:
(1) the m/z label of the peak under the cursor is always drawn (bold, inside the panel, on no other label and not under the dashed guide), also when the bar
reaches the right edge, and its bar is the thick one; (2) a live indicator follows the cursor: a low-contrast chip with RT (min) and m/z that never takes a pointer
event, stays inside the map and flips near the edges, and a dot on the XIC at the cursor RT; the arrow keys move both; (3) the segmented buttons mark | snap | sight
sit on one baseline, with no seam between two active neighbours and the height of the 3D button, in the light and the dark theme."""
# covers: mzlab/web/mappa.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8937, wd="/tmp/wd_mappa_cursore_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "!!document.querySelector('#pick')")
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")])
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); wait_for_js(pg, f"!!{M}._mz")
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); ready(pg)                      # full screen turns the sight on
        wait_for_js(pg, f"!{M}._mz.sp.hidden && !{M}._mz.xc.hidden")
        at = lambda rt, mz: pg.evaluate(f"(()=>{{const a={M}._a,r={M}.cv.getBoundingClientRect();return [r.left+a.X({rt}),r.top+a.Y({mz})]}})()")
        def hover(rt, mz, dx=0, dy=0):
            x, y = at(rt, mz); pg.mouse.move(x + dx, y + dy)
            wait_for_js(pg, f"(()=>{{const s={M}._mz,h=s.hov;return !!h&&Math.abs(h.rt-{rt})<0.5&&!!s.shown&&s.shown.sp&&s.shown.xi&&s.shown.pt===h}})()", 20000)
        # the peaks of the spectrum at the cursor RT, strongest first
        def peaks():
            hover(14.38, 364.3)
            return pg.evaluate(f"(()=>{{const s={M}._mz,a={M}._a,sp=s.spec;const o=[];for(let i=0;i<sp.mz.length;i++)if(sp.mz[i]>=a.y0&&sp.mz[i]<=a.y1)o.push([sp.mz[i],sp.y[i]]);return o.sort((u,v)=>v[1]-u[1]).slice(0,6)}})()")
        def label_of_cursor_peak():
            top = peaks(); assert len(top) >= 3, top; seen = 0
            for mz, _ in top:                                                   # the strongest bars reach the right edge: the label cannot sit beyond the tip
                hover(14.38, mz)
                L = pg.evaluate(f"({M}._mz.lbls||[]).map(l=>({{...l}}))"); W = pg.evaluate(f"{M}._mz.sp.getBoundingClientRect().width"); gd = pg.evaluate(f"{M}._mz.gd")
                cur = [l for l in L if l.get("cur")]; assert len(cur) == 1, (mz, L)
                c = cur[0]; assert abs(float(c["t"]) - mz) < 0.06, (c["t"], mz)
                assert c["x"] >= 0 and c["x"] + c["w"] <= W, ("clipped at the edge", c, W)
                if gd: assert not (c["x"] < gd["end"] + 1 and c["y"] - 2 < gd["y"] < c["y"] + c["h"] + 2), ("under the guide", c, gd)
                for o in L:
                    if o is not c: assert o["y"] + o["h"] <= c["y"] or c["y"] + c["h"] <= o["y"] or o["x"] + o["w"] <= c["x"] or c["x"] + c["w"] <= o["x"], ("overlap", c, o)
                assert pg.evaluate(f"{M}._mz.curPk") == c["t"], "the chip names the same peak"
                seen += 1
            assert seen == len(top)
        step("the m/z label of the peak under the cursor is always drawn, inside the panel, on nothing else", label_of_cursor_peak)
        def left_label():                                                      # the case of the report: the bar reaches the right edge, the label goes above it
            top = peaks(); lefts = 0
            for mz, _ in top:
                hover(14.38, mz); lefts += sum(1 for l in pg.evaluate(f"{M}._mz.lbls") if l.get("cur") and not l["right"])
            assert lefts >= 1, "no strong peak went to the left: the case is not covered"
        step("a peak whose bar reaches the right edge still gets its label (above the bar)", left_label)
        chip = f"{M}._mz.chip"
        def live():
            hover(14.38, 200.0)
            wait_for_js(pg, f"!{chip}.hidden && !{M}._mz.live.hidden")
            t = pg.evaluate(f"{chip}.textContent"); assert t.startswith("RT 14.") and " min · m/z 200" in t, t
            assert pg.evaluate(f"getComputedStyle({chip}).pointerEvents") == "none" and pg.evaluate(f"getComputedStyle({M}._mz.live).pointerEvents") == "none"
            assert pg.evaluate(f"{chip}.getAttribute('aria-hidden')") == "true"
            g = pg.evaluate(f"(()=>{{const c={chip}.getBoundingClientRect(),m={M}.cv.getBoundingClientRect(),d={M}._mz.live.lastElementChild.getBoundingClientRect(),x={M}._mz.xc.getBoundingClientRect();return {{c:[c.left,c.top,c.right,c.bottom],m:[m.left,m.top,m.right,m.bottom],d:[(d.left+d.right)/2,(d.top+d.bottom)/2],x:[x.left,x.top,x.right,x.bottom]}}}})()")
            px, py = at(14.38, 200.0)
            assert g["m"][0] <= g["c"][0] and g["c"][2] <= g["m"][2] and g["m"][1] <= g["c"][1] and g["c"][3] <= g["m"][3], ("chip inside the map", g)
            assert not (g["c"][0] <= px <= g["c"][2] and g["c"][1] <= py <= g["c"][3]), ("the chip covers the cursor", g)
            ex = pg.evaluate(f"{M}._mz.xg.X(14.38)") + g["x"][0] + pg.evaluate(f"{M}._mz.xc.clientLeft")
            assert abs(g["d"][0] - ex) < 1.5 and g["x"][1] <= g["d"][1] <= g["x"][3], ("dot on the trace at the cursor RT", g, ex)
        step("a chip with RT and m/z and a dot on the XIC follow the cursor", live)
        def flips():
            a = pg.evaluate(f"(()=>{{const a={M}._a;return [a.x1,a.y0]}})()")
            hover(a[0] - 0.05, a[1] + 2)                                          # the bottom right corner of the map
            wait_for_js(pg, f"!{chip}.hidden")
            g = pg.evaluate(f"(()=>{{const c={chip}.getBoundingClientRect(),m={M}.cv.getBoundingClientRect(),a={M}._a,x=m.left+a.X({a[0] - 0.05}),y=m.top+a.Y({a[1] + 2});return {{c:[c.left,c.top,c.right,c.bottom],m:[m.left,m.top,m.right,m.bottom],x,y}}}})()")
            assert g["c"][2] <= g["m"][2] and g["c"][3] <= g["m"][3], ("flipped inside the map", g)
            assert g["c"][2] <= g["x"] or g["c"][0] >= g["x"], ("beside the cursor, not on it", g)
        step("the chip flips to the other side near the edges", flips)
        def keys():
            pg.mouse.move(5, 5); wait_for_js(pg, f"{chip}.hidden")                  # the pointer is away: nothing shown
            pg.evaluate(M + ".cv.focus()"); t0 = None
            for k in ("ArrowRight", "ArrowRight", "ArrowUp"):
                pg.keyboard.press(k)
            wait_for_js(pg, f"!{chip}.hidden && /RT [0-9.]+ min/.test({chip}.textContent)")
            t0 = pg.evaluate(f"{chip}.textContent"); pg.keyboard.press("ArrowRight"); pg.keyboard.press("ArrowRight")
            wait_for_js(pg, f"{chip}.textContent !== {t0!r}")
        step("the arrow keys move the chip and the dot too", keys)
        def leaves():
            hover(14.38, 200.0); pg.mouse.move(5, 5)
            wait_for_js(pg, f"{chip}.hidden && {M}._mz.live.hidden")
        step("the chip and the dot go away with the pointer", leaves)
        def segs():
            pg.evaluate(M + ".cv.focus()")
            for theme in ("light", "dark"):
                pg.evaluate(f"document.documentElement.dataset.theme='{theme}'")
                pg.wait_for_function("document.fonts.status==='loaded'")
                g = pg.evaluate(f"""(()=>{{const e={M}.el,bs=[...e.querySelectorAll('.mzbt button')],d=e.querySelector('[data-o=view][data-v="3d"]'),sg=e.querySelector('.mzbt');
                  for(const b of bs)if(b.dataset.mz!=='mark'){{ if(!b.classList.contains('on'))b.click(); }}
                  const rc=x=>x.getBoundingClientRect(),cs=x=>getComputedStyle(x);
                  return {{bs:bs.map(b=>({{on:b.classList.contains('on'),top:rc(b).top,bot:rc(b).bottom,l:rc(b).left,r:rc(b).right,bg:cs(b).backgroundColor,fg:cs(b).color}})),seg:{{top:rc(sg).top,bot:rc(sg).bottom}},d:{{top:rc(d).top,bot:rc(d).bottom}}}}}})()""")
                b = g["bs"]; assert [x["on"] for x in b][1:] == [True, True], (theme, b)
                assert len({round(x["top"], 2) for x in b}) == 1 and len({round(x["bot"], 2) for x in b}) == 1, (theme, "one baseline", b)
                for u, v in zip(b, b[1:]): assert abs(u["r"] - v["l"]) < 0.01, (theme, "no seam and no gap between neighbours", u, v)
                assert b[1]["bg"] == b[2]["bg"], (theme, "one fill", b)
                assert abs((g["seg"]["bot"] - g["seg"]["top"]) - (g["d"]["bot"] - g["d"]["top"])) < 0.6, (theme, "same height as 3D", g)
                assert abs(g["seg"]["top"] - g["d"]["top"]) < 0.6, (theme, "same top as 3D", g)
        step("mark | snap | sight: one baseline, one fill, the height of the 3D button, in both themes", segs)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
