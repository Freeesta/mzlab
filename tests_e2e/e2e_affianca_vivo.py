# e2e_affianca_vivo: live reorder preview of the panel grip: the other panels slide aside while a panel is dragged (mouse, touch, keyboard), a dashed box marks
# the drop, Esc cancels and everything returns; at 1400 and 390 px. MZLAB_SHOT_DIR=dir also writes the mid-drag screenshots (deviceScaleFactor 2) there.
# covers: mzlab/web/arrange.js
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
SHOTS = os.environ.get("MZLAB_SHOT_DIR")
r = Run(port=8881, wd="/tmp/wd_affianca_vivo")
try:
    with sync_playwright() as p:
        pg = r.page(p, touch=True, device_scale_factor=2)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.click("text=Load data"); ready(pg)
        pg.evaluate("E.panels.filter(q => q.type === 'spec').slice(1).forEach(q => q.close && q.close())"); wait_for_js(pg, "stackOrder().length === 2")
        order = lambda: pg.evaluate("stackOrder().map(q => q.id)")
        settled = "stackOrder().every(q => !q.el.style.transform && Math.abs(q.el.getBoundingClientRect().top - document.querySelector('#dpanels').getBoundingClientRect().top - q.y) < 1.5) && !document.querySelector('.arrange-prev') && !document.querySelector('#dpanels.arrange-live')"
        rect = lambda i: pg.evaluate("(i) => { const r = stackOrder()[i].el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }", i)
        top, bot = order()
        def grip(i):
            return pg.evaluate("(i) => { const q = stackOrder()[i]; q.el.dispatchEvent(new MouseEvent('mouseover')); const g = q.el.querySelector('.arrange-grip').getBoundingClientRect(); return [g.x + g.width / 2, g.y + g.height / 2]; }", i)
        live = lambda: pg.evaluate("document.querySelector('#arrange-live').textContent")
        def shot(name):
            if SHOTS: os.makedirs(SHOTS, exist_ok=True); pg.screenshot(path=os.path.join(SHOTS, name + ".png"))
        for W in (1400, 390):
            pg.set_viewport_size({"width": W, "height": 2200}); wait_for_js(pg, settled)
            tag = f"{W}px: "
            def lower():
                wait_for_js(pg, settled); t0, b0 = rect(0), rect(1)
                g = grip(0); pg.mouse.move(*g); pg.mouse.down(); pg.mouse.move(g[0] + 20, g[1] + 20, steps=3)
                pg.mouse.move(b0[0] + b0[2] * 0.5, b0[1] + b0[3] * 0.9, steps=6)
                wait_for_js(pg, "document.querySelector('.arrange-prev') !== null")
                b1, prev, d1 = rect(1), pg.evaluate("(() => { const r = document.querySelector('.arrange-prev').getBoundingClientRect(); return [r.x, r.y, r.width, r.height] })()"), rect(0)
                assert b1[1] < b0[1] - 5, ("panel 2 did not move up during the drag", b0, b1)
                assert prev[1] >= b1[1] + b1[3] - 1, ("the placeholder is not below the panel that moved up", prev, b1)
                assert abs(prev[1] - (t0[1] + b0[3] + (b0[1] - t0[1] - t0[3]))) < 3, ("the placeholder is not at the final position", prev, t0, b0)
                assert abs(d1[1] - t0[1]) > 5, ("the dragged panel does not follow the pointer", d1, t0)
                shot(f"drag-{W}")
                pg.mouse.up(); wait_for_js(pg, settled)
                assert order() == [bot, top], order()
                assert abs(rect(1)[1] - (b0[1] + b0[3] + (b0[1] - t0[1] - t0[3]))) < 3, ("the panel did not land at the placeholder", rect(1))
            step(tag + "TIC dragged below its spectrum: the spectrum moves up during the drag, the placeholder is at the final position, the drop lands there", lower)
            def back():
                wait_for_js(pg, settled); b0 = rect(1); g = grip(0)       # now the spectrum is first
                pg.mouse.move(*g); pg.mouse.down(); pg.mouse.move(g[0] + 20, g[1] + 20, steps=3); pg.mouse.move(b0[0] + b0[2] * 0.5, b0[1] + b0[3] * 0.9, steps=6)
                wait_for_js(pg, "document.querySelector('.arrange-prev') !== null")
                pg.keyboard.press("Escape"); wait_for_js(pg, settled); pg.mouse.up()
                assert order() == [bot, top], order()
                assert pg.evaluate("stackOrder().every(q => !q.el.style.transform && !q.el.classList.contains('arrange-drag'))")
            step(tag + "Esc during the drag puts everything back", back)
            def touch():
                wait_for_js(pg, settled); b0 = rect(1); g = grip(0); cdp = pg.context.new_cdp_session(pg)
                pt = lambda x, y: [{"x": x, "y": y, "id": 1}]
                cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": pt(*g)})
                for k in range(1, 7): cdp.send("Input.dispatchTouchEvent", {"type": "touchMove", "touchPoints": pt(g[0] + (b0[0] + b0[2] * 0.5 - g[0]) * k / 6, g[1] + (b0[1] + b0[3] * 0.9 - g[1]) * k / 6)})
                wait_for_js(pg, "document.querySelector('.arrange-prev') !== null")
                cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []}); wait_for_js(pg, settled)
                assert order() == [top, bot], order()
            step(tag + "touch: the same drag with a finger moves the panel back below", touch)
            def keys():
                wait_for_js(pg, settled); b0 = rect(1)
                pg.evaluate("stackOrder()[0].el.querySelector('.arrange-grip').focus()"); pg.keyboard.press("Space")
                assert "picked up" in live(), live()
                assert not pg.evaluate("document.querySelector('.arrange-menu')"), "Space opened the menu"
                pg.keyboard.press("ArrowDown"); wait_for_js(pg, "document.querySelector('.arrange-prev') !== null")
                assert rect(1)[1] < b0[1] - 5, ("the other panel did not move up", b0, rect(1))
                wait_for_js(pg, "document.querySelector('#arrange-live').textContent.includes('row')")
                shot(f"keys-{W}")
                pg.keyboard.press("Escape"); wait_for_js(pg, settled)
                assert order() == [top, bot], order(); assert "cancelled" in live(), live()
                pg.keyboard.press("Space"); pg.keyboard.press("ArrowDown"); wait_for_js(pg, "document.querySelector('.arrange-prev') !== null"); pg.keyboard.press("Space")
                wait_for_js(pg, f"stackOrder()[0].id === {bot}"); wait_for_js(pg, settled)
                assert order() == [bot, top], order()
                assert pg.evaluate("document.activeElement.classList.contains('arrange-grip')"), "focus lost"
                pg.keyboard.press("Space"); pg.keyboard.press("ArrowDown"); wait_for_js(pg, "document.querySelector('.arrange-prev') !== null"); pg.keyboard.press("Space")
                wait_for_js(pg, f"stackOrder()[0].id === {top}"); wait_for_js(pg, settled)
            step(tag + "keyboard: Space picks up (no menu), ArrowDown moves with the preview and an announcement, Escape cancels, Space drops", keys)
            def calm():
                # the test browser asks for reduced motion: the others jump, no transition
                wait_for_js(pg, settled); g = grip(0); b0 = rect(1)
                pg.mouse.move(*g); pg.mouse.down(); pg.mouse.move(g[0] + 20, g[1] + 20, steps=3); pg.mouse.move(b0[0] + b0[2] * 0.5, b0[1] + b0[3] * 0.9, steps=4)
                wait_for_js(pg, "document.querySelector('.arrange-prev') !== null")
                dur = pg.evaluate("getComputedStyle(stackOrder()[1].el).transitionDuration")
                pg.keyboard.press("Escape"); wait_for_js(pg, settled); pg.mouse.up()
                assert set(dur.split(", ")) == {"0s"}, dur
            step(tag + "reduced motion: no animation, only the jump", calm)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
