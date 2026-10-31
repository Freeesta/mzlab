"""Frame-accurate probe + guard: «Hide panels» / «Show panels» (#sb-toggle) must not move or resize the buttons inside the plot panels
after the first frame of the toggle (no «placed, then widened» second layout pass), and must not shift the layout.
Per requestAnimationFrame it records the LAYOUT box (offset*, so the FLIP transforms of transizioni.js do not count) of every button/select/input
in the panel headers and of the panels themselves, 20 toggles in a row, at three window widths, CPU x1 and x4.
Set MZLAB_PROBE_REPORT=1 to print the per-element numbers."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

TOGGLES = 20
WIDTHS = (1440, 1180, 1000)
TOL = 1                      # px
MAX_LS = 0.02                # layout-shift score over the 20 toggles

PROBE = """() => {
  const rel = (el, top) => { let x = 0, y = 0, n = el; while (n && n !== top) { x += n.offsetLeft; y += n.offsetTop; n = n.offsetParent; } return [x, y, el.offsetWidth, el.offsetHeight]; };
  const P = window.__probe = { els: new Map(), frames: [], ls: 0, long: 0, on: false };
  const track = () => { document.querySelectorAll('#dpanels>.pnl').forEach(p => {
    if (!P.els.has(p)) P.els.set(p, [p.dataset.pid || p.id || 'pnl' + P.els.size, null]);
    p.querySelectorAll('.hd button, .hd select, .hd input').forEach((b, i) => { if (!P.els.has(b)) P.els.set(b, [(p.id || 'pnl') + ':' + (b.dataset.a || b.dataset.o || b.className || b.tagName) + '#' + i, p]); }); }); };
  P.track = track;
  P.snap = () => { const f = []; P.els.forEach(([name, top], el) => { f.push([name, el.isConnected && el.offsetParent ? rel(el, top || document.querySelector('#dpanels')) : null]); }); return f; };
  new PerformanceObserver(l => l.getEntries().forEach(e => { if (!e.hadRecentInput && P.on) P.ls += e.value; })).observe({ type: 'layout-shift', buffered: false });
  try { new PerformanceObserver(l => l.getEntries().forEach(e => { if (P.on) P.long++; })).observe({ type: 'longtask', buffered: false }); } catch (_) {}
  P.run = (ms) => new Promise(res => { P.frames = []; P.on = true; const t0 = performance.now();
    const tick = () => { P.frames.push(P.snap()); if (performance.now() - t0 < ms) requestAnimationFrame(tick); else { P.on = false; res(P.frames); } };
    document.querySelector('#sb-toggle').click(); requestAnimationFrame(tick); });
}"""


def analyse(frames):
    """Per element: frames (from the first one) whose box differs from the last frame's box by more than TOL; elements absent in a frame count as different."""
    last = {n: b for n, b in frames[-1]}
    res = {}
    for fr in frames:
        for n, b in fr:
            f = last.get(n)
            if f is None and b is None: continue
            bad = (f is None) != (b is None) or (b is not None and any(abs(u - v) > TOL for u, v in zip(b, f)))
            if bad: res[n] = res.get(n, 0) + 1
    return res


def cold(p, vw):
    r = Run(port=8874, wd="/tmp/wd_pannelli_stabili"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": vw, "height": 900}, locale="it-IT", reduced_motion="no-preference")
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.barra.lr','{\"open\":true}');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
    pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); pg.wait_for_timeout(800)
    pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(500)
    pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1})"); pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.evaluate("addPanel('map',{tab:'full'})")
    ready(pg); pg.wait_for_timeout(1500)
    return r, ctx, pg, errs


def measure(p, vw, cpu):
    r, ctx, pg, errs = cold(p, vw)
    try:
        cdp = ctx.new_cdp_session(pg); cdp.send("Emulation.setCPUThrottlingRate", {"rate": cpu})
        pg.evaluate(PROBE)
        glitch, worst, frames_n = {}, 0, 0
        for i in range(TOGGLES):
            pg.evaluate("window.__probe.track()")
            fr = pg.evaluate("window.__probe.run(700)")
            a = analyse(fr); frames_n += len(fr)
            for n, c in a.items(): glitch[n] = glitch.get(n, 0) + c
            worst = max(worst, max(a.values(), default=0))
            pg.wait_for_timeout(150)
        ls = pg.evaluate("window.__probe.ls"); long = pg.evaluate("window.__probe.long")
        npan = pg.evaluate("E.panels.length")
        assert not errs, errs
        return dict(vw=vw, cpu=cpu, panels=npan, frames=frames_n, glitch=glitch, total=sum(glitch.values()), worst=worst, ls=ls, long=long)
    finally: ctx.close(); r.close()


steps = []
try:
    with sync_playwright() as p:
        for cpu in (1, 4):
            for vw in WIDTHS:
                m = measure(p, vw, cpu)
                print(f"vw={m['vw']} cpu=x{m['cpu']} panels={m['panels']} frames={m['frames']} glitch-frames={m['total']} elements={len(m['glitch'])} worst-per-toggle={m['worst']} layout-shift={m['ls']:.4f} longtasks={m['long']}")
                if get_env("PROBE_REPORT"):
                    for n, c in sorted(m["glitch"].items(), key=lambda x: -x[1])[:25]: print("   ", c, n)
                ok = m["total"] == 0 and m["ls"] <= MAX_LS
                steps.append((f"{vw}px cpu x{cpu}", "ok" if ok else f"FAIL: glitch frames {m['total']} ({len(m['glitch'])} elements), layout-shift {m['ls']:.4f}"))
finally:
    for n, s in steps: print(s, "-", n)
    if any(s != "ok" for _, s in steps): sys.exit(1)
    print("ALL OK")
