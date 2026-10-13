"""Hint under the toolbar: shown only the FIRST time the user hides the tools (qqq.barra.hint, marked at once, so never again, also after a reload),
closes by itself after 10 s, with ×, Esc or the first tool opened; polite live region."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
def load(pg):
    pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); pg.wait_for_timeout(800)
    pg.click("text=Carica dati"); ready(pg)
def fresh(p, tour_state=True, hint=None):
    r = Run(port=8872, wd="/tmp/wd_hint"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": 1440, "height": 900}, locale="it-IT", reduced_motion="reduce"); pg = ctx.new_page()
    init = "try{" + ("localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));" if tour_state else "") + (f"localStorage.setItem('qqq.barra.hint','{hint}');" if hint else "") + "}catch(_){}"
    pg.add_init_script(init); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
    return r, ctx, pg
vis = lambda pg: pg.evaluate("(() => { const e = document.getElementById('bar-hint'); if (!e) return null; const b = e.getBoundingClientRect(), s = document.getElementById('stbar').getBoundingClientRect(); return {w: b.width, below: b.top >= s.bottom, live: e.getAttribute('aria-live'), role: e.getAttribute('role'), inside: b.right <= innerWidth && b.left >= 0}; })()")
shut = lambda pg: (pg.click("#sb-toggle"), pg.wait_for_timeout(400))
with sync_playwright() as p:
    def first():
        r, ctx, pg = fresh(p)
        try:
            load(pg); pg.wait_for_timeout(2000)
            assert vis(pg) is None and pg.evaluate("localStorage.getItem('qqq.barra.hint')") is None, "the hint must not appear at load"
            shut(pg)
            v = vis(pg); assert v and v["w"] > 100 and v["below"] and v["inside"] and v["live"] == "polite" and v["role"] == "status", v
            assert pg.inner_text("#bar-hint").startswith("Gli strumenti sono qui")
            assert pg.evaluate("localStorage.getItem('qqq.barra.hint')") == "1", "marked as seen when shown"
            ar = pg.evaluate("(() => { const h = document.getElementById('bar-hint'), t = document.getElementById('sb-toggle').getBoundingClientRect(); const x = parseFloat(getComputedStyle(h).getPropertyValue('--ha')) + 5 + h.getBoundingClientRect().left; return [x, t.left, t.right]; })()")
            assert ar[1] - 2 <= ar[0] <= ar[2] + 2, ("the arrow does not point at the Panel button", ar)
            hold(pg, 8500); assert vis(pg), "still there before 10 s"
            hold(pg, 2500); assert vis(pg) is None, "gone by itself after 10 s"
        finally: ctx.close(); r.close()
    step("first hide of the tools shows it, it disappears by itself after 10 s", first)
    def second():
        r, ctx, pg = fresh(p)
        try:
            load(pg); pg.wait_for_timeout(1500)
            shut(pg); assert vis(pg); pg.click("#bar-hint button"); pg.wait_for_timeout(300); assert vis(pg) is None      # × closes it
            shut(pg); shut(pg); assert vis(pg) is None, "the second hide does not show it"
            pg.reload(); pg.wait_for_timeout(2000); shut(pg); assert vis(pg) is None, "not after a reload either"
        finally: ctx.close(); r.close()
    step("× closes it; the second hide and a reload never show it again", second)
    def esc_tool():
        r, ctx, pg = fresh(p)
        try:
            load(pg); pg.wait_for_timeout(1500); shut(pg); assert vis(pg)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert vis(pg) is None
            assert pg.evaluate("BARRA.commands.every(c=>c.id!=='hint')"), "no palette command for the hint"
            pg.evaluate("BARRA.hintShow()"); pg.wait_for_timeout(500); assert vis(pg), "hintShow did not reopen it"
            pg.click("#stbar [data-cmd=adducts]"); pg.wait_for_timeout(500); assert vis(pg) is None      # first tool closes it
        finally: ctx.close(); r.close()
    step("Esc closes, hintShow reopens (no palette command), first tool closes", esc_tool)
    def seen():
        r, ctx, pg = fresh(p, hint="1")
        try: load(pg); pg.wait_for_timeout(1500); shut(pg); assert vis(pg) is None
        finally: ctx.close(); r.close()
    step("not shown when already seen", seen)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
