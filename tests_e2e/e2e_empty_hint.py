"""The empty state of a panel with nothing chosen (mzlab/web/empty-hint.js), on the built site, at 1400 and 390 px: an MRM / SIM panel with no channel never looks blank. It shows a prompt
«Choose a channel» (a real button: Enter / Space open the same picker) and a curved arrow whose end sits just under the picker button; the arrow follows a resize, gives way to a plain
«Choose a channel above» line on a very narrow panel, is decorative (aria-hidden) and disappears when a channel is ticked. Synthetic SIM files of tools/wiff_sim.py, no laboratory data.
The same prompt also points outside the panel: with no file ticked, a chromatogram, XIC or MRM panel points at the file list of the side bar, and an XIC panel with no trace at the «+ XIC» button.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_empty_hint.py"""
# covers: mzlab/web/empty-hint.js, mzlab/web/grafici.js, mzlab/web/lang/en/more-lv-defaults.js
import re, subprocess, sys, os, tempfile
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

D = Path(tempfile.mkdtemp(prefix="empty_hint_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
r = Run(port=8878, wd="/tmp/wd_empty_hint")
P = "E.panels.find(p => p.type === 'mrm')"
# the end of the arrow (last point of its «C» segment) and the bottom centre of the picker button, both in the page's coordinates
READ = """(() => { const p = %s, root = p.el.querySelector('.eh'); if (!root) return null;
  const go = root.querySelector('.eh-go'), ln = root.querySelector('.eh-ln'), t = p.el.querySelector('.canb[data-o=canali]'), rr = root.getBoundingClientRect();
  const d = ln.getAttribute('d') || '', n = (d.match(/-?\\d+(\\.\\d+)?/g) || []).map(Number), tb = t && t.getBoundingClientRect(), gb = go.getBoundingClientRect(), cb = p.cv.getBoundingClientRect();
  return { text: go.textContent, disabled: go.disabled, vis: gb.width > 0 && gb.height > 0, inCanvas: gb.left >= cb.left - 1 && gb.right <= cb.right + 1 && gb.top >= cb.top - 1 && gb.bottom <= cb.bottom + 1,
    arrow: n.length >= 8, end: n.length >= 8 ? [rr.left + n[n.length - 2], rr.top + n[n.length - 1]] : null, start: n.length >= 2 ? [rr.left + n[0], rr.top + n[1]] : null,
    picker: tb ? [tb.left + tb.width / 2, tb.bottom] : null, promptTop: gb.top, hidden: root.querySelector('.eh-ar').getAttribute('aria-hidden'), W: p.el.getBoundingClientRect().width }; })()""" % P

def state(pg): return pg.evaluate(READ)

# an empty state whose control is outside the panel: the arrow is in the fixed layer; its end must land beside the live rectangle of that control (the file list, the «+ XIC» button)
OUT = """(() => { const p = %s, e = p && p._eh; if (!e) return null; const go = e.go, t = document.querySelector(%s), tb = t && t.getBoundingClientRect(), n = e.fx ? (e.fx.ln.getAttribute('d') || '').match(/-?\\d+(\\.\\d+)?/g) : null;
  const end = n && n.length >= 8 ? [+n[n.length - 2], +n[n.length - 1]] : null, gb = go.getBoundingClientRect();
  const gap = !end || !tb ? null : Math.max(tb.left - end[0], end[0] - tb.right, tb.top - end[1], end[1] - tb.bottom);
  return { text: go.textContent, disabled: go.disabled, vis: gb.width > 0 && gb.height > 0, arrow: !!end, gap, onScreen: !!tb && tb.width > 0 && tb.bottom > 0 && tb.top < innerHeight && tb.right > 0 && tb.left < innerWidth,
    fixedLayers: document.querySelectorAll('.eh-fx').length, hidden: e.fx ? e.fx.svg.getAttribute('aria-hidden') : null }; })()"""

def out(pg, panel, sel, label, arrow=True):
    for _ in range(40):      # the layout settles a few frames after a resize
        s = pg.evaluate(OUT % (panel, repr(sel)))
        if s and s["arrow"] == arrow: break
        frames(pg, 3)
    s = pg.evaluate(OUT % (panel, repr(sel))); need(s and s["vis"], "%s: no prompt: %s" % (label, s))
    if arrow:
        need(s["arrow"] and s["onScreen"], "%s: no arrow to the control: %s" % (label, s))
        need(-1 <= s["gap"] <= 14, "%s: the arrow ends %.1f px from the control (want 0..14): %s" % (label, s["gap"], s))
        need(s["hidden"] == "true", "%s: the arrow is decorative: aria-hidden: %s" % (label, s))
    else:
        need(not s["arrow"], "%s: the control is off screen, so no arrow: %s" % (label, s))
    return s

def near(pg, label):
    # the host refits a panel (and its toolbar) a moment after a resize: wait until the arrow rests under the button, then check
    for _ in range(40):
        s = state(pg)
        if s and s["arrow"] and s["picker"] and abs(s["end"][0] - s["picker"][0]) <= 3: break
        frames(pg, 3)
    s = state(pg); need(s and s["vis"] and s["arrow"] and s["picker"], "%s: no prompt or no arrow: %s" % (label, s))
    dx, dy = abs(s["end"][0] - s["picker"][0]), s["end"][1] - s["picker"][1]
    need(dx <= 3 and 0 <= dy <= 14, "%s: the arrow ends %.1f px beside and %.1f px under the picker button (want <= 3 and 0..14): %s" % (label, dx, dy, s))
    need(s["start"][1] > s["end"][1], "%s: the arrow must run upwards from the prompt: %s" % (label, s))
    need(s["start"][1] <= s["promptTop"] + 1, "%s: the arrow starts inside the prompt: %s" % (label, s))
    return s

try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Load data"); ready(pg)
        wait_for_js(pg, "E.files.length >= 1 && E.files.every(f => f.sim > 0)", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        def none():
            pg.evaluate("%s._canOpen = true; ctl(%s)" % (P, P)); frames(pg, 2)
            pg.click("%s" % ".pnl.mrm .can-none"); wait_for_js(pg, "%s._a == null && %s.el.querySelector('.eh .eh-go')" % (P, P), timeout=20000)
            pg.evaluate("%s._canOpen = false; ctl(%s)" % (P, P)); frames(pg, 3)
        def idle(): frames(pg, 3)

        def wide():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 4); none(); frames(pg, 3)
            s = near(pg, "1400 px")
            need(s["text"] == "Choose a channel" and not s["disabled"] and s["inCanvas"], "1400 px: prompt: %s" % s)
            need(s["hidden"] == "true", "the arrow is decorative: aria-hidden: %s" % s)
            need(pg.evaluate("%s.cv.getBoundingClientRect().height" % P) > 80, "the canvas is too short for the prompt")
        step("1400 px, no channel chosen: the prompt «Choose a channel» sits in the panel and the arrow ends just under the picker button", wide)

        def resize():
            for w in (900, 600, 1400):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 5); near(pg, "%d px" % w)
            pg.evaluate("(() => { const q = %s; q.el.style.width = '420px'; })()" % P); frames(pg, 5); near(pg, "panel 420 px wide")
            pg.evaluate("(() => { const q = %s; q.el.style.width = ''; })()" % P); frames(pg, 5); near(pg, "panel back")
        step("a resize of the window or of the panel moves the arrow: its end stays under the picker button", resize)

        def phone():
            pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 6)
            s = near(pg, "390 px")
            need(s["text"] == "Choose a channel" and s["inCanvas"], "390 px: prompt: %s" % s)
            need(pg.evaluate("document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"), "390 px: the page scrolls sideways")
        step("390 px: prompt visible inside the canvas, arrow end under the picker button, no sideways scroll", phone)

        def narrow():
            pg.set_viewport_size({"width": 320, "height": 700}); frames(pg, 6)
            s = state(pg); need(s and s["vis"] and s["W"] < 320, "a 320 px screen must give a panel under 320 px: %s" % s)
            need(not s["arrow"], "a very narrow panel must drop the arrow: %s" % s)
            need(s["text"] == "Choose a channel above" and not s["disabled"], "plain line: %s" % s)
            pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 6); near(pg, "back at 390 px")
        step("a very narrow screen (320 px): «Choose a channel above», no arrow, still a button", narrow)

        def behind():
            # the window grows while the panel is still a frame behind (no refit yet) and a size notification of the panel comes in that frame: a panel that follows the window
            # must not take it for a hand on the resize corner (full = false) and keep its old width for good (the red of the CI: the panel kept 312 px at every later width)
            st = "(() => { const q = %s; return [q.full, q.el.offsetWidth, document.querySelector('#dpanels').clientWidth]; })()" % P
            pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 6)
            pg.set_viewport_size({"width": 320, "height": 700}); frames(pg, 6)
            pg.evaluate("window.__fw = window.fitWidth; window.fitWidth = () => {}")      # the refit of the window is held back: the panel is a frame behind
            try:
                pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 3)
                need(pg.evaluate(st)[1] < pg.evaluate(st)[2] - 8, "the probe needs a panel behind the window: %s" % pg.evaluate(st))
                pg.evaluate("(() => { const q = %s; q.el.style.height = (q.el.offsetHeight - 10) + 'px'; })()" % P); frames(pg, 4)
                need(pg.evaluate(st)[0], "a size notification of a panel that is a frame behind the window made it a hand-sized panel: %s" % pg.evaluate(st))
            finally:
                pg.evaluate("window.fitWidth = window.__fw; dispatchEvent(new Event('resize'))"); frames(pg, 6)
            pg.evaluate("(() => { const q = %s; q.el.style.height = ''; })()" % P); frames(pg, 4)
            f, w, h = pg.evaluate(st); need(f and abs(w - h) <= 1, "the panel does not follow the window: full %s, panel %s px, host %s px" % (f, w, h))
            near(pg, "after the frame behind")
        step("a panel a frame behind the window follows it: a size notification in that frame does not detach it", behind)

        def opens():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 4)
            pg.click(".pnl.mrm .eh-go"); wait_for_js(pg, "document.querySelector('.pnl.mrm .can-pop:not([hidden])')", timeout=10000)
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('.pnl.mrm .can-pop:not([hidden])')", timeout=10000)
            pg.evaluate("%s._canOpen = false; ctl(%s)" % (P, P)); frames(pg, 3)
            pg.focus(".pnl.mrm .eh-go"); need(pg.evaluate("document.activeElement.classList.contains('eh-go')"), "the prompt cannot take the focus")
            pg.keyboard.press("Enter"); wait_for_js(pg, "document.querySelector('.pnl.mrm .can-pop:not([hidden])')", timeout=10000)
            pg.evaluate("%s._canOpen = false; ctl(%s)" % (P, P)); frames(pg, 3)
            pg.focus(".pnl.mrm .eh-go"); pg.keyboard.press("Space"); wait_for_js(pg, "document.querySelector('.pnl.mrm .can-pop:not([hidden])')", timeout=10000)
        step("the prompt opens the same picker: click, Enter and Space", opens)

        def gone():
            pg.click(".can-pop:not([hidden]) .can-all"); wait_for_js(pg, "%s._a && %s._a.sr.length > 0" % (P, P), timeout=20000)
            pg.evaluate("%s._canOpen = false; ctl(%s)" % (P, P)); frames(pg, 3)
            need(pg.evaluate("!%s.el.querySelector('.eh')" % P), "the prompt must go once a channel is plotted")
        step("ticking a channel removes the prompt and the arrow", gone)

        def again():
            none(); frames(pg, 3); near(pg, "back to empty")
        step("unticking everything brings it back", again)

        def not_blank():
            # the empty panel is never plain blank: the prompt is a real, named, visible control inside the canvas
            s = state(pg); need(s["vis"] and s["text"].strip() and s["inCanvas"], s)
            need(pg.evaluate("getComputedStyle(%s.el.querySelector('.eh-go')).borderTopStyle" % P) == "dashed", "prompt styling")
        step("the empty panel is never plain blank", not_blank)
        def words():
            # a panel with no control to point at shows the words of its message: a missing-key warning means a text went through the catalog twice
            pg.evaluate("(() => { E.files.forEach(f => f.vis = false); renderFileList(); E.panels.forEach(q => draw(q)); })()"); frames(pg, 8)
            t = pg.evaluate("E.panels.map(q => (q.el.querySelector('.eh-go') || {}).textContent)")
            need(all(x and "msg." not in x and "empty." not in x for x in t), "no file shown: the prompts must read as sentences: %s" % t)
            bad = [x for x in r.errs if "missing key" in str(x)]; need(not bad, bad)
        step("no file shown: the panels say their message, no missing i18n key", words)

        def files_wide():
            # no file ticked: the MRM panel points at the file list of the side bar (a fixed layer, the panel clips its own children), landing on its edge
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 4)
            pg.evaluate("window.BARRA && BARRA.show('file'); E.files.forEach(f => f.vis = false); renderFileList(); E.panels.forEach(q => draw(q))"); frames(pg, 8)
            wait_for_js(pg, "%s._eh && %s._eh.fx" % (P, P), timeout=20000)
            s = out(pg, P, "#flst", "1400 px")
            need(s["text"] == "Choose a file to show" and not s["disabled"], "1400 px: prompt: %s" % s)
            pg.set_viewport_size({"width": 1100, "height": 900}); frames(pg, 6); out(pg, P, "#flst", "1100 px")
            pg.evaluate("document.scrollingElement.scrollTop = 0"); frames(pg, 3)
        step("no file ticked, 1400 px: the MRM panel prompt «Choose a file to show» and an arrow that ends at the edge of the file list", files_wide)

        def files_click():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 5)
            pg.click(".pnl.mrm .eh-go"); frames(pg, 3)
            need(pg.evaluate("document.activeElement && document.activeElement.matches('#flst input[type=checkbox]')"), "the prompt must take the focus to a file tick in the list")
        step("the prompt takes the focus to the first unticked file of the list", files_click)

        def files_gone():
            pg.evaluate("E.files[0].vis = true; renderFileList(); E.panels.forEach(q => draw(q))"); frames(pg, 8)
            need(pg.evaluate("!%s.el.querySelector('.eh') || %s.el.querySelector('.eh-go').textContent !== 'Choose a file to show'" % (P, P)), "ticking a file must remove the file prompt")
            need(pg.evaluate("document.querySelectorAll('.eh-fx').length") == 0 or not pg.evaluate("%s._eh && %s._eh.fx" % (P, P)), "the fixed arrow layer of a panel with a file must go")
            pg.evaluate("E.files.forEach(f => f.vis = false); renderFileList(); E.panels.forEach(q => draw(q))"); frames(pg, 8)
            out(pg, P, "#flst", "unticked again")
        step("ticking a file removes the prompt and its arrow; unticking brings them back", files_gone)

        def files_phone():
            # on a phone the side bar is a sheet that is shut over the workspace: the control is off screen, so a plain line with no arrow; the prompt opens the sheet
            pg.set_viewport_size({"width": 390, "height": 844}); frames(pg, 8)
            pg.evaluate("window.BARRA && BARRA.close && BARRA.close()"); frames(pg, 6)
            s = out(pg, P, "#flst", "390 px", arrow=False)
            need(s["text"] == "Tick a file in the Files list" and not s["disabled"] and s["fixedLayers"] == 0, "390 px: plain line, still a button, no fixed layer: %s" % s)
            need(pg.evaluate("document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"), "390 px: the page scrolls sideways")
            pg.click(".pnl.mrm .eh-go"); wait_for_js(pg, "document.querySelector('#v-data.sb-open')", timeout=10000)
        step("390 px, side bar shut: «Tick a file in the Files list», no arrow, and the prompt opens the side bar", files_phone)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()

    # a full-scan file: the chromatogram panel (no file ticked) and the XIC panel (no trace) point outside themselves
    r2 = Run(port=8879, wd="/tmp/wd_empty_hint_fs")
    with sync_playwright() as p:
        pg = r2.page(p)
        fs = D / "B_FullMass-t0.mzML"; ds.full_scan(fs, 0, np.random.default_rng(1))
        stage(pg, [fs]); pg.click("text=Load data"); ready(pg); open_first(pg)
        wait_for_js(pg, "E.panels.some(q => q.type === 'chrom' && q._a)", timeout=60000)
        C = "E.panels.find(q => q.type === 'chrom')"; X = "E.panels.find(q => q.type === 'xic')"
        def redraw(): pg.evaluate("renderFileList(); E.panels.forEach(q => draw(q))"); frames(pg, 8)

        def chrom_files():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 4)
            pg.evaluate("window.BARRA && BARRA.show('file'); E.files.forEach(f => f.vis = false)"); redraw()
            s = out(pg, C, "#flst", "chromatogram, 1400 px"); need(s["text"] == "Choose a file to show", s)
            pg.evaluate("E.files[0].vis = true"); redraw(); need(pg.evaluate("!%s._eh" % C), "a ticked file must remove the prompt")
        step("no file ticked: the chromatogram panel points at the file list", chrom_files)

        def xic_none():
            pg.evaluate("E.files[0].vis = true"); redraw()
            pg.evaluate("setTab('full')"); pg.wait_for_selector("#np-xic", state="visible", timeout=30000)
            pg.click("#np-xic"); pg.fill("#xic-mz", "364"); pg.click("#xic-go"); wait_for_js(pg, "%s && %s._a && %s._a.sr.length > 0" % (X, X, X), timeout=60000)
            pg.evaluate("(() => { const q = %s; q.traces = []; q._xt = 0; ctl(q); draw(q); })()" % X); frames(pg, 10)
            pg.evaluate("document.scrollingElement.scrollTop = 0"); frames(pg, 3)
            s = out(pg, X, "#np-xic", "xic, 1400 px")
            need(s["text"] == "Add an XIC" and not s["disabled"], "xic prompt: %s" % s)
            pg.click(".pnl.xic .eh-go"); wait_for_js(pg, "document.querySelector('#xicdlg[open]')", timeout=10000)
            pg.keyboard.press("Escape"); frames(pg, 3)
        step("an XIC panel with no trace points at the «+ XIC» button, and the prompt opens its dialog", xic_none)

        def xic_off():
            # the button is not on screen (a shut or collapsed toolbar): the line says what to do in words, no arrow
            pg.evaluate("document.querySelector('#np-xic').style.display = 'none'"); frames(pg, 6)
            s = out(pg, X, "#np-xic", "xic, button hidden", arrow=False); need(s["text"].startswith("Add an XIC with the"), s)
            pg.evaluate("document.querySelector('#np-xic').style.display = ''"); frames(pg, 6); out(pg, X, "#np-xic", "xic, button back")
        step("the «+ XIC» button off screen: the plain line, no arrow; back on screen: the arrow", xic_off)

        def xic_files():
            pg.evaluate("E.files.forEach(f => f.vis = false)"); redraw()
            need(pg.evaluate("%s._eh.spec.key" % X) == "empty.file", "no file ticked: the XIC panel asks for a file first")
            out(pg, X, "#flst", "xic, no file")
        step("an XIC panel with no file ticked asks for a file first", xic_files)
        step("no page error (full scan)", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r2.errs if x[0] == "pageerror"]))
    r2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    for q in (r, globals().get("r2")):
        try: q and q.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
