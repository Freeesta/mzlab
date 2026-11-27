"""The empty state of a panel with nothing chosen (mzlab/web/empty-hint.js), on the built site, at 1400 and 390 px: an MRM / SIM panel with no channel never looks blank. It shows a prompt
«Choose a channel» (a real button: Enter / Space open the same picker) and a curved arrow whose end sits just under the picker button; the arrow follows a resize, gives way to a plain
«Choose a channel above» line on a very narrow panel, is decorative (aria-hidden) and disappears when a channel is ticked. Synthetic SIM files of tools/wiff_sim.py, no laboratory data.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_empty_hint.py"""
# covers: mzlab/web/empty-hint.js, mzlab/web/grafici.js, mzlab/web/lang/en/more-lv-defaults.js
import re, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

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
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
