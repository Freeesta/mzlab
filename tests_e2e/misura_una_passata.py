"""Bound of a one-pass load of a plain mzML (copy to OPFS and index from the same bytes): times each phase of today's two passes on the built site. Measures, never asserts.

    python3 tests_e2e/misura_una_passata.py --site site --files a.mzML b.mzML [--runs 5] [--cpu 1,4] [--arms due,una] [--out run.json]

One run = one drop of one file on a fresh copy of a warm persistent Chromium profile (cross-origin isolated), files interleaved run by run. From the `mzlab-index` events
of the file (the first event of a stage is posted at once, later ones at most every 100 ms): copy (stage `load`), pool start (`start`/`wait`), scan table (`scan`), builder read
of the OPFS copy with decoding (`read`), rounds over the scratch file (`index`), tail (open + first chart to all indexed). Then, with nothing else running, a worker reads the
OPFS copy in 4 MiB blocks through a read-only SyncAccessHandle as the builder does (I/O alone) and the dropped File through FileReaderSync (the copy's source alone).
Bound of one pass: the builder's re-read of the copy goes and its decoding overlaps the copy, gain <= min(scan + read, copy + I/O); with the CPU of the browser's
processes (/proc, every 25 ms) the builder's work moved into the copy runs only on the cores the copy leaves idle: gain <= min(scan + read, copy x min(1, idle cores / builder
cores) + I/O) (`bound_cpu`). Both against drop -> indexed.
Arms (`--arms`, interleaved run by run): `due` = today's two passes, `una` = the one-pass spike (mzlab/web/una-passata.js, switched on by the OPFS file `mzlab/una-passata.on`); each run also
gives the SHA-256 of the finished `.mzidx`, whether the one pass held (`one_pass`, from the console line of una-passata.js) and the peak of the summed RSS of the browser's processes (`rss_mb`)."""
import argparse, json, os, shutil, statistics, sys, tempfile, threading, time
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
import scenari_caricamento as sc      # noqa: E402  (site server, profiles, probe)

# a worker made from a Blob (worker-src blob: is allowed): times a whole read of the OPFS copy (4 MiB blocks, read-only handle) and of the dropped File
BENCH = r"""async ({ name, withFile }) => {
  const src = `onmessage = async ev => { const { name, file } = ev.data, B = 4 << 20, out = {};
    const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files');
    for (const k of ['opfs1', 'opfs2']) { const h = await (await d.getFileHandle(name)).createSyncAccessHandle({ mode: 'read-only' }); const n = h.getSize(), buf = new Uint8Array(B);
      const t = performance.now(); for (let o = 0; o < n; o += B) h.read(buf.subarray(0, Math.min(B, n - o)), { at: o }); out[k] = performance.now() - t; out.size = n; h.close(); }
    if (file) { const fr = new FileReaderSync(), t = performance.now(); for (let o = 0; o < file.size; o += B) fr.readAsArrayBuffer(file.slice(o, o + B)); out.blob = performance.now() - t; }
    postMessage(out); };`;
  const w = new Worker(URL.createObjectURL(new Blob([src], { type: 'text/javascript' })));
  const file = withFile ? document.getElementById('__bin').files[0] : null;
  const r = await new Promise((ok, ko) => { w.onmessage = e => ok(e.data); w.onerror = e => ko(String(e.message || e)); w.postMessage({ name, file }); });
  w.terminate(); return r; }"""


class CpuSampler(threading.Thread):
    """CPU time and summed RSS of all the browser's processes (those started with the profile), every 25 ms: (epoch ms, CPU seconds) and the RSS peak from /proc, no browser call."""
    def __init__(s, prof): super().__init__(daemon=True); s.prof, s.on, s.v, s.tck, s.page, s.rss = prof.encode(), True, [], os.sysconf("SC_CLK_TCK"), os.sysconf("SC_PAGE_SIZE"), 0
    def total(s):
        tot = rss = 0
        for d in os.listdir("/proc"):
            if not d.isdigit(): continue
            try:
                if s.prof in open(f"/proc/{d}/cmdline", "rb").read():
                    f = open(f"/proc/{d}/stat").read().rsplit(")", 1)[1].split(); tot += int(f[11]) + int(f[12]); rss += int(f[21]) * s.page
            except Exception: pass
        s.rss = max(s.rss, rss)
        return tot / s.tck
    def run(s):
        while s.on: s.v.append((time.time() * 1000, s.total())); sc._WAIT(0.025)
    def cores(s, a, b):
        """Average busy cores between epoch ms a and b (the samples nearest each end)."""
        if not s.v or b <= a: return None
        near = lambda t: min(s.v, key=lambda x: abs(x[0] - t))
        (ta, ca), (tb, cb) = near(a), near(b)
        return round((cb - ca) / ((tb - ta) / 1000), 2) if tb > ta else None


def phases(s, name):
    """The stage boundaries of one file, ms from the drop: the first event of each stage (and the last event)."""
    ev = [x for x in s["idx"] if x[1] == name]
    first = {}
    for t, _, ph, _ in ev: first.setdefault(ph or "-", t)
    end = ev[-1][0] if ev else None
    nxt = lambda *after: min([first[p] for p in after if p in first] or [end])
    o = {"copy0": first.get("load"), "copy1": nxt("start", "wait", "scan", "read", "index"), "scan0": first.get("scan"), "read0": first.get("read"), "index0": first.get("index"), "last": end}
    o["copy"] = o["copy1"] - o["copy0"] if o["copy0"] is not None else None
    o["pool"] = (o["scan0"] - o["copy1"]) if o["scan0"] is not None else None
    o["scan"] = (o["read0"] - o["scan0"]) if o["read0"] is not None and o["scan0"] is not None else None
    o["read"] = ((o["index0"] if o["index0"] is not None else end) - o["read0"]) if o["read0"] is not None else None
    o["rounds"] = (end - o["index0"]) if o["index0"] is not None else 0
    return o


# the switch of the spike in OPFS (`mzlab/una-passata.on`), made or removed before the drop
SWITCH = r"""async on => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab', { create: true });
  if (on) await d.getFileHandle('una-passata.on', { create: true }); else await d.removeEntry('una-passata.on').catch(() => null); }"""
# SHA-256 of the finished index of the file
IDXSHA = r"""async name => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx');
  const b = await (await (await d.getFileHandle(name + '.mzidx')).getFile()).arrayBuffer();
  return [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join(''); }"""


def one_run(p, site, cpu, path, tmpl, arm="due"):
    prof = tempfile.mkdtemp(prefix="mzlab-1p-"); shutil.rmtree(prof)
    shutil.copytree(tmpl, prof, symlinks=True); (Path(prof) / ".ok").unlink(missing_ok=True)
    site.hold, site.release = (), None; site.log.clear()
    ctx = sc.open_ctx(p, prof); pg = ctx.pages[0] if ctx.pages else ctx.new_page(); cpu_s = CpuSampler(prof)
    try:
        una = []
        pg.on("console", lambda m: una.append(m.text) if m.text.startswith("una-passata ") else None)
        pg.goto(f"http://127.0.0.1:{site.port}/"); sc.wait_ready(pg); pg.evaluate(SWITCH, arm == "una"); sc.nap(pg, 3000); cdp = ctx.new_cdp_session(pg)
        if cpu > 1: cdp.send("Emulation.setCPUThrottlingRate", {"rate": cpu})
        cpu_s.start(); pg.set_input_files("#pick", [path]); sc.poll(pg, "!!__s.tall"); cpu_s.on = False
        s = pg.evaluate("__s"); t0e = s["t0e"]
        if cpu > 1: cdp.send("Emulation.setCPUThrottlingRate", {"rate": 1})
        sc.nap(pg, 1500)
        pg.evaluate("(() => { const i = document.createElement('input'); i.type = 'file'; i.id = '__bin'; i.hidden = true; document.body.append(i); })()")
        pg.set_input_files("#__bin", [path])
        b = pg.evaluate(BENCH, {"name": Path(path).name, "withFile": True})
        o = phases(s, Path(path).name); o.update(tprov=s.get("tprov"), tchart=s.get("tchart"), tall=s.get("tall"), io=round(min(b["opfs1"], b["opfs2"])), io_first=round(b["opfs1"]), blob=round(b.get("blob", 0)), size=b["size"])
        o.update(arm=arm, rss_mb=round(cpu_s.rss / 2**20), mzidx=pg.evaluate(IDXSHA, Path(path).name))
        if una: u = json.loads(una[-1].split(" ", 1)[1]); o.update(one_pass=u.get("one_pass"), live=u.get("live"), from_file=u.get("from_file"))
        o["tail"] = o["tall"] - o["last"] if o.get("tall") and o.get("last") is not None else None
        builder = (o["scan"] or 0) + (o["read"] or 0)
        o["bound"] = min(builder, (o["copy"] or 0) + o["io"]); o["bound_pool"] = min(builder + (o["pool"] or 0), (o["copy"] or 0) + o["io"] + (o["pool"] or 0))
        o["bound_pct"] = round(100 * o["bound"] / o["tall"], 1) if o.get("tall") else None
        o["bound_pool_pct"] = round(100 * o["bound_pool"] / o["tall"], 1) if o.get("tall") else None
        o["decode"] = (o["read"] or 0) - o["io"]
        # CPU-aware bound: the builder's work moved into the copy runs on the cores the copy leaves idle, at the builder's own parallelism
        ncpu = os.cpu_count() or 1
        o["cores_copy"] = cpu_s.cores(t0e + o["copy0"], t0e + o["copy1"]) if o["copy0"] is not None else None
        o["cores_build"] = cpu_s.cores(t0e + o["scan0"], t0e + o["last"]) if o["scan0"] is not None else None
        if o["cores_copy"] is not None and o["cores_build"]:
            o["bound_cpu"] = round(min(builder, (o["copy"] or 0) * min(1.0, max(0.0, ncpu - o["cores_copy"]) / o["cores_build"]) + o["io"]))
            o["bound_cpu_pct"] = round(100 * o["bound_cpu"] / o["tall"], 1) if o.get("tall") else None
        return o
    finally:
        try: ctx.close()
        except Exception: pass
        shutil.rmtree(prof, ignore_errors=True)


KEYS = ("tall", "tprov", "tchart", "rss_mb", "live", "from_file", "copy", "pool", "scan", "read", "rounds", "tail", "io", "io_first", "blob", "decode", "bound", "bound_pct", "bound_pool", "bound_pool_pct", "cores_copy", "cores_build", "bound_cpu", "bound_cpu_pct")


def table(res):
    out = ["| file | CPU | runs | " + " | ".join(KEYS) + " | one pass | .mzidx |", "|" + "---|" * (len(KEYS) + 5)]
    for (f, cpu), rs in res.items():
        cell = lambda k: (lambda v: "-" if not v else f"{statistics.median(v):g} [{min(v):g}-{max(v):g}]")([r[k] for r in rs if r.get(k) is not None])
        held = sum(1 for r in rs if r.get("one_pass")); shas = sorted({r.get("mzidx", "")[:12] for r in rs})
        out.append(f"| {f} | {cpu}x | {len(rs)} | " + " | ".join(cell(k) for k in KEYS) + f" | held {held} | {' '.join(shas)} |")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--site", default="site"); ap.add_argument("--files", nargs="+", required=True); ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--cpu", default="1"); ap.add_argument("--arms", default="due"); ap.add_argument("--wd", default=None); ap.add_argument("--out", default=None)
    a = ap.parse_args()
    wd = Path(a.wd or tempfile.mkdtemp(prefix="mzlab-1p-wd-")); wd.mkdir(parents=True, exist_ok=True)
    site = sc.Site(Path(a.site).resolve(), 8978); res = {}
    try:
        with sync_playwright() as p:
            sc.make_warm(p, site, wd / "warm-profile")
            for i in range(a.runs):
                for cpu in [int(x) for x in a.cpu.split(",")]:
                    for f in a.files:
                        for arm in a.arms.split(","):
                            tag = f"{Path(f).name} {arm}"
                            try: o = one_run(p, site, cpu, str(Path(f).resolve()), wd / "warm-profile", arm)
                            except Exception as e: print(f"{tag} {cpu}x run {i + 1}: FAILED {str(e)[:200]}", flush=True); continue
                            res.setdefault((tag, cpu), []).append(o); print(f"{tag} {cpu}x run {i + 1}: " + json.dumps(o), flush=True)
    finally:
        site.close()
    print(table(res), flush=True)
    if a.out: Path(a.out).write_text(json.dumps({f"{k[0]}|{k[1]}": v for k, v in res.items()}))


if __name__ == "__main__":
    main()
