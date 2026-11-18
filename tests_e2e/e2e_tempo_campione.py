"""The time point written in a sample name (t0, 3h, 8h, ...) on the built site with the Rust engine: the loading table proposes it (yellow, editable), the file list shows a small muted tag
beside the name of a SIM file («180min», in the unit of the table, with an accessible label), a name that only looks like a time (H20, a 90-10 ratio, a blank) has none, a time typed in the table replaces the proposal and the tag,
«Group by» offers the time, and the Manual-integrations table has a Time column after the sample (tag on screen, minutes in the CSV). Synthetic SIM .wiff of tools/wiff_sim.py (sim_tempo: generic labels, invented numbers). Needs the site and the .wasm (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_tempo_campione.py"""
# covers: mzlab/web/integrazioni-manuali.js, mzlab/web/explore.js, mzlab/web/ordine.js, mzlab/web/ruoli-carica.js, crates/mzlab-core/src/samples/names.rs, tools/wiff_sim.py
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
WANT = {"blk 90-10": (None, None), "RET t0 (2)": (0, "t0"), "RET 3h_2": (180, "180min"), "RET 8h (2)": (480, "480min"), "PERM 3h": (180, "180min"), "H20 demi": (None, None)}
TAGS = "Object.fromEntries([...document.querySelectorAll('#flst .fl')].map(e => [e.querySelector('.nm').textContent, [...e.querySelectorAll('small abbr.ftag')].map(a => a.textContent)]))"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim_tempo.wiff", D / "sim_tempo.wiff.scan"]); pg.click("text=Load data"); ready(pg); wait_for_js(pg, "E.files.length >= %d && E.files.every(f => f.sim > 0)" % len(WANT), timeout=120000)
        wait_for_js(pg, "document.querySelectorAll('#flst .fl small abbr.ftag').length >= 5 && ST.files.every(f => f.prop)", timeout=30000)
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
            need(lab and lab[0] and lab[0] == lab[1] and "180" in lab[0], lab)
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
        def unit():
            sel = lambda u: (pg.select_option("#cmp-tu", u), wait_for_js(pg, "tunitGet() === '%s'" % u))
            sel("h")
            need(pg.evaluate(TAGS)["PERM 3h"] == ["3h"] and pg.evaluate(TAGS)["RET 8h (2)"] == ["8h"] and pg.evaluate(TAGS)["H20 demi"] == ["1.5h"] and pg.evaluate(TAGS)["RET t0 (2)"] == ["t0"], pg.evaluate(TAGS))
            sel("min")
            need(pg.evaluate(TAGS)["PERM 3h"] == ["180min"] and pg.evaluate(TAGS)["RET 8h (2)"] == ["480min"] and pg.evaluate(TAGS)["H20 demi"] == ["90min"], pg.evaluate(TAGS))
        step("the tags in the file list follow the Time unit selector of the sample table, live", unit)
        def manint():
            drawer(pg)
            pg.click("#flst .fl >> text=PERM 3h"); wait_for_js(pg, "E.files.some(f => f.vis && !f.gone)", 30000)
            pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
            pg.evaluate("(() => { E.browse = false; E.files.forEach(f => { f.vis = true; }); const p = E.panels.find(q => q.type === 'mrm'); p.tr = ''; draw(p); })()")
            wait_for_js(pg, "(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && new Set(p._a.sr.map(s => s.k)).size === E.files.length; })()", timeout=60000)
            for n in ("PERM 3h", "RET t0 (2)", "H20 demi", "RET 3h_2"):      # the last two: a time typed above, a time emptied above
                pg.evaluate("n => { const p = E.panels.find(q => q.type === 'mrm' && q._a); const f = E.files.find(x => x.label === n); addInt(p, p._a.sr.filter(x => x.k === f.k)[0], 1.0, 1.3, false); }", n)
            wait_for_js(pg, "document.querySelectorAll('#manint tbody tr[data-org=manual]').length === 4", 60000)
            col = lambda: pg.evaluate("Object.fromEntries([...document.querySelectorAll('#manint tbody tr[data-org=manual]')].map(r => [r.children[1].textContent, r.children[2].textContent]))")
            need(col() == {"PERM 3h": "3h", "RET t0 (2)": "t0", "H20 demi": "90min", "RET 3h_2": ""}, col())
            need(pg.evaluate("document.querySelector('#manint thead th:nth-child(3)').textContent") == pg.evaluate("I18N.t('canali.manint.h.time')"), "the Time header sits after the sample")
            pg.evaluate("document.querySelector('#manint').scrollIntoView()")
            with pg.expect_download() as d: pg.click("#manint [data-dl]")
            rows = Path(d.value.path()).read_text(encoding="utf-8").strip().split("\n")
            head = rows[0] if not rows[0].startswith("# mzLab ") else rows[1]
            need(head.split(",")[:3] == ["channel", "sample", "time_min"], head)
            tm = {l.split(",")[1]: l.split(",")[2] for l in rows if l.split(",")[1:2] and l.startswith(("253", "251", "2"))}
            need(tm.get("PERM 3h") == "180" and tm.get("RET t0 (2)") == "0" and tm.get("H20 demi") == "90" and tm.get("RET 3h_2") == "", tm)      # minutes in the file, tags only on screen
            drawer(pg, "stat"); wait_for_js(pg, "[...document.querySelectorAll('#cmp-tab input[data-m=time]')].some(i => i.offsetWidth > 0)", 8000)
            pg.locator("#cmp-tab input[data-m=time][data-f='H20 demi.mzML']").fill("30"); pg.keyboard.press("Tab")
            wait_for_js(pg, "[...document.querySelectorAll('#manint tbody tr[data-org=manual]')].some(r => r.children[1].textContent === 'H20 demi' && r.children[2].textContent === '30min')", 30000)
        step("the Manual-integrations table shows the time of each sample, in minutes in the CSV, and follows an edit of the sample table", manint)
        def views():
            # the Files panel, the sample table and the loading table are painted from the same state: every edit shows in all of them at once
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            drawer(pg, "stat"); wait_for_js(pg, "!!document.querySelector('#cmp-tab')", 8000)
            cmp_names = "[...document.querySelectorAll('#cmp-tab tbody tr td:first-child')].map(td => td.textContent.replace(/^\\s*\\d+\\s*/, '').trim())"
            side = "[...document.querySelectorAll('#flst .fl .nm')].map(e => e.textContent)"
            # role: set in the sample table -> tag in the panel and role in the loading table
            pg.select_option("#cmp-tab select[data-m=type][data-f='PERM 3h.mzML']", "blank")
            wait_for_js(pg, "[...document.querySelectorAll('#flst .fl')].some(e => e.querySelector('.nm').textContent === 'PERM 3h' && [...e.querySelectorAll('small abbr.ftag')].some(a => a.textContent === 'blk'))", 8000)
            need(pg.evaluate("RUOLI.roleOf(ST.files.find(f => f.name === 'PERM 3h.mzML'))") == "blank", "the loading table did not take the role")
            # rename: from the panel -> sample table
            drawer(pg); pg.locator("#flst .fl .nm", has_text="RET t0 (2)").click(button="right"); pg.locator("#ctx div", has_text="Rename").click(); pg.wait_for_selector("#askin")
            pg.fill("#askin", "Renamed sample"); pg.keyboard.press("Enter")
            wait_for_js(pg, "%s.includes('Renamed sample')" % cmp_names, 8000)
            need("Renamed sample" in pg.evaluate(side), pg.evaluate(side))
            # group: Group by time in the panel -> the sample table shows the group tag at once
            pg.evaluate("GRUPPI.by({by: 'time'})")
            wait_for_js(pg, "[...document.querySelectorAll('#cmp-tab .ftag')].length > 0", 8000)
            need(pg.evaluate("[...document.querySelectorAll('#cmp-tab .ftag')].map(e => e.textContent).every(t => E.files.some(f => f.ugrp === t))"), "the group tags of the table differ from the panel")
            # delete: the x of the panel -> gone from the sample table and the loading table
            n0 = len(pg.evaluate(cmp_names)); drawer(pg); pg.locator("#flst .fl", has_text="RET 8h (2)").locator(".fl-x").dispatch_event("click")
            wait_for_js(pg, "!%s.includes('RET 8h (2)')" % cmp_names, 8000)
            need(len(pg.evaluate(cmp_names)) == n0 - 1 and "RET 8h (2)" not in pg.evaluate(side), pg.evaluate(cmp_names))
            need(pg.evaluate("![...document.querySelectorAll('#flist tbody tr')].some(r => r.textContent.includes('RET 8h (2)') && r.querySelector('input[data-k=use]').checked)"), "the loading table still lists the deleted file as open")
        step("role, rename, group, delete and time unit show at once in the Files panel, the sample table and the loading table", views)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
