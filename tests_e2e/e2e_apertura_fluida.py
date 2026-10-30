"""Opening is smooth: from the opening of the first file (by itself at the end of the loading, soloFile) to the settled work area nothing jumps. Records the layout shifts (PerformanceObserver) and the boxes of the main regions at every
frame of the opening (synthetic mzML only), then asserts a cumulative layout shift under the threshold, the same boxes from the landing to the settled area, and the fade-in."""
import json, os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

CLS_MAX = 0.01
REGIONS = ["#v-data", "#dfiles", "#tools", "#dpanels", "#sb-pane", "#dpanels .pnl", "#dpanels .pnl canvas", "#sb-panels"]
PROBE = """
() => {
  const R = %s, S = window.__shift = {cls: 0, all: [], frames: [], t0: null, anims: []};
  document.addEventListener('animationstart', e => { if (e.animationName === 'tr-fade') S.anims.push({el: e.target.id, dur: getComputedStyle(e.target).animationDuration, t: performance.now() - S.t0}); }, true);
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) { if (S.t0 == null || e.startTime < S.t0) continue; S.cls += e.value; S.all.push({t: e.startTime - S.t0, v: e.value, input: e.hadRecentInput,
    src: (e.sources || []).map(s => { const n = s.node; return n && n.nodeType === 1 ? (n.id ? '#' + n.id : n.tagName.toLowerCase() + (n.className && n.className.baseVal === undefined ? '.' + String(n.className).split(' ')[0] : '')) : '?'; }) }); } })
    .observe({type: 'layout-shift', buffered: false}); } catch (_) {}
  // page coordinates: a scroll (scrollToBlock of the first file) is not a move of the layout
  const box = s => { const e = document.querySelector(s); if (!e || e.hidden) return null; const r = e.getBoundingClientRect(); return r.width * r.height ? [r.x + scrollX, r.y + scrollY, r.width, r.height].map(v => Math.round(v * 10) / 10) : null; };
  const tick = () => { if (S.t0 != null) { const f = {t: performance.now() - S.t0, op: getComputedStyle(document.querySelector('#v-data')).opacity}; for (const s of R) f[s] = box(s); S.frames.push(f); }
    if (S.t0 == null || !S.stop) requestAnimationFrame(tick); };
  requestAnimationFrame(tick);
  // the clock starts when the first file opens (by itself, OPENNOW.first)
  const so = window.soloFile; window.soloFile = function (...a) { if (S.t0 == null) S.t0 = performance.now(); return so.apply(this, a); };
}
""" % json.dumps(REGIONS)


def synth(d):
    d = Path(d); d.mkdir(parents=True, exist_ok=True); rng = np.random.default_rng(3)
    for t in (0, 15, 30): ds.full_scan(d / f"S_FullScan-t{t}.mzML", t, rng)
    return [d / f"S_FullScan-t{t}.mzML" for t in (0, 15, 30)]


def measure(pg, files):
    pg.wait_for_selector("#start:not([hidden])", timeout=60000)
    pg.evaluate(PROBE)
    stage(pg, files)
    open_first(pg)
    ready(pg)
    fermo(pg, "#dpanels .pnl"); frames(pg, 30)
    return pg.evaluate("(() => { window.__shift.stop = true; return window.__shift; })()")


def moves(frames, key, tol=0.6):
    """Distinct boxes of a region over the frames where it is visible: [(t, box)] at every change."""
    out = []
    for f in frames:
        b = f.get(key)
        if b and (not out or max(abs(x - y) for x, y in zip(b, out[-1][1])) > tol): out.append((round(f["t"]), b))
    return out


def settled(frames, key, since, tol=0.6):
    """The box of `key` from the first frame where it is visible (and not before `since` ms) to the end: [x, y, w, h] extremes must stay within `tol`."""
    boxes = [f[key] for f in frames if f.get(key) and f["t"] >= since]
    assert boxes, key + " never visible"
    spread = [max(b[i] for b in boxes) - min(b[i] for b in boxes) for i in range(4)]
    return spread


def second_page(r, p, motion):
    pg = r.b.new_page(viewport={"width": 1500, "height": 2200}, reduced_motion=motion, locale="it-IT")
    pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/")
    return pg


if __name__ == "__main__":
    steps = []
    def step(name, fn):
        try: fn(); steps.append((name, "ok"))
        except Exception as e:
            import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_apertura_fluida.py")][-1]
            steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
    r = Run(port=8830 + 170, wd="/tmp/wd_apertura_fluida"); res = {}
    try:
        files = synth(tempfile.mkdtemp())
        with sync_playwright() as p:
            pg = r.page(p)
            res = measure(pg, files)
            if os.environ.get("MZLAB_E2E_JSON"): Path(os.environ["MZLAB_E2E_JSON"]).write_text(json.dumps(res))
            fr = res["frames"]
            print("CLS %.4f" % res["cls"], [(round(a["t"]), round(a["v"], 4), a["src"]) for a in res["all"]])
            def cls_small():
                assert res["cls"] < CLS_MAX, f"cumulative layout shift {res['cls']:.4f} >= {CLS_MAX}: {res['all']}"
                # the one move allowed: the side panel takes the width of the names when the first file is shown (barra.js effW), the work area follows it
                shown, area = next(f["t"] for f in fr if f.get("#dpanels .pnl")), fr[0]["t"]
                late = [a for a in res["all"] if a["t"] > 60 and a["v"] > 0.0005 and not (a["src"] == ["#v-workspace"] and area <= a["t"] <= shown + 50)]
                assert not late, f"layout shifts after the work area was drawn: {late}"
            step("cumulative layout shift of the opening is below the threshold", cls_small)
            def same_boxes():
                area = fr[0]["t"]                     # the first frame of the opening
                shown = next(f["t"] for f in fr if f.get("#dpanels .pnl"))               # the first file shown: the side panel takes the width of the names, under the fade
                sp = settled(fr, "#v-data", area); assert max(sp) <= 0.6, f"#v-data moved or changed size after the first paint: {sp}"
                for k in ("#dfiles", "#sb-pane", "#tools", "#sb-panels"):
                    sp = settled(fr, k, shown); assert max(sp) <= 0.6, f"{k} moved or changed size after the first file was shown: {sp}"
                sp = settled(fr, "#dpanels", shown); assert max(sp[:3]) <= 0.6, f"#dpanels moved or changed width: {sp}"
                sp = settled(fr, "#dpanels .pnl", 0); assert max(sp) <= 0.6, f"the first panel moved or was resized after it appeared: {sp} {moves(fr, '#dpanels .pnl')} {moves(fr, '#dfiles')}"
                sp = settled(fr, "#dpanels .pnl canvas", 0); assert max(sp) <= 0.6, f"the first plot canvas was resized after it appeared: {sp}"
            step("work area regions keep their boxes from the first paint to the settled state", same_boxes)
            def reduced_no_fade():
                assert all(a["dur"] in ("0s", "0ms") for a in res["anims"]), res["anims"]
            step("reduced motion: no fade", reduced_no_fade)
            pg.close()
            pg2 = second_page(r, p, "no-preference")
            res2 = measure(pg2, files)
            def fade():
                a = [x for x in res2["anims"] if x["el"] in ("dpanels", "tools", "sb-panels")]
                assert {x["el"] for x in a} == {"dpanels", "tools", "sb-panels"}, res2["anims"]
                assert all(0.12 <= float(x["dur"][:-1]) <= 0.18 for x in a), a
                assert res2["cls"] < CLS_MAX, res2["cls"]
            step("the content fades in (120-180 ms, opacity only) when it replaces the landing", fade)
            def opacity_only():
                kf = pg2.evaluate("""() => { const a = document.getAnimations(); return a.length; }""")
                css = (ROOT / "mzlab" / "web" / "transizioni.css").read_text(encoding="utf-8")
                assert "@keyframes tr-fade{from{opacity:0}}" in css
            step("the fade animates opacity only", opacity_only)
            # first visit: from the navigation (landing painted with the side panel open) through the files and the first click to the settled first panel
            pg3 = r.b.new_page(viewport={"width": 1500, "height": 1000}, reduced_motion="reduce", locale="it-IT")
            pg3.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
                                "window.__nav={cls:0,all:[]};try{new PerformanceObserver(l=>{for(const e of l.getEntries())if(!e.hadRecentInput){window.__nav.cls+=e.value;window.__nav.all.push([Math.round(e.startTime),+e.value.toFixed(4),(e.sources||[]).map(x=>{const n=x.node;return n&&n.nodeType===1?(n.id?'#'+n.id:n.tagName.toLowerCase()+'.'+String(n.className).split(' ')[0]):'?'}).join(' ')])}}).observe({type:'layout-shift',buffered:true})}catch(_){}")
            pg3.goto(f"http://127.0.0.1:{r.port}/")
            def landing_first_paint():
                pg3.wait_for_selector("#start:not([hidden])", timeout=60000); attendi(pg3, "window.qqReady === true", 60000)
                st = pg3.evaluate("""() => ({open: document.querySelector('#v-data').classList.contains('sb-open'), hint: !document.querySelector('#fl-hint').hidden, dz: !document.querySelector('#fl-dz').hidden,
                  grey: getComputedStyle(document.querySelector('#np-chrom')).cursor === 'not-allowed',
                  big: document.querySelector('#drop').getBoundingClientRect().height >= innerHeight * 0.4,
                  off: ['#np-chrom', '#np-closeall', '#np-method', '#fsel'].every(s => document.querySelector(s).disabled && document.querySelector(s).getAttribute('aria-disabled') === 'true'),
                  calc: !!document.querySelector('#stbar [data-cmd=calc]:not([disabled])'), dpanels: getComputedStyle(document.querySelector('#dpanels')).display})""")
                assert st == {"open": True, "hint": True, "dz": False, "grey": True, "big": True, "off": True, "calc": True, "dpanels": "none"}, st
            step("first paint: side panel open with its drop hint, data buttons disabled, calculators usable, landing in the work area", landing_first_paint)
            def drop_by_keyboard():
                pg3.evaluate("document.activeElement && document.activeElement.blur()")
                for _ in range(80):
                    pg3.keyboard.press("Tab")
                    if pg3.evaluate("document.activeElement && document.activeElement.id") == "drop": break
                assert pg3.evaluate("document.activeElement.id") == "drop", pg3.evaluate("document.activeElement.outerHTML.slice(0, 120)")
                assert pg3.evaluate("document.activeElement.getAttribute('role') === 'button' && document.activeElement.getAttribute('aria-label').length > 10")
                assert pg3.evaluate("[...document.querySelectorAll('#start button, #start [tabindex], #start a, #start input:not([hidden]), #start summary')].filter(e => e.offsetParent)[0].id") == "drop"
                pg3.evaluate("document.querySelector('#pick').addEventListener('click', () => { window.__picked = (window.__picked || 0) + 1 })")
                pg3.keyboard.press("Enter"); pg3.keyboard.press("Space")
                attendi(pg3, "window.__picked === 2", 5000)
            step("#drop is reached with Tab, first in the landing, and Enter opens the picker", drop_by_keyboard)
            def cls_from_navigation():
                measure(pg3, files)
                nav = pg3.evaluate("window.__nav")
                assert nav["cls"] < CLS_MAX, f"layout shift from the navigation {nav['cls']:.4f} >= {CLS_MAX}: {nav['all']}"
            step("first visit: layout shift from the navigation to the settled first panel is below the threshold", cls_from_navigation)
    finally:
        r.close()
    bad = [s_ for s_ in steps if s_[1] != "ok"]
    for n, s_ in steps: print(("OK   " if s_ == "ok" else "FAIL ") + n + ("" if s_ == "ok" else "  -> " + s_))
    errs = [e for e in r.errs if e[0] in ("pageerror",)]
    print("page errors:", errs)
    sys.exit(1 if bad or errs else 0)
