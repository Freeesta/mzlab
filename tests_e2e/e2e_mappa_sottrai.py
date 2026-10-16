"""Map «Subtract file» chooser: after the file of the map changes, the files offered are all the other loaded ones (the starting file included)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
r = Run(port=8896, wd="/tmp/wd_mappa_sottrai")
OPTS = "(()=>{const p=E.panels.find(q=>q.type==='map'); return [...p.el.querySelectorAll('[data-o=corrf] option')].map(o=>o.textContent)})()"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1280, "height": 950})
        ctx.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1')}catch(_){}")
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t15.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); pg.wait_for_timeout(800)
        first = pg.evaluate("E.files[E.panels.find(q=>q.type==='map').k].label")
        assert first.endswith("t0"), first
        o0 = pg.evaluate(OPTS); assert len(o0) == 2 and not any(x.endswith("t0") for x in o0), o0
        pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='map'); const s=p.el.querySelector('select[data-o=k]'); s.value=String(E.files.find(f=>f.label.endsWith('t30')).k); s.dispatchEvent(new Event('change',{bubbles:true}))})()")
        ready(pg); pg.wait_for_timeout(800)
        o1 = pg.evaluate(OPTS)
        assert sorted(o1) == sorted(x for x in pg.evaluate("E.files.map(f=>f.label)") if not x.endswith("t30")), o1
        assert any(x.endswith("t0") for x in o1), o1
        steps.append(("the starting file is offered after the map changes file", "ok"))
        b.close()
finally:
    r.stop() if hasattr(r, "stop") else None
for s in steps: print(s)
print(r.errs if r.errs else "")
assert all(s[1] == "ok" for s in steps) and steps
