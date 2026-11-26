"""The colour key of the SIM / MRM ion lists on the built site (synthetic SIM .wiff of tools/wiff_sim.py `sim3`: a blank and three samples, seven ions, no laboratory data): every row of the
channel picker and of the «Extract ion ›» submenu carries a swatch with the line (colour and dash) of its trace, read from the one `p._look` the drawing writes from the same SIMUI.style() call as the
traces; the swatch is hollow and dimmed while the row is off; ticking, unticking, stacking and overlaying never recolour a trace; with several samples the colour is the sample's and the swatch shows only
the dash; the hover or focus of a row lights its trace; past the palette length the colours repeat in a fixed order with another dash. Needs the site and the .wasm (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_trace_key.py"""
# covers: mzlab/web/sim-ui.js, mzlab/web/canali.js, mzlab/web/canali.css, mzlab/web/menu-sotto.js, tools/wiff_sim.py
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
N = len(json.loads((D / "sim3.json").read_text())["ioni"])
r = Run(port=8876, wd="/tmp/wd_trace_key")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))

P = "E.panels.find(p => p.type === 'mrm')"
# rows of the picker (all of them, ticked or not) and the traces drawn, both with the colour as the browser computes it
READ = """(() => { const p = %s, a = p._a, norm = c => { const e = document.createElement('i'); e.style.color = c; document.body.appendChild(e); const v = getComputedStyle(e).color; e.remove(); return v; };
  const rows = [...p.el.querySelectorAll('.can-pop .can-row[data-group]')].map(r => { const s = r.querySelector('.trsw'); return { label: r.querySelector('.can-et').textContent, on: r.getAttribute('aria-selected') === 'true',
    sw: !!s && !s.classList.contains('none'), off: !!s && s.classList.contains('off'), color: s ? getComputedStyle(s).color : null, tc: s ? s.dataset.tc : null, td: s ? s.dataset.td : null }; });
  const tr = (a ? a.sr : []).map(s => ({ label: SIMUI.label(s.tk), color: norm(s.color), dash: s.dash.join(','), k: s.k }));
  return { rows, tr, hl: p._hl ?? null }; })()""" % P

def read(pg): return pg.evaluate(READ)
def by_label(tr): return {t["label"]: t for t in tr}

try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Load data"); ready(pg); wait_for_js(pg, "E.files.length >= 4 && E.files.every(f => f.sim > 0)", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        pg.evaluate("(() => { E.files.forEach((f, i) => { f.vis = i === 0; }); const p = %s; p.tr = ''; renderFileList(); ctl(p); draw(p); })()" % P)
        wait_for_js(pg, "%s._a && %s._a.sr.length === %d" % (P, P, N), timeout=30000)
        base = {}

        def single():
            d = read(pg); base.update(by_label(d["tr"]))
            need(len(d["rows"]) == N and len(d["tr"]) == N, (len(d["rows"]), len(d["tr"])))
            for row in d["rows"]:
                t = base.get(row["label"]); need(t, "no trace for the row %s" % row["label"])
                need(row["sw"] and not row["off"], "ticked row without a plain swatch: %s" % row)
                need(row["color"] == t["color"], "row %s swatch %s, trace %s" % (row["label"], row["color"], t["color"]))
                need(row["td"] == t["dash"], "row %s dash %s, trace %s" % (row["label"], row["td"], t["dash"]))
            need(len({t["color"] for t in base.values()}) == N, "seven ions, one sample: the colours must differ %s" % [t["color"] for t in base.values()])
        step("one sample, more ions than dashes: each row's swatch has exactly the colour and dash of its trace", single)

        def toggle(i): pg.evaluate("%s.el.querySelectorAll('.can-pop .can-row[data-group] .can-hit')[%d].click()" % (P, i))
        def untick():
            pg.evaluate("%s._canOpen = true; ctl(%s)" % (P, P))
            for i in (0, 2, 3): toggle(i); wait_for_js(pg, "%s.el.querySelectorAll('.can-pop .can-row[aria-selected=true]').length === %d" % (P, N - (1 if i == 0 else 2 if i == 2 else 3)), timeout=20000)
            wait_for_js(pg, "%s._a.sr.length === %d" % (P, N - 3), timeout=20000)
            d = read(pg); need(len(d["tr"]) == N - 3, len(d["tr"]))
            for t in d["tr"]: need(t["color"] == base[t["label"]]["color"] and t["dash"] == base[t["label"]]["dash"], "unticking others recoloured %s: %s vs %s" % (t["label"], t, base[t["label"]]))
            offs = [x for x in d["rows"] if not x["on"]]; need(len(offs) == 3, offs)
            for row in d["rows"]:
                need(row["sw"] and row["off"] == (not row["on"]), "row %s: off class must follow the tick: %s" % (row["label"], row))
                need(row["color"] == base[row["label"]]["color"], "an unticked row keeps the colour of its trace: %s" % row)
        step("unticking rows: the others keep their colour; unticked rows show the swatch hollow, same colour", untick)

        def retick():
            for i in (0, 2, 3): toggle(i); frames(pg, 2)
            wait_for_js(pg, "%s._a.sr.length === %d" % (P, N), timeout=20000)
            d = read(pg)
            for t in d["tr"]: need(t["color"] == base[t["label"]]["color"], "re-ticked %s changed colour: %s vs %s" % (t["label"], t["color"], base[t["label"]]["color"]))
            need(all(not x["off"] for x in d["rows"]), "a row still dimmed")
        step("re-ticking restores the same traces with the same colours", retick)

        def stacked():
            pg.evaluate("(() => { const p = %s; p.mode = 'stk'; draw(p); })()" % P); frames(pg, 3)
            d = read(pg); need(len(d["tr"]) == N, len(d["tr"]))
            for t in d["tr"]: need(t["color"] == base[t["label"]]["color"], "stacked: %s %s vs %s" % (t["label"], t["color"], base[t["label"]]["color"]))
            for row in d["rows"]: need(row["color"] == base[row["label"]]["color"], "stacked: row %s" % row)
            pg.evaluate("(() => { const p = %s; p.mode = ''; draw(p); })()" % P); frames(pg, 3)
        step("stacked and overlaid views keep the colour of a trace", stacked)

        def hover():
            pg.evaluate("%s._canOpen = true; ctl(%s)" % (P, P)); frames(pg, 2)
            sel = ".can-pop .can-row[data-group]"
            before = pg.evaluate("%s.cv.toDataURL()" % P)
            pg.locator(".can-pop:not([hidden]) .can-row[data-group]").nth(1).hover()
            wait_for_js(pg, "%s._hl != null" % P, timeout=10000); frames(pg, 3)
            d = read(pg); lab = pg.evaluate("%s.el.querySelectorAll('.can-pop .can-row[data-group]')[1].querySelector('.can-et').textContent" % P)
            need(pg.evaluate("SIMUI.label(%s._hl)" % P) == lab, "lit %s, hovered %s" % (d["hl"], lab))
            need(pg.evaluate("%s.cv.toDataURL()" % P) != before, "hovering a row did not change the drawing")
            pg.mouse.move(5, 5); wait_for_js(pg, "%s._hl == null" % P, timeout=10000); frames(pg, 3)
            need(pg.evaluate("%s.cv.toDataURL()" % P) == before, "the drawing did not come back after the mouse left")
        step("hovering a row draws its trace on and fades the others; leaving restores the drawing", hover)

        def several():
            pg.evaluate("(() => { E.files.forEach(f => { f.vis = true; }); renderFileList(); const p = %s; p.tr = ''; ctl(p); draw(p); })()" % P)
            wait_for_js(pg, "%s._a && new Set(%s._a.sr.map(s => s.k)).size >= 2" % (P, P), timeout=30000)
            d = read(pg)
            need(len({t["k"] for t in d["tr"]}) >= 2, "one sample only")
            dash = {}
            for t in d["tr"]: dash.setdefault(t["label"], set()).add(t["dash"])
            for row in d["rows"]:
                need(row["sw"] and row["tc"] == "", "several samples: the colour is the sample's, so the row has no colour of its own: %s" % row)
                need(len(dash[row["label"]]) == 1 and row["td"] in dash[row["label"]], "row %s dash %s, traces %s" % (row["label"], row["td"], dash[row["label"]]))
        step("several samples: colour = sample, so the swatch shows only the ion's dash, equal to its traces' dash", several)

        def repeats():
            # 17 and more ions of one sample: the palette order repeats, one dash pattern further each round, and no (colour, dash) pair is used twice up to the end of the dashes
            s = pg.evaluate("(() => { const f = { color: '#123456' }, n = 17, o = []; for (let i = 0; i < 48; i++) { const st = SIMUI.style(f, i, 48, 1); o.push([st.color, st.dash.join(',')]); } return { o, pal: PAL.slice() }; })()")
            o, pal = s["o"], s["pal"]; m = len(pal)
            need(len({c for c, _ in o[:m]}) == m, "the first round must use distinct colours")
            need(all(o[i][0] == pal[i % m] for i in range(48)), "the colour of ion i is PAL[i % PAL.length]")
            need(all(o[i][1] != o[i + m][1] for i in range(48 - m)), "a repeated colour must come with another dash")
            need(len({tuple(x) for x in o}) == 48, "colour + dash pairs must stay distinct for 48 ions")
        step("17+ ions: colours follow the palette order and repeat with another dash", repeats)

        def submenu():
            pg.evaluate("(() => { E.files.forEach((f, i) => { f.vis = i === 0; }); renderFileList(); const p = %s; p.tr = ''; ctl(p); draw(p); })()" % P)
            wait_for_js(pg, "%s._a && %s._a.sr.length === %d && new Set(%s._a.sr.map(s => s.k)).size === 1" % (P, P, N, P), timeout=30000)
            box = pg.locator("canvas").first.bounding_box()
            pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right"); pg.wait_for_selector("#ctx:not([hidden])")
            pg.hover("#ctx .hsub")
            rows = pg.evaluate("[...document.querySelectorAll('#ctx .ctxsub [role=menuitemcheckbox]')].slice(1).map(e => { const s = e.querySelector('.sw'); return { label: e.getAttribute('aria-label'), sw: !!s, off: !!s && s.classList.contains('off'), c: s ? getComputedStyle(s).backgroundColor : null }; })")
            need(len(rows) == N and all(x["sw"] and not x["off"] for x in rows), rows)
            for x in rows:
                t = base[x["label"].split(" ")[0]]; need(x["c"] == t["color"], "submenu row %s swatch %s, trace %s" % (x["label"], x["c"], t["color"]))
            pg.keyboard.press("Escape")
        step("the Extract ion submenu rows carry the same swatch", submenu)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
