"""C40 phase 1, measurement only: the peak memory of the browser while it loads a huge synthetic DDA mzML (File from disk -> OPFS -> scan table -> .mzidx).
For each size in MZLAB_ENORMI_GB (default "1", comma separated; `m` = the indexed-mzML of 1.2 million small scans) the file is written by tools/mzml_enorme.py, given
to the start screen and loaded while the renderer PSS and the WebAssembly memory of the page and of every worker are sampled; the table prints the peak of each,
the OPFS usage, whether the 192 MB budget held (error.memoryBudget), and what broke. No threshold is asserted and no time is printed.
Run: MZLAB_ENORMI_GB=1,2 python3 tests_e2e/e2e_file_enormi.py   (the site must be built, as for e2e_memoria_misure.py; the disk needs ~3 x the size)
"""
import json, os, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from e2e_memoria_misure import INIT, pss, procs, browser_pid


def make(tmp, spec):
    out = tmp / "enorme.mzML"
    args = ["--scans", "1200000", "--peaks1", "40", "--peaks2", "25"]  if spec == "m" else ["--gb", spec]
    subprocess.run([sys.executable, str(ROOT / "tools" / "mzml_enorme.py"), str(out), *args], check=True)
    return out


def sample(pg, cdp_pid):
    """Renderer PSS (MB) and the WebAssembly memory (MB) of the page and of every worker, right now."""
    rend = sum(pss(p) for p, k in procs(cdp_pid) if k == "renderer")
    wasm = []
    for tgt in [pg] + list(pg.workers):
        try:
            r = (tgt.evaluate("() => self.__mz ? self.__mz.report() : null") if tgt is not pg else pg.evaluate("() => window.__mz ? window.__mz.report() : null"))
            wasm += [(getattr(tgt, "url", "page")[-40:], round(m, 1)) for m in (r or {}).get("wasm_mb", [])]
        except Exception:
            pass
    return rend, wasm


def one(r, p, spec, tmp):
    f = make(tmp, spec)
    size_gb = f.stat().st_size / 1e9
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": 1400, "height": 900}, reduced_motion="reduce", locale="it-IT")
    ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
    ctx.add_init_script(INIT)
    pg = ctx.new_page(); errs = []
    pg.on("pageerror", lambda e: errs.append(str(e)[:200]))
    pg.goto(f"http://127.0.0.1:{r.port}/?perf=1"); attendi(pg, "window.qqReady === true"); ready(pg)
    root = browser_pid()
    peak_r, peak_w, how = 0.0, [], "loaded"
    pg.set_input_files("#pick", [str(f)])
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=600000)
    pg.click("#opbtn")
    for _ in range(4000):
        rend, wasm = sample(pg, root)
        peak_r = max(peak_r, rend)
        if sum(m for _, m in wasm) > sum(m for _, m in peak_w): peak_w = wasm
        if pg.evaluate("!document.querySelector('#v-data').hidden && window.PERF && PERF.rows.some(x => x.kind === 'disegno' && x.what === 'draw chrom')"): break
        bad = pg.evaluate("(document.querySelector('.toast-err, #fe-err, [role=alert]') || {}).innerText || ''")
        if bad.strip(): how = "error: " + bad.strip().replace("\n", " ")[:160]; break
        pg.wait_for_timeout(1500)
    else:
        how = "not finished"
    st = pg.evaluate("navigator.storage.estimate().then(s => s.usage / 1048576)")
    miss = pg.evaluate("(window.MZLAB_LR_MISS || []).length")
    row = {"file_gb": round(size_gb, 2), "renderer_pss_peak_mb": round(peak_r, 1), "wasm_peak": peak_w, "wasm_peak_sum_mb": round(sum(m for _, m in peak_w), 1),
           "opfs_mb": round(st, 0), "result": how, "page_errors": errs[:3], "lr_miss": miss}
    b.close(); f.unlink(missing_ok=True)
    return row


def main():
    specs = [s for s in os.environ.get("MZLAB_ENORMI_GB", "1").split(",") if s]
    r = Run(port=8897, wd="/tmp/wd97")
    rows = []
    try:
        with sync_playwright() as p:
            for spec in specs:
                tmp = Path(tempfile.mkdtemp(prefix="c40_"))
                try:
                    rows.append(one(r, p, spec, tmp))
                finally:
                    import shutil; shutil.rmtree(tmp, ignore_errors=True)
                print(json.dumps(rows[-1]), flush=True)
    finally:
        r.close()
    dst = get_env("E2E_JSON")
    if dst: Path(dst).write_text(json.dumps(rows, indent=1))
    print("\n  file GB | renderer PSS peak | wasm peak (sum) | OPFS MB | result")
    for x in rows:
        print(f"  {x['file_gb']:>7} | {x['renderer_pss_peak_mb']:>17} | {x['wasm_peak_sum_mb']:>15} | {x['opfs_mb']:>7} | {x['result']}")
    print("DONE")


if __name__ == "__main__":
    main()
