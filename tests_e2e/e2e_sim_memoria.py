"""The SIM view remembers the ion the student chose (synthetic SIM .wiff files of tools/wiff_sim.py, no laboratory data): with ion B picked (not the first), a sample
unticked and ticked again opens on ion B; with two samples ticked ion B applies to both; a sample removed and another one with ion B ticked opens on ion B; a sample
of another list (no ion B) opens on the default, with no error. Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root:
python3 tests_e2e/e2e_sim_memoria.py"""
# covers: mzlab/web/tabs.js
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="sim_mem_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
B = "253.1"
r = Run(port=8876, wd="/tmp/wd_sim_memoria")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim.wiff", D / "sim.wiff.scan", D / "sim_listab.wiff", D / "sim_listab.wiff.scan"]); pg.click("text=Carica dati"); ready(pg)
        wait_for_js(pg, "E.files.length >= 6 && E.files.every(f => f.sim > 0)", timeout=120000)
        K = pg.evaluate("Object.fromEntries(E.files.map(f => [f.file, f.k]))")
        S1, S2, OTHER = K["sim_s1.mzML"], K["sim_s2.mzML"], K["B_T10.mzML"]
        open_first(pg)
        PP = "E.panels.find(p => p.type === 'mrm' && p.el && p.el.isConnected)"
        Q = PP + ".el"
        tr = lambda: json.loads(pg.evaluate("JSON.stringify(%s.tr)" % PP))
        box = lambda k: pg.click('#flst input[data-k="%d"]' % k)
        def drawn(ks):
            wait_for_js(pg, "(() => { const p = %s; return !!(p && p._a && p._a.sr.length && %s.every(k => p._a.sr.some(s => s.k === k))); })()" % (PP, json.dumps(ks)), timeout=60000)
            return pg.evaluate("[...new Set(%s._a.sr.map(s => s.ion))]" % PP)
        def gone():
            wait_for_js(pg, "!E.panels.some(p => p.type === 'mrm' && p.el && p.el.isConnected)", timeout=30000)
        def first():
            ions = drawn([S1])
            need(tr() != [B] and B not in ions, (tr(), ions))
        step("the view opens on its default, not on ion B", first)
        def pick():
            pg.click(".canb"); pg.wait_for_selector(".can-row", state="visible")
            pg.evaluate("[...%s.querySelectorAll('.can-row')].find(r => r.textContent.includes('%s')).querySelector('.can-et').click()" % (Q, B)); frames(pg, 2)
            pg.keyboard.press("Escape")
            need(tr() == [B], tr()); need(drawn([S1]) == [B], drawn([S1]))
        step("the student picks ion B", pick)
        def again():
            box(S1); gone()
            box(S1); need(drawn([S1]) == [B], drawn([S1])); need(tr() == [B], tr())
        step("the sample unticked and ticked again opens on ion B", again)
        def overlay():
            box(S2); ions = drawn([S1, S2])
            need(ions == [B], ions)
        step("with two samples ticked ion B applies to both", overlay)
        def removed():
            box(S2); frames(pg, 2)
            pg.evaluate("document.querySelector('#flst [data-rm=\"%d\"]').click()" % S1); gone()
            box(S2); need(drawn([S2]) == [B], drawn([S2])); need(tr() == [B], tr())
        step("a sample removed, another sample with ion B ticked: ion B", removed)
        def other():
            box(S2); gone()
            box(OTHER); ions = drawn([OTHER])
            need(B not in ions and B not in (tr() or []), (tr(), ions))
            need(not [x for x in r.errs if x[0] == "pageerror"], r.errs)
        step("a sample without ion B opens on the default, with no error", other)
        def back():
            box(OTHER); gone()
            box(S2); need(drawn([S2]) == [B], drawn([S2]))
        step("the default of the other list does not replace the choice: ion B again", back)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
