"""Spectrum bar: XIC of several overlaid files = one panel with one trace per file, tooltip «XIC of N files», active file in bold; «Follows» as a symbol;
overlay as a toggle (disabled with one file); title «MS» + level in superscript, time without «RT»; bar without truncation from 700 to 2000 px."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_barra_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
TT = lambda k, a="{}": pg.evaluate(f"I18N.t('{k}', {a})")
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
PEAKS = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "e2e_xic_lente.py")).read().split('PEAKS = "')[1].split('"\nLENS')[0]
r = Run(port=8883, wd="/tmp/wd83")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        pg.evaluate("(()=>{E.browse=false;tabFiles('full').forEach(f=>f.vis=true);renderFileList();redrawAll()})()"); ready(pg); frames(pg, 6)
        OV = f"{SPEC}.el.querySelector('input[data-o=all]')"
        def title():
            t = pg.evaluate(f"(()=>{{const e={SPEC}.el.querySelector('.ttl');return [e.innerHTML,e.textContent,e.getAttribute('aria-label'),{SPEC}.el.querySelector('.rtl').textContent,{SPEC}.el.querySelector('.rtl').title]}})()"); print(t)
            assert t[0] == "MS<sup>1</sup>" and t[1] == "MS1" and t[2] == TT("ctl.spec.lv", "{n:1}"), t
            assert "RT" not in t[3] and t[3].endswith(" min") and t[4] == TT("ctl.spec.rt"), t
        step("title is MS¹ (copied as MS1, aria-label MS level 1), time without RT with its tooltip", title)
        def overlay_toggle():
            b = pg.evaluate(f"(()=>{{const i={OV},l=i.closest('label');return [i.disabled,i.checked,l.className,l.title]}})()"); print(b)
            assert b[0] is False and b[1] is False and "on" not in b[2].split() and b[3] == TT("ctl.spec.overlay"), b
            pg.evaluate(f"{OV}.click()"); ready(pg); frames(pg, 6)
            b = pg.evaluate(f"(()=>{{const i={OV},l=i.closest('label');return [i.checked,l.classList.contains('on'),{SPEC}.all,{SPEC}._a.data.length]}})()"); print(b)
            assert b[0] and b[1] and b[2] and b[3] == 2, b
        step("overlay is a toggle: pressed state visible, two files drawn", overlay_toggle)
        def follow():
            f = pg.evaluate(f"(()=>{{const b={SPEC}.el.querySelector('.fol');return [b.textContent,b.getAttribute('aria-label'),b.title]}})()"); print(f)
            assert f[0].startswith("←") and len(f[0]) <= 3 and f[1] and f[2], f
            pg.click('.pnl.spec .fol'); frames(pg, 3)
            assert pg.locator("#ctx").count() and pg.evaluate("document.querySelector('#ctx')&&!document.querySelector('#ctx').hidden"), "menu not reachable"
            pg.keyboard.press("Escape")
        step("follow is a symbol with aria-label and tooltip, the menu opens", follow)
        def xic_many():
            ps = pg.evaluate(PEAKS % SPEC); t = ps[1]
            tip = pg.evaluate(f"{SPEC}.el.querySelector('[data-a=xicb]').title"); print(tip); assert tip == TT("xicbtn.n", "{n:2}"), tip
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=xicb]').click()")
            pg.mouse.move(t["x"], t["y"]); pg.mouse.click(t["x"], t["y"]); ready(pg); frames(pg, 10)
            xs = pg.evaluate("E.panels.filter(p=>p.type==='xic').map(p=>[p.traces.length,p._a&&p._a.sr&&p._a.sr.length,p.leg.innerText,p.leg.querySelectorAll('.tg.act').length])"); print(xs)
            assert len(xs) == 1 and xs[0][1] == 2 and xs[0][3] == 1, xs
            assert xs[0][2].count("B_FullMass") == 2, xs
        step("XIC with two overlaid files: one panel, two traces, legend with the file names, active file in bold", xic_many)
        def one_file():
            pg.evaluate(f"{OV}.click()"); ready(pg); frames(pg, 4)
            pg.evaluate("(()=>{const fs=tabFiles('full');fs[1].vis=false;renderFileList();redrawAll()})()"); ready(pg); frames(pg, 6)
            d = pg.evaluate(f"(()=>{{const i={OV},l=i.closest('label');return [i.disabled,getComputedStyle(l).opacity<1]}})()"); print(d)
            assert d[0] and d[1], d
        step("overlay is greyed out and disabled with a single file", one_file)
        def widths():
            pg.evaluate("(()=>{tabFiles('full').forEach(f=>f.vis=true);renderFileList();redrawAll()})()"); ready(pg)
            for w in (700, 1000, 1600, 2000):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 8)
                bad = pg.evaluate(f"(()=>{{const e={SPEC}.el,o=[];e.querySelectorAll('.hd .ttl,.hd .rtl,.hd .ctl button,.hd .ctl select').forEach(x=>{{if(x.offsetParent&&x.scrollWidth>x.clientWidth+1&&getComputedStyle(x).textOverflow==='ellipsis')o.push(x.className||x.tagName)}});return o}})()")
                assert not bad, (w, bad)
        step("no truncated element in the bar between 700 and 2000 px", widths)
        print([e for e in r.errs if e[0] != "console.warning"])
        assert not pg.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length")
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
