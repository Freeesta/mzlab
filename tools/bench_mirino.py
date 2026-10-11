"""Cost of the sight (crosshair) of the RT x m/z map: `python3 tools/bench_mirino.py [--scrivi]` (Chromium, development server, synthetic files).
Per pointer move: time of the requests, of the drawing, requests per move and requests still in flight; `--scrivi` prints the markdown table."""
import sys, os, json, statistics, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests_e2e"))
from lib import *          # noqa: E402,F401
import numpy as np         # noqa: E402
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds   # noqa: E402
from mzlab import demo     # noqa: E402

ROWS = "PERF.rows.filter(r=>r.kind==='api').map(r=>({u:r.what,ms:r.ms,py:/python ([0-9.]+)/.test(r.extra)?+/python ([0-9.]+)/.exec(r.extra)[1]:null}))"
HOOK = ("(()=>{window.__f=[];const f=window.fetch;window.fetch=function(u,...a){const r={u:String(u),t0:performance.now()};window.__f.push(r);"
        "return f.call(this,u,...a).then(x=>{r.t1=performance.now();return x},e=>{r.t1=performance.now();throw e})}})()")


def lr_file(path, nscan=1500, npk=500):
    """Full Scan of the 3200 look-alike: nscan scans of npk centroids each (a real file has ~1000-3000 scans)."""
    rng = np.random.default_rng(5); specs = []
    for i, rt in enumerate(np.linspace(0.2, 22, nscan)):
        mz = np.sort(rng.uniform(80, 600, npk)); it = rng.lognormal(np.log(3e3), 0.8, npk)
        specs.append(demo._spectrum(i, 1, float(rt), mz, it))
    head = ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="b"><spectrumList count="%d">' % len(specs))
    Path(path).write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")


def pct(v, q):
    v = sorted(v); return v[min(len(v) - 1, int(len(v) * q))] if v else float("nan")


def run(kind, moves=120, url_extra="?perf", site=False):
    wd = HERE / "shots" / "mirino_tmp"; wd.mkdir(parents=True, exist_ok=True)
    if kind == "lr": f = wd / "big_lr-t0.mzML"; lr_file(f) if not f.exists() else None
    else:                                     # HR: the Full Scans of a synthetic Orbitrap DDA file (the map needs a Full Scan tab)
        import lib_hr
        src = wd / ds.HR_NAMES["exploris"]; ds.hr_dda(src, np.random.default_rng(31), "exploris"); f = Path(lib_hr.hr_variants(src, wd)[0])
    if site:                                  # the built site (Pyodide in a worker, no server): python3 tools/build_site.py first
        import subprocess
        class _S:
            port = 8898; errs = []; b = None
            srv = subprocess.Popen([sys.executable, "-m", "http.server", "8898", "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        r = _S()
    else: r = Run(port=8897, wd="/tmp/wd_mirino")
    out = {}
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(); r.b = b
            pg = b.new_context(viewport={"width": 1440, "height": 950}).new_page()
            pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
            pg.goto(f"http://127.0.0.1:{r.port}/{url_extra}"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_timeout(800)
            pg.set_input_files("#pick", [str(f)]); pg.wait_for_timeout(800)
            pg.click("#opbtn"); pg.wait_for_function("typeof E!==\"undefined\"&&E.files.length", timeout=180000); ready(pg)
            if kind == "hr":                                       # HR: the file is listed, one click opens it
                pg.wait_for_timeout(1500); pg.evaluate("document.querySelector('#flist .fl, .fl')?.click()"); ready(pg); pg.wait_for_timeout(1000)
            if kind == "hr": pg.evaluate("reveal(addPanel('map',{k:E.files[0].k}))")
            else: pg.click("#np-map")
            ready(pg); pg.wait_for_timeout(600)
            M = "E.panels.find(q=>q.type==='map')"
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); pg.wait_for_timeout(1000); ready(pg)
            c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
            pg.evaluate("PERF.rows.length=0; window.__mir.length=0")
            pg.evaluate("window.__lt=[];try{new PerformanceObserver(l=>l.getEntries().forEach(e=>window.__lt.push(e.duration))).observe({entryTypes:['longtask']})}catch(e){}")
            pg.evaluate("window.__fr=[];(function t(a){if(window.__frOn!==false){window.__fr.push(a);requestAnimationFrame(t)}})(performance.now())")
            rng = np.random.default_rng(1); x, y = c[0] + c[2] * .4, c[1] + c[3] * .5
            t0 = time.time()
            for i in range(moves):                    # a drag of ~2 s at 60 Hz: ~8 px per event
                x += 6 + rng.normal(0, 2); y += rng.normal(0, 4)
                pg.mouse.move(min(max(x, c[0] + 60), c[0] + c[2] - 120), min(max(y, c[1] + 40), c[1] + c[3] - 60))
                pg.wait_for_timeout(16)
            drag_s = time.time() - t0
            fr = pg.evaluate("window.__fr.slice(-Math.floor(%d/16))" % (drag_s * 1000)); gaps = [b - a for a, b in zip(fr, fr[1:])]
            upd = pg.evaluate("window.__mirPrev||0"); lt = pg.evaluate("window.__lt"); mf = pg.evaluate("window.__mirFrame||[0]")
            pg.wait_for_function("(window.__qqF||0)===0", timeout=60000); pg.wait_for_timeout(1500)
            fl = pg.evaluate(ROWS); mir = pg.evaluate("window.__mir")
            # a slow sweep along the RT axis: 60 events 100 ms apart (each a few scans further): requests the engine gets and plots updated
            pg.evaluate("PERF.rows.length=0; window.__mir.length=0")
            xs0, ys0 = c[0] + c[2] * .2, c[1] + c[3] * .45
            for i in range(60):
                pg.mouse.move(xs0 + i * 9, ys0 + (i % 5) * 3); pg.wait_for_timeout(100)
            pg.wait_for_function("(window.__qqF||0)===0", timeout=60000); pg.wait_for_timeout(800)
            sweep_raw = (pg.evaluate(ROWS), pg.evaluate("window.__mir"))
            # 30 stops: the sight moves ~40 px and rests until both plots show the data of the new position
            stops = []
            for i in range(30):
                x2 = c[0] + c[2] * (.3 + .4 * ((i * 7) % 11) / 11); y2 = c[1] + c[3] * (.25 + .5 * ((i * 5) % 9) / 9)
                pg.evaluate("window.__mir.length=0")
                pg.mouse.move(x2, y2, steps=3)
                try: pg.wait_for_function("window.__mir.length>0", timeout=5000); stops.append(pg.evaluate("window.__mir[window.__mir.length-1].total"))
                except Exception: stops.append(5000)
                pg.wait_for_timeout(120)
            # the same path in a key sequence: 40 arrows
            pg.evaluate("PERF.rows.length=0; window.__mir.length=0")
            for i in range(40):
                pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(16)
            pg.wait_for_function("(window.__qqF||0)===0", timeout=60000); pg.wait_for_timeout(1500)
            fk = pg.evaluate(ROWS); mk = pg.evaluate("window.__mir")
            st = pg.evaluate(f"(()=>{{const s={M}._mz;return {{spec:s.spec&&s.spec.key,xic:s.xic&&s.xic.key}}}})()")
            pg.evaluate("window.__long=0")
            b.close()
    finally:
        r.srv.terminate()
    def summ(fl, mir, n):
        api = fl; by = {}; py = {}
        for q in api:
            nm = q["u"].split("?")[0]; by.setdefault(nm, []).append(q["ms"])
            if q["py"] is not None: py.setdefault(nm, []).append(q["py"])
        d = {"events": n, "requests": len(api), "per_event": round(len(api) / n, 2), "draws": len(mir)}
        for k in ("fetch", "draw", "total"): d[k] = (round(statistics.median([m[k] for m in mir]), 1), round(pct([m[k] for m in mir], .95), 1)) if mir else None
        d["by"] = {k: (len(v), round(statistics.median(v), 1), round(pct(v, .95), 1)) for k, v in by.items()}
        d["python_ms"] = {k: (round(statistics.median(v), 1), round(pct(v, .95), 1)) for k, v in py.items()}
        return d
    sweep = summ(*sweep_raw, 60)
    return {"kind": kind, "pointer": summ(fl, mir, moves), "keys": summ(fk, mk, 40), "final": st, "drag_s": round(drag_s, 2),
            "frame_ms": (round(pct(gaps, .5), 1), round(pct(gaps, .95), 1), round(max(gaps), 1)), "previews_drawn": upd, "mine_frame_ms": (round(pct(mf, .5), 2), round(pct(mf, .95), 2), round(max(mf), 2)), "long_tasks": len(lt), "long_task_max_ms": round(max(lt), 1) if lt else 0,
            "sweep": sweep, "stop_latency_ms": (round(statistics.median(stops), 1), round(pct(stops, .95), 1))}


if __name__ == "__main__":
    kinds = [a for a in sys.argv[1:] if a in ("lr", "hr")] or ["lr", "hr"]
    for k in kinds: print(json.dumps(run(k, site="--site" in sys.argv)))
