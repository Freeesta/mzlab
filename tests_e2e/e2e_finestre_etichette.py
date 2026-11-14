"""Dialogs: every radio, checkbox and field sits next to its label (no control pushed to the far edge of a wide dialog, no radio centred away from its text),
at 1000/1600/2000 px and on iPad (1024x768, 1366x1024, touch): the baseline of an integration, the peak detection settings, the table of the integrations and the
sample columns."""
# covers: mzlab/web/integra-base.js, mzlab/web/app.css, mzlab/web/picchi.js
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_finestre_etichette.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:400]))

r = Run(port=8962, wd="/tmp/wd_finestre_etichette")
SKIP = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1')}catch(_){}"
# every visible label of the open dialog with its control (label.control: wrapped or for=): the text of the label (its text nodes outside the control) and the
# control on one row at most 120 px apart (fields of a section share a column: a short label is that far from it at most) (a radio or checkbox before its text, the centre of its mark at most 24 px from it), or the control under the text, aligned to it; label of a radio
# or checkbox at least 24 px tall (the target); nothing past the right edge of the dialog
PAIRS = r"""(() => { const d = [...document.querySelectorAll('dialog[open]')].pop(); if (!d) return ['no dialog open'];
  const bad = [], dr = d.getBoundingClientRect(), vis = e => e.offsetWidth && e.offsetHeight && getComputedStyle(e).visibility !== 'hidden';
  let n = 0;
  for (const l of d.querySelectorAll('label')) { const c = l.control; if (!c || !vis(l) || !vis(c) || c.type === 'hidden') continue;
    const rg = document.createRange(), rs = []; const w = document.createTreeWalker(l, NodeFilter.SHOW_TEXT);
    for (let t; (t = w.nextNode());) { if (!t.textContent.trim() || c.contains(t) || (t.parentElement.closest('select,button,input'))) continue; rg.selectNodeContents(t); rs.push(...rg.getClientRects()); }
    if (!rs.length) continue; n++;
    const T = { left: Math.min(...rs.map(q => q.left)), right: Math.max(...rs.map(q => q.right)), top: Math.min(...rs.map(q => q.top)), bottom: Math.max(...rs.map(q => q.bottom)) };
    const C = c.getBoundingClientRect(), nm = (l.textContent.trim() || c.id || c.name).slice(0, 40), box = c.type === 'radio' || c.type === 'checkbox';
    if (box && C.width > 32) bad.push(`«${nm}»: the ${c.type} is ${Math.round(C.width)} px wide`);
    const row = Math.min(T.bottom, C.bottom) - Math.max(T.top, C.top) > 0;
    if (row) { const gap = box ? T.left - (C.left + C.right) / 2 : Math.max(C.left - T.right, T.left - C.right, 0);   // a radio or checkbox: from the centre of its mark
      if (box ? gap < 0 || gap > 24 : gap > 120) bad.push(`«${nm}»: ${c.type || 'control'} ${Math.round(gap)} px from its text`); }
    else if (Math.abs(C.left - T.left) > 48 || C.top - T.bottom > 24 || T.top - C.bottom > 24) bad.push(`«${nm}»: control away from its text (${Math.round(C.left - T.left)}, ${Math.round(C.top - T.bottom)})`);
    if (box && l.getBoundingClientRect().height < 23.5) bad.push(`«${nm}»: label ${Math.round(l.getBoundingClientRect().height)} px tall`);
    if (C.right > dr.right + 1 || C.left < dr.left - 1) bad.push(`«${nm}»: control outside the dialog`); }
  return n ? bad : ['no labelled control'];
})()"""
WIDTHS = [((1000, 900), False), ((1600, 1000), False), ((2000, 1100), False), ((1024, 768), True), ((1366, 1024), True)]


def page(size, touch):
    k = dict(viewport={"width": size[0], "height": size[1]}, locale="it-IT", reduced_motion="reduce")
    if touch: k.update(has_touch=True, is_mobile=True)
    c = r.b.new_context(**k); c.add_init_script(SKIP)
    q = c.new_page(); q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_selector("#pick", state="attached"); ready(q)
    return q


def check(q, what, size, opened):
    wait_for_js(q, opened, 30000); frames(q, 3)
    bad = q.evaluate(PAIRS); assert not bad, (size, what, bad)
    q.keyboard.press("Escape"); wait_for_js(q, "!document.querySelector('dialog[open]')")


def mrm(size, touch):
    q = page(size, touch)
    try:
        q.set_input_files("#pick", [mz("B_MRM-t0"), mz("B_MRM-t5")]); hold(q, 1000)
        q.evaluate("document.getElementById('colsbtn')?.click()")
        if q.evaluate("!!document.querySelector('dialog[open]')"): check(q, "sample columns", size, "!!document.querySelector('dialog[open]')")
        q.click("text=Carica dati"); ready(q)
        wait_for_js(q, "!!document.querySelector('.pnl.mrm canvas') && E.panels.some(p => p.type === 'mrm' && p._a)", 60000)
        q.evaluate("document.querySelector('.pnl.mrm [data-a=iauto]').click()"); frames(q, 2)
        b = q.locator(".pnl.mrm canvas").first.bounding_box()
        for f in (0.70, 0.71, 0.69, 0.72):
            q.mouse.click(b["x"] + b["width"] * f, b["y"] + b["height"] * 0.5); frames(q, 3)
            if q.evaluate("E.panels.some(p => p.type === 'mrm' && p.ints && p.ints.length)"): break
        assert q.evaluate("E.panels.some(p => p.type === 'mrm' && p.ints && p.ints.length)"), (size, "no integration")
        # the baseline dialog (key B / «Linea di base…»), with a custom line so that its fields are live
        q.evaluate("carica('integra-base.js').then(() => { const p = E.panels.find(p => p.type === 'mrm' && p.ints.length); INTBASE.open(p, p.ints[0]); })")
        wait_for_js(q, "!!document.querySelector('#bigdlg[open] input[name=ib-m]')", 30000)
        q.click("#bigdlg input[name=ib-m][value=line]"); frames(q, 2)
        w = q.evaluate("document.getElementById('bigdlg').getBoundingClientRect().width")
        check(q, "baseline", size, "!!document.querySelector('#bigdlg[open] #ib-pts')")
        assert w <= 40 * 16 + 1, (size, "the baseline dialog is at most 40rem wide", w)
        q.evaluate("peakParams()"); check(q, "peak detection", size, "!!document.querySelector('#bigdlg[open] [data-pk]')")
        q.evaluate("showInts()"); check(q, "integrations", size, "!!document.querySelector('#bigdlg[open] #ig-sn')")
    finally:
        q.context.close()


bad = ["not run"]
try:
    with sync_playwright() as p:
        r.page(p).context.close()      # the browser of the run (Chromium, or WebKit with --browser)
        step("dialogs: every radio, checkbox and field next to its label (1000, 1600, 2000 px, iPad 1024x768 and 1366x1024)", lambda: [mrm(s, t) for s, t in WIDTHS])
    bad = [s for s in steps if s[1] != "ok"]
    for n, s in steps: print(("OK " if s == "ok" else "FAIL ") + n + (" " + s if s != "ok" else ""))
finally:
    r.close()
sys.exit(1 if bad else 0)
