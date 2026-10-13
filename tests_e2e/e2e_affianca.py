# e2e_affianca: grip of the panels: drag to a side zone, menu (no dragging), Alt+arrows, Esc, divider, undo (Ctrl+Z), narrow window
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8875, wd="/tmp/wd_affianca")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1}); addPanel('spec',{tab:E.tab,k:E.cur,level:1})")
        pg.wait_for_timeout(400)
        rows = lambda: pg.evaluate("(() => { const l = stackOrder().slice(-3).map(q => q.id); return LAYOUT.righeDi(stackOrder()).map(r => r.map(q => q.id).filter(i => l.includes(i))).filter(r => r.length) })()")
        rects = lambda: pg.evaluate("stackOrder().slice(-3).map(q => [q.id, q.x, q.y, q.w, q.h])")
        def grip_box(i):
            return pg.evaluate("(i) => { const q = stackOrder().at(i - 3); q.el.dispatchEvent(new MouseEvent('mouseover')); const b = q.el.querySelector('.aff-grip').getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2, b.width, b.height]; }", i)
        def drag_to_right_of(src, dst):
            b = grip_box(src)
            t = pg.evaluate("(i) => { const r = stackOrder().at(i - 3).el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }", dst)
            pg.mouse.move(b[0], b[1]); pg.mouse.down(); pg.mouse.move(b[0] + 20, b[1] + 20, steps=3)
            pg.mouse.move(t[0] + t[2] * 0.9, t[1] + t[3] * 0.5, steps=6)
            return t
        n0 = len(rows())
        def target_size():
            b = grip_box(0)
            assert b[2] >= 24 and b[3] >= 24, b
        step("grip has a target of at least 24 px", target_size)
        def drag():
            drag_to_right_of(2, 1)
            assert pg.evaluate("document.querySelector('.aff-prev') !== null"), "no preview"
            assert pg.evaluate("document.querySelector('.aff-prev').getAttribute('aria-hidden')") == "true"
            pg.mouse.up(); pg.wait_for_timeout(300)
            rr = rows(); assert [len(x) for x in rr] == [1, 2], rr
            g = rects(); assert g[1][2] == g[2][2] and g[1][1] < g[2][1], g
        step("drag the grip to the right zone of another panel: two side by side", drag)
        step("state is coherent", lambda: pg.evaluate("STATO.verifica()") == [] or (_ for _ in ()).throw(AssertionError("stato")))
        def undo():
            pg.keyboard.press("Control+z"); pg.wait_for_timeout(300)
            assert [len(x) for x in rows()] == [1, 1, 1], rows()
        step("Ctrl+Z restores the arrangement", undo)
        def esc():
            drag_to_right_of(2, 1); pg.keyboard.press("Escape"); pg.mouse.up(); pg.wait_for_timeout(200)
            assert [len(x) for x in rows()] == [1, 1, 1], rows()
            assert pg.evaluate("document.querySelector('.aff-prev') === null")
        step("Esc cancels the drag", esc)
        def menu():
            b = grip_box(2); pg.mouse.click(b[0], b[1]); pg.wait_for_timeout(100)
            assert pg.evaluate("!!document.querySelector('.aff-menu [role=menuitem]')")
            pg.click(".aff-menu button >> nth=-5"); pg.wait_for_timeout(300)
            assert [len(x) for x in rows()] == [1, 2], rows()
        step("menu 'Place beside' does it without dragging", menu)
        def equal_full():
            b = grip_box(2); pg.mouse.click(b[0], b[1]); pg.click(".aff-menu button >> nth=-2"); pg.wait_for_timeout(300)
            assert [len(x) for x in rows()] == [1, 1, 1], rows()
        step("menu 'Full width'", equal_full)
        def keys():
            g = grip_box(2); pg.evaluate("stackOrder().at(-1).el.querySelector('.aff-grip').focus()")
            pg.keyboard.press("Alt+ArrowLeft"); pg.wait_for_timeout(300)
            assert [len(x) for x in rows()] == [1, 2], rows()
            assert "2" in pg.inner_text("#aff-live") or pg.inner_text("#aff-live") != ""
            pg.keyboard.press("Alt+ArrowDown"); pg.wait_for_timeout(300)
            assert [len(x) for x in rows()] == [1, 1, 1], rows()
        step("Alt+arrows on the grip", keys)
        def divider():
            pg.evaluate("(() => { const s = stackOrder().slice(-3); const a = s[1], b = s[2]; AFFIANCA.fai(b, {k:'dx', t:a.id}, false); })()"); pg.wait_for_timeout(300)
            s = pg.query_selector(".aff-sep"); assert s, "no divider"
            assert pg.evaluate("document.querySelector('.aff-sep').getAttribute('role')") == "separator"
            w0 = pg.evaluate("stackOrder().at(-2).w")
            s.focus(); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(300)
            w1 = pg.evaluate("stackOrder().at(-2).w"); assert w1 == w0 + 16, (w0, w1)
            pg.keyboard.press("Enter"); pg.wait_for_timeout(300)
            a, b = pg.evaluate("stackOrder().slice(-2).map(q => q.w)"); assert abs(a - b) <= 8, (a, b)
        step("divider: keys change the widths, Enter makes them equal", divider)
        def narrow():
            pg.set_viewport_size({"width": 860, "height": 2200}); pg.wait_for_timeout(1000)
            g = rects(); assert g[1][3] == g[2][3] and g[1][2] != g[2][2], g
            assert [len(x) for x in rows()] == [1, 2], rows()           # arrangement kept
            pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(1000)
            g = rects(); assert g[1][2] == g[2][2] and g[1][3] < 700, g
        step("narrow window shows them stacked, the arrangement is kept", narrow)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
