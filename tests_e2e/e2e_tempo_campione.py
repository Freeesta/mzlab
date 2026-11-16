"""The time point written in a sample name (t0, 3h, 8h, ...) on the built site with the Rust engine: the loading table proposes it (yellow, editable), the file list shows a small muted tag
beside the name of a SIM file («3h», with an accessible label), a name that only looks like a time (H20, a 90-10 ratio, a blank) has none, a time typed in the table replaces the proposal and the tag,
and «Group by» offers the time. Synthetic SIM .wiff of tools/wiff_sim.py (sim_tempo: generic labels, invented numbers). Needs the site and the .wasm (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_tempo_campione.py"""
# covers: mzlab/web/explore.js, mzlab/web/ordine.js, mzlab/web/ruoli-carica.js, crates/mzlab-core/src/samples/names.rs, tools/wiff_sim.py
import subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import *
lib.AUTO_TIMES = False       # the times must come from the names, not from the helper that fills the missing ones

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="sim_tempo_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
r = Run(port=8933, wd="/tmp/wd_tempo")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))
# name of the sample -> (minutes proposed, tag in the file list)
WANT = {"blk 90-10": (None, None), "RET t0 (2)": (0, "t0"), "RET 3h_2": (180, "3h"), "RET 8h (2)": (480, "8h"), "PERM 3h": (180, "3h"), "H20 demi": (None, None)}
TAGS = "Object.fromEntries([...document.querySelectorAll('#flst .fl')].map(e => [e.querySelector('.nm').textContent, [...e.querySelectorAll('small abbr.ftag')].map(a => a.textContent)]))"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim_tempo.wiff", D / "sim_tempo.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); wait_for_js(pg, "E.files.length >= %d && E.files.every(f => f.sim > 0)" % len(WANT), timeout=120000)
        pg.wait_for_timeout(500)
        def proposed():
            got = pg.evaluate("Object.fromEntries(E.files.map(f => [f.label, f.time ?? null]))")
            for name, (minutes, _) in WANT.items():
                need(got.get(name) == minutes, (name, got.get(name), minutes))
        step("the engine reads t0, 3h and 8h from the names and nothing from H20 or 90-10", proposed)
        def tags():
            t = pg.evaluate(TAGS)
            for name, (_, tag) in WANT.items():
                if tag: need(t.get(name) == [tag], (name, t.get(name)))
            need(t["blk 90-10"] == ["blk"], t["blk 90-10"])
            need(t["H20 demi"] == [], t["H20 demi"])
        step("the file list tags a time point beside the name (only when one was read)", tags)
        def label():
            lab = pg.evaluate("[...document.querySelectorAll('#flst .fl')].filter(e => e.querySelector('.nm').textContent === 'RET 3h_2').map(e => { const a = e.querySelector('small abbr.ftag'); return [a.getAttribute('aria-label'), a.title]; })[0]")
            need(lab and lab[0] and lab[0] == lab[1] and "3" in lab[0], lab)
        step("the tag has an accessible label naming the time", label)
        def cls():
            need(pg.evaluate("(() => { const a = document.querySelector('#flst .fl small abbr.ftag'); return !!a && getComputedStyle(a).textDecorationLine === 'none'; })()"), "the tag is a plain muted abbr")
        step("the tag looks like the «blk» one", cls)
        def table():
            vals = pg.evaluate("Object.fromEntries([...document.querySelectorAll('#flist input[data-k=time]')].map(i => [ST.files[+i.dataset.i].name.replace(/\\.mzML$/, ''), [i.value, i.classList.contains('guess')]]))")
            need(vals["RET 3h_2"] == ["180", True] and vals["RET t0 (2)"] == ["0", True] and vals["PERM 3h"][0] == "180", vals)
            need(vals["H20 demi"][0] == "", vals["H20 demi"])
        step("the table shows the proposed time in yellow, empty for H20", table)
        def group():
            ans = pg.evaluate("carica('gruppi.js').then(() => GRUPPI.vars())")
            v = [x for x in ans["variabili"] if x["id"] == "time"]
            need(v, [x["id"] for x in ans["variabili"]])
            labels = {x["etichetta"]: len(x["file"]) for x in v[0]["valori"]}
            need(labels == {"t0": 1, "3h": 2, "8h": 1}, labels)
        step("Group by offers the time", group)
        def override():
            drawer(pg, "stat"); wait_for_js(pg, "[...document.querySelectorAll('#cmp-tab input[data-m=time]')].some(i => i.offsetWidth > 0)", 8000)
            box = lambda n: pg.locator("#cmp-tab input[data-m=time][data-f='%s.mzML']" % n)
            box("H20 demi").fill("90"); box("H20 demi").press("Tab")
            wait_for_js(pg, "E.files.find(f => f.label === 'H20 demi').time === 90")
            need(pg.evaluate(TAGS)["H20 demi"] == ["90min"], pg.evaluate(TAGS)["H20 demi"])
            box("RET 3h_2").fill(""); box("RET 3h_2").press("Tab")
            wait_for_js(pg, "E.files.find(f => f.label === 'RET 3h_2').time == null")
            need(pg.evaluate(TAGS)["RET 3h_2"] == [], pg.evaluate(TAGS)["RET 3h_2"])
        step("a time typed in the sample table replaces the tag, an emptied one removes it", override)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
