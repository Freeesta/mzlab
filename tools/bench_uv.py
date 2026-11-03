"""Speed of the UV spectrum of the DAD on the built site: a double click on the PDA chromatogram -> the UV spectrum panel drawn, and the
single-wavelength (λ) trace, on synthetic mzML files with the optical matrix as `MS:1000804` spectra (the layout of a variant `.wiff` through
the `sciex` module, `tools/dati_sintetici_uv.py`).

    python3 tools/bench_uv.py [--site DIR] [--files 3] [--uv 2400] [--wl 300] [--scans 1500] [--reps 5] [--json]

Gestures, the time from the double click (captured in the page) to the end of the draw of the UV panel: D1 the first double click (cold: the matrix
is read), D2.. the next ones at other times (warm), L1 the first λ trace and L2 the next. Phases: the `api/` requests (wall time, by route), the rest
= page work and drawing. Nothing of the product is changed: the hooks wrap `fetch` and `draw`."""
import argparse, json, shutil, subprocess, sys, tempfile, time, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(ROOT / "tests_e2e"))
import dati_sintetici_uv as du  # noqa: E402

TMO = 600000                                                   # ms: a gesture that does not end by then is reported as stuck, with the state of the page
HOOK = r"""(() => { const V = window.__U = { ev: 0, fe: [], dr: [] };
  window.addEventListener('dblclick', () => { V.ev = performance.now(); V.fe = []; V.dr = []; }, true);
  const wrap = () => { if (!window.draw || window.draw.__u) return;
    const F = window.fetch; window.fetch = function (u, ...a) { const r = { u: String(u && u.url || u), t0: performance.now() }; V.fe.push(r); return F.call(this, u, ...a).then(x => { r.t1 = performance.now(); return x; }); };
    const d0 = window.draw; window.draw = async function (p) { try { return await d0(p); } finally { V.dr.push({ id: p && p.id, uv: !!(p && p.uv), lam: !!(p && p.kind === 'pda' && p.mz0 != null), t1: performance.now(), ok: !!(p && p._a) }); } }; window.draw.__u = 1; };
  V.wrap = wrap; })()"""


def serve(site, port):
    p = subprocess.Popen([sys.executable, str(ROOT / "tests_e2e" / "servi.py"), str(port), str(site)], cwd=str(ROOT), stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
    for _ in range(150):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).read(); return p
        except Exception: time.sleep(0.1)
    return p


def phases(v, t_end):
    """{route: ms} of the requests after the event and the total."""
    out = {}
    for r in v["fe"]:
        if "api/" not in r["u"] or r.get("t1") is None: continue
        k = r["u"].split("api/")[1].split("?")[0] + ("?kind=pda" if "kind=pda" in r["u"] else "")
        out[k] = round(out.get(k, 0) + r["t1"] - r["t0"])
    out["total"] = round(t_end - v["ev"])
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--site", default=str(ROOT / "site")); ap.add_argument("--files", type=int, default=3); ap.add_argument("--uv", type=int, default=2400)
    ap.add_argument("--wl", type=int, default=300); ap.add_argument("--scans", type=int, default=1500); ap.add_argument("--reps", type=int, default=5)
    ap.add_argument("--json", action="store_true"); ap.add_argument("--port", type=int, default=8893)
    a = ap.parse_args()
    from playwright.sync_api import sync_playwright
    from lib import stage, open_first, ready, attendi, pick_kind
    wd = Path(tempfile.mkdtemp(prefix="bench_uv_"))
    files = [wd / f"DAD_s{i + 1}.mzML" for i in range(a.files)]
    for i, f in enumerate(files): du.write(f, a.uv, a.wl, a.scans, 500, i + 1)
    srv = serve(a.site, a.port); res = {"files": a.files, "mb": round(sum(f.stat().st_size for f in files) / 1e6, 1), "uv": a.uv, "wl": a.wl, "gestures": {}}
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(); pg = b.new_page(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce")
            pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            pg.add_init_script(HOOK)
            pg.on("crash", lambda *_: print("PAGE CRASHED", flush=True)); pg.on("pageerror", lambda e: print("pageerror", str(e)[:200], flush=True))
            pg.goto(f"http://127.0.0.1:{a.port}/"); pg.wait_for_timeout(800)
            stage(pg, files, timeout=600000); open_first(pg, timeout=600000)
            pg.evaluate("E.files.forEach(f=>f.vis=true); syncBrowse(); redrawAll()"); ready(pg, timeout=600000)
            pick_kind(pg, "pda"); ready(pg, timeout=600000)
            C = "E.panels.find(q=>q.type==='chrom'&&q.kind==='pda')"
            attendi(pg, f"!!({C} && {C}._a)", 600000); pg.evaluate("__U.wrap()")

            def dbl(rt, lam=False):
                box = pg.evaluate(f"(()=>{{const p={C};p.cv.scrollIntoView({{block:'center'}});const b=p.cv.getBoundingClientRect(),a=p._a;return {{x:b.x+a.X({rt}),y:b.y+M.t+(a.H-M.t-M.b)/2}}}})()")
                n0 = pg.evaluate("E.panels.filter(q=>q.uv).length")
                pg.mouse.dblclick(box["x"], box["y"])
                attendi(pg, f"E.panels.filter(q=>q.uv&&q._a).length>{n0} && __U.dr.some(d=>d.ok&&d.id===E.panels.filter(q=>q.uv).pop().id)", TMO)
                v = pg.evaluate("__U"); nid = pg.evaluate("E.panels.filter(q=>q.uv).pop().id"); t = min(d["t1"] for d in v["dr"] if d["id"] == nid and d["ok"])
                ntr = pg.evaluate("(()=>{const u=E.panels.filter(q=>q.uv); const s=u[u.length-1]; return s._a.uvd.ser ? s._a.uvd.ser.length : 1})()")
                return {**phases(v, t), "traces": ntr}
            def run(name, fn, *a):
                try: res["gestures"][name] = fn(*a)
                except Exception as e:
                    st = pg.evaluate("({panels: E.panels.map(q=>[q.id,q.type,q.kind,!!q.uv,!!q._a]), dr: __U.dr.slice(-5), fe: __U.fe.map(r=>[r.u.slice(0,60), r.t1 ? Math.round(r.t1-r.t0) : null])})")
                    res["gestures"][name] = {"total": -1, "stuck": str(e)[:80], "state": json.dumps(st)[:600]}
                print(name, res["gestures"][name], flush=True)
            run("D1 first double click", dbl, 3.1)
            for i, rt in enumerate([6.4, 8.5, 12.2, 15.7, 4.0, 10.0][: a.reps]):
                run(f"D{i + 2} next double click", dbl, rt)

            def lam(nm):
                pg.evaluate("__U.ev=performance.now(); __U.fe=[]; __U.dr=[]")
                ids = pg.evaluate("E.panels.map(q=>q.id)")
                pg.evaluate(f"wlOpen({C}, {nm}, {nm})")
                attendi(pg, f"__U.dr.some(d=>d.lam&&d.ok&&!{json.dumps(ids)}.includes(d.id))", TMO)
                v = pg.evaluate("__U"); return phases(v, min(d["t1"] for d in v["dr"] if d["lam"] and d["ok"] and d["id"] not in ids))
            run("L1 first λ trace", lam, 254)
            run("L2 next λ trace", lam, 280)
            b.close()
    finally:
        srv.terminate(); shutil.rmtree(wd, ignore_errors=True)
    if a.json: print(json.dumps(res)); return
    print(f"{a.files} files, {res['mb']} MB, {a.uv} UV spectra x {a.wl} wavelengths each")
    for g, ph in res["gestures"].items(): print(f"  {g:28s} {ph['total']:>8} ms   " + "  ".join(f"{k} {v}" for k, v in ph.items() if k not in ("total", "state")))


if __name__ == "__main__":
    main()
