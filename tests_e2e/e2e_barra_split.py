"""Toolbar split buttons of the HR group (Scan, Formulas): the active state covers the main part AND the caret (one control), the group is named, the caret announces its menu. Synthetic Orbitrap DDA file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_barra_split.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\\n")[0][:240]))
D = synth("hrsplit")
r = Run(port=8979, wd="/tmp/wd_hrsplit")
JS = """[...document.querySelectorAll('#stbar .tb-split')].filter(g => g.offsetParent).map(g => { const m = g.querySelector('.tb-main'), c = g.querySelector('.tb-car'), sm = getComputedStyle(m), sc = getComputedStyle(c);
  return {id: g.dataset.cmd, role: g.getAttribute('role'), name: g.getAttribute('aria-label'), on: g.classList.contains('on'), pressed: m.getAttribute('aria-pressed'), popup: c.getAttribute('aria-haspopup'), exp: c.getAttribute('aria-expanded'),
    mb: sm.borderTopColor, cb: sc.borderTopColor, mbg: sm.backgroundColor, cbg: sc.backgroundColor, mc: sm.color, cc: sc.color}; })"""
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1000})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def state():
            info = pg.evaluate(JS); assert len(info) >= 2, info
            for g in info:
                assert g["role"] == "group" and g["name"] and g["popup"] == "menu" and g["exp"] == "false", g
                if g["on"]: assert g["pressed"] == "true" and g["mb"] == g["cb"] and g["mbg"] == g["cbg"] and g["mc"] == g["cc"], g
                else: assert g["pressed"] == "false", g
            return info
        def each():
            for g in state():
                sel = f'#stbar [data-cmd-main="{g["id"]}"]'
                grp = "document.querySelector('#stbar .tb-split[data-cmd=" + g["id"] + "]').classList.contains('on')"
                pg.click(sel); attendi(pg, grp)
                for th in ("light", "dark"):
                    pg.evaluate("t => document.documentElement.setAttribute('data-theme', t)", th); frames(pg)
                    now = {x["id"]: x for x in state()}
                    assert now[g["id"]]["on"], (th, now[g["id"]])
                    assert not any(x["on"] for k, x in now.items() if k != g["id"]), now
                pg.click(sel); attendi(pg, "!" + grp)
                now = {x["id"]: x for x in state()}
                assert not now[g["id"]]["on"], now[g["id"]]
        step("every split button: main part and caret share the active state, and lose it together (light and dark)", each)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
print("--- ERRORS", getattr(r, "errs", [])[:5]); sys.exit(1 if bad else 0)
