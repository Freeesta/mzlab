"""A baseline handle never goes below the zero line (integra-base.js, mzlab_core::baseline `at`): dragged under the axis with the mouse or pushed down with the
arrow keys it stops at zero, typed or restored from a session below zero it is put back on it, and the area, the height and the drawn fill are the engine's
numbers for the held line, never counting a part under zero."""
# covers: mzlab/web/integra-base.js
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_zero.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "integra_zero_tmp"; D.mkdir(parents=True, exist_ok=True)
OUT = Path(os.environ.get("ZERO_SHOTS", str(D))); OUT.mkdir(parents=True, exist_ok=True)
LRF = D / "B_FullMass-t30.mzML"
ds.full_scan(LRF, 30, np.random.default_rng(2026))
PN = "E.panels.find(q=>q.type==='xic')"
IT = PN + ".ints[0]"

def xic(pg, mz):
    pg.evaluate("openXic(null)"); pg.wait_for_selector("#xic-mz"); pg.fill("#xic-mz", mz); pg.select_option("#xic-file", index=1); pg.click("#xic-go"); ready(pg)
    wait_for_js(pg, f"(()=>{{const p={PN};return !!(p&&p._a&&p._a.sr&&p._a.sr.length)}})()")

def engine(pg):
    """the engine's measure for the record's own two points (what the page must show)"""
    return json.loads(pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i={IT};return QUALITA.peakArea(s.x,s.y,i.a,i.b,2,i.bl)}})()"))

def settle(pg):
    e = engine(pg)
    wait_for_js(pg, f"{IT}.area==={e['area']!r} && {IT}.below==={e['below']!r}")
    return e

def line_record(pg):
    """one integration of the peak at 14.3 min with a custom line (mode «line», through the dialog as the user does)"""
    pg.evaluate(f"(()=>{{const p={PN};p.ints=[];autoInt(p,p._a.sr[0],14.3);draw(p)}})()")
    wait_for_js(pg, f"{PN}.ints.length===1 && Number.isFinite({IT}.area)")
    pg.evaluate(f"(()=>{{const p={PN};p.isel={IT}.id;setActive(p);p.cv.focus()}})()")
    pg.keyboard.press("b"); pg.wait_for_selector("#bigdlg[open] input[name=ib-m]")
    pg.check("#bigdlg input[name=ib-m][value=line]"); pg.click("#ib-ok"); wait_for_js(pg, "!document.querySelector('#bigdlg').open")
    wait_for_js(pg, f"{IT}.bm==='line'"); settle(pg)

def point(pg, k):
    b = pg.locator(f".ibp[data-k='{k}']").bounding_box(); return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2, b

def axis_y(pg):
    """the page y of the zero line of the trace shown"""
    return pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],r=p.cv.getBoundingClientRect();return r.top+p._a.Y(p._a.U(s,0))}})()")

r = Run(port=8997, wd="/tmp/wd_integra_zero")
try:
    with sync_playwright() as p:
        pg = r.page(p, device_scale_factor=2)
        load(pg, [LRF], 4000, 1)
        xic(pg, "194")
        line_record(pg)
        held = {}
        def mouse():
            x, y, b = point(pg, 0)
            pg.evaluate(f"(()=>{{const i={IT};i.bl[1]=0;draw({PN})}})()"); wait_for_js(pg, f"{IT}.bl[1]===0"); held["area"] = settle(pg)["area"]     # the line on the axis: the number a handle held at zero must give
            x, y, b = point(pg, 0)
            pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(x, y - 40, steps=4); pg.mouse.move(x, axis_y(pg) + 120, steps=8)
            pg.screenshot(path=str(OUT / "zero-dragging.png"))
            pg.mouse.up()
            y0 = pg.evaluate(f"{IT}.bl[1]"); assert y0 == 0, f"the handle went to {y0} counts, below the zero line"
            hx, hy, _ = point(pg, 0); assert abs(hy - axis_y(pg)) < 1.5, (hy, axis_y(pg))      # the handle sits on the axis, not under it
            e = settle(pg); assert e["area"] == held["area"] and e["below"] >= 0, (e, held)  # the area is the one of the line at zero
            assert pg.evaluate(f"{IT}.clamped||0") == 0, "a handle at zero is not a clamped line"
        step("mouse: a handle dragged under the axis stops at zero and the area does not change", mouse)
        def keys():
            pg.locator(".ibp[data-k='0']").focus()
            for _ in range(6): pg.keyboard.press("Shift+ArrowDown")
            assert pg.evaluate(f"{IT}.bl[1]") == 0 and pg.evaluate(f"{IT}.bl[3]") >= 0
            assert settle(pg)["area"] == held["area"]
        step("keyboard: the down arrow stops at zero", keys)
        def typed():
            pg.evaluate(f"(()=>{{const p={PN};p.isel={IT}.id;setActive(p);p.cv.focus()}})()")
            pg.keyboard.press("b"); pg.wait_for_selector("#bigdlg[open] input[name=ib-m]")
            pg.fill("[data-ib='1']", "-50000"); pg.click("#ib-ok"); wait_for_js(pg, "!document.querySelector('#bigdlg').open")
            assert pg.evaluate(f"{IT}.bl[1]") == 0, pg.evaluate(f"{IT}.bl")
            msg = pg.evaluate("I18N.t('int.base.floor')"); wait_for_js(pg, f"[...document.querySelectorAll('body *')].some(e=>e.children.length===0&&e.textContent==={msg!r}&&e.offsetParent!==null)", 4000)
            assert settle(pg)["area"] == held["area"]
        step("typed: a point below zero in the dialog is put on zero and the page says so", typed)
        def restored():
            pg.evaluate(f"(()=>{{const i={IT};i.bl[1]=-30000;i.bl[3]=-1000;draw({PN})}})()")
            wait_for_js(pg, f"{IT}.bl[1]===0 && {IT}.bl[3]===0")                    # a session saved with a point under zero
            e = settle(pg); assert e["area"] > 0 and "clamped" not in e or e.get("clamped", 0) == 0, e
        step("restored: a session record with points under zero is put back on zero", restored)
        def engine_clamp():
            # the engine alone, for a line that is handed to it under zero (CLI and exports): held at zero, `clamped` says how much it took out
            e = json.loads(pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i={IT};return QUALITA.peakArea(s.x,s.y,i.a,i.b,2,[i.bl[0],-30000,i.bl[2],5000])}})()"))
            assert e["clamped"] > 0 and e["area"] > 0 and e["below"] >= 0, e
        step("the engine holds a line handed to it under zero at zero", engine_clamp)
        errs = [e for e in r.errs if e[0] == "pageerror"]; assert not errs, errs
        pg.locator(".pnl.xic").first.screenshot(path=str(OUT / "zero-after.png"))
finally:
    r.close()
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
