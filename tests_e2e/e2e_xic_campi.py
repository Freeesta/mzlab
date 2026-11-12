"""XIC panel (low resolution): the «m/z from / to» fields of the «Range» popover (not the title row) take ABSOLUTE m/z edges, keep what was typed and extract that window, whichever edge is
written first; bad input is refused with a calm note and the old edges back, Esc drops a half-written pair; undo, redo and a reload keep the edges;
a clicked peak gets the default window [n-0.2, n+0.3] around its nearest integer n, and the popover fits an iPad-wide window. A range typed in «Extract ions»
(«250.8-251.5») is that window, never an m/z shifted below zero."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_xic_campi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("xic_campi")
r = Run(port=8907, wd="/tmp/wd_xic_campi")
X = "E.panels.find(q => q.type === 'xic' && q.tab === E.tab)"
def vals(pg): return pg.evaluate(f"(() => {{ const h = {X}.el; return [h.querySelector('[data-o=xlo]').value, h.querySelector('[data-o=xhi]').value]; }})()")
def edges(pg, i=0): return pg.evaluate(f"(() => {{ const t = {X}.traces[{i}]; return [+(t.mz - t.w).toFixed(2), +(t.mz + t.w).toFixed(2)]; }})()")
def field(pg, k): return pg.locator(f"#dpanels .pnl.xic .hd [data-o={k}]")
def write(pg, k, txt, key=None):
    f = field(pg, k)
    if not f.is_visible(): pg.locator("#dpanels .pnl.xic .hd [data-a=cpar]").click()
    f.click(); f.press("ControlOrMeta+a"); f.press_sequentially(txt)
    if key: f.press(key)
    # Enter: the header is drawn again (fresh fields); Tab: the focus is on the other edge, which waits
    q = f"document.querySelector('#dpanels .pnl.xic .hd [data-o={k}]')"
    wait_for_js(pg, f"{q}.value === {q}.defaultValue" if key == "Enter" else f"document.activeElement && document.activeElement.dataset.o === '{'xhi' if k == 'xlo' else 'xlo'}'", 5000); frames(pg, 3)
def drawn(pg):  # highest point of the drawn XIC: an empty window draws a flat trace with ymax near 1e-9
    frames(pg, 4); ready(pg); pg.wait_for_function(f"{X}._a && {X}._a.ymax != null", timeout=30000)
    return pg.evaluate(f"{X}._a.ymax")
def note(pg): return pg.evaluate(f"(() => {{ const m = {X}.el.querySelector('.xr-msg'); return m && !m.hidden ? m.textContent : ''; }})()")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        def dialog_range():
            pg.evaluate("openXic(null,{})"); pg.wait_for_selector("#xic-mz", state="visible"); pg.fill("#xic-mz", "250.8-251.5")
            pg.wait_for_function("document.querySelector('.xrw .xr-sum') && /251.5/.test(document.querySelector('.xrw .xr-sum').textContent)", timeout=15000)
            pg.click("#xic-go"); wait_for_js(pg, "E.panels.some(q => q.type === 'xic')"); frames(pg, 4); ready(pg)
            assert vals(pg) == ["250.8", "251.5"], f"the range typed in «Extract ions» is not the window: {vals(pg)}"
            assert "-" not in pg.evaluate(f"{X}.traces[0].label"), pg.evaluate(f"{X}.traces[0].label")
            assert drawn(pg) > 1, f"the XIC of the range is empty: ymax {drawn(pg)}"
        step("«Extract ions» with 250.8-251.5: the window is that range, the trace has signal", dialog_range)
        def move_up():
            write(pg, "xlo", "260.6", "Tab"); write(pg, "xhi", "261.6", "Enter")
            assert vals(pg) == ["260.6", "261.6"], f"fields: {vals(pg)}"
            assert edges(pg) == [260.6, 261.6], f"trace window: {edges(pg)}"
            assert note(pg) == "", note(pg)
            assert drawn(pg) > 1, drawn(pg)
        step("header: «from» written first above the old «to», then «to»: the window moves there", move_up)
        def narrow_wide():
            write(pg, "xlo", "260.9", "Tab"); write(pg, "xhi", "261.3", "Enter")
            assert vals(pg) == ["260.9", "261.3"] and edges(pg) == [260.9, 261.3], (vals(pg), edges(pg))
            write(pg, "xhi", "263.5", "Enter")
            assert vals(pg) == ["260.9", "263.5"] and edges(pg) == [260.9, 263.5], (vals(pg), edges(pg))
            assert drawn(pg) > 1, drawn(pg)
        step("narrower and wider windows than the unit one are kept", narrow_wide)
        def refused():
            for txt, key in (("270", "xic.edge.order"), ("abc", "xic.edge.num"), ("", "xic.edge.num"), ("-3", "xic.edge.num")):
                write(pg, "xlo", txt, "Enter")
                assert vals(pg) == ["260.9", "263.5"], f"{txt!r}: old edges not back: {vals(pg)}"
                assert edges(pg) == [260.9, 263.5], f"{txt!r}: {edges(pg)}"
                want = pg.evaluate(f"I18N.t('{key}')"); wait_for_js(pg, f"(() => {{ const m = {X}.el.querySelector('.xr-msg'); return m && !m.hidden && m.textContent === {want!r}; }})()", 5000)
        step("from > to, letters, empty, negative: refused with a calm note, old edges back", refused)
        def undo_redo():
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            pg.keyboard.press("Control+z"); wait_for_js(pg, f"{X}.el.querySelector('[data-o=xhi]').value === '261.3'", 5000)
            assert vals(pg) == ["260.9", "261.3"], vals(pg)
            pg.keyboard.press("Control+y"); wait_for_js(pg, f"{X}.el.querySelector('[data-o=xhi]').value === '263.5'", 5000)
        step("undo and redo bring the edges back", undo_redo)
        def several():
            pg.evaluate(f"xicDirect(300.4, null, {X})"); frames(pg, 4); ready(pg)
            assert pg.evaluate(f"{X}.traces.length") == 2
            assert edges(pg, 1) == [299.8, 300.3] and pg.evaluate(f"{X}.traces[1].label") == "m/z 299.8\u2013300.3", (edges(pg, 1), pg.evaluate(f"{X}.traces[1].label"))
            pg.select_option("#dpanels .pnl.xic .hd [data-o=xt]", "1"); frames(pg, 2)
            write(pg, "xlo", "299.9", "Tab"); write(pg, "xhi", "300.5", "Enter")
            assert edges(pg, 1) == [299.9, 300.5], edges(pg, 1)
            assert edges(pg, 0) == [260.9, 263.5], f"the other series changed: {edges(pg, 0)}"
        step("two overlaid XICs (one from a clicked peak, default 299.8-300.3): the second series takes its own edges", several)
        def place():
            assert pg.evaluate(f"[...{X}.el.querySelectorAll('[data-o=xlo],[data-o=xhi]')].every(i => i.closest('.cpop'))"), "m/z edges outside «Range»"
            assert pg.evaluate(f"[...{X}.el.querySelectorAll('[data-o=xlo],[data-o=xhi]')].every(i => i.getAttribute('aria-label'))")
            b = pg.locator("#dpanels .pnl.xic .hd [data-a=cpar]")
            if pg.evaluate(f"!!{X}._parOpen"): b.click()
            b.focus(); pg.keyboard.press("Enter"); assert b.get_attribute("aria-expanded") == "true"
            f = field(pg, "xlo"); f.click(); f.press("ControlOrMeta+a"); f.press_sequentially("299.0"); f.press("Escape")
            wait_for_js(pg, f"{X}.el.querySelector('.cpop').hidden && document.activeElement && document.activeElement.dataset.a === 'cpar'", 5000)
            assert edges(pg, 1) == [299.9, 300.5] and vals(pg) == ["299.9", "300.5"], (edges(pg, 1), vals(pg))
        step("the edges live in «Range» (aria-labels); Enter opens it, Esc drops a half-written pair and gives the focus back", place)
        def ipad():
            pg.set_viewport_size({"width": 820, "height": 1180}); frames(pg, 4)
            b = pg.locator("#dpanels .pnl.xic .hd [data-a=cpar]")
            if not field(pg, "xlo").is_visible(): b.click()
            for k in ("xlo", "xhi"):
                bb = field(pg, k).bounding_box(); assert bb and bb["x"] >= 0 and bb["x"] + bb["width"] <= 820 and bb["width"] >= 24 and bb["height"] >= 24, (k, bb)
            b.click(); pg.set_viewport_size({"width": 1280, "height": 800}); frames(pg, 4)
        step("iPad width: the popover fields are on screen, at least 24x24 px", ipad)
        def reload():
            saved(pg); pg.reload(); wait_for_js(pg, "typeof E !== 'undefined' && E.panels.some(q => q.type === 'xic' && q.traces.length === 2)", 90000); ready(pg)
            assert edges(pg, 0) == [260.9, 263.5] and edges(pg, 1) == [299.9, 300.5], (edges(pg, 0), edges(pg, 1))
            assert vals(pg) == ["260.9", "263.5"] or vals(pg) == ["299.9", "300.5"], vals(pg)
        step("a reload keeps the edges", reload)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
