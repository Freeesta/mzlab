"""The colour key of the SIM / MRM ion lists on the built site (synthetic SIM .wiff files of tools/wiff_sim.py, no laboratory data). Rule under test: every ion has ITS OWN colour (its place among ALL ions
of the panel in the palette), the samples are told apart by dash; the rows of the channel picker and of the «Extract ion ›» submenus (MRM panel and chromatogram panel) carry a swatch with the line of the
trace, read from the one `p._look` the drawing writes from the same SIMUI.style() call as the traces; hollow and dimmed while the row is off; ticking, unticking, stacking, overlaying and moving a file
never recolour a trace; hover or focus of a row lights its trace. Checked on 7 ions x 1 and 2 samples (sim3), 4 ions x 2 samples (sim), 3 ions x 4 samples (listab) and the palette order of 2..48 ions.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_trace_key.py"""
# covers: mzlab/web/sim-ui.js, mzlab/web/canali.js, mzlab/web/canali.css, mzlab/web/menu-sotto.js, mzlab/web/grafici.js, tools/wiff_sim.py
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="trace_key_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
N3 = len(json.loads((D / "sim3.json").read_text())["ioni"])
r = Run(port=8876, wd="/tmp/wd_trace_key")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))

P = "E.panels.find(p => p.type === 'mrm')"
# rows of the picker (all of them, ticked or not) and the traces drawn, both with the colour as the browser computes it
READ = """(() => { const p = %s, a = p._a, norm = c => { const e = document.createElement('i'); e.style.color = c; document.body.appendChild(e); const v = getComputedStyle(e).color; e.remove(); return v; };
  const rows = [...p.el.querySelectorAll('.can-pop .can-row[data-group]')].map(r => { const s = r.querySelector('.trsw'); return { label: r.querySelector('.can-et').textContent, on: r.getAttribute('aria-selected') === 'true',
    sw: !!s && !s.classList.contains('none'), off: !!s && s.classList.contains('off'), color: s ? getComputedStyle(s).color : null, td: s ? s.dataset.td : null }; });
  const tr = (a ? a.sr : []).map(s => ({ label: SIMUI.label(s.tk), color: norm(s.color), dash: s.dash.join(','), k: s.k }));
  return { rows, tr, hl: p._hl ?? null }; })()""" % P

def read(pg): return pg.evaluate(READ)

def check(d, n, samples, ticked=None):
    """rows = n ions, each swatch = the colour of every trace of its ion; ions differ in colour; samples differ in dash"""
    need(len(d["rows"]) == n, "rows %d != %d" % (len(d["rows"]), n))
    col, dash = {}, {}
    for t in d["tr"]: col.setdefault(t["label"], set()).add(t["color"]); dash.setdefault(t["label"], {})[t["k"]] = t["dash"]
    need(all(len(c) == 1 for c in col.values()), "one ion, several colours: %s" % col)
    need(len({next(iter(c)) for c in col.values()}) == len(col), "two ions share a colour: %s" % col)
    for row in d["rows"]:
        need(row["sw"], "row without a swatch: %s" % row)
        need(row["off"] == (not row["on"]), "off class must follow the tick: %s" % row)
        if row["label"] in col: need(row["color"] == next(iter(col[row["label"]])), "row %s swatch %s, trace %s" % (row["label"], row["color"], col[row["label"]]))
    if samples > 1:
        for lab, per in dash.items(): need(len(set(per.values())) == len(per) >= 2, "samples must differ in dash on %s: %s" % (lab, per))
    return {k: next(iter(v)) for k, v in col.items()}

def session(pg, name, n_files):
    stage(pg, [D / (name + ".wiff"), D / (name + ".wiff.scan")]); pg.click("text=Load data"); ready(pg)
    wait_for_js(pg, "E.files.length >= %d && E.files.every(f => f.sim > 0)" % n_files, timeout=120000)
    open_first(pg)
    pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)

def show(pg, ks, n_traces, n_samples):
    pg.evaluate("(() => { const on = %s; E.files.forEach((f, i) => { f.vis = on.includes(i); }); renderFileList(); const p = %s; p.tr = ''; ctl(p); draw(p); })()" % (json.dumps(ks), P))
    wait_for_js(pg, "%s._a && %s._a.sr.length === %d && new Set(%s._a.sr.map(s => s.k)).size === %d" % (P, P, n_traces, P, n_samples), timeout=30000)

try:
    with sync_playwright() as p:
        pg = r.page(p)
        session(pg, "sim3", 4)
        show(pg, [0], N3, 1)
        base = {}
        def toggle(i): pg.evaluate("%s.el.querySelectorAll('.can-pop .can-row[data-group] .can-hit')[%d].click()" % (P, i))
        def popup(): pg.evaluate("%s._canOpen = true; ctl(%s)" % (P, P)); frames(pg, 2)

        def single():
            base.update(check(read(pg), N3, 1))
            need(len(base) == N3, base)
        step("one sample, 7 ions: each row's swatch has exactly the colour of its ion's trace; the ions differ", single)

        def untick():
            popup()
            for i, left in ((0, N3 - 1), (2, N3 - 2), (3, N3 - 3)):
                toggle(i); wait_for_js(pg, "%s.el.querySelectorAll('.can-pop .can-row[aria-selected=true]').length === %d" % (P, left), timeout=20000)
            wait_for_js(pg, "%s._a.sr.length === %d" % (P, N3 - 3), timeout=20000)
            d = read(pg); got = check(d, N3, 1)
            need(len(d["tr"]) == N3 - 3 and sum(not x["on"] for x in d["rows"]) == 3, "three rows unticked")
            for lab, c in got.items(): need(c == base[lab], "unticking recoloured %s: %s vs %s" % (lab, c, base[lab]))
            for x in d["rows"]: need(x["color"] == base[x["label"]], "an unticked row keeps the colour of its ion: %s" % x)
        step("unticking rows: the other traces keep their colour; unticked rows show the same colour, hollow", untick)

        def retick():
            for i in (0, 2, 3): toggle(i); frames(pg, 2)
            wait_for_js(pg, "%s._a.sr.length === %d" % (P, N3), timeout=20000)
            need(check(read(pg), N3, 1) == base, "re-ticking changed a colour")
        step("re-ticking restores the traces with the same colours", retick)

        def stacked():
            pg.evaluate("(() => { const p = %s; p.mode = 'stk'; draw(p); })()" % P); frames(pg, 3)
            need(check(read(pg), N3, 1) == base, "stacked view recoloured")
            pg.evaluate("(() => { const p = %s; p.mode = ''; draw(p); })()" % P); frames(pg, 3)
        step("stacked and overlaid views keep the colour of a trace", stacked)

        def hover():
            popup()
            before = pg.evaluate("%s.cv.toDataURL()" % P)
            pg.locator(".can-pop:not([hidden]) .can-row[data-group]").nth(1).hover()
            wait_for_js(pg, "%s._hl != null" % P, timeout=10000); frames(pg, 3)
            lab = pg.evaluate("%s.el.querySelectorAll('.can-pop .can-row[data-group]')[1].querySelector('.can-et').textContent" % P)
            need(pg.evaluate("SIMUI.label(%s._hl)" % P) == lab, "lit %s, hovered %s" % (pg.evaluate("%s._hl" % P), lab))
            need(pg.evaluate("%s.cv.toDataURL()" % P) != before, "hovering a row did not change the drawing")
            pg.mouse.move(5, 5); wait_for_js(pg, "%s._hl == null" % P, timeout=10000); frames(pg, 3)
            need(pg.evaluate("%s.cv.toDataURL()" % P) == before, "the drawing did not come back after the mouse left")
        step("hovering a row draws its trace on and fades the others; leaving restores the drawing", hover)

        def two_samples():
            show(pg, [0, 1], 2 * N3, 2)
            need(check(read(pg), N3, 2) == base, "a second sample changed the colour of an ion")
        step("2 samples x 7 ions: the ion keeps its colour, the samples differ by dash, the swatch is the ion's colour", two_samples)

        def submenus():
            show(pg, [0], N3, 1)
            pg.evaluate("%s._canOpen = false; ctl(%s)" % (P, P)); pg.keyboard.press("Escape"); frames(pg, 2)
            pg.evaluate("%s.cv.scrollIntoView({ block: 'center' })" % P); frames(pg, 2)
            box = pg.evaluate("(() => { const b = %s.cv.getBoundingClientRect(); return { x: b.x + 40, y: b.y + 40, width: b.width - 80, height: b.height - 80 }; })()" % P)
            pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right"); pg.wait_for_selector("#ctx:not([hidden])"); pg.hover("#ctx .hsub")
            JS = "[...document.querySelectorAll('#ctx .ctxsub [role=menuitemcheckbox]')].slice(1).map(e => { const s = e.querySelector('.sw'); return { label: e.getAttribute('aria-label').split(' ')[0], sw: !!s, off: !!s && s.classList.contains('off'), c: s ? getComputedStyle(s).color : null }; })"
            rows = pg.evaluate(JS)
            need(len(rows) == N3 and all(x["sw"] and not x["off"] for x in rows), "MRM panel menu, all ticked: %s" % rows)
            for x in rows: need(x["c"] == base[x["label"]], "MRM menu row %s swatch %s, trace %s" % (x["label"], x["c"], base[x["label"]]))
            pg.keyboard.press("Escape")
            # one ion left: its row is on, the others hollow (same colour)
            pg.evaluate("(() => { const p = %s; p.tr = SIMUI.ions(p).slice(1, 2).map(x => x.key); ctl(p); draw(p); })()" % P); wait_for_js(pg, "%s._a.sr.length === 1" % P, timeout=20000)
            pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right"); pg.wait_for_selector("#ctx:not([hidden])"); pg.hover("#ctx .hsub")
            rows = pg.evaluate(JS)
            need([x["off"] for x in rows].count(False) == 1 and all(x["c"] == base[x["label"]] for x in rows), "MRM menu, one ticked: %s" % rows)
            pg.keyboard.press("Escape")
            # a chromatogram panel of the same tab: the same swatches, from the same style() call, all off (the entry opens an MRM panel)
            pg.evaluate("(() => { const c = addPanel('chrom', { tab: 'mrm', x: 0, y: 1e5, w: hostWidth(), h: 260, full: true }); window._cq = c.id; })()")
            wait_for_js(pg, "(() => { const c = E.panels.find(q => q.id === window._cq); return !!c && !!c._a; })()", timeout=30000)
            pg.evaluate("E.panels.find(q => q.id === window._cq).cv.scrollIntoView({ block: 'center' })"); frames(pg, 2)
            cb = pg.evaluate("(() => { const b = E.panels.find(q => q.id === window._cq).cv.getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })()")
            pg.mouse.click(cb[0], cb[1], button="right"); pg.wait_for_selector("#ctx:not([hidden])"); pg.hover("#ctx .hsub")
            rows = pg.evaluate(JS)
            need(len(rows) == N3 and all(x["sw"] and x["off"] for x in rows), "chromatogram menu: %s" % rows)
            for x in rows: need(x["c"] == base[x["label"]], "chromatogram menu row %s swatch %s, MRM trace %s" % (x["label"], x["c"], base[x["label"]]))
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(2).hover()
            wait_for_js(pg, "E.panels.find(q => q.id === window._cq)._hl != null", timeout=10000)
            pg.keyboard.press("Escape")
        step("the Extract ion submenus (MRM panel and chromatogram panel) carry the same swatches, off state and hover included", submenus)

        def reorder():
            first = pg.evaluate("E.files[0].label")
            pg.evaluate("(() => { reorderFiles(E.files[1].k, E.files[0].k, true); })()"); frames(pg, 3)
            need(pg.evaluate("E.files[0].label") != first, "the file did not move in the list")
            show(pg, [0, 1], 2 * N3, 2)
            need(check(read(pg), N3, 2) == base, "moving a file in the list recoloured an ion")
        step("moving a file in the list does not recolour an ion", reorder)

        def palette():
            o = pg.evaluate("(() => { const f = { color: '#123456', k: 0 }, r = {}; for (const n of [2, 3, 4, 7, 17, 48]) { r[n] = []; for (let i = 0; i < n; i++) { const st = SIMUI.style(f, i, n, 0); r[n].push([st.color, st.dash.join(',')]); } } return { r, pal: PAL.slice() }; })()")
            pal, m = o["pal"], len(o["pal"])
            need(m >= 8, "palette of %d colours" % m)
            for n in (2, 3, 4, 7): need(len({c for c, _ in o["r"][str(n)]}) == n, "%d ions need %d colours: %s" % (n, n, o["r"][str(n)]))
            a = o["r"]["48"]
            need(all(a[i][0] == pal[i % m] for i in range(48)), "the colour of ion i is PAL[i % PAL.length]")
            need(all(a[i][1] != a[i + m][1] for i in range(48 - m)), "a repeated colour must come with another dash")
            need(len({tuple(x) for x in a}) == 48, "colour + dash pairs must stay distinct for 48 ions")
            need(len({tuple(x) for x in o["r"]["17"]}) == 17, "17 ions")
        step("2..48 ions: the colours follow the palette order and repeat with another dash", palette)
        step("no page error (sim3)", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))

        # the same checks on the other synthetic files: 4 ions x 2 samples, 3 ions x 4 samples
        for name, n_ions, n_files in (("sim", 4, 2), ("sim_listab", 3, 4)):
            def other(name=name, n_ions=n_ions, n_files=n_files):
                pg2 = r.page(p); session(pg2, name, n_files)
                show(pg2, list(range(n_files)), n_ions * n_files, n_files)
                d = pg2.evaluate(READ); got = check(d, n_ions, n_files)
                need(len(got) == n_ions, got)
            step("%s: %d ions x %d samples: distinct colour per ion, distinct dash per sample, swatches equal the traces" % (name, n_ions, n_files), other)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
