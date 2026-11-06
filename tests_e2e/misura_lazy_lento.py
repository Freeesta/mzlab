"""Probe of the modules read on first use (`carica`) on a slow link, first visit: how long the page waits for `elementare-pannello.js` (first
opening of «Elemental composition») and `origine.js` (first «Where does this ion come from?») right after a file is loaded, while the service
worker fills its cache in the background. Measures, never asserts: used by docs/agenti/misure-caricamento.md.
Link model: 1.6 Mbit/s down shared by every connection and 150 ms before each answer, applied by the local server (CDP network emulation does
not reach the workers and is not applied reliably to the service worker); CPU x4 over CDP (misura_ricarica.Throttle). The server speaks HTTP/1.1
(six connections per host), GitHub Pages HTTP/2: a request queues behind open ones here, there it shares the bandwidth only.
"""
from __future__ import annotations

import argparse, json, os, statistics, sys, threading, time
from http.server import ThreadingHTTPServer
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import misura_ricarica as mr
import misura_lista_molti as mlm

RATE, RTT, CHUNK = 1_600_000 / 8, 0.150, 4096


class Link:
    """One downlink for all connections: each chunk waits for its turn on the line (bytes / RATE)."""
    lock, free = threading.Lock(), 0.0

    @classmethod
    def take(cls, n: int) -> None:
        with cls.lock:
            now = time.monotonic(); cls.free = end = max(now, cls.free) + n / RATE
        time.sleep(end - now)


class SlowFile:
    def __init__(self, raw):
        self.raw = raw

    def write(self, b) -> int:
        b = bytes(b)
        for i in range(0, len(b), CHUNK):
            Link.take(len(b[i:i + CHUNK])); self.raw.write(b[i:i + CHUNK])
        return len(b)

    def flush(self):
        self.raw.flush()


class SlowMirror(mr.Mirror):
    def setup(self):
        super().setup(); self.wfile = SlowFile(self.wfile)

    def do_GET(self):
        time.sleep(RTT); super().do_GET()


def serve(folder: str) -> str:
    SlowMirror.root = Path(folder); SlowMirror.upstream = ""
    srv = ThreadingHTTPServer(("127.0.0.1", 0), SlowMirror)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{srv.server_address[1]}/mzlab/"

MODS = ["m:elementare-pannello.js", "m:origine.js"]
TIME = """async n => { const t = performance.now(); await carica(n); return [Math.round(performance.now() - t), Math.round(performance.now())]; }"""


def one(b, thr, base: str, files: list[str], mod: str, delay: int) -> dict:
    r = mr.Run(b, thr, base, os.devnull)
    pg = r.pg
    t0 = mr.t_wall()
    pg.goto(base + "#/data")
    r.until_true(mr.USABLE)
    usable = mr.t_wall() - t0
    pg.set_input_files("#pick", files)
    r.until_true(f"UPBUSY === 0 && E.files.filter(f => !f.gone).length === {len(files)}")
    loaded = mr.t_wall() - t0
    time.sleep(delay / 1000)
    t1 = mr.t_wall()
    wait, at = pg.evaluate(TIME, mod)
    time.sleep(0.5)
    # the fetches of the service worker that shared the link with the module (still open when it was asked for)
    busy = [[x[1].replace(base, ""), round(x[3] / 1024)] for x in thr.sw if x[0] >= t0 and x[0] < t1 + wait and x[4] > t1 and mod[2:] not in x[1]]
    pg.context.close()
    return {"mod": mod, "delay_ms": delay, "usable_ms": usable, "loaded_ms": loaded, "wait_ms": wait, "page_ms": at, "sw_open": len(busy), "sw_open_kb": sum(k for _, k in busy), "sw_open_files": busy[:8]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--site", required=True, help="built site folder (tools/prepara_sito.py)")
    ap.add_argument("--delays", default="0,3000,6000,9000,12000", help="ms between the file loaded and the first use, one fresh visit each")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    from playwright.sync_api import sync_playwright
    base = serve(a.site)
    files = mlm.small_files(Path("/tmp/misura_lazy_lento_in"), 2)
    exe = "/opt/pw-browsers/chromium"
    res = []
    with sync_playwright() as p:
        b = p.chromium.launch(**({"executable_path": exe} if os.path.isfile(exe) else {}))
        thr = mr.Throttle(b, "cpu4")
        for d in [int(x) for x in a.delays.split(",")]:
            for m in MODS:
                x = one(b, thr, base, files, m, d); res.append(x); print(json.dumps(x), flush=True)
        b.close()
    summ = {m: statistics.median([x["wait_ms"] for x in res if x["mod"] == m]) for m in MODS}
    print("median wait ms:", summ)
    Path(a.out).write_text(json.dumps({"runs": res, "median_wait_ms": summ}, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
