"""Files tab on a touch tablet (iPad 810x1080 and 1024x1366, coarse pointer): every tappable control of the list, of its header row, of the toolbar and of the card headers
is at least 44x44 CSS px (the box itself, or its invisible hit area, checked with elementFromPoint at the corners), neighbouring targets never overlap, the rows are shorter than
the 63 px pitch they had, and the desktop (fine pointer) list keeps its compact rows. Light and dark, focus rings and accessible names stay.
Run from the repo root: python3 tests_e2e/e2e_lista_tocco.py"""
# covers: mzlab/web/app.css, mzlab/web/arrange.js, mzlab/web/explore.js, mzlab/web/lista-file.js
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np, dati_sintetici as ds
tmp = Path(tempfile.mkdtemp(prefix="tocco_"))
FULL, MS2 = [], []
for i, t in enumerate((5, 0, 60, 45, 30, 15, 10)):
    p = tmp / f"t{t}.mzML"; ds.full_scan(p, t, np.random.default_rng(i)); FULL.append(str(p))
p = tmp / "blk.mzML"; ds.full_scan(p, 0, np.random.default_rng(99)); FULL.append(str(p))
for i in range(2):
    p = tmp / f"P_{i}.mzML"; ds.ms2(p, np.random.default_rng(i), 15); MS2.append(str(p))
OLD_PITCH = 63          # row pitch (px) of the touch list before the compact rows
MIN = 44
# the controls under test; a pill (button.ppair) is small on purpose and has an invisible 44 px hit area centred on it
CONTROLS = """() => {
  const sels = ['#flst .fl:not(.pr):not(.pend) > input[data-k]', '#flst .fl-x', '#flst .fgh > input', '#flst .fgh .chv', '#flst .fl.pr > input[data-pr]', '#flst .pr-tog',
    '#sb-by', '#addf', '#sb-header .sb-dock-btn', '#fprev', '#fnext', '#fsel', '#np-chrom', '#np-spec', '#np-undo', '#np-redo', '#np-closeall',
    '#dpanels .pnl .hd :is(button,select,[role=button]):not(.ttl)', '#dpanels .pnl .hd input:not([type=hidden]):not([type=checkbox])'];
  const seen = new Set(), out = [];
  for (const s of sels) for (const e of document.querySelectorAll(s)) {
    if (seen.has(e) || e.disabled || e.closest('[hidden]')) continue; seen.add(e);
    const b = e.getBoundingClientRect(); if (!b.width || !b.height) continue;
    const hit = e.matches('button.ppair') ? { l: b.left + b.width / 2 - 22, t: b.top + b.height / 2 - 22, w: 44, h: 44 } : { l: b.left, t: b.top, w: b.width, h: b.height };
    // a point is the control's own when the element on top is the control, one of its children or (for a hit area) the control itself
    const own = (x, y) => { const t = document.elementFromPoint(x, y); return !!t && (t === e || e.contains(t)) ? true : (t ? (t.id || t.className || t.tagName).toString().slice(0, 25) : 'null'); };
    const rad = Math.max(...['borderTopLeftRadius', 'borderTopRightRadius', 'borderBottomLeftRadius', 'borderBottomRightRadius'].map(q => parseFloat(getComputedStyle(e)[q]) || 0));
    const k = 1.5 + 0.3 * Math.min(rad, hit.h / 2), pts = [[hit.l + k, hit.t + k], [hit.l + hit.w - k, hit.t + k], [hit.l + k, hit.t + hit.h - k], [hit.l + hit.w - k, hit.t + hit.h - k]];
    out.push({ n: (e.id || e.className || e.tagName).toString().slice(0, 30), w: Math.round(hit.w * 10) / 10, h: Math.round(hit.h * 10) / 10, l: hit.l, t: hit.t,
      onscreen: hit.t >= 0 && hit.t + hit.h <= innerHeight && hit.l >= 0 && hit.l + hit.w <= innerWidth + 0.5, corners: pts.map(p => own(p[0], p[1])) });
  }
  return out; }"""
PITCH = """() => { const rows = [...document.querySelectorAll('#flst .fl:not(.pr):not(.pend)')].filter(e => e.getBoundingClientRect().height);
  const tops = rows.map(e => e.getBoundingClientRect().top); return { n: rows.length, pitch: tops.length > 1 ? (tops[tops.length - 1] - tops[0]) / (tops.length - 1) : null,
    gh: (document.querySelector('#flst .fgh') || { getBoundingClientRect: () => ({ height: 0 }) }).getBoundingClientRect().height, coarse: matchMedia('(pointer:coarse)').matches }; }"""
FOCUS = """() => { const out = {}; for (const [n, q] of Object.entries({ cb: '#flst .fl > input[data-k]', x: '#flst .fl-x', by: '#sb-by', gall: '#flst .fgh > input' })) {
  const e = document.querySelector(q); if (!e || !e.getBoundingClientRect().width) { out[n] = null; continue; } e.focus({ focusVisible: true });
  const c = getComputedStyle(e); out[n] = { ring: c.outlineStyle !== 'none' && parseFloat(c.outlineWidth) >= 2, name: e.getAttribute('aria-label') || e.getAttribute('title') || (e.textContent || '').trim() }; } return out; }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:400]))
def theme(pg, t):
    pg.evaluate("t => { document.documentElement.dataset.theme = t }", t); ready(pg)
def load(pg, files):
    wait_for_js(pg, "window.qqReady === true", 60000)
    pg.set_input_files("#pick", files); open_first(pg); ready(pg)
    pg.evaluate("document.querySelectorAll('#flst .fgh .chv:not(.on)').forEach(c => c.click())"); ready(pg)
    if pg.is_hidden("#flst"): pg.click("#sb-toggle"); ready(pg)
def targets_ok(pg, label):
    got = pg.evaluate(CONTROLS)
    assert got, f"{label}: no control found"
    bad = [c for c in got if c["w"] < MIN - 0.5 or c["h"] < MIN - 0.5]
    assert not bad, f"{label}: under {MIN} px: " + ", ".join(f"{c['n']} {c['w']}x{c['h']}" for c in bad)
    miss = [c for c in got if c["onscreen"] and not all(v is True for v in c["corners"])]
    assert not miss, f"{label}: corner of the hit area is covered by something else: " + ", ".join(c["n"] + str(sorted(set(v for v in c["corners"] if v is not True))) for c in miss)
    # neighbouring targets of the same band never overlap (a mis-tap on the x or the box next to it)
    for i, a in enumerate(got):
        for b in got[i + 1:]:
            if a["n"] == b["n"] and a["n"] != "fl-x": pass
            ox = min(a["l"] + a["w"], b["l"] + b["w"]) - max(a["l"], b["l"]); oy = min(a["t"] + a["h"], b["t"] + b["h"]) - max(a["t"], b["t"])
            assert not (ox > 1 and oy > 1), f"{label}: targets overlap: {a['n']} and {b['n']} by {ox:.0f}x{oy:.0f}"
    return got
r = Run(port=8898, wd="/tmp/wd_tocco")
try:
    with sync_playwright() as p:
        for (w, h) in ((810, 1080), (1024, 1366)):
            pg = r.page(p, touch=True, is_mobile=True, viewport={"width": w, "height": h})
            load(pg, FULL)
            def pitch(w=w, pg=pg):
                m = pg.evaluate(PITCH)
                assert m["coarse"], "touch emulation did not give a coarse pointer"
                assert m["n"] == 8, f"expected 8 rows, got {m['n']}"
                assert m["pitch"] <= MIN + 2 and m["pitch"] < OLD_PITCH, f"row pitch {m['pitch']} px (was {OLD_PITCH})"
                assert m["gh"] >= MIN - 0.5, f"group header {m['gh']} px tall"
                # no row overlaps the next one
                ov = pg.evaluate("() => { const r = [...document.querySelectorAll('#flst .fl:not(.pr)')].map(e => e.getBoundingClientRect()); return r.slice(1).some((b, i) => b.top < r[i].bottom - 0.6) }")
                assert not ov, "file rows overlap"
            step(f"{w}: rows are {MIN + 1} px apart or less (was {OLD_PITCH})", pitch)
            for t in ("light", "dark"):
                step(f"{w} {t}: every control of the Files tab, toolbar and card headers is {MIN}x{MIN} with its corners hitting it", lambda t=t, w=w, pg=pg: (theme(pg, t), targets_ok(pg, f"{w} {t}")))
                step(f"{w} {t}: focus rings and accessible names stay", lambda t=t, pg=pg: (theme(pg, t), (lambda f: (
                    [f[n] and f[n]["ring"] or (_ for _ in ()).throw(AssertionError(f"{n}: no focus ring")) for n in f if f[n]],
                    [f[n] and f[n]["name"] or (_ for _ in ()).throw(AssertionError(f"{n}: no accessible name")) for n in f if f[n]],
                    (f["x"] or {}).get("name", "").startswith("Remove") or (_ for _ in ()).throw(AssertionError("the x lost its «Remove NAME» name"))))(pg.evaluate(FOCUS))))
            # a tap on the middle of a row selects it, a tap on the x removes it (so the x is not under a neighbour)
            def taps(pg=pg):
                b = pg.evaluate("() => { const r = document.querySelectorAll('#flst .fl:not(.pr)')[1].getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 } }")
                top = pg.evaluate("([x, y]) => document.elementFromPoint(x, y).closest('.fl') && document.elementFromPoint(x, y).closest('.fl').dataset.k", [b["x"], b["y"]])
                assert top is not None, "the middle of a row is not on the row"
            step(f"{w}: the middle of a row is the row", taps)
            pg.close()
        # precursor rows (MS2) of a 1024 tablet: their tick box and arrow are targets too
        pg = r.page(p, touch=True, is_mobile=True, viewport={"width": 1024, "height": 1366})
        load(pg, MS2)
        pg.evaluate("document.querySelectorAll('#flst .pr-container').forEach(c => { c.style.display = '' })")
        step("1024: the precursor rows of an MS2 file are 44 px targets", lambda: targets_ok(pg, "ms2"))
        pg.close()
        # the desktop keeps its compact list
        pg = r.page(p, viewport={"width": 1300, "height": 900})
        load(pg, FULL)
        def desktop():
            m = pg.evaluate(PITCH)
            assert not m["coarse"], "the desktop context has a coarse pointer"
            assert m["pitch"] < 45, f"desktop row pitch grew to {m['pitch']} px"
            cb = pg.evaluate("(() => { const b = document.querySelector('#flst .fl > input[data-k]').getBoundingClientRect(); return [b.width, b.height] })()")
            assert cb == [24, 24], f"desktop tick box is {cb}"
        step("desktop (fine pointer): rows and tick box keep their compact size", desktop)
        pg.close()
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
errs = [e for e in r.errs if e[0] in ("pageerror",)]
if errs: print("PAGE ERRORS", errs[:3])
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
