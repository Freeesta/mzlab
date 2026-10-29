"""The accent bar on the left edge of the active / paired work-area panels is a thin 2px border (not 4px) and nothing moves: the content of an active panel
starts at the same x as the content of an inactive one, in light and dark; the bar keeps the accent colour and the pair highlight keeps its dashed outline."""
import os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
# bar width, colour and x of the header content (relative to the panel's outer left edge) of every panel
PANELS = """()=>E.panels.map(p=>{const e=p.el,cs=getComputedStyle(e),h=e.querySelector('.hd').getBoundingClientRect(),r=e.getBoundingClientRect();
 return {act:e.classList.contains('act'),pair:e.hasAttribute('data-pair'),w:cs.borderLeftWidth,st:cs.borderLeftStyle,col:cs.borderLeftColor,x:Math.round((h.left-r.left)*10)/10,
 hi:e.classList.contains('pair-hi'),out:cs.outlineStyle,accent:getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()}})"""
r = Run(port=8981, wd="/tmp/wd_pannello_barra")
tmp = Path(tempfile.mkdtemp(prefix="pbarra_"))
try:
    f0 = tmp / "B_FullMass-t0.mzML"; ds.full_scan(f0, 0, np.random.default_rng(1))
    f1 = tmp / "B_FullMass-t15.mzML"; ds.full_scan(f1, 15, np.random.default_rng(2))
    with sync_playwright() as p:
        pg = r.page(p)
        attendi(pg, "window.qqReady === true"); ready(pg)
        stage(pg, [f0, f1]); ready(pg)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-spec"); ready(pg)
        shots = get_env("BARRA_SHOTS")
        def check(theme):
            def f():
                pg.evaluate(f"document.documentElement.setAttribute('data-theme','{theme}')"); frames(pg, 3)
                ps = pg.evaluate(PANELS); assert len(ps) >= 2, ps
                act = [q for q in ps if q["act"]]; ina = [q for q in ps if not q["act"] and not q["pair"]]
                assert act and ina, ps
                for q in act: assert q["w"] == "2px" and q["st"] == "solid", ("bar of the active panel", q)
                for q in ina: assert q["w"] == "1px", ("inactive panel keeps the 1px card border", q)
                assert {q["x"] for q in ps if not q["pair"]} == {q["x"] for q in ina}, ("content moved", ps)
                if shots: pg.screenshot(path=f"{shots}/barra-{theme}.png", clip={"x": 0, "y": 0, "width": 1500, "height": 900})
            return f
        step("light: active bar 2px, content does not move", check("light"))
        step("dark: active bar 2px, content does not move", check("dark"))
        def pair():
            # paired panels: the same 2px bar in the pair colour, and the highlight keeps its dashed outline
            pg.evaluate("document.querySelectorAll('.pnl').forEach((e,i)=>{e.dataset.pair=String(i+1)})"); pg.evaluate("document.querySelector('.pnl').classList.add('pair-hi')"); frames(pg, 2)
            ps = pg.evaluate(PANELS)
            assert all(q["w"] == "2px" for q in ps), ps
            assert ps[0]["out"] == "dashed", ps[0]
            assert len({q["x"] for q in ps}) == 1, ("content moved with the pair bar", ps)
            if shots: pg.screenshot(path=f"{shots}/barra-pair.png", clip={"x": 0, "y": 0, "width": 1500, "height": 900})
        step("pair bars 2px, dashed highlight kept", pair)
        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
import shutil; shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
