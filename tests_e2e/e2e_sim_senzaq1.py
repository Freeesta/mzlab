"""SIM channels whose file gives no Q1 (synthetic SIM .wiff of tools/wiff_sim.py whose method names no ion, no laboratory data): the seven channels stay apart, each is called
«ione N» (never «null»), the picker has one row per channel, the chromatogram menu extracts one of them and the plot keeps one trace, the banner of a long list says «SIM files»,
and the method panel shows «—» for what the file does not say. Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_sim_senzaq1.py"""
import subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="sim_nq1_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
N = 7
r = Run(port=8876, wd="/tmp/wd_sim_nq1")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))
PANEL = "E.panels.find(p => p.type === 'mrm')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim_senzaq1.wiff", D / "sim_senzaq1.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 2 && E.files.every(f => f.sim > 0)", timeout=120000)
        step("every file counts its seven channels", lambda: need(pg.evaluate("E.files.every(f => f.sim === %d)" % N), pg.evaluate("E.files.map(f => f.sim)")))
        open_first(pg)
        pg.click("#np-mrm"); attendi(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        def names():
            t = pg.evaluate("SIMUI.ions(%s).map(x => SIMUI.label(x.key))" % PANEL)
            need(len(t) == N and len(set(t)) == N and all(x.startswith(("ione", "ion")) for x in t) and not any("null" in x for x in t), t)
        step("the seven channels are told apart and called «ione N»", names)
        def picker():
            pg.click(".can-b, .canb"); pg.wait_for_selector(".can-row", state="visible")
            rows = pg.evaluate("[...document.querySelectorAll('.can-pop')].map(e => [...e.querySelectorAll('.can-row .can-et')].map(x => x.textContent))")
            need(rows and all(len(x) == N and not any("null" in y for y in x) and all(y.startswith(("ione", "ion")) for y in x) for x in rows), rows)
        step("the picker has one row per channel", picker)
        pg.keyboard.press("Escape")
        def submenu():
            box = pg.locator("canvas").first.bounding_box()
            pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right")
            pg.wait_for_selector("#ctx:not([hidden])"); pg.hover("#ctx .hsub")
            need(pg.evaluate("document.querySelectorAll('#ctx .ctxsub:not([hidden]) [role=menuitemcheckbox]').length") == N + 1, "rows")
            need(not pg.evaluate("[...document.querySelectorAll('#ctx .ctxsub')].some(e => /null/.test(e.textContent))"), "null in the menu")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(3).click()
            attendi(pg, "%s._a && new Set(%s._a.sr.map(s => s.ion)).size === 1" % (PANEL, PANEL), timeout=30000)
        step("the menu extracts one channel and the plot keeps one ion", submenu)
        def banner():
            need(pg.evaluate("I18N.t('mrm.capNoticeSim', {n: 12, t: 27})").endswith("27 file SIM.") or "SIM files" in pg.evaluate("I18N.t('mrm.capNoticeSim', {n: 12, t: 27})"), "text")
        step("the long-list banner says SIM files", banner)
        def method():
            h = pg.evaluate("getMrm(E.files[0].k).then(t => SIMUI.methodTable(t))")
            need(h and "null" not in h and "—" in h and "Q3" not in h and "CE" not in h and h.count("<tr>") == N + 1, h)
        step("the method table of a SIM sample shows «—» and no Q3 or CE column", method)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
