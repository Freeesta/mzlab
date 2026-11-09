"""«Range» popover of a chromatogram: the m/z and RT «from / to» pairs are read when the focus leaves the pair, so Tab moves between the fields without
losing the focus; a reversed pair or letters are refused with a calm note in the popover and the old values back; undo, redo and a reload keep them."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_intervallo_campi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("intervallo_campi")
r = Run(port=8908, wd="/tmp/wd_intervallo_campi")
C = "E.panels.find(q => q.type === 'chrom' && q.tab === E.tab)"
POP = "#dpanels .pnl.chrom .cpop"
def field(pg, k): return pg.locator(f"{POP} [data-o={k}]")
def vals(pg, a, b): return pg.evaluate(f"[document.querySelector('{POP} [data-o={a}]').value, document.querySelector('{POP} [data-o={b}]').value]")
def active(pg): return pg.evaluate("document.activeElement && document.activeElement.dataset ? document.activeElement.dataset.o || '' : ''")
def write(pg, k, txt, key):
    f = field(pg, k); f.click(); f.press("ControlOrMeta+a"); f.press_sequentially(txt); f.press(key); frames(pg, 3)
def note(pg): return pg.evaluate(f"(() => {{ const m = document.querySelector('{POP} .xr-msg'); return m && !m.hidden ? m.textContent : ''; }})()")
def wait_note(pg, key):
    want = pg.evaluate(f"I18N.t('{key}')")
    wait_for_js(pg, f"(() => {{ const m = document.querySelector('{POP} .xr-msg'); return m && !m.hidden && m.textContent === {want!r}; }})()", 5000)
def mz(pg): return pg.evaluate(f"[{C}.mz0, {C}.mz1]")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        pg.click("#dpanels .pnl.chrom [data-a=cpar]"); pg.wait_for_selector(f"{POP} [data-o=mz0]", state="visible")
        def tab_keeps_focus():
            write(pg, "mz0", "300", "Tab")
            wait_for_js(pg, "document.activeElement && document.activeElement.dataset.o === 'mz1'", 5000)
            write(pg, "mz1", "200", "Enter"); wait_note(pg, "ctl.range.order")
            assert mz(pg) == [None, None], f"a reversed m/z pair was taken: {mz(pg)}"
            assert vals(pg, "mz0", "mz1") == ["", ""], vals(pg, "mz0", "mz1")
        step("m/z: Tab keeps the focus in the pair; «from» above «to» is refused with a note, old values back", tab_keeps_focus)
        def mz_ok():
            write(pg, "mz0", "200", "Tab"); write(pg, "mz1", "300", "Enter")
            wait_for_js(pg, f"{C}.mz0 === 200 && {C}.mz1 === 300", 5000)
            write(pg, "mz1", "350", "Tab")
            wait_for_js(pg, f"{C}.mz1 === 350", 5000); frames(pg, 3)
            assert active(pg) == "rt0", f"Tab out of the m/z pair lost the focus: {active(pg)!r}"
            assert vals(pg, "mz0", "mz1") == ["200", "350"], vals(pg, "mz0", "mz1")
        step("m/z: a valid pair is applied; Tab to RT after a change keeps the focus on the next field", mz_ok)
        def rt_bad():
            write(pg, "rt0", "5", "Tab")
            wait_for_js(pg, "document.activeElement && document.activeElement.dataset.o === 'rt1'", 5000)
            write(pg, "rt1", "2", "Enter"); wait_note(pg, "ctl.range.order")
            assert pg.evaluate(f"{C}.zoom") is None, pg.evaluate(f"{C}.zoom")
            assert vals(pg, "rt0", "rt1") == ["", ""], vals(pg, "rt0", "rt1")
        step("RT: «from» above «to» is refused with a note, not ignored silently", rt_bad)
        def letters():
            write(pg, "mz0", "abc", "Enter"); wait_note(pg, "ctl.range.num")
            assert mz(pg) == [200, 350] and vals(pg, "mz0", "mz1") == ["200", "350"], (mz(pg), vals(pg, "mz0", "mz1"))
        step("letters are refused with a note, old values back", letters)
        def rt_ok():
            write(pg, "rt0", "1", "Tab"); write(pg, "rt1", "3", "Enter")
            wait_for_js(pg, f"{C}.zoom && Math.abs({C}.zoom[0] - 1) < 0.01 && Math.abs({C}.zoom[1] - 3) < 0.01", 5000)
        step("RT: a valid pair zooms the panel", rt_ok)
        def undo_redo():
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            pg.keyboard.press("Control+z"); wait_for_js(pg, f"{C}.zoom == null && {C}.mz1 === 350", 5000)   # the RT zoom is the last step
            pg.keyboard.press("Control+z"); wait_for_js(pg, f"{C}.mz1 === 300", 5000)
            pg.keyboard.press("Control+y"); wait_for_js(pg, f"{C}.mz1 === 350", 5000)
            pg.keyboard.press("Control+y"); wait_for_js(pg, f"{C}.zoom != null", 5000)
        step("undo and redo bring the RT zoom and the m/z pair back", undo_redo)
        def reload():
            saved(pg); pg.reload(); wait_for_js(pg, f"typeof E !== 'undefined' && E.panels.some(q => q.type === 'chrom') && {C}.mz1 != null", 90000); ready(pg)
            assert mz(pg) == [200, 350], mz(pg)
        step("a reload keeps the m/z pair", reload)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
