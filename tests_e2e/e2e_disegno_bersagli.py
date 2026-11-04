"""Draw tab, touch targets and names (WCAG 2.5.8): side cards and Ketcher controls >= 24 px, 40 px context-menu rows, named copy triangle,
long-press tooltip with the tool name, skip link to Quick draw, and no change of the desktop layout."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_disegno_bersagli.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:600]))
r = Run(port=8862, wd="/tmp/wd_disegno_bersagli")
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
READY = "document.getElementById('kframe').classList.contains('kwait') === false && !!(window.TPDraw && TPDraw.ready())"
def open_draw(ctx):
    pg = ctx.new_page(); pg.add_init_script(INIT)
    pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "!!document.querySelector('#nav [data-v=draw]')")
    pg.click("#nav [data-v=draw]"); attendi(pg, READY, timeout=60000); hold(pg, 2000)
    return pg
# every visible control of the aside and of the Ketcher iframe, as page coordinates; the aside is measured with every <details> open
TARGETS = """(() => {
  const SEL = 'button,a[href],input:not([type=hidden]),select,summary,label,[role=menuitem]', out = [];
  const add = (root, dx, dy, where) => { for (const e of root.querySelectorAll(SEL)) {
    if (e.closest('[hidden]')) continue; const cs = e.ownerDocument.defaultView.getComputedStyle(e);
    if (cs.visibility === 'hidden' || cs.display === 'none') continue;
    if (e.tagName === 'LABEL' && !e.querySelector('input,select')) continue;
    if (e.tagName === 'INPUT' && e.closest('label') && e.type !== 'checkbox' && e.type !== 'radio') { /* a field inside a label: both count */ }
    const b = e.getBoundingClientRect(); if (b.width < 1 || b.height < 1) continue;
    const id = e.id ? '#' + e.id : e.getAttribute('data-testid') ? '[' + e.getAttribute('data-testid') + ']' : (e.textContent || e.title || '').trim().slice(0, 24);
    out.push({n: where + ' ' + e.tagName.toLowerCase() + ' ' + id, x: b.left + dx, y: b.top + dy, w: b.width, h: b.height}); } };
  for (const d of document.querySelectorAll('#v-draw aside details')) d.open = true;
  add(document.querySelector('#v-draw aside'), 0, 0, 'aside');
  const f = document.getElementById('kframe'), fb = f.getBoundingClientRect(); add(f.contentDocument, fb.left, fb.top, 'ketcher');
  return out; })()"""
def small(items):
    """Targets below 24x24 that do not meet the spacing exception (a 24 px circle centred on them touches no other target)."""
    bad = []
    for a in items:
        if a["w"] >= 23.5 and a["h"] >= 23.5: continue
        cx, cy = a["x"] + a["w"] / 2, a["y"] + a["h"] / 2; ok = True
        for b in items:
            if b is a: continue
            if b["x"] <= a["x"] + 0.5 and b["y"] <= a["y"] + 0.5 and b["x"] + b["w"] >= a["x"] + a["w"] - 0.5 and b["y"] + b["h"] >= a["y"] + a["h"] - 0.5: continue   # a target inside another (checkbox in its label)
            if a["x"] <= b["x"] + 0.5 and a["y"] <= b["y"] + 0.5 and a["x"] + a["w"] >= b["x"] + b["w"] - 0.5 and a["y"] + a["h"] >= b["y"] + b["h"] - 0.5: continue
            dx = max(b["x"] - cx, 0, cx - (b["x"] + b["w"])); dy = max(b["y"] - cy, 0, cy - (b["y"] + b["h"]))
            if (dx * dx + dy * dy) ** .5 < 12 - 0.01: ok = False; break
        if not ok: bad.append(f'{a["n"]} {a["w"]:.0f}x{a["h"]:.0f}')
    return bad
def tip_probe(pg, tid):
    """Centre of a Ketcher toolbar button, in page coordinates."""
    return pg.evaluate("""(tid) => { const f = document.getElementById('kframe'), fb = f.getBoundingClientRect();
      const b = [...f.contentDocument.querySelectorAll('[data-testid="' + tid + '"]')].find(e => e.getBoundingClientRect().width > 0); if (!b) return null;
      const x = b.getBoundingClientRect(); return [fb.left + x.left + x.width / 2, fb.top + x.top + x.height / 2]; }""", tid)
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 820, "height": 1180}, has_touch=True, is_mobile=True)
        pg = open_draw(ctx); cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": 0} for x, y in pts]})
        def open_dialog():
            pg.evaluate("document.getElementById('kframe').contentDocument.querySelector('[data-testid=open-file-button]').click()"); hold(pg, 800)
        def sizes():
            items = pg.evaluate(TARGETS); bad = small(items)
            assert not bad, "too small: " + "; ".join(bad[:12]) + (f" (+{len(bad) - 12})" if len(bad) > 12 else "")
        step("1 controls >= 24 px (aside and Ketcher)", sizes)
        def dialog():
            open_dialog()
            try:
                items = pg.evaluate(TARGETS); bad = small(items); assert not bad, "dialog: " + "; ".join(bad[:8])
                txt = pg.evaluate("(document.getElementById('kframe').contentDocument.body.innerText || '')")
                assert "drag file here" not in txt.lower(), "drag text visible"
                assert "image recognition" not in txt.lower(), "dead tile visible"
            finally:
                pg.evaluate("(() => { const d = document.getElementById('kframe').contentDocument; const c = d.querySelector('[data-testid=close-window-button]'); if (c) c.click(); })()"); hold(pg, 300)
        step("1b open-structure dialog", dialog)
        def menu():
            pg.evaluate("TPDraw.load('c1ccccc1')"); hold(pg, 600)
            c = pg.evaluate("(() => { const f = document.getElementById('kframe'), b = f.getBoundingClientRect(); return [b.left + b.width / 2, b.top + 300]; })()")
            pg.evaluate("""(c) => { const d = document.getElementById('kframe').contentDocument, e = d.elementFromPoint(c[0] - document.getElementById('kframe').getBoundingClientRect().left, c[1] - document.getElementById('kframe').getBoundingClientRect().top) || d.body;
              e.dispatchEvent(new MouseEvent('contextmenu', {bubbles: true, cancelable: true, clientX: c[0] - document.getElementById('kframe').getBoundingClientRect().left, clientY: c[1] - document.getElementById('kframe').getBoundingClientRect().top, view: d.defaultView})); }""", c); hold(pg, 400)
            hs = pg.evaluate("[...document.querySelectorAll('#ctx [role=menuitem]')].filter(e => e.getBoundingClientRect().height > 0).map(e => e.getBoundingClientRect().height)")
            assert hs, "no menu rows"
            assert min(hs) >= 39.5, f"rows {min(hs)} px"
            pg.keyboard.press("Escape"); hold(pg, 200)
        step("2 context menu rows >= 40 px", menu)
        def triangle():
            n = pg.evaluate("""(() => { const t = document.getElementById('kframe').contentDocument.querySelector('[data-testid=copy-button-dropdown-triangle]'); return t ? (t.getAttribute('aria-label') || '').trim() : null; })()""")
            assert n, f"copy triangle aria-label {n!r}"
        step("3 copy triangle has an aria-label", triangle)
        def longpress():
            sel = lambda: pg.evaluate("(() => { const d = document.getElementById('kframe').contentDocument; return [...d.querySelectorAll('[data-testid=erase]')].some(e => e.getBoundingClientRect().width > 0 && /selected|active/.test(e.className + ' ' + (e.getAttribute('aria-pressed') || ''))); })()")
            pg.evaluate("document.getElementById('kframe').contentWindow.ketcher.editor.tool('select'), 1")
            e = tip_probe(pg, "erase"); assert e, "no erase button"
            T("touchStart", [tuple(e)]); hold(pg, 650)
            tip = pg.evaluate("(() => { const t = document.getElementById('kframe').contentDocument.getElementById('tp-tip'); return t && t.hasAttribute('data-on') ? [t.textContent.trim(), t.getAttribute('aria-live')] : null; })()")
            T("touchEnd", []); hold(pg, 300)
            assert tip and tip[0], f"no tooltip {tip}"; assert tip[1] == "polite", f"aria-live {tip[1]}"
            assert not sel(), "long press selected the tool"
            T("touchStart", [tuple(e)]); hold(pg, 60); T("touchEnd", []); hold(pg, 400)
            assert sel(), "tap did not select the tool"
        step("4 long press shows the name, tap selects", longpress)
        def skip():
            pg.evaluate("document.activeElement && document.activeElement.blur(); document.querySelector('header, #hdr, .hd, body').scrollIntoView()")
            pg.evaluate("(() => { const h = document.querySelector('button.hq'); h.focus(); })()")
            ok = False
            for _ in range(80):
                pg.keyboard.press("Tab")
                if pg.evaluate("(document.activeElement && document.activeElement.classList && document.activeElement.classList.contains('tp-skip')) ? 1 : 0"): ok = True; break
            assert ok, "Tab did not reach the skip link"
            assert pg.evaluate("document.activeElement.getBoundingClientRect().width > 1"), "skip link not visible on focus"
            pg.keyboard.press("Enter"); hold(pg, 200)
            assert pg.evaluate("document.activeElement.id") == "ex-smi", "focus not on #ex-smi"
        step("5 skip link to Quick draw", skip)
        ctx.close()
        def desktop():
            ctx = r.b.new_context(viewport={"width": 1500, "height": 900}); pg = open_draw(ctx)
            rects = pg.evaluate("[...document.querySelectorAll('#v-draw aside *')].filter(e => e.getBoundingClientRect().width > 0).map(e => { const b = e.getBoundingClientRect(); return [e.tagName + (e.id ? '#' + e.id : ''), Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)]; })")
            ref = "/tmp/bersagli_desktop_ref.json"          # rects of main, saved with MZLAB_BERSAGLI_SAVE=1 on main; without it only the rule that matters is checked
            if os.environ.get("MZLAB_BERSAGLI_SAVE"): json.dump(rects, open(ref, "w")); return
            if os.path.exists(ref):
                old = json.load(open(ref)); assert old == rects, "aside rects differ from main: " + str([(a, b) for a, b in zip(old, rects) if a != b][:3])
            h = pg.evaluate("document.querySelector('#lb-f').closest('label').getBoundingClientRect().height"); assert h < 30, f"desktop label {h} px"
            assert pg.evaluate("!document.querySelector('.tp-skip') || getComputedStyle(document.querySelector('.tp-skip')).position === 'absolute' || getComputedStyle(document.querySelector('.tp-skip')).position === 'fixed'")
            ctx.close()
        step("6 desktop unchanged", desktop)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(n, s)
print("FAIL" if bad else "OK")
sys.exit(1 if bad else 0)
