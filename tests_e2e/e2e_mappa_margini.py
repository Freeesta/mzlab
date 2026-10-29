"""Margin plots of the 2D map (sight spectrum `sp`, XIC `xc`) with MOTION ON (the panel grows by a FLIP scale animation, transizioni.js): at pixel ratio 1, 1.25 and 2 the bitmap of each
canvas is round(layout box x ratio) when the sight is turned on, after it moves onto a narrow peak, after a window resize and after the panel goes full screen and back; the XIC of a flat
baseline + narrow peak file draws its baseline just above the axis. Screenshots of every state go to tests_e2e/shots/margini_*.png (ignored by git)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from sintesi_margini import margini_mzml
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
M = "E.panels.find(q=>q.type==='map')"
SHOTS = Path(get_env("SHOTS") or ROOT / "tests_e2e" / "shots"); SHOTS.mkdir(exist_ok=True)
F = margini_mzml(Path(tempfile.mkdtemp()) / "B_Margini-t0.mzML")
r = Run(port=8899, wd="/tmp/wd_margini")
def sizes(pg):
    return pg.evaluate("(()=>{const o={dpr:devicePixelRatio};for(const k of['sp','xc']){const c=" + M + "._mz[k];o[k]={w:c.width,h:c.height,cw:c.clientWidth,ch:c.clientHeight}}return o})()")
def settle(pg):
    ready(pg); frames(pg, 3); attendi(pg, "document.getAnimations().length === 0"); frames(pg, 2)               # the FLIP animation of the panel is over
def check(pg, label):
    s = sizes(pg); d = s["dpr"]
    for k in ("sp", "xc"):
        c = s[k]; assert c["cw"] > 0 and c["ch"] > 0, (label, k, c)
        assert abs(c["w"] - round(c["cw"] * d)) <= 1 and abs(c["h"] - round(c["ch"] * d)) <= 1, (label, k, "bitmap != layout box x dpr", c, d)
        rect = pg.evaluate("(()=>{const c=" + M + "._mz['" + k + "'].getBoundingClientRect();return [c.width,c.height]})()")      # at rest the rect is the layout box (+ the 1 px border)
        assert abs(rect[0] - c["cw"]) <= 2 and abs(rect[1] - c["ch"]) <= 2, (label, k, rect, c)
def baseline(pg):
    # flat baseline ion (2e4) with a narrow peak far away: in a column away from the peak the XIC fills a strip just above the axis
    got = pg.evaluate("""(()=>{const p=""" + M + """,c=p._mz.xc,d=c.width/c.clientWidth,g=c.getContext('2d'),W=c.width,H=c.height,mb=Math.round(22*d);
      let hit=0,cols=0;for(let x=Math.round(W*.25);x<W*.6;x+=Math.round(W/40)){cols++;const px=g.getImageData(x,H-mb-Math.round(5*d),1,Math.round(4*d)).data;let a=0;for(let i=3;i<px.length;i+=4)a=Math.max(a,px[i]);if(a>0)hit++}return [hit,cols]})()""")
    assert got[0] >= got[1] * .8, ("no baseline just above the XIC axis", got)
def toPeak(pg):
    pg.evaluate(M + ".cv.focus()")
    cal = []
    for dx in (150, 650):
        b = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        pg.mouse.move(b[0] + dx, b[1] + b[3] / 2); frames(pg, 2); cal.append((b[0] + dx, pg.evaluate(f"MAPPA.point({M}).rt")))
    px = cal[0][0] + (14.4 - cal[0][1]) * (cal[1][0] - cal[0][0]) / (cal[1][1] - cal[0][1])
    py = pg.evaluate(f"(()=>{{const p={M},a=p._a,c=p.cv.getBoundingClientRect();let lo=0,hi=a.H;for(let i=0;i<40;i++){{const m=(lo+hi)/2;if(a.mzAt(m)>300.1)lo=m;else hi=m}}return c.top+(lo+hi)/2}})()")
    pg.mouse.move(px - 30, py); pg.mouse.move(px, py); ready(pg); attendi(pg, "(s => !!s && !!s.sp && !!s.xi)(" + M + "._mz.shown)")
    for _ in range(8):                                                           # a pixel is about one m/z: the arrows (one bin) bring the XIC window (centre n + 0.3) onto the ion at 300.3
        c = pg.evaluate(f"{M}._mz.xic.c")
        if abs(c - 300.3) < .01: break
        pg.keyboard.press("ArrowUp" if c < 300.3 else "ArrowDown"); ready(pg); attendi(pg, "(s => !!s && !!s.sp && !!s.xi)(" + M + "._mz.shown)")
    assert abs(pg.evaluate(f"{M}._mz.xic.c") - 300.3) < .01 and abs(pg.evaluate(f"MAPPA.point({M}).rt") - 14.4) < .3, (pg.evaluate(f"MAPPA.point({M})"), pg.evaluate(f"{M}._mz.xic.c"))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        for dpr in (1, 1.25, 2):
            pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}, device_scale_factor=dpr).new_page()          # motion ON: the FLIP animation is what broke the bitmaps
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "typeof E !== 'undefined'")
            pg.set_input_files("#pick", [str(F)]); attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1")
            pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, M + "._a")
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); ready(pg)
            def shot(tag):
                for k in ("sp", "xc"): pg.evaluate_handle(f"{M}._mz.{k}").as_element().screenshot(path=str(SHOTS / f"margini_{dpr}_{tag}_{k}.png"))
            def on():
                pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("m"); settle(pg); check(pg, f"dpr {dpr}: sight on"); shot("open")
            step(f"dpr {dpr}: sight on (panel grows with the FLIP animation): bitmaps = layout box x dpr", on)
            def peak():
                toPeak(pg); settle(pg); check(pg, f"dpr {dpr}: on the peak"); shot("peak"); baseline(pg)
            step(f"dpr {dpr}: sight on the narrow peak: bitmaps right, XIC baseline drawn above the axis", peak)
            def resize():
                pg.set_viewport_size({"width": 1180, "height": 850}); settle(pg); check(pg, f"dpr {dpr}: resized"); shot("resize")
                pg.set_viewport_size({"width": 1440, "height": 950}); settle(pg); check(pg, f"dpr {dpr}: resized back")
            step(f"dpr {dpr}: window resized and back", resize)
            def expand():
                pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); attendi(pg, M + ".el.classList.contains('max')"); settle(pg); check(pg, f"dpr {dpr}: full screen"); shot("max")
                pg.keyboard.press("Escape"); attendi(pg, "!" + M + ".el.classList.contains('max')"); ready(pg)
                pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("m"); settle(pg); check(pg, f"dpr {dpr}: back in the page, sight on again")      # the sight follows the view: off in the page
            step(f"dpr {dpr}: panel full screen and back", expand)
            pg.context.close()
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
sys.exit(1 if any(not s[1].startswith("ok") for s in steps) or r.errs else 0)
