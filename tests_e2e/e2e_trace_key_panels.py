"""The colour key in the three trace lists the SIM/MRM key did not cover, on the built site with synthetic files only (tools/dati_sintetici.py, tools/wiff_dad_mrm.py):
the MS2 transition select (a swatch strip beside it and a square on each option), the legend of the XIC list (traces and files) and the DAD curves on an MRM panel. Rule under test:
a swatch has the colour and dash of its trace, read from the one `p._look` (or the one line function of dad-mrm.js) the drawing uses, hollow and dimmed while the trace is off, and
hover or focus of a swatch draws its trace on and fades the others. Needs the site (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_trace_key_panels.py"""
# covers: mzlab/web/grafici.js, mzlab/web/pannelli.js, mzlab/web/dad-mrm.js, mzlab/web/sim-ui.js, mzlab/web/canali.css, tools/wiff_dad_mrm.py
import subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))

D = Path(tempfile.mkdtemp(prefix="trace_key_panels_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
r = Run(port=8877, wd="/tmp/wd_trace_key_panels")
# the colour as the browser computes it, for a swatch (its `color`) and for a trace (its stroke colour)
NORM = "(c => { const e = document.createElement('i'); e.style.color = c; document.body.appendChild(e); const v = getComputedStyle(e).color; e.remove(); return v; })"

try:
    with sync_playwright() as p:
        # ------------------------------------------------------------------ the MS2 transition select
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_MS2-t15")])
        wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=30000)
        pg.click("text=Load data"); ready(pg)
        open_kind(pg, "ms2"); pg.wait_for_selector("select[data-o=tr]", timeout=30000)
        MS = "E.panels.find(q => q.type === 'chrom' && q.tab === 'ms2')"
        ROWS = "(() => { const p = %s, n = %s; return [...p.el.querySelectorAll('.trkey .trk')].map(e => { const s = e.querySelector('.trsw'); return { k: e.dataset.lk, c: getComputedStyle(s).color, d: s.dataset.td, off: s.classList.contains('off') }; })" % (MS, NORM) + ".map(x => ({ ...x, c: n(x.c) })); })()"
        TRS = "(() => { const p = %s, n = %s; return p._a.sr.map(s => ({ k: s.tk, c: n(s.color), d: s.dash.join(',') })); })()" % (MS, NORM)

        def all_rows():
            pg.select_option("select[data-o=tr]", "__all"); wait_for_js(pg, "%s._a && %s._a.sr.length > 1 && %s.el.querySelectorAll('.trkey .trk').length === %s._a.sr.length" % (MS, MS, MS, MS), timeout=30000)
            rows, trs = pg.evaluate(ROWS), pg.evaluate(TRS)
            need(len(rows) == len(trs) > 1, (rows, trs))
            for t in trs:
                row = next(x for x in rows if x["k"] == t["k"])
                need(row["c"] == t["c"] and row["d"] == t["d"], "swatch %s vs trace %s" % (row, t))
            need(len({t["c"] for t in trs}) == len(trs), "the transitions must differ in colour: %s" % trs)
        step("MS2 select, every transition: one swatch per trace with its colour and dash", all_rows)

        def options():
            on = pg.evaluate("[...document.querySelectorAll('select[data-o=tr] option[value*=\">\"]')].map(o => o.textContent.trim()[0])")
            need(on and all(c == "■" for c in on[:len(pg.evaluate(TRS))]), on)
        step("MS2 select, every transition: the options carry a filled square", options)

        def one_row():
            val = pg.evaluate("%s._trs[1].key" % MS)
            pg.select_option("select[data-o=tr]", val); wait_for_js(pg, "%s._a.sr.length === 1" % MS, timeout=30000)
            rows = pg.evaluate(ROWS); need(len(rows) == 1 and rows[0]["k"] == val and not rows[0]["off"], rows)
            need(rows[0]["c"] == pg.evaluate(TRS)[0]["c"], "the swatch is not the trace colour")
            sq = pg.evaluate("[...document.querySelectorAll('select[data-o=tr] option[value*=\">\"]')].map(o => o.textContent.trim()[0])")
            need(sq.count("■") == 1 and sq.count("□") == len(sq) - 1, sq)
            # the colour of the one drawn does not depend on the others: same as in the all-transitions view
            need(rows[0]["c"] == pg.evaluate("(() => { const p = %s, n = %s, l = p._look[%r]; return n(l.c); })()" % (MS, NORM, val)), "look differs")
        step("MS2 select, one transition: its swatch alone, the other options hollow squares", one_row)

        def hover_ms2():
            before = pg.evaluate("%s.cv.toDataURL()" % MS)
            pg.locator("%s" % ".trkey .trk").first.hover()
            wait_for_js(pg, "%s._hl != null" % MS, timeout=10000); frames(pg, 3)
            pg.mouse.move(5, 5); wait_for_js(pg, "%s._hl == null" % MS, timeout=10000); frames(pg, 3)
            need(pg.evaluate("%s.cv.toDataURL()" % MS) == before, "the drawing did not come back after the mouse left")
        step("MS2 select: hover of the key lights its trace, leaving restores the drawing", hover_ms2)

        def none_row():
            pg.select_option("select[data-o=tr]", ""); wait_for_js(pg, "%s.el.querySelectorAll('.trkey').length === 0" % MS, timeout=15000)
        step("MS2 select, none: no key", none_row)

        # ------------------------------------------------------------------ the XIC list legend (traces and files)
        pg2 = r.page(p); pg2.set_viewport_size({"width": 1400, "height": 1000})
        pg2.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        wait_for_js(pg2, "document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=60000)
        pg2.click("text=Load data"); ready(pg2); open_kind(pg2, "full")
        XP = "E.panels.find(q => q.type === 'xic')"
        pg2.evaluate("openXic(null, { mz: 229.1 })"); wait_for_js(pg2, "!!%s && !!%s._a" % (XP, XP), timeout=60000)
        pg2.evaluate("(() => { const p = %s; addTrace(p, 305.0); addTrace(p, 150.1); ctl(p); draw(p); })()" % XP)
        wait_for_js(pg2, "%s._a.sr.length >= 6" % XP, timeout=60000)
        LEG = "(() => { const p = %s, n = %s; return { rows: [...p.leg.querySelectorAll('[data-lk]')].map(e => { const s = e.querySelector('.trsw'); return { k: e.dataset.lk, c: s && getComputedStyle(s).color && n(getComputedStyle(s).color), d: s && s.dataset.td, off: !!s && s.classList.contains('off') }; }), tr: p._a.sr.map(s => ({ k: s.tk, f: s.tf, c: n(s.color), d: s.dash.join(',') })) }; })()" % (XP, NORM)

        def xic_leg():
            d = pg2.evaluate(LEG); need(len(d["rows"]) >= 5, d)
            for t in d["tr"]:
                row = next((x for x in d["rows"] if x["k"] == t["k"]), None)
                need(row is not None and row["c"] == t["c"], "trace entry %s vs trace %s" % (row, t))
            files = [x for x in d["rows"] if x["k"].startswith("f")]
            need(len(files) == 2 and files[0]["d"] != files[1]["d"], "the files must differ in dash: %s" % files)
            need(all(not x["off"] for x in d["rows"]), d["rows"])
        step("XIC legend: a trace entry has the colour of its trace; the files differ in dash", xic_leg)

        def xic_off():
            pg2.locator("%s" % ".leg .tg[data-lk^=t]").first.click(); frames(pg2, 3)
            d = pg2.evaluate(LEG); need(sum(x["off"] for x in d["rows"] if x["k"].startswith("t")) == 1, d["rows"])
            pg2.locator("%s" % ".leg .tg[data-lk^=t]").first.click(); frames(pg2, 3)
        step("XIC legend: a hidden trace shows a hollow dimmed swatch", xic_off)

        def xic_hover():
            before = pg2.evaluate("%s.cv.toDataURL()" % XP)
            pg2.locator(".leg .tg[data-lk^=t]").nth(1).hover()
            wait_for_js(pg2, "%s._hl != null" % XP, timeout=10000); frames(pg2, 3)
            need(pg2.evaluate("%s.cv.toDataURL()" % XP) != before, "hovering an entry did not change the drawing")
            pg2.mouse.move(5, 5); wait_for_js(pg2, "%s._hl == null" % XP, timeout=10000); frames(pg2, 3)
            need(pg2.evaluate("%s.cv.toDataURL()" % XP) == before, "the drawing did not come back")
        step("XIC legend: hover of an entry draws its trace on, leaving restores the drawing", xic_hover)

        # ------------------------------------------------------------------ the DAD curves on an MRM panel
        pg3 = r.page(p); pg3.set_viewport_size({"width": 1400, "height": 1000})
        pg3.set_input_files("#pick", [str(D / "dadmrm.wiff"), str(D / "dadmrm.wiff.scan")])
        wait_for_js(pg3, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000)
        pg3.click("text=Load data"); ready(pg3); open_first(pg3)
        pg3.click("#np-mrm"); wait_for_js(pg3, "E.panels.some(q => q.type === 'mrm' && q._a)", timeout=60000)
        MR = "E.panels.find(q => q.type === 'mrm')"
        pg3.evaluate("(() => { const p = %s; p.overlay = 'twc'; draw(p); })()" % MR)
        wait_for_js(pg3, "%s.leg.querySelectorAll('[data-lk^=twc]').length >= 1" % MR, timeout=60000)

        def twc_key():
            d = pg3.evaluate("(() => { const p = %s, n = %s, s = p.leg.querySelector('[data-lk^=twc] .trsw'); return { c: n(getComputedStyle(s).color), d: s.dataset.td, f: n(E.files.find(f => f.pda).color), off: s.classList.contains('off') }; })()" % (MR, NORM))
            need(d["c"] == d["f"] and d["d"] == "2,3" and not d["off"], d)
        step("DAD curves: the legend entry has the colour and dash of the curve", twc_key)

        def twc_off():
            pg3.locator(".leg [data-lk^=twc]").first.click(); frames(pg3, 3)
            need(pg3.evaluate("%s.leg.querySelector('[data-lk^=twc] .trsw').classList.contains('off')" % MR), "hidden curve must show a hollow swatch")
            pg3.locator(".leg [data-lk^=twc]").first.click(); frames(pg3, 3)
        step("DAD curves: hiding the curve gives a hollow dimmed swatch", twc_off)

        def twc_hover():
            before = pg3.evaluate("%s.cv.toDataURL()" % MR)
            pg3.locator(".leg [data-lk^=twc]").first.hover()
            wait_for_js(pg3, "%s._hl != null" % MR, timeout=10000); frames(pg3, 3)
            need(pg3.evaluate("%s.cv.toDataURL()" % MR) != before, "hover did not change the drawing")
            pg3.mouse.move(5, 5); wait_for_js(pg3, "%s._hl == null" % MR, timeout=10000); frames(pg3, 3)
        step("DAD curves: hover of the entry lights the curve", twc_hover)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
