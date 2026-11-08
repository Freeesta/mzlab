"""The baseline of a Data-view integration (integra-base.js, the numbers from mzlab_core::baseline through QUALITA.peakArea): the three modes on the same peak
(the line between the edges, from zero, a custom line), the custom line moved by mouse, by keyboard and by finger on a 1024x768 touch viewport, in the light and
the dark theme; the part under the line is reported apart and drawn hatched; the default of the peak dialog; a record without a mode keeps its old number and fields."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_modi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "integra_modi_tmp"; D.mkdir(parents=True, exist_ok=True)
LRF = D / "B_FullMass-t30.mzML"
ds.full_scan(LRF, 30, np.random.default_rng(2026))
IB = {"zero": 1, "line": 2}
PN = "E.panels.find(q=>q.type==='xic')"
IT = PN + ".ints[0]"

def xic(pg, mz):
    pg.evaluate("openXic(null)"); pg.wait_for_selector("#xic-mz"); pg.fill("#xic-mz", mz); pg.select_option("#xic-file", index=1); pg.click("#xic-go"); ready(pg)
    wait_for_js(pg, f"(()=>{{const p={PN};return !!(p&&p._a&&p._a.sr&&p._a.sr.length)}})()")

def integrate(pg, rt):
    pg.evaluate(f"(()=>{{const p={PN};p.ints=[];autoInt(p,p._a.sr[0],{rt});draw(p)}})()")
    wait_for_js(pg, f"{PN}.ints.length===1 && Number.isFinite({IT}.area)")

def mode(pg, m, level=False):
    """the dialog of the integration through its key (B), the mode chosen and applied; waits for the engine's number"""
    pg.evaluate(f"(()=>{{const p={PN};p.isel={IT}.id;setActive(p);p.cv.focus()}})()")
    pg.evaluate(f"(()=>{{const p={PN},r=p.cv.getBoundingClientRect(),x=r.left+p._a.X({IT}.rt);p.cur={IT}.rt}})()")
    pg.keyboard.press("b"); pg.wait_for_selector("#bigdlg[open] input[name=ib-m]")
    pg.check(f"#bigdlg input[name=ib-m][value={m}]")
    if level: pg.check("#ib-lvl")
    pg.click("#ib-ok"); wait_for_js(pg, "!document.querySelector('#bigdlg').open")
    wait_for_js(pg, f"({IT}.bm||'edges')===%r" % m)
    if m in IB: settle(pg, m)
    return pg.evaluate(f"(()=>{{const i={IT};return {{area:i.area,below:i.below,height:i.height,rt:i.rt,bm:i.bm,bl:i.bl}}}})()")

def engine(pg, m, line=None):
    return json.loads(pg.evaluate(f"(([m,l])=>{{const p={PN},s=p._a.sr[0],i={IT};return QUALITA.peakArea(s.x,s.y,i.a,i.b,m,l)}})", [m, line or []]))

def settle(pg, m):
    """until the engine answers the panel shows the previous measure: wait for the engine's number for the record's current line"""
    e = engine(pg, IB[m], pg.evaluate(f"{IT}.bl||[]"))
    wait_for_js(pg, f"{IT}.area==={e['area']!r} && {IT}.below==={e['below']!r}")
    return e

def point(pg, k):
    b = pg.locator(f".ibp[data-k='{k}']").bounding_box(); return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2, b

r = Run(port=8984, wd="/tmp/wd_integra_modi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [LRF], 4000, 1)
        xic(pg, "364")
        integrate(pg, 14.3)
        old = pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i={IT};return {{area:i.area,integ:integ(s,i.a,i.b).area,keys:Object.keys(JSON.parse(JSON.stringify(i))).sort()}}}})()")
        res = {}
        def unchanged():
            assert old["area"] == old["integ"], old                                  # the line between the edges: the JS number of always
            assert not {"bm", "bl", "below", "lvl"} & set(old["keys"]), old["keys"]  # a record without a mode saves the fields it had
        step("a record without a mode keeps its area and its fields", unchanged)
        def three():
            res["edges"] = old["area"]
            z = mode(pg, "zero"); e = engine(pg, 1)
            assert z["bm"] == "zero" and z["area"] == e["area"] and z["below"] == 0 and z["area"] > res["edges"], (z, e, res)
            res["zero"] = z["area"]
            l = mode(pg, "line"); e = engine(pg, 2, l["bl"])
            assert l["bm"] == "line" and len(l["bl"]) == 4 and l["area"] == e["area"], (l, e)
            assert abs(l["area"] - res["edges"]) <= 1e-9 * res["edges"], (l, res)  # the line starts on the edge line: the same area as the edges
            res["line"] = l["area"]
            tab = pg.evaluate("(async()=>{await showInts();return document.querySelector('#bigbody table').innerText})()")
            assert "Linea personalizzata" in tab or "Custom line" in tab, tab
            pg.evaluate("Q('#bigdlg').close()")
        step("the three modes on the same peak: edges, zero (more area), a line on the edges (the same area)", three)
        def mouse():
            x, y, b = point(pg, 0)
            assert b["width"] >= 24 and b["height"] >= 24, b
            before = pg.evaluate(f"{IT}.bl.slice()")
            pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(x, y - 60, steps=6); pg.mouse.up()
            wait_for_js(pg, f"{IT}.bl[1]>{before[1]}")
            settle(pg, "line"); assert pg.evaluate(f"{IT}.area") < res["line"]
            v = pg.evaluate(f"(()=>{{const i={IT};return {{a:i.area,b:i.below,bl:i.bl}}}})()"); e = engine(pg, 2, v["bl"])
            assert v["a"] == e["area"] and v["b"] == e["below"], (v, e)
        step("mouse: dragging point 1 up lowers the area, the engine's number", mouse)
        def keys():
            pg.locator(".ibp[data-k='1']").focus(); y0 = pg.evaluate(f"{IT}.bl[3]"); t0 = pg.evaluate(f"{IT}.bl[2]")
            for _ in range(8): pg.keyboard.press("Shift+ArrowUp")
            pg.keyboard.press("ArrowLeft")
            wait_for_js(pg, f"{IT}.bl[3]>{y0} && {IT}.bl[2]<{t0}")
            settle(pg, "line"); assert pg.evaluate(f"{IT}.below") > 0               # the line now cuts through the peak
            v = pg.evaluate(f"(()=>{{const i={IT};return {{a:i.area,b:i.below,bl:i.bl}}}})()"); e = engine(pg, 2, v["bl"])
            assert v["a"] == e["area"] and v["b"] == e["below"] > 0, (v, e)
            assert pg.locator(".ibp[data-k='1']").get_attribute("aria-label"), "no accessible name"
            pg.evaluate("STORIA.annulla()"); wait_for_js(pg, f"{IT}.bl[2]=={t0}")         # one undo per key press
        step("keyboard: arrows move point 2 (time and intensity), the part under the line is reported, undo", keys)
        def note():
            tab = pg.evaluate("(async()=>{await showInts();return document.querySelector('#bigbody').innerText})()")
            assert "Sotto la linea" in tab or "Below the line" in tab, tab
            pg.evaluate("Q('#bigdlg').close()")
        step("the table carries the baseline, the part below and the note", note)
        def themes():
            for th in ("light", "dark"):
                pg.evaluate("t => document.documentElement.setAttribute('data-theme', t)", th); pg.evaluate(f"draw({PN})"); wait_for_js(pg, f"document.documentElement.dataset.theme===%r && Number.isFinite({IT}.area)" % th)
                pg.locator(".pnl.xic").first.screenshot(path=str(D / f"modi-{th}.png"))
                hatched = pg.evaluate(f"""(()=>{{const p={PN},i={IT},s=p._a.sr[0],g=p.cv.getContext('2d'),d=devicePixelRatio,X=p._a.X,x=X(i.bl[2]-0.02);
                  const y0=p._a.Y(p._a.U(s,INTBASE.lineAt(i,i.bl[2]-0.02))),px=g.getImageData(Math.round(x*d)-4,Math.round(y0*d)+2,8,8).data;let n=0;for(let k=3;k<px.length;k+=4)if(px[k]>0)n++;return n}})()""")
                assert hatched > 0, (th, hatched)
            pg.evaluate("document.documentElement.setAttribute('data-theme', 'light')")
        step("light and dark theme: the chart draws the line and the hatching", themes)
        def level():
            l = mode(pg, "line", level=True)
            assert l["bl"][1] == l["bl"][3], l
        step("a level: both points at the same intensity", level)
        def default():
            pg.evaluate("PK.base=1"); integrate(pg, 14.3)
            assert pg.evaluate(f"{IT}.bm") == "zero" and abs(pg.evaluate(f"{IT}.area") - res["zero"]) <= 1e-9 * res["zero"]
            pg.evaluate("Object.assign(PK, PK_DEF)")
        step("the default of the peak dialog applies to a new integration", default)
        miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).length"); assert miss == 0, miss
        errs = [e for e in r.errs if e[0] == "pageerror"]; assert not errs, errs
        # finger and pen: a 1024x768 touch viewport, the same records
        def touch():
            tp = r.b.new_page(viewport={"width": 1024, "height": 768}, has_touch=True, reduced_motion="reduce")
            tp.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            tp.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(tp, "typeof E==='object' && !!document.querySelector('#pick')")
            load(tp, [LRF], 4000, 1); xic(tp, "364"); integrate(tp, 14.3); mode(tp, "line")
            tp.locator(".ibp[data-k='0']").scroll_into_view_if_needed()
            x, y, b = point(tp, 0); assert b["width"] >= 24, b
            y1 = tp.evaluate(f"{IT}.bl[1]"); cdp = tp.context.new_cdp_session(tp)
            ev = lambda t, yy: cdp.send("Input.dispatchTouchEvent", {"type": t, "touchPoints": [] if t == "touchEnd" else [{"x": x, "y": yy}]})
            ev("touchStart", y); [ev("touchMove", y - 8 * k) for k in range(1, 6)]; ev("touchEnd", y - 40)
            wait_for_js(tp, f"{IT}.bl[1]>{y1}"); settle(tp, "line")
            v = tp.evaluate(f"(()=>{{const i={IT};return {{a:i.area,bl:i.bl}}}})()"); e = engine(tp, 2, v["bl"])
            assert v["a"] == e["area"], (v, e)
            tp.screenshot(path=str(D / "modi-touch.png")); tp.close()
        step("touch 1024x768: a finger drags point 1", touch)
finally:
    r.close()
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
