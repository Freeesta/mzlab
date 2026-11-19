"""Shared by the four header-walk tests (e2e_giro_*): the page probes (OPEN, GEOM, HDR) and the walks (walk, round_trip, floating_outside_data). Not a test."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

# what is open: dialogs, menus, pop-ups, pressed header buttons, the side panel and its tool, full screen
OPEN = r"""() => { const vis = e => { if (!e.isConnected || e.hidden) return false; const cs = getComputedStyle(e); if (cs.display === 'none' || cs.visibility === 'hidden') return false; const b = e.getBoundingClientRect(); return b.width > 2 && b.height > 2; };
 const lab = e => e.id ? '#' + e.id : e.tagName.toLowerCase() + (typeof e.className === 'string' && e.className ? '.' + e.className.trim().split(/\s+/)[0] : '');
 const dlg = [...document.querySelectorAll('dialog[open], [role=dialog], [role=menu], #helppop, #calcdlg')].filter(vis).map(lab);
 return {dlg: [...new Set(dlg)].sort(), exp: [...document.querySelectorAll("header [aria-expanded=true], [role=menu] [aria-expanded=true]")].filter(vis).map(lab).sort(), pressed: [...document.querySelectorAll('header [aria-pressed=true]')].map(lab).sort(),
  sb: vis(document.querySelector('#sb-pane')) ? BARRA.curTab : null, view: S.view, fs: !!document.fullscreenElement}; }"""
# the main blocks, measured from the top of the page (a new panel scrolls to itself: that is not a move); panels by position in E.panels
GEOM = r"""() => { window.scrollTo(0, 0); const o = {}, put = (k, e) => { if (!e || !e.isConnected) return; const b = e.getBoundingClientRect(); if (!b.width && !b.height) return; o[k] = [b.x, b.y, b.width, b.height].map(Math.round); };
 for (const s of ['header', '#stbar', '#sb-pane', '#tools', '#v-workspace', '#start', '#v-draw']) put(s, document.querySelector(s));
 E.panels.forEach((p, i) => put('panel' + i + ':' + p.type, p.el)); return o; }"""
# the header buttons a user sees, by a selector that finds them again (the tabs of the views are walked apart)
HDR = r"""() => [...document.querySelectorAll('header button')].filter(b => { const r = b.getBoundingClientRect(); return r.width > 0 && r.height > 0 && !b.disabled && !b.closest('#nav'); })
 .map(b => b.id ? '#' + b.id : b.dataset.cmd ? `header [data-cmd="${b.dataset.cmd}"]` : b.dataset.cmdMain ? `header [data-cmd-main="${b.dataset.cmdMain}"]` : b.dataset.cmdCar ? `header [data-cmd-car="${b.dataset.cmdCar}"]` : b.dataset.help != null ? `header .hq[data-help="${b.dataset.help}"]` : null).filter(Boolean)"""
# outside Data these open their window: a click that does nothing there is a dead button
FLOATING = {"#np-calc2", "#np-add", "#np-nl", "#np-addloss", "#np-liste", "#np-pt", "#tb-search", "#tb-more", "#np-set", "#newrun"}
# outside Data these go back to Data with their side panel page (the Adducts & losses lists never float: docs/agenti/data-3200.md)
TO_DATA = {"#np-addloss": "addloss"}
MOVE_PX = 2       # rounding of sub-pixel layouts; a slide or a jump is tens of pixels


def moved(a, b):
    return {k: (a.get(k), b.get(k)) for k in set(a) | set(b) if a.get(k) is None or b.get(k) is None or max(abs(x - y) for x, y in zip(a[k], b[k])) > MOVE_PX}


def back_to(pg, base):
    """Esc, then the side panel as it was: what a user does to get rid of what a click opened."""
    for _ in range(2):
        if pg.evaluate(OPEN) == base: return
        pg.keyboard.press("Escape"); frames(pg, 3)
    if pg.evaluate("!!document.fullscreenElement"): pg.evaluate("document.exitFullscreen()"); wait_for_js(pg, "!document.fullscreenElement")
    if pg.evaluate(OPEN)["sb"] != base["sb"]:
        pg.evaluate("sb => { if (sb) BARRA.show(sb, {noFocus: true, instant: true}); else BARRA.close(); }", base["sb"]); frames(pg, 3)


def to_data(pg, sel, base):
    """A TO_DATA button outside Data: Data with its page open; then Data's side panel as it was and back to the view it came from."""
    tab0 = pg.evaluate("BARRA.curTab")
    pg.click(sel); wait_for_js(pg, f"S.view === 'data' && BARRA.isDataView() && BARRA.curTab === '{TO_DATA[sel]}'"); frames(pg, 3); ready(pg, settle=200)
    pg.evaluate("t => BARRA.show(t, {noFocus: true, instant: true})", tab0); frames(pg, 3)
    view(pg, base["view"]); back_to(pg, base)


def walk(pg, where):
    """Every header button: it reacts, Esc or a second click puts the page back, nothing moved."""
    bad = []
    for sel in pg.evaluate(HDR):
        base, g0 = pg.evaluate(OPEN), pg.evaluate(GEOM)
        if sel in TO_DATA and base["view"] != "data":
            to_data(pg, sel, base); ready(pg, settle=200)
            if pg.evaluate(OPEN) != base: bad.append(f"{sel}: state {base} → {pg.evaluate(OPEN)}")
            m = moved(g0, pg.evaluate(GEOM))
            if m: bad.append(f"{sel}: moved {m}")
            continue
        pg.click(sel); frames(pg, 3); ready(pg, settle=200)
        st = pg.evaluate(OPEN)
        if st == base: bad.append(f"{sel}: nothing opened")
        else:
            pg.keyboard.press("Escape"); frames(pg, 3)
            if pg.evaluate(OPEN) != base and sel not in ("#np-pt", "#newrun", "#np-set", "#tb-search", "#tb-more"):
                pg.click(sel); frames(pg, 3)
            # leaving full screen is asynchronous (exitFullscreen resolves later): wait for it, bounded, before reading what is left
            try: pg.wait_for_function("fs => !!document.fullscreenElement === fs", arg=base["fs"], timeout=5000)
            except Exception: pass                     # still in full screen: the check below reports it
            # a window, menu or full screen must be gone; a tool of the side panel may close the panel (a second click hides it, data-bar.md)
            left = {k: v for k, v in pg.evaluate(OPEN).items() if k in ("dlg", "exp", "fs") and v != base[k]}
            if left: bad.append(f"{sel}: left {left}")
        back_to(pg, base); ready(pg, settle=200)
        m = moved(g0, pg.evaluate(GEOM))
        if m: bad.append(f"{sel}: moved {m}")
    assert not bad, f"{where}: " + " | ".join(bad[:6])


def view(pg, v):
    pg.click(f'#nav [data-v="{v}"]'); wait_for_js(pg, f"S.view === '{v}'"); frames(pg, 3); ready(pg, settle=200)


def round_trip(pg, views):
    """Data → each view → Data: the Data blocks end where they were, the side panel as it was."""
    g0, s0, bad = pg.evaluate(GEOM), pg.evaluate(OPEN), []
    for v in views:
        view(pg, v); view(pg, "data")
        m, s1 = moved(g0, pg.evaluate(GEOM)), pg.evaluate(OPEN)
        if m: bad.append(f"via {v}: moved {m}")
        if s1 != s0: bad.append(f"via {v}: state {s0} → {s1}")
    assert not bad, " | ".join(bad[:4])


def floating_outside_data(pg, v):
    """In Drawing, Flow and Tabui the tool buttons open their window (the side panel is Data's)."""
    view(pg, v)
    dead = []
    for sel in FLOATING:
        if not pg.is_visible(sel): continue
        base = pg.evaluate(OPEN)
        if sel in TO_DATA: to_data(pg, sel, base); continue
        pg.click(sel); frames(pg, 3)
        if pg.evaluate(OPEN) == base: dead.append(sel)
        back_to(pg, base)
    assert not dead, f"dead buttons in {v}: {dead}"


def clean(r):
    """No console error and no failed request in the pages of this test (the optional modules and the sandboxed Ketcher note are known)."""
    errs = [e for e in r.errs if "wasm-feat/manifest.json" not in str(e)          # the optional modules are not built for the e2e site (tools/prepara_sito.py)
            and not (e[0] == "console.error" and "404" in str(e[1]))               # its console twin
            and "allow-scripts and allow-same-origin" not in str(e)]               # Chromium's note on the sandboxed Ketcher iframe (docs/agenti/drawing.md)
    assert not errs, errs[:5]


def make_step(steps):
    """The `step(name, fn)` of a test: runs it, records ok or the failing line; the time is printed so a slow step shows in the log."""
    import time, traceback
    def step(name, fn):
        t0 = time.time()
        try: fn(); steps.append((name, "ok"))
        except Exception as e:
            ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if os.path.basename(f.filename).startswith("e2e_giro")][-1]
            steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
        print(f"[{time.time() - t0:6.1f} s] {name}", flush=True)
    return step


def finish(steps):
    for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
    sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
