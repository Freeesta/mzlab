"""Flat settings panel: no language switch, «Data and calculation» and the tour button visible
without expanding anything, no fold around a single control, no overflow at 360 px, axe clean (light, dark, high contrast). Synthetic data."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_impostazioni_piatte.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
AXE = axe_path()
RUN = ("axe.run('#uipset', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations"
       ".filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.slice(0,3).map(n => n.target.join(' '))]))")
r = Run(port=8884, wd="/tmp/wd_imp_piatte")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        shutil.rmtree("/tmp/s_imp_piatte", ignore_errors=True)
        stage(pg, synth.make_series("/tmp/s_imp_piatte", [0, 15, 60])); pg.click("text=Load data"); ready(pg)
        def opened():
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            assert pg.locator("#uipset details.imp-g").count() == 0, "no fold in the plain panel"
            assert pg.is_visible("#uip-merge") and pg.is_visible("#uip-tour"), "«Data and calculation» switch and the tour button show without expanding"
            assert pg.locator("#uip-g-data > h3, #uip-g-data h3").count() == 0, "a single control has no group title"
            assert pg.is_visible("#uip-bugbtn") and pg.get_attribute("#uip-bugbtn", "aria-expanded") == "false"
            assert pg.locator("#uipset .seglang, input[name=uip-lang]").count() == 0, "no language switch: the program is English only"
        step("flat panel: nothing folded, no language switch", opened)
        def bug_toggle():
            pg.click("#uip-bugbtn"); pg.wait_for_selector("#bug-what", timeout=10000); assert pg.get_attribute("#uip-bugbtn", "aria-expanded") == "true"
            pg.click("#uip-bugbtn"); assert pg.is_hidden("#uip-bug") and pg.get_attribute("#uip-bugbtn", "aria-expanded") == "false"
        step("bug report: plain button opens and closes the form", bug_toggle)
        def narrow():
            for w in (360, 1400):
                pg.set_viewport_size({"width": w, "height": 800}); wait_for_js(pg, f"innerWidth === {w}")
                m = pg.evaluate("(() => { const d = document.querySelector('#uipset'); return d.scrollWidth - d.clientWidth; })()")
                assert m <= 0, (w, m)
        step("no horizontal overflow at 360 and 1400 px", narrow)
        if AXE:
            def axe_themes():
                pg.click("#np-set"); pg.wait_for_selector("#uipset")
                if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=AXE)
                for t, bg in (("light", ""), ("dark", ""), ("light", "hc")):
                    pg.evaluate("t => { document.documentElement.dataset.theme = t[0]; if (t[1]) document.documentElement.dataset.a11yBg = t[1]; else delete document.documentElement.dataset.a11yBg; }", [t, bg])
                    v = pg.evaluate(RUN); assert not v, (t, bg, v)
            step("axe on the panel: light, dark, high contrast", axe_themes)
finally:
    r.close(); r.report()
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
