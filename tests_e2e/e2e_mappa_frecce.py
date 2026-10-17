"""Arrows ◀ ▶ next to the file menu of the 2D map: the previous / next file of the list, the same reference (difference) and the same window; disabled (not hidden)
when there is no file before / after; the reference is jumped. Three files (the third is a copy of the first under another name)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8913, wd="/tmp/wd_mappa_frecce_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "!!document.querySelector('#pick')")
        fl = lambda n, f: {"name": n, "mimeType": "application/xml", "buffer": (EX / f).read_bytes()}      # paths and buffers cannot be mixed
        pg.set_input_files("#pick", [fl("FullScan_t0.mzML", "FullScan_t0.mzML"), fl("FullScan_t5.mzML", "FullScan_t0.mzML"), fl("FullScan_t30.mzML", "FullScan_t30.mzML")]); attendi(pg, "!!document.querySelector('#opbtn')")
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, "!!E.panels.find(q=>q.type==='map')")
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        lab = lambda: pg.evaluate("E.files[E.cur].label")           # one file shown: the map follows the bar
        prev, nxt = ".pnl.map [data-a=mapnav][data-d='-1']", ".pnl.map [data-a=mapnav][data-d='1']"
        def start():                                                                 # t0 is the reference, t5 is shown
            pg.locator(".pnl.map select[data-o=k]").select_option(index=1); attendi(pg, "E.files[E.cur].label==='FullScan_t5'"); ready(pg)
            pg.evaluate(f"(()=>{{const q={M},f=E.files;q.corr='file';q.ref=f[0].k;ctl(q);draw(q)}})()"); ready(pg); attendi(pg, f"MAPPA.corrState({M}).now[0]==='f'")
            assert lab() == "FullScan_t5", lab()
        step("setup: t5 minus t0", start)
        def disabled():
            assert pg.locator(prev).count() == 1 and pg.locator(nxt).count() == 1      # always there
            assert pg.is_disabled(prev) and not pg.is_disabled(nxt)                    # before t5 there is only the reference
            assert pg.get_attribute(nxt, "title") and pg.get_attribute(prev, "aria-label")
        step("arrows: previous disabled (only the reference before), next enabled", disabled)
        def forward():
            pg.evaluate(f"{M}.zoom=[2,10]"); pg.click(nxt); attendi(pg, "E.files[E.cur].label==='FullScan_t30'"); ready(pg)
            assert lab() == "FullScan_t30", lab()
            assert pg.evaluate(f"MAPPA.corrState({M}).now") == "f" + str(pg.evaluate("E.files[0].k")), "the reference stays t0"
            assert pg.evaluate(f"{M}.zoom") == [2, 10], pg.evaluate(f"{M}.zoom")        # the window stays
            assert pg.is_disabled(nxt) and not pg.is_disabled(prev)
        step("next: t30, same reference and same window; next disabled at the end", forward)
        def back():
            pg.click(prev); attendi(pg, "E.files[E.cur].label==='FullScan_t5'"); ready(pg)
            assert lab() == "FullScan_t5", lab()
            assert pg.is_disabled(prev)
        step("previous: back to t5, the reference t0 is jumped", back)
        def no_ref():
            pg.evaluate(f"(()=>{{const q={M};q.corr='none';q.ref='';q.bgr=null;ctl(q);draw(q)}})()"); ready(pg); attendi(pg, f"MAPPA.corrState({M}).now===''")
            assert not pg.is_disabled(prev), "without a reference t0 is a normal neighbour"
            pg.click(prev); attendi(pg, "E.files[E.cur].label==='FullScan_t0'"); ready(pg); assert lab() == "FullScan_t0", lab()
        step("without a reference the previous file is t0", no_ref)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
