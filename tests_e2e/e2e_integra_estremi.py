"""The two ends of an integrated area on its baseline (integra-estremi.js, the point from mzlab_core::intwin through QUALITA.windowEnd) as handles:
24 px sliders with a name, on the baseline; the keyboard moves an end one point (Shift five), the mouse and a finger drag it onto a point of the trace,
never past the other end; the area follows from the engine; the live text and the data table carry the new ends; the page and `mzlab-cli peak-window`
give the same bytes; the default baseline stays zero; light and dark theme on a 1024x768 touch viewport. Synthetic low-resolution file only."""
import sys, os, subprocess, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_estremi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "integra_estremi_tmp"; D.mkdir(parents=True, exist_ok=True)
LRF = D / "B_FullMass-t30.mzML"
ds.full_scan(LRF, 30, np.random.default_rng(2026))
CLI = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]
PN = "E.panels.find(q=>q.type==='xic')"
IT = PN + ".ints[0]"
SKIP = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"

def xic(pg, mz):
    pg.evaluate("openXic(null)"); pg.wait_for_selector("#xic-mz"); pg.fill("#xic-mz", mz); pg.select_option("#xic-file", index=1); pg.click("#xic-go"); ready(pg)
    wait_for_js(pg, f"(()=>{{const p={PN};return !!(p&&p._a&&p._a.sr&&p._a.sr.length)}})()")

def integrate(pg, rt):
    pg.evaluate(f"(()=>{{const p={PN};p.ints=[];autoInt(p,p._a.sr[0],{rt});draw(p)}})()")
    wait_for_js(pg, f"{PN}.ints.length===1 && Number.isFinite({IT}.area)")
    wait_for_js(pg, "document.querySelectorAll('.iep:not([hidden])').length===2")
    settle(pg)

def rec(pg):
    return pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i={IT};return {{a:Math.min(i.a,i.b),b:Math.max(i.a,i.b),ea:i.a<=i.b?'a':'b',area:i.area,bm:i.bm,ia:nearIdx(s.x,Math.min(i.a,i.b)),ib:nearIdx(s.x,Math.max(i.a,i.b)),x:Array.from(s.x)}}}})()")

def settle(pg):
    """the engine's area of the record's current window, from zero"""
    e = json.loads(pg.evaluate(f"(()=>{{const s={PN}._a.sr[0],i={IT};return QUALITA.peakArea(s.x,s.y,i.a,i.b,1,[])}})()"))
    wait_for_js(pg, f"{IT}.area==={e['area']!r}")
    return e

def handle(pg, k):
    b = pg.locator(f".iep[data-e='{k}']").bounding_box(); return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2, b

def page_end(pg, cur, other, at, st):
    return pg.evaluate("([c,o,a,st])=>{const s=E.panels.find(q=>q.type==='xic')._a.sr[0];return QUALITA.windowEnd(s.x,c,o,a,st,null,null)}", [cur, other, at, st])

def cli_end(csv, cur, other, at, st):
    move = ["--at", repr(at)] if at is not None else ["--step", str(st)]
    return subprocess.run(CLI + ["peak-window", str(csv), "--end", repr(cur), "--other", repr(other), *move], cwd=ROOT, check=True, capture_output=True, text=True).stdout

r = Run(port=8987, wd="/tmp/wd_integra_estremi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [LRF], 4000, 1)
        xic(pg, "364")
        integrate(pg, 14.3)
        tr = pg.evaluate(f"(()=>{{const s={PN}._a.sr[0];return {{x:Array.from(s.x),y:Array.from(s.y)}}}})()")
        csv = D / "trace.csv"; csv.write_text("".join(f"{a!r},{b!r}\n" for a, b in zip(tr["x"], tr["y"])))
        def on_baseline():
            v = rec(pg); assert v["bm"] == "zero", v                             # the default baseline stays zero
            for k, t in (("0", v["a"]), ("1", v["b"])):
                x, y, b = handle(pg, k)
                assert b["width"] >= 24 and b["height"] >= 24, b
                loc = pg.locator(f".iep[data-e='{k}']")
                assert loc.get_attribute("role") == "slider" and f"{t:.3f}" in (loc.get_attribute("aria-label") or ""), loc.get_attribute("aria-label")
                want = pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],r=p.cv.getBoundingClientRect();return [r.left+p._a.X({t}),r.top+p._a.Y(p._a.U(s,0))]}})()")
                # over its point; the two ends of a window narrower than a handle step apart, never on each other
                assert b["x"] - 1 <= want[0] <= b["x"] + b["width"] + 1 and abs(y - want[1]) <= 1, (k, b, want)
            b0, b1 = handle(pg, "0")[2], handle(pg, "1")[2]
            assert b0["x"] + b0["width"] <= b1["x"] + 0.5, (b0, b1)
            assert pg.evaluate("getComputedStyle(document.querySelector('.iep')).transitionDuration") in ("0s", "0s, 0s"), "reduced motion: no transition"
        step("two named 24 px sliders sit where the area meets the zero line, side by side", on_baseline)
        def keys():
            v = rec(pg); loc = pg.locator(".iep[data-e='0']"); loc.focus()
            want = page_end(pg, v["a"], v["b"], None, -1)
            assert want + "\n" == cli_end(csv, v["a"], v["b"], None, -1), want
            pg.keyboard.press("ArrowLeft"); wait_for_js(pg, f"Math.min({IT}.a,{IT}.b)==={v['x'][v['ia'] - 1]!r}")
            assert json.loads(want)["t"] == v["x"][v["ia"] - 1]
            pg.keyboard.press("Shift+ArrowLeft"); wait_for_js(pg, f"Math.min({IT}.a,{IT}.b)==={v['x'][v['ia'] - 6]!r}")
            settle(pg); w = rec(pg); assert w["area"] > v["area"], (w["area"], v["area"])
            live = pg.evaluate('([rt,a])=>I18N.t("int.end.liveA",{rt:rt.toFixed(3),area:I18N.sciE(a)})', [w['a'], w['area']])
            try: wait_for_js(pg, f"(document.querySelector('.iep-live')||{{}}).textContent==={live!r}", 5000)
            except Exception: raise AssertionError((live, pg.evaluate(f"(()=>{{const p={PN},i={IT};return [document.querySelector('.iep-live').textContent,i.area,INTBASE.area(p,p._a.sr[0],i),p._iepSay?1:0,document.querySelectorAll('.iep-live').length]}})()")))
            assert pg.evaluate("document.querySelector('.iep-live').getAttribute('aria-live')") == "polite"
            assert f"{w['a']:.3f}" in loc.get_attribute("aria-label") and pg.evaluate("document.activeElement.classList.contains('iep')")
            pg.evaluate("STORIA.annulla()"); wait_for_js(pg, f"Math.min({IT}.a,{IT}.b)==={v['x'][v['ia'] - 1]!r}")   # one undo per key press
        step("keyboard: one point, Shift five, the area and the live text follow, the same bytes as mzlab-cli, undo", keys)
        def mouse():
            v = rec(pg); x, y, _ = handle(pg, "1")
            px = x + 70; at = pg.evaluate(f"(([px])=>{{const p={PN};return xOf(p,px-p.cv.getBoundingClientRect().left)}})", [px])
            want = page_end(pg, v["b"], v["a"], at, 0)
            assert want + "\n" == cli_end(csv, v["b"], v["a"], at, 0), want
            pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(px, y, steps=7); pg.mouse.up()
            t = json.loads(want)["t"]; wait_for_js(pg, f"Math.max({IT}.a,{IT}.b)==={t!r}")
            assert t in v["x"], "not a point of the trace"
            settle(pg)
            x, y, _ = handle(pg, "1"); a0 = handle(pg, "0")[0]
            pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(a0 - 200, y, steps=8); pg.mouse.up()
            wait_for_js(pg, f"Math.max({IT}.a,{IT}.b)==={v['x'][rec(pg)['ia'] + 1]!r}")            # it stops one point after the start
            settle(pg)
        step("mouse: the end snaps to a point of the trace, never past the start, the area follows", mouse)
        def table():
            v = rec(pg)
            pg.evaluate(f"{PN}.cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            assert pg.evaluate("document.querySelectorAll('#tdati tbody tr').length") > 0
            rows = pg.evaluate(f"TDATI_P.data({PN}).rows")          # every row of the table (the dialog shows the first ones and adds the rest on scroll)
            names = [pg.evaluate("([k,a,b])=>I18N.t(k,{a:TDATI.num(a,2),b:TDATI.num(b,2)})", [k, v["a"], v["b"]]) for k in ("tdati.int.a", "tdati.int.b")]
            for n, t in zip(names, (v["a"], v["b"])):
                got = [r_ for r_ in rows if r_[0].endswith(n)]
                assert got and got[0][1] == pg.evaluate("t=>TDATI.num(t,4)", t), (n, got)
            pg.keyboard.press("Escape")
        step("the data table carries the final start and end", table)
        errs = [e for e in r.errs if e[0] == "pageerror"]; assert not errs, errs
        def touch():
            tp = r.b.new_page(viewport={"width": 1024, "height": 768}, has_touch=True, reduced_motion="reduce"); tp.add_init_script(SKIP)
            tp.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(tp, "typeof E==='object' && !!document.querySelector('#pick')")
            load(tp, [LRF], 4000, 1); xic(tp, "364"); integrate(tp, 14.3)
            tp.locator(".iep[data-e='0']").scroll_into_view_if_needed()
            v = rec(tp); x, y, b = handle(tp, "0"); assert b["width"] >= 24, b
            cdp = tp.context.new_cdp_session(tp)
            ev = lambda t, xx: cdp.send("Input.dispatchTouchEvent", {"type": t, "touchPoints": [] if t == "touchEnd" else [{"x": xx, "y": y}]})
            ev("touchStart", x); [ev("touchMove", x - 8 * k) for k in range(1, 6)]; ev("touchEnd", x - 40)
            wait_for_js(tp, f"Math.min({IT}.a,{IT}.b)<{v['a']!r}"); settle(tp)
            assert rec(tp)["a"] in v["x"], "not a point of the trace"
            looks = []
            for th in ("light", "dark"):
                tp.evaluate("t => document.documentElement.setAttribute('data-theme', t)", th); tp.evaluate(f"draw({PN})")
                wait_for_js(tp, "document.querySelectorAll('.iep:not([hidden])').length===2")
                looks.append(tp.evaluate("getComputedStyle(document.querySelector('.iep'),'::after').borderTopColor"))
                tp.locator(".pnl.xic").first.screenshot(path=str(D / f"estremi-{th}.png"))
            assert looks[0] != looks[1], looks                                    # the ring follows the theme's variables
            tp.close()
        step("touch 1024x768: a finger drags the start; light and dark theme", touch)
finally:
    r.close()
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
