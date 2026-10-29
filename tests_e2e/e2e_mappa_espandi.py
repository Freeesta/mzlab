"""The expand button of the 2D map is the shared in-page one: no browser full screen, same box as the other panels, canvases follow the box, collapse restores it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
C = "E.panels.find(q=>q.type==='chrom')"
steps = []
r = Run(port=8903, wd="/tmp/wd_mapespandi_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "typeof E !== 'undefined'")
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 2")
        pg.click("#opbtn"); ready(pg); pg.click("#np-chrom"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, M + "._a && " + C + ".el")
        box = lambda P: pg.evaluate(f"(()=>{{const r={P}.el.getBoundingClientRect();return [r.left+scrollX,r.top+scrollY,r.width,r.height].map(Math.round)}})()")
        expand = lambda P: (pg.click(f".pnl.{P} [data-a=max]"), attendi(pg, f"document.querySelector('.pnl.{P}').classList.contains('max')"), ready(pg))
        collapse = lambda P: (pg.click(f".pnl.{P} [data-a=max]"), attendi(pg, f"!document.querySelector('.pnl.{P}').classList.contains('max')"), ready(pg))
        vbox = lambda P: pg.evaluate(f"(()=>{{const r={P}.el.getBoundingClientRect();return [r.left,r.top,r.width,r.height].map(Math.round)}})()")        # viewport box: an expanded panel is fixed
        ref = {}
        def other_panel():
            before = box(C); expand("chrom"); ref["box"] = vbox(C); collapse("chrom"); assert box(C) == before, (box(C), before)
            assert ref["box"][2] > 1000 and ref["box"][3] > 600, ref
        step("reference: the box of an expanded chromatogram panel", other_panel)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        before = {}
        def map_expand():
            before["box"] = box(M); expand("map")
            assert pg.evaluate("document.fullscreenElement") is None, "browser full screen entered"
            assert vbox(M) == ref["box"], (vbox(M), ref["box"])
            assert pg.evaluate("document.activeElement===" + M + ".cv"), "map canvas not focused"
        step("map expand: in-page, no fullscreenElement, same box as other panels", map_expand)
        def canvases():
            frames(pg, 4); ready(pg)
            bad = pg.evaluate(f"""(()=>{{const s={M}._mz,o=[];for(const [n,c] of [['map',{M}.cv],['sp',s.sp],['xc',s.xc]]){{if(c.hidden||!c.offsetWidth)continue;const r=c.getBoundingClientRect(),d=window.devicePixelRatio||1;
              if(Math.abs(c.width-r.width*d)>2||Math.abs(c.height-r.height*d)>2)o.push([n,c.width,c.height,r.width,r.height])}}return o}})()""")
            assert not bad, bad
            assert pg.evaluate(f"!{M}._mz.sp.hidden && !{M}._mz.xc.hidden"), "sight margin plots not shown when expanded"
        step("canvases: bitmap size matches the CSS box (map, spectrum, XIC)", canvases)
        def esc_works():
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('.pnl.map').classList.contains('max')"); ready(pg)
            assert box(M) == before["box"], (box(M), before["box"])
            assert pg.evaluate("document.fullscreenElement") is None
        step("Esc collapses; the previous box is restored", esc_works)
        def toggle_f():
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); attendi(pg, M + ".el.classList.contains('max')"); ready(pg)
            assert pg.evaluate("document.fullscreenElement") is None and vbox(M) == ref["box"]
            collapse("map"); assert box(M) == before["box"], (box(M), before["box"])
        step("F key and the button toggle the same way", toggle_f)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
