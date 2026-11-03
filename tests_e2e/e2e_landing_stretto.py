"""Narrow window (< 900 px) on the landing: the side panel starts closed, the toggle says «Files (N)», the sample table is the file list
(its file names open the file), the table scrolls inside its box and the page never scrolls sideways; the phone page keeps its own layout."""
import json, os, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
import e2e_landing as L

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:400]))

STATE = """() => { const v = document.querySelector('#v-data'), tg = document.querySelector('#sb-toggle'), w = document.querySelector('#flist .ldwrap'), q = s => { const e = document.querySelector(s); if (!e || e.hidden) return false; const b = e.getBoundingClientRect(); return b.width > 0 && b.height > 0; };
  return {sb_open: v.classList.contains('sb-open'), sheet: v.classList.contains('sb-sheet'), toggle: tg ? tg.innerText.trim() : null, toggle_aria: tg ? tg.getAttribute('aria-label') : null,
    drop: q('#drop'), table: q('#flist .ldtab'), work: document.querySelector('#v-workspace').dataset.work, page_sw: document.documentElement.scrollWidth, iw: innerWidth,
    box_sw: w ? w.scrollWidth : 0, box_cw: w ? w.clientWidth : 0, name_w: (document.querySelector('#flist .ldtab td:nth-child(2) button, #flist .ldtab td:nth-child(2)') || {getBoundingClientRect: () => ({width: 0})}).getBoundingClientRect().width}; }"""

r = Run(port=9021, wd="/tmp/wd_landing_stretto")
try:
    lr, hr = L.synth(tempfile.mkdtemp())
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        pg = L.new_page(r, p, 820, 900, "?telefono=0")
        pg.wait_for_selector("#drop", state="attached"); frames(pg, 6)
        s0 = pg.evaluate(STATE)
        step("landing: the side panel is closed and the drop area is in view", lambda: (_ for _ in ()).throw(AssertionError(s0)) if s0["sb_open"] or not s0["drop"] else None)
        step("landing: the toggle says «Files (0)»", lambda: (_ for _ in ()).throw(AssertionError(s0["toggle"])) if "(0)" not in (s0["toggle"] or "") or s0["toggle_aria"] != s0["toggle"] else None)
        step("landing: no horizontal scroll", lambda: (_ for _ in ()).throw(AssertionError(s0)) if s0["page_sw"] > s0["iw"] else None)
        L.drop_files(pg, lr, "#drop")
        pg.wait_for_function("document.querySelectorAll('#flist tbody tr').length >= %d" % len(lr), timeout=90000); ready(pg)
        s1 = pg.evaluate(STATE)
        step("files read: the toggle counts them", lambda: (_ for _ in ()).throw(AssertionError(s1["toggle"])) if "(%d)" % len(lr) not in (s1["toggle"] or "") else None)
        # the sheet: the toggle opens it over the work area; «Sample table» closes it and shows the table
        pg.click("#sb-toggle"); frames(pg, 6)
        s2 = pg.evaluate(STATE)
        step("the toggle opens the sheet", lambda: (_ for _ in ()).throw(AssertionError(s2)) if not (s2["sb_open"] and s2["sheet"]) else None)
        pg.click("#samptab"); frames(pg, 6)
        s3 = pg.evaluate(STATE)
        step("«Sample table» closes the sheet and the table fills the width", lambda: (_ for _ in ()).throw(AssertionError(s3)) if s3["sb_open"] or s3["work"] != "table" or not s3["table"] else None)
        step("the table scrolls inside its box, the page does not", lambda: (_ for _ in ()).throw(AssertionError(s3)) if s3["page_sw"] > s3["iw"] or s3["box_sw"] <= s3["box_cw"] else None)
        step("a file name keeps room to read (more than 90 px)", lambda: (_ for _ in ()).throw(AssertionError(s3["name_w"])) if s3["name_w"] < 90 else None)
        # a file name opens the file; afterwards the sheet works as before
        n = pg.locator("#flist .ldtab button[data-open]").count()
        step("every row has its «open» button", lambda: (_ for _ in ()).throw(AssertionError(n)) if n < len(lr) else None)
        pg.locator("#flist .ldtab button[data-open]").nth(1).click(); frames(pg, 8)
        pg.wait_for_selector(".pnl", timeout=60000)
        s4 = pg.evaluate(STATE)
        step("a file name opens the file (panels in view, sheet closed)", lambda: (_ for _ in ()).throw(AssertionError(s4)) if s4["work"] != "panels" or s4["sb_open"] else None)
        pg.click("#sb-toggle"); frames(pg, 6)
        s5 = pg.evaluate(STATE)
        step("after the first click the toggle opens the sheet again", lambda: (_ for _ in ()).throw(AssertionError(s5)) if not (s5["sb_open"] and s5["sheet"]) else None)
        step("no horizontal scroll in the sheet", lambda: (_ for _ in ()).throw(AssertionError(s5)) if s5["page_sw"] > s5["iw"] else None)
        pg.close()
        # wide window: the toggle keeps its verb, not the count
        pg = L.new_page(r, p, 1440, 900, "?telefono=0"); pg.wait_for_selector("#drop", state="attached"); frames(pg, 6)
        w0 = pg.evaluate(STATE)
        step("1440 px: the panel is open and the toggle is not «Files (N)»", lambda: (_ for _ in ()).throw(AssertionError(w0)) if not w0["sb_open"] or "(" in (w0["toggle"] or "") else None)
        pg.close()
        # phone: its own page, unchanged
        ph = r.b.new_page(viewport={"width": 390, "height": 800}, reduced_motion="reduce", locale="it-IT", has_touch=True)
        ph.add_init_script(L.INIT); ph.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        ph.goto(f"http://127.0.0.1:{r.port}/?telefono=1"); frames(ph, 10)
        pw = ph.evaluate("() => ({sw: document.documentElement.scrollWidth, iw: innerWidth, tg: (document.querySelector('#sb-toggle') || {innerText: ''}).innerText})")
        step("phone: no horizontal scroll and no «Files (N)» toggle", lambda: (_ for _ in ()).throw(AssertionError(pw)) if pw["sw"] > pw["iw"] or "(" in pw["tg"] else None)
        ph.close()
finally:
    r.close()
errs = [e for e in r.errs if e[0] == "pageerror"]
steps.append(("no page error", "ok" if not errs else "FAIL: " + str(errs[:2])))
for n_, s_ in steps: print(("OK   " if s_ == "ok" else "FAIL ") + n_ + ("" if s_ == "ok" else "  -> " + s_))
sys.exit(1 if any(s_ != "ok" for _, s_ in steps) else 0)
