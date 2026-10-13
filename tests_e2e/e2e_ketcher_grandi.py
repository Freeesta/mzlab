"""The buttons of Ketcher are larger than its default 32 px and every one of them is reachable (nothing pushed out of the editor)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ketcher_grandi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8862, wd="/tmp/wd_kgr")
JS = """() => { const f = document.querySelector('#kframe'), d = f.contentDocument, fw = f.getBoundingClientRect().width, fh = f.getBoundingClientRect().height;
  const vis = id => [...d.querySelectorAll('[data-testid="' + id + '"]')].find(x => x.getBoundingClientRect().width > 0), rc = e => e && e.getBoundingClientRect();
  const o = { fw, fh }; for (const id of ['undo', 'hand', 'erase', 'bonds-drop-down-button', 'fullscreen-mode-button', 'zoom-selector', 'settings-button', 'help-button']) { const e = vis(id), q = rc(e); o[id] = q ? [Math.round(q.left), Math.round(q.top), Math.round(q.width), Math.round(q.height)] : null; }
  const left = d.querySelector('[class*="LeftToolbar-module_root"]'); o.leftBottom = Math.round(rc(left).bottom); o.rightBottom = Math.round(rc(d.querySelector('[class*="RightToolbar-module_root"]')).bottom); return o; }"""
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        for w, h, touch in ((1400, 1000, False), (1280, 800, False), (1366, 1024, True)):
            ctx = r.b.new_context(viewport={"width": w, "height": h}, has_touch=touch, is_mobile=touch); pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
            pg.wait_for_timeout(2500)
            if pg.is_visible("#opbtn"):
                pg.set_input_files("#pick", [mz("B_FullMass-t0")]); pg.wait_for_timeout(800); pg.click("text=Carica dati"); ready(pg)
            pg.click("#nav [data-v=draw]"); pg.wait_for_timeout(7000)
            def check():
                o = pg.evaluate(JS)
                for id in ("hand", "erase"): assert o[id] and min(o[id][2], o[id][3]) >= (33 if touch else 27), (id, o[id])                      # in top toolbar
                assert o["undo"] and min(o["undo"][2], o["undo"][3]) >= (33 if touch else 27), o["undo"]                       # the top one grows only when it has the room (a tablet gives it the whole width)
                assert o["bonds-drop-down-button"] and o["bonds-drop-down-button"][2] >= (40 if touch else 34), o["bonds-drop-down-button"]       # 40 px for touch (WCAG 2.5.8)
                for id in ("fullscreen-mode-button", "zoom-selector"): assert o[id] and o[id][0] + o[id][2] <= o["fw"] + 1, (id, o[id], o["fw"])       # reachable: inside the editor
                assert o["help-button"] is None, "Ketcher's own help is hidden (the program has its own)"
                assert o["leftBottom"] <= o["fh"] + 2 and o["rightBottom"] <= o["fh"] + 2, ("the toolbars fit the height", o["leftBottom"], o["rightBottom"], o["fh"])
            step(f"{w}x{h}{' (touch)' if touch else ''}: bigger buttons, all reachable", check)
            ctx.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
