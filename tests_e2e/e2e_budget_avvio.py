"""Regression budget of the page's own load (docs/agenti/avvio-rapido.md): on the built site, a reload served by the service worker and a first
visit must keep few requests and bytes before the first paint, few render-blocking files, no layout shift, and no long main-thread task with
the CPU slowed 4x, both while the shell comes up and until every start-up script of the list L (carica.js) has run. Counts, bytes and task
lengths, never wall-clock times; the margins are stated in BUDGET. Needs the site with the engine (python3 tools/prepara_sito.py): without it
the page shows the engine error banner, which moves the page and is reported as such, not as a layout shift."""
from pathlib import Path
import re, socket, subprocess, sys
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port, hold

# measured on the site at the time of writing (misura_primo_paint.py): 22 critical files and ~605 KB before the first paint, 11 of them
# render-blocking, CLS 0, longest task while the shell comes up ~50 ms at CPU x4. Margins: +30 % on counts and bytes, CLS and the long task at
# the thresholds of the brief (0.02, 100 ms); a long task is counted from the navigation to the drop area visible.
# lt_all_ms: longest task from the navigation until every script of L has run, CPU x4 (devtools trace: 34-61 ms, median 53, over 5 rounds).
# Both long-task budgets take the best of ROUNDS reloads (CI runners spread 51-128 ms for the same code; an injected 120 ms task is in all 5).
BUDGET = {"req_fcp": 29, "kb_fcp": 790, "blocking": 15, "cls": 0.02, "lt_ms": 100, "lt_all_ms": 100}
ROUNDS = 5
SRC = (ROOT / "mzlab" / "web" / "carica.js").read_text(encoding="utf-8")
L = re.findall(r'"([^"]+)"', re.search(r"var L = \[(.*?)\];", SRC, re.S).group(1))
# every script of L has run when carica(n) resolves for each (the promise of its tag's onload); a failed one rejects
ALL_L = "(names) => Promise.all(names.map(n => carica(n))).then(() => ({ ms: Math.round(performance.now()), lt: __b.lt.map(x => Math.round(x[1])) }))"
ENGINE_BANNER = "(() => { const e = document.getElementById('avvio-errore'); return !!(e && e.offsetHeight > 0); })()"

PROBE = r"""(() => { const v = window.__b = { lt: [], cls: 0, fcp: 0, drop: 0 };
  const obs = (t, f) => { try { new PerformanceObserver(l => l.getEntries().forEach(f)).observe({ type: t, buffered: true }); } catch (_) {} };
  obs("longtask", e => v.lt.push([e.startTime, e.duration]));
  obs("paint", e => { if (e.name === "first-contentful-paint") v.fcp = e.startTime; });
  obs("layout-shift", e => { if (!e.hadRecentInput) v.cls += e.value; });
  const poll = () => { const d = document.getElementById("drop"); if (d && d.offsetParent && d.getBoundingClientRect().height > 0) { v.drop = performance.now(); return; } setTimeout(poll, 10); };
  poll();
  try { localStorage.setItem("mzlab.tour.lr", JSON.stringify({ stato: "saltato" })); localStorage.setItem("mzlab.barra.hint", "1"); } catch (_) {}
})()"""
# the critical set: render-blocking files and the head's preloads (fixed by the page, not by how fast the cache answers; non-blocking scripts that
# merely start before the paint on a fast machine are not counted)
SNAP = """(() => { const r = performance.getEntriesByType("resource"), f = __b.fcp,
    pre = r.filter(e => e.startTime <= f && (e.renderBlockingStatus === "blocking" || e.initiatorType === "link"));
  return { fcp: f, drop: __b.drop, cls: __b.cls, lt: __b.lt.filter(x => x[0] <= __b.drop).map(x => Math.round(x[1])),
    req_fcp: pre.length, kb_fcp: Math.round(pre.reduce((s, e) => s + e.decodedBodySize, 0) / 1024),
    blocking: r.filter(e => e.renderBlockingStatus === "blocking").map(e => e.name.replace(location.origin, "")) }; })()"""

errs = []
def check(name, got, cap):
    ok = got <= cap if not isinstance(cap, float) else got < cap
    print("  ", (name, got, "<=" if ok else ">", cap, "ok" if ok else "FAIL"))
    if not ok: errs.append(name)

def wait_shell(pg):
    for _ in range(600):
        try:
            if pg.evaluate("!!(window.__b && __b.drop && __b.fcp)"): return
        except Exception: pass
        hold(pg, 50)
    raise TimeoutError("the drop area never appeared")

class Stop(Exception):
    pass

def engine_up(pg, where):
    if pg.evaluate(ENGINE_BANNER):
        print(f"  engine error banner shown ({where}): the site has no working engine .wasm; build it with python3 tools/prepara_sito.py")
        errs.append(f"engine missing ({where})")
        return False
    return True

PORT = free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: socket.create_connection(("127.0.0.1", PORT), 0.2).close(); break
    except OSError: pass
URL = f"http://127.0.0.1:{PORT}/#/data"
try:
    if not (ROOT / "site" / "sw.js").exists():
        print("SKIP: no site/ (python3 tools/build_site.py)"); sys.exit(0)
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1400, "height": 900})
        ctx.add_init_script(PROBE)
        pg = ctx.new_page()
        pg.goto(URL)
        wait_shell(pg)
        hold(pg, 1500)                                   # the isolation reload of the first visit (browser.js isolateOnce) comes and goes
        wait_shell(pg)
        cold = pg.evaluate(SNAP)
        print("first visit:", {k: v for k, v in cold.items() if k != "blocking"}, len(cold["blocking"]), "blocking")
        check("first visit: critical files before the first paint", cold["req_fcp"], BUDGET["req_fcp"])
        if not engine_up(pg, "first visit"):
            raise Stop
        pg.reload(); wait_shell(pg)
        warm = pg.evaluate(SNAP)
        print("reload:", {k: v for k, v in warm.items() if k != "blocking"})
        check("reload: critical files before the first paint", warm["req_fcp"], BUDGET["req_fcp"])
        check("reload: KB of the critical files", warm["kb_fcp"], BUDGET["kb_fcp"])
        check("reload: render-blocking files", len(warm["blocking"]), BUDGET["blocking"])
        check("reload: layout shift", round(warm["cls"], 4), BUDGET["cls"])
        cdp = ctx.new_cdp_session(pg)
        cdp.send("Emulation.setCPUThrottlingRate", {"rate": 4})
        # best of ROUNDS slowed reloads: a long task of our own code is in every reload, a busy runner only adds to some of them
        to_drop, to_all, shift = [], [], 0
        for _ in range(ROUNDS):
            pg.reload(); wait_shell(pg)
            slow = pg.evaluate(SNAP)
            every = pg.evaluate(ALL_L, L)
            to_drop.append(max(slow["lt"] or [0])); to_all.append(max(every["lt"] or [0])); shift = max(shift, slow["cls"])
            print(f"CPU x4: drop area at {round(slow['drop'])} ms, all {len(L)} start-up scripts run at {every['ms']} ms, long tasks {every['lt']}")
        check(f"CPU x4: longest task until the drop area (best of {ROUNDS})", min(to_drop), BUDGET["lt_ms"])
        check("CPU x4: layout shift", round(shift, 4), BUDGET["cls"])
        check(f"CPU x4: longest task until every start-up script has run (best of {ROUNDS})", min(to_all), BUDGET["lt_all_ms"])
        b.close()
except Stop:
    pass
finally:
    srv.terminate()
print("FAILED:", errs) if errs else print("ALL OK")
sys.exit(1 if errs else 0)
