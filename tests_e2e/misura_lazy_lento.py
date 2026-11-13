"""Probe of the modules read on first use (`carica`) on a slow link, first visit: how long the page waits for `elementare-pannello.js` (first
opening of «Elemental composition») and `origine.js` (first «Where does this ion come from?») right after a file is loaded, while the service
worker fills its cache in the background; `--probe cmp` instead times the click on «Elemental composition» (BARRA.show) until its panel is
drawn, as a user does it. Measures, never asserts: used by docs/agenti/misure-caricamento.md.
Link model: 1.6 Mbit/s down shared by every connection and 150 ms before each answer, applied by the local server (tools/slow_network.py); CPU x4 over CDP (misura_ricarica.Throttle). The server speaks HTTP/1.1
(six connections per host), GitHub Pages HTTP/2: a request queues behind open ones here, there it shares the bandwidth only.
"""
from __future__ import annotations

import argparse, json, os, statistics, sys, threading
from http.server import ThreadingHTTPServer
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import misura_ricarica as mr
import misura_lista_molti as mlm
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
import slow_network

RATE, RTT = 1_600_000, 0.150


def serve(folder: str) -> str:
    SlowMirror = slow_network.lento(mr.Mirror, RATE, RTT)
    SlowMirror.root = Path(folder); SlowMirror.upstream = ""
    srv = ThreadingHTTPServer(("127.0.0.1", 0), SlowMirror)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{srv.server_address[1]}/mzlab/"

MODS = ["m:elementare-pannello.js", "m:origine.js"]
TIME = """async ([n, d]) => { await new Promise(r => setTimeout(r, d)); const t = performance.now(); await carica(n); return [Math.round(performance.now() - t), Math.round(performance.now())]; }"""
CMP = """async d => { await new Promise(r => setTimeout(r, d)); const t = performance.now(); BARRA.show("cmp");
  while (!document.querySelector("#hri-body .cx")) await new Promise(r => requestAnimationFrame(r));
  return [Math.round(performance.now() - t), Math.round(performance.now())]; }"""


def one(b, thr, base: str, files: list[str], mod: str, delay: int) -> dict:
    r = mr.Run(b, thr, base, os.devnull)
    pg = r.pg
    t0 = mr.t_wall()
    pg.goto(base + "#/data")
    r.until_true(mr.USABLE)
    usable = mr.t_wall() - t0
    pg.set_input_files("#pick", files)
    # >=: a DDA file opens as two rows (MS1, MS2)
    r.until_true(f"UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length >= {len(files)}")
    loaded = mr.t_wall() - t0
    t1 = mr.t_wall() + delay
    wait, at = pg.evaluate(CMP, delay) if mod == "cmp" else pg.evaluate(TIME, [mod, delay])
    # the fetches of the service worker that shared the link with the module (open when it was asked for; still open ones count with no size)
    seen = list(thr.sw) + [x for x in list(thr._pending.values()) if x[0] < t1 + wait]
    busy = [[x[1].replace(base, ""), round(x[3] / 1024)] for x in seen if x[0] >= t0 and x[0] < t1 + wait and (x[4] > t1 or not x[4]) and "elementare-pannello" not in x[1] and mod[2:] not in x[1]]
    pg.context.close()
    return {"mod": mod, "delay_ms": delay, "usable_ms": usable, "loaded_ms": loaded, "wait_ms": wait, "page_ms": at, "sw_open": len(busy), "sw_open_kb": sum(k for _, k in busy), "sw_open_files": busy[:8]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--site", required=True, help="built site folder (tools/prepara_sito.py)")
    ap.add_argument("--delays", default="0,3000,6000,9000,12000", help="ms between the file loaded and the first use, one fresh visit each")
    ap.add_argument("--probe", choices=["carica", "cmp"], default="carica")
    ap.add_argument("--file", help="data file to load instead of two small synthetic ones (an HR mzML for --probe cmp)")
    ap.add_argument("--runs", type=int, default=1, help="fresh visits per delay")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    from playwright.sync_api import sync_playwright
    base = serve(a.site)
    files = [a.file] if a.file else mlm.small_files(Path("/tmp/misura_lazy_lento_in"), 2)
    mods = ["cmp"] if a.probe == "cmp" else MODS
    exe = "/opt/pw-browsers/chromium"
    res = []
    with sync_playwright() as p:
        b = p.chromium.launch(**({"executable_path": exe} if os.path.isfile(exe) else {}))
        thr = mr.Throttle(b, "cpu4")
        for d in [int(x) for x in a.delays.split(",")] * a.runs:
            for m in mods:
                x = one(b, thr, base, files, m, d); res.append(x); print(json.dumps(x), flush=True)
        b.close()
    summ = {m: statistics.median([x["wait_ms"] for x in res if x["mod"] == m]) for m in mods}
    print("median wait ms:", summ)
    Path(a.out).write_text(json.dumps({"runs": res, "median_wait_ms": summ}, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
