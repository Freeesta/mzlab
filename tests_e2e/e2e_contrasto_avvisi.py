"""Warning / info / error / success boxes (class .avv) and the «Acquisition method» window: text readable (>= 4.5:1) in the light and in the dark theme.
Checks the contrast of every box variant from its computed colours, and axe-core (color-contrast) on the method window when axe is available."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_contrasto_avvisi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))
AXE = axe_path()
SHOTS = os.environ.get("MZ_SHOTS")
# contrast of the text of a box over its own background, composited on the panel (the tints are opaque color-mix results)
RATIO = """() => {
  const lum = c => { const m = c.match(/[\\d.]+/g).map(Number); if (c.startsWith('color(')) for (let i = 0; i < 3; i++) m[i] *= 255; const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : Math.pow((v + .055) / 1.055, 2.4); }; return .2126 * f(m[0]) + .7152 * f(m[1]) + .0722 * f(m[2]); };
  const host = document.body, out = [];
  for (const k of ["", "warn", "bad", "ok", "info"]) {
    const d = document.createElement("div"); d.className = "avv " + k; d.textContent = "x"; host.appendChild(d);
    const cs = getComputedStyle(d), a = lum(cs.color), b = lum(cs.backgroundColor);
    out.push([k || "base", Math.round(((Math.max(a, b) + .05) / (Math.min(a, b) + .05)) * 100) / 100, cs.backgroundColor]);
    d.remove();
  }
  return out;
}"""
r = Run(port=8868, wd="/tmp/wd_avvisi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(800)
        if AXE: pg.add_script_tag(path=AXE)
        for theme in ("light", "dark"):
            pg.evaluate(f"document.documentElement.setAttribute('data-theme','{theme}')"); pg.wait_for_timeout(200)
            def boxes():
                bad = [x for x in pg.evaluate(RATIO) if x[1] < 4.5]; assert not bad, bad
            step(f"{theme}: every box variant has contrast >= 4.5:1", boxes)
            def method():
                pg.click("#np-method"); pg.wait_for_selector(".avv.warn"); pg.wait_for_timeout(300)
                if SHOTS: pg.screenshot(path=os.path.join(SHOTS, f"metodo-{theme}.png"))
                if AXE:
                    v = pg.evaluate("axe.run(document, {runOnly: {type: 'rule', values: ['color-contrast']}}).then(r => r.violations.map(v => v.nodes.map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,2).join(' | '))).flat())")
                    assert not v, v
            step(f"{theme}: the «Acquisition method» window, warning box included, is readable", method)
            pg.keyboard.press("Escape")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
