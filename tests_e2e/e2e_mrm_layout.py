# e2e_mrm_layout: a file with only MRM opens only the MRM block (no empty graphs); one transitions list (no second selector, no extra integration dropdown); integrating in stacked or cascade returns to overlaid
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
VIS = "[...document.querySelectorAll('.pnl')].filter(e => e.offsetParent).map(e => [...e.classList].find(c => ['chrom','xic','spec','mrm','map'].includes(c)))"
r = Run(port=8879, wd="/tmp/wd_mrm_layout")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_MRM-STD_0_6ppm"), mz("B_MRM-t0")])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(1500)
        def only_mrm():
            v = pg.evaluate(VIS); assert v == ["mrm"], v
            assert pg.evaluate("E.panels.every(q => q.tab === 'mrm')"), "panels of another type"
        step("MRM-only files: only the MRM panel is open", only_mrm)
        def menu():
            q = "document.querySelector('.pnl.mrm')"
            assert pg.evaluate(f"!{q}.querySelector('select[data-o=tr]')"), "second transitions selector still there"
            assert pg.evaluate(f"!{q}.querySelector('[data-a=intf]') || {q}.querySelector('[data-a=intf]').hidden"), "extra integration target dropdown"
            assert not pg.evaluate(f"!!{q}.querySelector('select[data-o=fk]')"), "file counter dropdown"
            assert pg.evaluate(f"{q}.querySelector('.pn-t, .ptitle, header, .hd') ? {q}.textContent.includes('Transizioni MRM') === false : true"), "old panel title"
            pg.evaluate(f"{q}.querySelector('.canb').click()"); frames(pg, 2)
            n = pg.evaluate(f"[...{q}.querySelectorAll('.can-row .can-n,.can-row')].length"); assert n > 1, n
            lab = pg.evaluate(f"[...{q}.querySelectorAll('.can-row')].map(x => x.getAttribute('aria-label') || '')")
            assert not any("\u2192" in l or "@" in l for l in lab), lab                          # one notation: q1>q3 role
            assert len(set(lab)) == len(lab), lab
            ov = pg.evaluate(f"(() => {{ const hd = {q}.querySelector('.can-hd'); const c = [...hd.children].map(e => e.getBoundingClientRect()); return c.slice(1).some((r, i) => r.left < c[i].right - 1); }})()")
            assert not ov, "columns overlap"
        step("transitions: one list, no second selector, no extra integration dropdown, one notation", menu)
        def ring():
            ol = "getComputedStyle(document.querySelector('.pnl.mrm canvas')).outlineStyle"
            box = pg.evaluate("(() => { const r = document.querySelector('.pnl.mrm canvas').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()")
            pg.mouse.click(box[0], box[1]); frames(pg, 2)
            assert pg.evaluate(ol) == "none", "focus ring after a mouse click"
            pg.keyboard.press("Shift"); pg.evaluate("document.querySelector('.pnl.mrm canvas').blur(); document.querySelector('.pnl.mrm canvas').focus()"); frames(pg, 2)
            assert pg.evaluate(ol) != "none", "no focus ring for keyboard focus"
        step("canvas focus ring: none after a mouse click, shown for keyboard focus", ring)
        def stacked():
            for m in ("stk", "cas"):
                pg.evaluate(f"(() => {{ const p = E.panels.find(q => q.type === 'mrm'); p.mode = '{m}'; ctl(p); draw(p); p.imode = 'man'; }})()"); frames(pg, 2)
                assert pg.evaluate(f"E.panels.find(q => q.type === 'mrm').mode") == m
                pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'mrm'); p.el.querySelector('[data-a=iauto]').click(); })()"); frames(pg, 2)
                assert pg.evaluate("E.panels.find(q => q.type === 'mrm').mode") == "ovl", m
                assert pg.evaluate("E.panels.find(q => q.type === 'mrm').el.querySelector('[data-o=mode]').value") == "ovl"
        step("integrate tool in stacked or cascade goes back to overlaid", stacked)
        def mixed():
            pg2 = r.page(p)
            pg2.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_MRM-t0")])
            pg2.wait_for_timeout(1000); pg2.click("text=Carica dati"); ready(pg2); pg2.wait_for_timeout(1500)
            v = pg2.evaluate(VIS); assert "mrm" not in v and "chrom" in v and "spec" in v, v         # first load: only the first file of the list is open
            pg2.evaluate("setVisOf(E.files.filter(f => f.kind === 'mrm'), true)"); ready(pg2)         # ticking the MRM file adds its block
            v = pg2.evaluate(VIS); assert "mrm" in v and "chrom" in v and "spec" in v, v
        step("full scan + MRM: the blocks with data stay", mixed)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
