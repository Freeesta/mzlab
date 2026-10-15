# e2e_affianca_zone: drop zones of the grip in the 4 directions of the target panel (above, below, left, right), each with its own label in the preview
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8879, wd="/tmp/wd_affianca_zone")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("E.panels.filter(q => q.type === 'spec').slice(1).forEach(q => q.close && q.close())"); attendi(pg, "stackOrder().length === 2")
        order = lambda: pg.evaluate("stackOrder().map(q => q.id)")
        rows = lambda: pg.evaluate("LAYOUT.righeDi(stackOrder()).map(r => r.map(q => q.id))")
        def drag(src, dst, fx, fy, release=True):
            seq = pg.evaluate("HSEQ")
            attendi(pg, "stackOrder().every(q => Math.abs(q.el.getBoundingClientRect().top - document.querySelector('#dpanels').getBoundingClientRect().top - q.y) < 1.5)")       # the last move has settled
            b = pg.evaluate("(i) => { const q = stackOrder()[i]; q.el.dispatchEvent(new MouseEvent('mouseover')); const g = q.el.querySelector('.aff-grip').getBoundingClientRect(); return [g.x + g.width / 2, g.y + g.height / 2]; }", src)
            t = pg.evaluate("(i) => { const r = stackOrder()[i].el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }", dst)
            pg.mouse.move(*b); pg.mouse.down(); pg.mouse.move(b[0] + 20, b[1] + 20, steps=3)
            pg.mouse.move(t[0] + t[2] * fx, t[1] + t[3] * fy, steps=6); attendi(pg, "document.querySelector('.aff-prev') !== null")
            lab = pg.evaluate("(() => { const e = document.querySelector('.aff-prev'); return e ? e.textContent : null })()")
            if release: pg.mouse.up(); attendi(pg, f"HSEQ > {seq} && !document.querySelector('.aff-prev')")
            return lab
        ids = order(); assert len(ids) == 2, ids
        top, bot = ids
        def above():
            lab = drag(1, 0, 0.5, 0.1); assert lab == "Sopra", lab
            assert order() == [bot, top], order()
        step("lower panel dropped on the top of the upper one goes above it (label «Sopra»)", above)
        def below():
            lab = drag(1, 0, 0.5, 0.9); assert lab == "Sotto", lab
            assert order() == [bot, top], order()
        step("dropped on the bottom of the upper panel it goes below it (label «Sotto»)", below)
        def corner():
            lab = drag(1, 0, 0.05, 0.05); assert lab == "Sopra", lab
            assert order() == [top, bot], order()
        step("the top corners are also «above»; the TIC dropped above its own spectrum goes alone past it", corner)
        def keys():
            pg.evaluate("stackOrder()[0].el.querySelector('.aff-grip').focus()"); pg.keyboard.press("Alt+ArrowDown"); attendi(pg, f"stackOrder()[0].id === {bot}")
            assert order() == [bot, top], order()
            pg.evaluate("stackOrder()[1].el.querySelector('.aff-grip').focus()"); pg.keyboard.press("Alt+ArrowUp"); attendi(pg, f"stackOrder()[0].id === {top}")
            assert order() == [top, bot], order()
        step("Alt+arrows swap the TIC with its own spectrum too", keys)
        def left():
            lab = drag(1, 0, 0.2, 0.5); assert lab == "A sinistra", lab
            assert [len(x) for x in rows()] == [2], rows()
        step("left of the target: side by side, dragged on the left", left)
        def right():
            lab = drag(0, 1, 0.8, 0.5, release=False); assert lab == "A destra", lab
            pg.keyboard.press("Escape"); pg.mouse.up()
        step("right of the target: label «A destra»", right)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
