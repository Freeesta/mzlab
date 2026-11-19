"""Reads made by the opening of one mzML on the built site (Rust/WASM engine, OPFS, Chromium headless): passes over the file, bytes, reads and ms
inside the readers, per request of the open pipeline (`lr_put`, `lr_index`, `lr_entry`, `lr_items`, `canali`, ...), from the `rdStat` hook of rust-worker.js.

    python3 tools/bench_open.py FILE.mzML [--site DIR] [--wd DIR] [--json]

Without `--site` the site is built in WD/site (as tools/bench_map.py). A synthetic DAD file: `tools/dati_sintetici_uv.py OUT --uv 30000 --wl 401 --scans 1500`
(174 MB). Compare two sites (before / after a change) by running it once on each. Nothing of the product is changed: the hooks wrap Worker."""
import argparse, json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import bench_map as bm  # noqa: E402


def run(pg, port, f):
    pg.goto(f"http://127.0.0.1:{port}/?riparti=1"); pg.wait_for_selector("#drop", timeout=120000)
    bm.wait(pg, "window.MZLAB_LR !== undefined", timeout=120000)
    t0 = time.time(); pg.set_input_files("#pick", [str(f)])
    bm.wait(pg, "typeof E!=='undefined' && E.files.length >= 1 && E.pending.length === 0", timeout=1800000)
    pg.evaluate("() => { if (!E.files.some(f => f.vis && !f.gone)) soloFile(E.files.find(f => !f.gone)); }")
    bm.wait(pg, "E.files.some(f => f.vis && !f.gone)", timeout=600000)
    pg.wait_for_timeout(1500)                                                 # the last answers of the worker carry the final `rdStat`
    wall = round(time.time() - t0, 2)
    rd = pg.evaluate("__B.rd || null") or {}
    size = f.stat().st_size
    ops = [{"op": o[0], "name": o[1], "total_ms": o[2], "read_ms": o[3], "reads": o[4]} for o in rd.get("ops", [])]
    return {"file_MB": round(size / 1e6, 1), "open_wall_s": wall, "reads": rd.get("n"), "bytes": rd.get("bytes"), "passes": round(rd.get("bytes", 0) / size, 2),
            "read_ms": round(rd.get("ms", 0)), "ops": ops}


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("file", type=Path); ap.add_argument("--site", type=Path); ap.add_argument("--wd", type=Path)
    ap.add_argument("--json", action="store_true"); ap.add_argument("--port", type=int, default=8872)
    a = ap.parse_args()
    wd = a.wd or Path(tempfile.gettempdir()) / "bench_open"; wd.mkdir(parents=True, exist_ok=True)
    site = a.site or wd / "site"
    if not (site / "static" / "wasm" / "mzlab_wasm.js").exists():
        if not (shutil.which("cargo") and shutil.which("wasm-bindgen")): sys.exit("the site needs cargo and wasm-bindgen (or --site DIR of a built site)")
        bm.build_site(site)
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, str(ROOT / "tests_e2e" / "servi.py"), str(a.port), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(1)
        with sync_playwright() as p:
            prof = wd / "profile"; shutil.rmtree(prof, ignore_errors=True)
            ctx = p.chromium.launch_persistent_context(str(prof), executable_path=os.environ.get("MZLAB_CHROMIUM") or None, viewport={"width": 1440, "height": 950})
            ctx.set_default_timeout(600000)
            ctx.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}" + bm.INIT)
            pg = ctx.pages[0] if ctx.pages else ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
            try:
                o = run(pg, a.port, a.file); o["page_errors"] = errs[:3]
            finally: ctx.close(); shutil.rmtree(prof, ignore_errors=True)
        if a.json: print(json.dumps(o))
        else:
            print(f"{o['file_MB']} MB: {o['passes']} passes, {o['bytes']} bytes, {o['reads']} reads, {o['read_ms']} ms in the readers, open {o['open_wall_s']} s")
            for r in o["ops"]: print(f"  {r['op']:<14} total {r['total_ms']:>6} ms  readers {r['read_ms']:>5} ms  reads {r['reads']:>4}  {r['name']}")
    finally: srv.terminate()


if __name__ == "__main__":
    main()
