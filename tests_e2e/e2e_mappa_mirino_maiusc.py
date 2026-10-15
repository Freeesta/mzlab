"""RT x m/z map, LR: (1) Shift held while the sight moves (no button down) locks it on the axis of the first pixels (same m/z / same RT) and frees it when released;
(2) the sight button shows the real state (aria-pressed + class «on»), at once, after a click and after the M key; (3) there is no «⋯» button next to it;
(4) the shape of a marker is decided when it is created and does not change when others are added (same RT = same shape, a new RT takes the next shape);
(5) the notice «the files stay in the browser» goes away by itself after 20 s, stops while the pointer is on it and closes with its X; (6) the toolbar labels have no option."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8933, wd="/tmp/wd_mirino_maiusc_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + repr(e)[:500]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "!!document.querySelector('#pick')")
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); attendi(pg, "!!document.querySelector('#opbtn')")
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, f"!!{M}._mz")
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        settle = lambda: ready(pg)
        btn = ".pnl.map [data-mz=mir]"
        state = lambda: (pg.get_attribute(btn, "aria-pressed"), "on" in (pg.get_attribute(btn, "class") or "").split(), pg.evaluate(f"!!{M}._mz.grid.classList.contains('mir')"))
        def button():
            assert state()[:2] == ("false", False), state()                       # in the page the sight starts off
            pg.click(btn); settle(); assert state() == ("true", True, True), state()
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("m"); settle(); assert state()[:2] == ("false", False), state()
            pg.keyboard.press("m"); settle(); assert state()[:2] == ("true", True), state()
            pg.keyboard.press("m"); settle()
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); ready(pg)
            assert state()[:2] == ("true", True), ("full screen: the sight is on and the button says so", state())
            pg.keyboard.press("f"); ready(pg)
            assert state()[:2] == ("false", False), ("back in the page: off", state())
        step("sight button: aria-pressed and «on» follow the real state (start, click, M, full screen)", button)
        def nomenu():
            assert pg.locator(".pnl.map [data-mz=menu]").count() == 0, "no dropdown next to the sight"
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("m"); settle()
            c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
            pg.mouse.click(c[0] + c[2] * .5, c[1] + c[3] * .5, button="right"); attendi(pg, "!document.querySelector('#ctx').hidden")
            assert pg.evaluate("!document.querySelector('#ctx').hidden"), "the right click still opens the menu"
            pg.keyboard.press("Escape")
        step("no «⋯» button; the right-click menu stays", nomenu)
        def shift():
            c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
            hov = lambda: pg.evaluate(f"({M}._mz.hov)")
            x0, y0 = c[0] + c[2] * .5, c[1] + c[3] * .5
            pg.mouse.move(x0, y0); pg.mouse.move(x0 + 3, y0 + 3); h0 = hov()
            pg.keyboard.down("Shift")
            pg.mouse.move(x0 + 60, y0 + 8, steps=6); h1 = hov()
            assert abs(h1["mz"] - h0["mz"]) < 1e-9 and h1["rt"] != h0["rt"], (h0, h1)            # horizontal: same m/z
            pg.mouse.move(x0 + 90, y0 + 120, steps=6); h2 = hov()
            assert abs(h2["mz"] - h0["mz"]) < 1e-9 and h2["rt"] != h1["rt"], (h1, h2)            # the axis stays, even if the pointer wanders vertically
            pg.keyboard.up("Shift"); attendi(pg, f"Math.abs({M}._mz.hov.mz - {h0['mz']}) > 1e-6"); h3 = hov()
            assert abs(h3["mz"] - h0["mz"]) > 1e-6, ("released: free again", h0, h3)
            pg.mouse.move(x0 + 3, y0 + 3); h4 = hov(); pg.keyboard.down("Shift")
            pg.mouse.move(x0 + 6, y0 + 90, steps=6); h5 = hov()
            assert abs(h5["rt"] - h4["rt"]) < 1e-9 and h5["mz"] != h4["mz"], (h4, h5)            # vertical: same RT
            pg.keyboard.up("Shift")
        step("Shift + sight: horizontal = same m/z, vertical = same RT, release = free", shift)
        def shapes():
            pg.evaluate(f"MAPPA.pts({M}).length = 0; MAPPA.prefs.snap = false")                  # no snap: the points stay where they are put
            add = lambda rt, mz: pg.evaluate(f"MAPPA.addPointAt({M}, {rt}, {mz}, 1)")
            sh = lambda: pg.evaluate(f"MAPPA.pts({M}).map((q, i) => MAPPA.grpShape({M}, i))")
            ax = pg.evaluate(f"[{M}._a.x0, {M}._a.x1, {M}._a.y0, {M}._a.y1]")
            r0, r1, r2 = ax[0] + (ax[1] - ax[0]) * .2, ax[0] + (ax[1] - ax[0]) * .5, ax[0] + (ax[1] - ax[0]) * .8
            m = lambda f: ax[2] + (ax[3] - ax[2]) * f
            add(r0, m(.3)); add(r0, m(.6)); ready(pg)
            s2 = sh(); assert s2[0] == s2[1] == "circle", s2                                      # same RT: same shape
            add(r1, m(.4)); ready(pg)
            s3 = sh(); assert s3[:2] == s2 and s3[2] == "square", s3                              # a new RT: the next shape, the old ones are not touched
            add(r2, m(.5)); add(r1, m(.7)); ready(pg)
            s5 = sh(); assert s5[:3] == s3 and s5[3] == "triangle" and s5[4] == "square", s5
            pg.evaluate(f"MAPPA.delPoint({M}, 0)"); ready(pg)
            assert sh() == s5[1:], ("removing a point changes no shape", sh(), s5)
            assert pg.evaluate(f"MAPPA.pts({M}).every(q => q.shape)"), "the shape is stored with the point"
        step("markers: the shape is decided at creation and never changes", shapes)
        def aviso():
            pg.clock.install()
            n = lambda: pg.locator("#resetnote").count()
            new = lambda: pg.evaluate("localStorage.removeItem('qqq.reset.note'); QQRESET.remind()")
            new(); assert n() == 1
            pg.clock.run_for(19000); assert n() == 1, "still there before 20 s"
            pg.clock.run_for(2000); assert n() == 0, "gone by itself after 20 s"
            new(); box = pg.locator("#resetnote").bounding_box(); pg.mouse.move(box["x"] + 8, box["y"] + 8)
            pg.clock.run_for(60000); assert n() == 1, "the pointer on it stops the timer"
            pg.mouse.move(5, 5); pg.clock.run_for(21000); assert n() == 0, "it goes when the pointer leaves"
            new(); pg.evaluate("document.querySelector('#resetnote button').focus()")
            pg.clock.run_for(60000); assert n() == 1, "the focus inside stops the timer"
            pg.evaluate("document.activeElement.blur()"); pg.clock.run_for(21000); assert n() == 0
            new(); pg.evaluate("document.querySelector('#resetnote [data-n=x]').click()"); assert n() == 0, "the X closes it at once"
        step("notice «files stay in the browser»: a 20 s timer, closes with its X", aviso)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
bad = [s for s in steps if s[1] != "ok"]; sys.exit(1 if bad else 0)
