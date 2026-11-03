"""Header walk: on every view (Dati, Disegno, Flow, Tabui), without samples and with one, every header button opens its window, menu or panel and
Esc (or the same button) closes it; the page comes back to the state it had and no main block (header, side panel, toolbar, work area, panels)
has moved. Leaving Dati for another view and coming back moves nothing (and slides nothing, with motion on); the page is never taller than the window; the first file keeps the width of the side panel; the Flow bar does not jump when its late part arrives; no console error, no failed request. Audit: docs/studi/AUDIT_PAGINE.md."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_giro_intestazione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))

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
# outside Dati these open their window: a click that does nothing there is a dead button
FLOATING = {"#np-calc2", "#np-add", "#np-nl", "#np-addloss", "#np-liste", "#np-pt", "#tb-search", "#tb-more", "#np-set", "#newrun"}
# outside Dati these go back to Dati with their side panel page (the Adducts & losses lists never float: docs/studi/ADDOTTI_PERDITE_PAGINA_UNICA.md §5)
TO_DATA = {"#np-addloss": "addloss"}
MOVE_PX = 2       # rounding of sub-pixel layouts; a slide or a jump is tens of pixels


def moved(a, b):
    return {k: (a.get(k), b.get(k)) for k in set(a) | set(b) if a.get(k) is None or b.get(k) is None or max(abs(x - y) for x, y in zip(a[k], b[k])) > MOVE_PX}


def back_to(pg, base):
    """Esc, then the side panel as it was: what a user does to get rid of what a click opened."""
    for _ in range(2):
        if pg.evaluate(OPEN) == base: return
        pg.keyboard.press("Escape"); frames(pg, 3)
    if pg.evaluate("!!document.fullscreenElement"): pg.evaluate("document.exitFullscreen()"); attendi(pg, "!document.fullscreenElement")
    if pg.evaluate(OPEN)["sb"] != base["sb"]:
        pg.evaluate("sb => { if (sb) BARRA.show(sb, {noFocus: true, instant: true}); else BARRA.close(); }", base["sb"]); frames(pg, 3)


def to_data(pg, sel, base):
    """A TO_DATA button outside Dati: Dati with its page open; then Dati's side panel as it was and back to the view it came from."""
    tab0 = pg.evaluate("BARRA.curTab")
    pg.click(sel); attendi(pg, f"S.view === 'data' && BARRA.isDataView() && BARRA.curTab === '{TO_DATA[sel]}'"); frames(pg, 3); ready(pg, settle=200)
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
            # a window, menu or full screen must be gone; a tool of the side panel may close the panel (a second click hides it, dati-barra.md)
            left = {k: v for k, v in pg.evaluate(OPEN).items() if k in ("dlg", "exp", "fs") and v != base[k]}
            if left: bad.append(f"{sel}: left {left}")
        back_to(pg, base); ready(pg, settle=200)
        m = moved(g0, pg.evaluate(GEOM))
        if m: bad.append(f"{sel}: moved {m}")
    assert not bad, f"{where}: " + " | ".join(bad[:6])


def view(pg, v):
    pg.click(f'#nav [data-v="{v}"]'); attendi(pg, f"S.view === '{v}'"); frames(pg, 3); ready(pg, settle=200)


def round_trip(pg, views):
    """Dati → each view → Dati: the Dati blocks end where they were, the side panel as it was."""
    g0, s0, bad = pg.evaluate(GEOM), pg.evaluate(OPEN), []
    for v in views:
        view(pg, v); view(pg, "data")
        m, s1 = moved(g0, pg.evaluate(GEOM)), pg.evaluate(OPEN)
        if m: bad.append(f"via {v}: moved {m}")
        if s1 != s0: bad.append(f"via {v}: state {s0} → {s1}")
    assert not bad, " | ".join(bad[:4])


def floating_outside_data(pg, v):
    """In Disegno, Flow and Tabui the tool buttons open their window (the side panel is Dati's)."""
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


r = Run(port=8996, wd="/tmp/wd_giro_intestazione")
VIEWS = ["draw", "flow", "tpmine"]
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        attendi(pg, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(pg)
        attendi(pg, "!!document.querySelector('#nav [data-v=flow]') && !!document.querySelector('#nav [data-v=tpmine]')")
        step("no samples, Dati: every header button opens and closes, nothing moves", lambda: walk(pg, "data, empty"))
        for v in VIEWS:
            step(f"no samples, {v}: every header button opens and closes, nothing moves", lambda v=v: (view(pg, v), walk(pg, f"{v}, empty"), view(pg, "data")))
        for v in VIEWS:
            step(f"no samples, {v}: the tool buttons open their window", lambda v=v: (floating_outside_data(pg, v), view(pg, "data")))
        step("no samples: Dati → each view → Dati moves nothing", lambda: round_trip(pg, VIEWS))

        def with_sample():
            stage(pg, [mz("B_FullMass-t15")]); open_first(pg); ready(pg)
            pg.evaluate("window.scrollTo(0, 0)"); frames(pg, 2)
        step("one sample loaded", with_sample)
        step("one sample, Dati: every header button opens and closes, nothing moves", lambda: walk(pg, "data, sample"))
        step("one sample, draw: every header button opens and closes, nothing moves", lambda: (view(pg, "draw"), walk(pg, "draw, sample"), view(pg, "data")))
        step("one sample: Dati → each view → Dati moves nothing", lambda: round_trip(pg, VIEWS))

        def side_shut():
            pg.evaluate("BARRA.close()"); frames(pg, 3); ready(pg, settle=200)
            round_trip(pg, ["draw"])
            pg.evaluate("BARRA.show('file', {noFocus: true, instant: true})"); frames(pg, 3)
        step("one sample, side panel shut: Dati → Disegno → Dati moves nothing", side_shut)

        def add_close_panels():
            bad = []
            for sel in ("#np-chrom", "#np-spec", "#np-map"):
                if not pg.is_visible(sel) or pg.is_disabled(sel): continue
                n0, g0 = pg.evaluate("E.panels.length"), pg.evaluate(GEOM)
                pg.click(sel); attendi(pg, f"E.panels.length === {n0 + 1}"); ready(pg, settle=200)
                pg.evaluate("E.panels[E.panels.length - 1].el.querySelector('.x').click()"); attendi(pg, f"E.panels.length === {n0}"); ready(pg, settle=200)
                m = moved(g0, pg.evaluate(GEOM))
                if m: bad.append(f"{sel}: {m}")
            assert not bad, " | ".join(bad)
        step("one sample: adding a chromatogram, spectrum and map panel and closing it with its × gives the work area back as it was", add_close_panels)


        # ---- movement over time, on pages of their own (with motion on, the first opening of Flow, a high-resolution file)
        SAMPLE = """(sels) => { window.__mv = {}; (function f() { for (const [k, s] of Object.entries(sels)) { const e = document.querySelector(s); if (e && e.offsetWidth) (window.__mv[k] = window.__mv[k] || []).push(Math.round(e.getBoundingClientRect().x * 10) / 10); } window.__go !== false && requestAnimationFrame(f); })(); window.__go = true; }"""
        def spread(pg, key):
            xs = pg.evaluate("(k) => window.__mv[k] || []", key)
            assert xs, f"{key}: never seen"
            return max(xs) - min(xs), sorted(set(xs))[:6]
        def fresh(motion=False):
            q = r.page(p, motion=motion); q.set_viewport_size({"width": 1440, "height": 900})
            attendi(q, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(q); return q

        def no_slide_back():
            q = fresh(True); stage(q, [mz("B_FullMass-t15")]); open_first(q); ready(q)
            for v in ("draw", "flow"):
                view(q, v); q.evaluate(SAMPLE, {"pane": "#sb-pane", "work": "#v-workspace"})
                q.click('#nav [data-v="data"]'); attendi(q, "S.view === 'data'"); hold(q, 1200)          # only time shows a slide
                for k in ("pane", "work"):
                    d, xs = spread(q, k); assert d <= MOVE_PX, f"back from {v}: {k} moved {d} px over time {xs}"
        step("one sample, motion on: coming back to Dati the side panel and the work area do not slide", no_slide_back)

        def not_taller():
            q = fresh(); bad = []
            for v in ("data", "draw"):
                view(q, v); q.evaluate("window.scrollTo(0, 0)")
                over = q.evaluate("document.documentElement.scrollHeight - innerHeight")
                if over > 0: bad.append(f"{v}: {over} px taller than the window")
            stage(q, [mz("B_FullMass-t15")]); open_first(q); ready(q)
            over = q.evaluate("document.documentElement.scrollHeight - innerHeight")
            if over > 0: bad.append(f"data with a sample: {over} px taller")
            assert not bad, " | ".join(bad)
        step("Dati and Disegno are never taller than the window (no page scroll bar)", not_taller)

        def keeps_width():
            q = fresh(); D = synth("giro_larg"); load(q, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
            w, dw = q.evaluate("document.querySelector('#dfiles').getBoundingClientRect().width"), q.evaluate("(BANCO.on() ? 340 : 280)")
            assert w >= dw - 1, f"the first file narrowed the side panel to {w} px (landing width {dw})"
        step("the first file keeps the landing width of the side panel", keeps_width)

        def flow_bar_still():
            q = fresh(); view(q, "flow")
            q.evaluate(SAMPLE, {"run": "#flx-run", "cli": "#flx-cli-show", "more": "#flx-more"})
            hold(q, 4000)          # the late modules arrive within a second or two
            for k in ("run", "cli", "more"):
                d, xs = spread(q, k); assert d <= MOVE_PX, f"{k} moved {d} px when the late part arrived {xs}"
        step("Flow, first opening: Run, Terminal and More do not shift when the late part of the bar arrives", flow_bar_still)

        def clean():
            errs = [e for e in r.errs if "wasm-feat/manifest.json" not in str(e)          # the optional modules are not built for the e2e site (tools/prepara_sito.py)
                    and not (e[0] == "console.error" and "404" in str(e[1]))               # its console twin
                    and "allow-scripts and allow-same-origin" not in str(e)]               # Chromium's note on the sandboxed Ketcher iframe (docs/agenti/disegno.md)
            assert not errs, errs[:5]
        step("no console error, no failed request", clean)
finally:
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
