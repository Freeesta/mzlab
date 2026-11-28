"""The empty state of a spectrum panel and a map panel (mzlab/web/empty-hint.js `note`), on the built site, at 1400 and 390 px. A spectrum panel with no time range chosen shows the prompt
«Choose a time range on the chromatogram» (a real button) and a curved arrow that ends at the live rectangle of the chromatogram panel that holds the choice; the arrow follows a resize, gives way
to the plain line when there is no room, is text only when there is no chromatogram, and goes once a range is chosen. A map panel with no scans shows its message as the same text-only prompt, and
the canvas of a panel that says something is never written on. Synthetic files of tools/dati_sintetici.py and tools/wiff_sim.py, no laboratory data.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_empty_hint_spectrum.py"""
# covers: mzlab/web/empty-hint.js, mzlab/web/picchi.js, mzlab/web/uv-spettro.js, mzlab/web/grafici.js, mzlab/web/lang/en/more-lv-defaults.js
import subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))

D = Path(tempfile.mkdtemp(prefix="empty_hint_spec_"))
S = "E.panels.find(q => q.type === 'spec')"
CH = "E.panels.find(q => q.type === 'chrom')"
# the prompt of the spectrum panel, its arrow (last point of the «C» segment, in the fixed layer, viewport coordinates) and the live rectangle of the chromatogram canvas
READ = """(() => { const p = %s, c = %s, e = p && p._eh; if (!e) return null;
  const go = e.go, gb = go.getBoundingClientRect(), cb = c && c.cv.getBoundingClientRect(), n = e.fx ? (e.fx.ln.getAttribute('d') || '').match(/-?\\d+(\\.\\d+)?/g) : null,
    end = n && n.length >= 8 ? [+n[n.length - 2], +n[n.length - 1]] : null, pb = p.cv.getBoundingClientRect();
  const gap = !end || !cb ? null : Math.max(cb.left - end[0], end[0] - cb.right, cb.top - end[1], end[1] - cb.bottom);
  return { key: e.spec.key, text: go.textContent, disabled: go.disabled, vis: gb.width > 0 && gb.height > 0, arrow: !!end, gap, hidden: e.fx ? e.fx.svg.getAttribute('aria-hidden') : null,
    inCanvas: gb.left >= pb.left - 1 && gb.right <= pb.right + 1 && gb.top >= pb.top - 1 && gb.bottom <= pb.bottom + 1, W: p.el.getBoundingClientRect().width, say: p._say }; })()""" % (S, CH)
def state(pg): return pg.evaluate(READ)
def settle(pg, arrow=True):
    for _ in range(40):      # the layout settles a few frames after a resize
        s = state(pg)
        if s and s["arrow"] == arrow: break
        frames(pg, 3)
    return state(pg)

def blank(pg, sel):
    """True when the canvas has no ink at all: nothing is written on it while the prompt is on top"""
    return pg.evaluate("""() => { const c = (%s).cv, d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; for (let i = 3; i < d.length; i += 4) if (d[i]) return false; return true; }""" % sel)

r = Run(port=8881, wd="/tmp/wd_empty_hint_spec")
r2 = None
try:
    with sync_playwright() as p:
        pg = r.page(p)
        fs = D / "B_FullMass-t0.mzML"; ds.full_scan(fs, 0, np.random.default_rng(1))
        stage(pg, [fs]); pg.click("text=Load data"); ready(pg); open_first(pg)
        wait_for_js(pg, "%s && %s._a && %s" % (CH, CH, S), timeout=60000)
        def none():
            pg.evaluate("(() => { const q = %s; q.r0 = null; q.r1 = null; q.si = null; draw(q); })()" % S); wait_for_js(pg, "%s._eh" % S, timeout=20000)

        def wide():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 4); none(); frames(pg, 4)
            pg.evaluate("document.scrollingElement.scrollTop = 0"); frames(pg, 3)
            s = settle(pg); need(s and s["vis"], "no prompt: %s" % s)
            need(s["key"] == "empty.spec" and s["text"] == "Choose a time range on the chromatogram" and not s["disabled"] and s["inCanvas"], "prompt: %s" % s)
            need(s["arrow"] and -1 <= s["gap"] <= 14, "the arrow must end at the chromatogram canvas (0..14 px): %s" % s)
            need(s["hidden"] == "true", "the arrow is decorative: aria-hidden: %s" % s)
            need(blank(pg, S), "nothing may be written on the canvas of the spectrum panel")
        step("1400 px, no time range: the prompt «Choose a time range on the chromatogram», an arrow that ends at the chromatogram, a blank canvas", wide)

        def resize():
            for w in (900, 600, 1400):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 5)
                s = settle(pg); need(s and s["arrow"] and -1 <= s["gap"] <= 14, "%d px: the arrow must follow the chromatogram: %s" % (w, s))
            pg.evaluate("(() => { %s.el.style.width = '420px'; })()" % S); frames(pg, 5)
            s = settle(pg); need(s and s["arrow"] and -1 <= s["gap"] <= 14, "panel 420 px: %s" % s)
            pg.evaluate("(() => { %s.el.style.width = ''; })()" % S); frames(pg, 5)
        step("a resize of the window or of the panel moves the arrow: its end stays at the chromatogram", resize)

        def phone():
            pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 6)
            s = settle(pg); need(s and s["vis"] and s["inCanvas"], "390 px: prompt: %s" % s)
            need(s["arrow"] and -1 <= s["gap"] <= 14, "390 px: the arrow ends at the chromatogram: %s" % s)
            need(pg.evaluate("document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"), "390 px: the page scrolls sideways")
        step("390 px: prompt inside the canvas, arrow at the chromatogram, no sideways scroll", phone)

        def narrow():
            pg.set_viewport_size({"width": 320, "height": 700}); frames(pg, 6)
            s = settle(pg, arrow=False); need(s and s["vis"] and s["W"] < 320, "a 320 px screen must give a panel under 320 px: %s" % s)
            need(not s["arrow"] and s["text"].startswith("Click or drag on the chromatogram") and not s["disabled"], "plain line, still a button: %s" % s)
            pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 6); settle(pg)
        step("a very narrow screen (320 px): the plain line «Click or drag on the chromatogram…», no arrow, still a button", narrow)

        def click():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 5)
            pg.evaluate("window.scrollTo(0, document.body.scrollHeight)"); frames(pg, 4)
            pg.click(".pnl.spec .eh-go"); frames(pg, 6)
            need(pg.evaluate("(() => { const b = %s.cv.getBoundingClientRect(); return b.bottom > 0 && b.top < innerHeight; })()" % CH), "the prompt must bring the chromatogram into view")
        step("the prompt brings the chromatogram into view", click)

        def say_kept():
            s = state(pg); need(s["say"] and s["say"].startswith("Click or drag on the chromatogram"), "p._say keeps the words for the data table and the screen reader: %s" % s)
        step("p._say still holds the words", say_kept)

        def gone():
            pg.evaluate("(() => { const q = %s, c = %s, d = c._a; q.r0 = d.x0 + (d.x1 - d.x0) * 0.4; q.r1 = q.r0 + (d.x1 - d.x0) * 0.02; draw(q); })()" % (S, CH))
            wait_for_js(pg, "!%s._eh && %s._a" % (S, S), timeout=30000)
            need(pg.evaluate("document.querySelectorAll('.eh-fx').length") == 0, "the arrow layer must go with the prompt")
        step("choosing a time range removes the prompt and the arrow, and the spectrum is drawn", gone)

        def again():
            none(); frames(pg, 4); s = settle(pg); need(s and s["arrow"], "unchoosing brings it back: %s" % s)
        step("clearing the range brings the prompt back", again)

        def nochrom():
            # no chromatogram to point at: text only, no button, no arrow
            pg.evaluate("E.panels.forEach(q => { if (q.type === 'chrom' || q.type === 'xic' || q.type === 'mrm') q._exp = true; }); draw(%s)" % S); frames(pg, 8)
            s = pg.evaluate("(() => { const e = %s._eh; return e && { key: e.spec.key, text: e.go.textContent, disabled: e.go.disabled, fx: !!e.fx }; })()" % S)
            need(s and s["key"] == "empty.note" and s["disabled"] and not s["fx"] and s["text"].startswith("Click or drag on the chromatogram"), "text only: %s" % s)
            pg.evaluate("E.panels.forEach(q => { q._exp = false; }); draw(%s)" % S); frames(pg, 8); settle(pg)
        step("no chromatogram panel: the same words as plain text, no arrow", nochrom)

        def other():
            # any other message of the spectrum panel (an unknown precursor) is the same text-only prompt
            pg.evaluate("(() => { const q = %s; q.level = 2; q.prec = 9999; draw(q); })()" % S)
            wait_for_js(pg, "%s._eh && %s._eh.spec.key === 'empty.note'" % (S, S), timeout=30000)
            s = state(pg); need(s["disabled"] and s["text"].startswith("None of the files shown has the precursor") and not s["arrow"], "unknown precursor: %s" % s)
            need(pg.evaluate("%s.leg.innerHTML === ''" % S), "the legend is reset")
        step("a spectrum with an unknown precursor: its message as the same text-only prompt, legend reset", other)
        step("no missing i18n key, no page error", lambda: need(not [x for x in r.errs if "missing key" in str(x) or x[0] == "pageerror"], r.errs))
    r.close()

    # a map panel with no scans (an MRM-only session)
    r2 = Run(port=8882, wd="/tmp/wd_empty_hint_spec_map")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
    with sync_playwright() as p:
        pg = r2.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Load data"); ready(pg)
        wait_for_js(pg, "E.files.length >= 1 && E.files.every(f => f.sim > 0)", timeout=120000); open_first(pg)
        def map_none():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 4)
            pg.evaluate("document.querySelector('#np-map').click()"); wait_for_js(pg, "E.panels.some(q => q.type === 'map' && q._eh)", timeout=30000)
            s = pg.evaluate("(() => { const q = E.panels.find(q => q.type === 'map'), e = q._eh, c = q.cv, d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let ink = false; for (let i = 3; i < d.length; i += 4) if (d[i]) { ink = true; break; }"
                            " const b = e.go.getBoundingClientRect(), cb = c.getBoundingClientRect(); return { key: e.spec.key, text: e.go.textContent, disabled: e.go.disabled, ink, inCanvas: b.left >= cb.left - 1 && b.right <= cb.right + 1 && b.top >= cb.top - 1 && b.bottom <= cb.bottom + 1 }; })()")
            need(s["key"] == "empty.note" and s["text"].startswith("Full scan or MS2 files are needed") and s["disabled"] and not s["ink"] and s["inCanvas"], "map prompt: %s" % s)
            for w in (390, 1400):
                pg.set_viewport_size({"width": w, "height": 844}); frames(pg, 6)
                need(pg.evaluate("(() => { const q = E.panels.find(q => q.type === 'map'), b = q._eh.go.getBoundingClientRect(), cb = q.cv.getBoundingClientRect(); return b.width > 0 && b.left >= cb.left - 1 && b.right <= cb.right + 1; })()"), "%d px: the map prompt must sit inside its canvas" % w)
        step("a map panel with no scans: its message is the text-only prompt, nothing written on the canvas, at 1400 and 390 px", map_none)
        step("no missing i18n key, no page error (map)", lambda: need(not [x for x in r2.errs if "missing key" in str(x) or x[0] == "pageerror"], r2.errs))
    r2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    for q in (r, r2):
        try: q and q.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
