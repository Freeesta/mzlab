"""An infusion has no real chromatogram: the XIC button of its spectrum is disabled (tooltip from the catalogue) and the lens (XIC preview over a peak)
never opens. The same data under an ordinary name keeps both (control)."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_xic_infusione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("xic_infusione")
shutil.copy(D / "B_FullMass-t0.mzML", D / "Infusion_syringe_t0.mzML")
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
def open_file(pg, name):
    stage(pg, [D / name])
    pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
def hover_peak(pg):
    pg.evaluate(f"{SPEC}.el.scrollIntoView({{block: 'center'}})"); frames(pg)
    pos = pg.evaluate(f"""(() => {{ const p = {SPEC}, q = p.cv.getBoundingClientRect(); return {{ q: [q.left, q.top, q.width, q.height], id: p.id }}; }})()""")
    left, top, w, h = pos["q"]
    # scan the plot width at mid height for the first place where the lens opens (control) or never does
    for k in range(30):
        pg.mouse.move(left + w * (0.12 + 0.03 * k), top + h * 0.35); frames(pg, 2)
        if pg.evaluate("[...document.querySelectorAll('.xl')].some(e => !e.hidden)"): return True
    return False
r = Run(port=8867, wd="/tmp/wd_xic_infusione")
try:
    with sync_playwright() as p:
        pg = r.page(p); open_file(pg, "B_FullMass-t0.mzML")
        def control():
            b = pg.evaluate("(() => { const e = document.querySelector('[data-a=xicb]'); return [e.disabled, e.title]; })()")
            assert b[0] is False, b
            assert hover_peak(pg), "the lens never opened on an ordinary file (the test cannot tell)"
        step("ordinary file: XIC button on, lens opens", control)
        r.b.close(); pg2 = r.page(p); open_file(pg2, "Infusion_syringe_t0.mzML")
        def off():
            b = pg2.evaluate("(() => { const e = document.querySelector('[data-a=xicb]'); return [e.disabled, e.title]; })()")
            assert b[0] is True and b[1] == pg2.evaluate("I18N.t('xicbtn.infusion')") and b[1] != "xicbtn.infusion", b
            assert not hover_peak(pg2), "the lens opened on an infusion"
        step("infusion: XIC button disabled with its tooltip, no lens", off)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
