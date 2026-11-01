# e2e_colonne_dialogo: the «Table columns» dialog of the sample table (IT and EN, 1400 and 400 px wide): preset columns in an aligned grid (box right before its label,
# both centred, no label on two lines, equal cells), the custom column form as ONE row Name | Type | Unit | Add with fields of equal height (one column on a phone)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
_pg = []
def close_dialog():
    # a failed step must not leave the dialog open for the next one
    if _pg: _pg[0].evaluate("document.getElementById('colsdlg')?.open && document.getElementById('colsdlg').close()")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
    finally: close_dialog()
PROBE = """() => {
  const d = document.getElementById('colsdlg'), box = d.getBoundingClientRect(), r = e => e.getBoundingClientRect();
  const sets = [...d.querySelectorAll('fieldset.colsset')];
  const pre = [...sets[0].querySelectorAll('label.colsrow')].map(l => { const cb = l.querySelector('input'), sp = l.querySelector('span'), rg = document.createRange(); rg.selectNodeContents(sp);
    return { lines: rg.getClientRects().length, gap: r(sp).left - r(cb).right, dy: (r(cb).top + r(cb).bottom) / 2 - (r(sp).top + r(sp).bottom) / 2, top: r(l).top, w: r(l).width, left: r(l).left, text: sp.textContent.trim() }; });
  const add = d.querySelector('.colsadd'), fl = [...add.querySelectorAll('label')].map(l => { const f = l.querySelector('input,select'); return { lt: r(l).top, ft: r(f).top, fh: r(f).height, fl: r(f).left, fr: r(f).right, text: l.textContent.trim() }; });
  const b = r(add.querySelector('button'));
  return { box: { l: box.left, r: box.right, w: box.width }, iw: innerWidth, pre, fl, btn: { top: b.top, bottom: b.bottom, left: b.left }, fb: [...add.querySelectorAll('input,select')].map(f => r(f).bottom) };
}"""
def check(pg, w, tag):
    pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 3)
    pg.click("#colsbtn"); pg.wait_for_selector("#colsdlg[open]"); frames(pg, 3)
    g = pg.evaluate(PROBE)
    assert g["box"]["l"] >= 0 and g["box"]["r"] <= g["iw"] + 1, (tag, "dialog outside the window", g["box"], g["iw"])
    assert len(g["pre"]) >= 10, (tag, len(g["pre"]))
    for x in g["pre"]:
        assert x["lines"] == 1, (tag, "label on two lines", x["text"])
        assert 0 <= x["gap"] <= 14, (tag, "checkbox not right before its label", x)
        assert abs(x["dy"]) <= 1.5, (tag, "checkbox and label not centred", x)
    assert max(x["w"] for x in g["pre"]) - min(x["w"] for x in g["pre"]) <= 1, (tag, "cells of different width")
    lefts = sorted({round(x["left"]) for x in g["pre"]}); assert len(lefts) == (2 if w > 520 else 1), (tag, "columns", lefts)
    tops = sorted({round(x["top"]) for x in g["pre"]}); steps_ = [b - a for a, b in zip(tops, tops[1:])]; assert max(steps_) - min(steps_) <= 1, (tag, "rows not evenly spaced", steps_)
    f = g["fl"]; assert len(f) == 3, (tag, f)
    if w > 520:
        assert max(x["lt"] for x in f) - min(x["lt"] for x in f) <= 1 and max(x["ft"] for x in f) - min(x["ft"] for x in f) <= 1, (tag, "Name / Type / Unit not on one row", f)
        assert max(x["fh"] for x in f) - min(x["fh"] for x in f) <= 1, (tag, "fields of different height", f)
        assert abs(g["btn"]["bottom"] - g["fb"][0]) <= 1.5, (tag, "Add not aligned with the fields", g["btn"], g["fb"])
        assert f[0]["fr"] <= f[1]["fl"] and f[1]["fr"] <= f[2]["fl"] and f[2]["fr"] <= g["btn"]["left"], (tag, "fields overlap", f)
    else:
        assert max(x["fl"] for x in f) - min(x["fl"] for x in f) <= 1 and f[0]["ft"] < f[1]["ft"] < f[2]["ft"], (tag, "not stacked on a phone", f)
    pg.keyboard.press("Escape"); frames(pg, 2)
r = Run(port=8875, wd="/tmp/wd_colonne")
try:
    with sync_playwright() as p:
        for lang in ("it", "en"):
            pg = r.page(p); _pg[:] = [pg]; pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); pg.wait_for_timeout(800)
            pg.set_viewport_size({"width": 1400, "height": 900})
            pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t10")])
            frames(pg, 3); pg.click("text=Carica dati" if lang == "it" else "text=Load data"); ready(pg)
            pg.evaluate("S.table = true; syncWork()"); attendi(pg, "!!document.querySelector('#colsbtn')", 30000)
            for w in (1400, 400):
                step(f"{lang} {w}px: aligned preset grid and one-row custom form", lambda w=w, lang=lang, pg=pg: check(pg, w, (lang, w)))
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
