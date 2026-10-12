"""The side panel is docked: hidden, or at full height on the left or right edge; the workspace narrows to the space left (never under it).
Always-visible header button, tool buttons, Ctrl/Cmd+B, side, sheet on narrow screens, themes, old saved states. No JS errors."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
BOX = """() => { const r = s => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return {x: b.x, r: b.right, w: b.width, h: b.height}; };
  const c = E.panels.find(p => p.cv); const cb = c && c.cv.getBoundingClientRect();
  return {side: r('#dfiles'), dp: r('#dpanels'), chart: cb && {x: cb.x, r: cb.right, w: cb.width}, vw: innerWidth, sw: document.documentElement.scrollWidth - document.documentElement.clientWidth}; }"""
def overlap(b):
    s, c = b["side"], b["chart"]
    return s and c and s["w"] > 0 and not (c["r"] <= s["x"] + 1 or c["x"] >= s["r"] - 1)
def cold(p, vw, vh, saved=None, scheme="light", touch=False):
    r = Run(port=8872, wd="/tmp/wd_pannello"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": vw, "height": vh}, locale="it-IT", has_touch=touch, color_scheme=scheme, reduced_motion="reduce")
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    init = "try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');"
    for k, v in (saved or {}).items(): init += f"localStorage.setItem('{k}',{v!r});"
    pg.add_init_script(init + "}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
    pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); pg.wait_for_timeout(800)
    pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(500)
    return r, ctx, pg, errs
try:
    with sync_playwright() as p:
        def base():
            r, ctx, pg, errs = cold(p, 1440, 900)
            try:
                assert pg.is_visible("#sb-toggle") and pg.is_visible("#dfiles"), "first start on a wide screen: File panel open"
                assert pg.get_attribute("#sb-toggle", "aria-pressed") == "true" and pg.get_attribute("#sb-toggle", "aria-label") == "Pannello laterale" and pg.get_attribute("#sb-toggle", "title")
                b1 = pg.evaluate(BOX)
                assert pg.evaluate("BARRA.curTab") == "file" and not overlap(b1), b1
                assert b1["side"]["x"] < 20 and b1["side"]["h"] > 400, b1["side"]
                assert abs(b1["dp"]["x"] - b1["side"]["r"]) < 30, "the workspace starts where the panel ends"
                pg.click("#sb-toggle"); pg.wait_for_timeout(600)
                b0 = pg.evaluate(BOX)
                assert not pg.is_visible("#dfiles") and pg.get_attribute("#sb-toggle", "aria-pressed") == "false" and b1["chart"]["w"] < b0["chart"]["w"] - 200, (b0, b1)
                pg.click("#stbar [data-cmd=adducts]"); pg.wait_for_timeout(600)
                b2 = pg.evaluate(BOX); assert pg.evaluate("BARRA.curTab") == "adducts" and b2["chart"]["w"] < b0["chart"]["w"] - 200 and not overlap(b2), b2
                pg.click("#sb-toggle"); pg.wait_for_timeout(500); assert not pg.is_visible("#dfiles")
                pg.click("#sb-toggle"); pg.wait_for_timeout(500); assert pg.evaluate("BARRA.curTab") == "adducts", "the toggle reopens the last tool"
                pg.click("#stbar [data-cmd=adducts]"); pg.wait_for_timeout(500); assert not pg.is_visible("#dfiles"), "a second click on the same tool hides it"
                pg.keyboard.press("Control+b"); pg.wait_for_timeout(400); assert pg.is_visible("#dfiles"), "Ctrl+B"
                pg.keyboard.press("Control+b"); pg.wait_for_timeout(400); assert not pg.is_visible("#dfiles")
                assert pg.evaluate("BARRA.commands.some(c => c.id === 'panel' && c.run)") and pg.locator("#sb-pin").count() == 0
            finally: ctx.close(); r.close()
            assert not errs, errs
        step("button always visible; File open on first start (wide screen); tool buttons dock the panel and narrow the charts; second click / Ctrl+B hide it", base)

        def lato():
            r, ctx, pg, errs = cold(p, 1440, 900, saved={"qqq.barra.lato": "right", "qqq.barra.lr": '{"tab":"losses","open":true,"w":340}'})
            try:
                b = pg.evaluate(BOX)
                assert pg.is_visible("#dfiles") and b["side"]["r"] > 1430 and b["side"]["w"] > 330 and not overlap(b), b
                assert b["dp"]["r"] <= b["side"]["x"] + 30 and b["sw"] <= 1
                pg.keyboard.press("Alt+Shift+ArrowLeft"); pg.wait_for_timeout(500); b = pg.evaluate(BOX)
                assert b["side"]["x"] < 20 and not overlap(b), b
                assert pg.evaluate("E.panels.filter(p=>p.full).every(p=>Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)")
            finally: ctx.close(); r.close()
            assert not errs, errs
        step("right and left side: no overlap, the workspace takes the rest", lato)

        def vecchi():
            for saved in ({"qqq.barra.lr": '{"tab":"file","pinned":true,"open":true,"w":0}', "qqq.barra.lato": "centro"},
                          {"qqq.barra.lr": '{"tab":"nope","open":"si","w":99999}'}, {"qqq.barra.lr": "not json"}):
                r, ctx, pg, errs = cold(p, 1440, 900, saved=saved)
                try:
                    st = pg.evaluate("JSON.parse(localStorage.getItem('qqq.barra.lr'))")
                    assert "pinned" not in st and 280 <= st["w"] <= 560 and st["tab"] in ("file", "calc", "losses", "adducts", "lists", "qaqc", "stat") and pg.evaluate("localStorage.getItem('qqq.barra.lato')") in ("left", "right"), st
                    b = pg.evaluate(BOX); assert not overlap(b) and b["sw"] <= 1, b
                finally: ctx.close(); r.close()
                assert not errs, errs
        step("old saved states (pinned, side, width 0 / huge, junk) are cleaned and harmless", vecchi)

        def schermi():
            for vw, vh, sheet in ((1024, 768, False), (1366, 1024, False), (820, 1180, True), (768, 1024, True)):
                r, ctx, pg, errs = cold(p, vw, vh, touch=True)
                try:
                    assert pg.is_visible("#sb-toggle")
                    if not pg.is_visible("#dfiles"): pg.click("#sb-toggle")      # open at first start from 900 px up
                    pg.wait_for_timeout(600); b = pg.evaluate(BOX)
                    assert pg.is_visible("#dfiles") and pg.evaluate("BARRA.sheet") == sheet, (vw, b)
                    if sheet: assert b["side"]["w"] >= vw - 30 and not pg.is_visible("#dpanels") and pg.is_visible("#sb-x"), (vw, b)
                    else: assert not overlap(b), (vw, b)
                    assert b["sw"] <= 1
                    pg.click("#sb-x"); pg.wait_for_timeout(500); assert not pg.is_visible("#dfiles") and pg.is_visible("#dpanels")
                finally: ctx.close(); r.close()
                assert not errs, errs
        step("iPad landscape docked, iPad portrait a sheet that replaces the workspace (× gives it back)", schermi)

        def temi():
            for scheme in ("light", "dark"):
                r, ctx, pg, errs = cold(p, 1440, 900, scheme=scheme)
                try:
                    pg.wait_for_timeout(500)
                    bg = pg.evaluate("getComputedStyle(document.querySelector('#sb-toggle')).backgroundColor"); assert bg != "rgba(0, 0, 0, 0)", (scheme, bg)
                    assert pg.is_visible("#dfiles") and not overlap(pg.evaluate(BOX))
                finally: ctx.close(); r.close()
                assert not errs, errs
        step("light and dark theme: the pressed button is highlighted, panel docked", temi)
finally: pass
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
