"""The .mzidx of an mzML built with helper workers (idx-aiuti.js: the arrays decoded on shared memory) is the same, byte for byte, as the one built alone
and as the native one (`cargo run --example mzidx`, when cargo is there); with a small budget too (rounds from the scratch file). Without cross-origin
isolation the helpers asked for are not started and the builder gives the same bytes, with no error. Build first: python tools/build_site.py
`--browser firefox|webkit` runs it in another engine."""
import base64, hashlib, http.server, json, random, shutil, socketserver, struct, subprocess, sys, tempfile, threading, zlib
from functools import partial
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

BROWSER = sys.argv[sys.argv.index("--browser") + 1] if "--browser" in sys.argv else "chromium"
STATIC = ROOT / "site" / "static"
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:400])); errs.append(name)

def arr(vals, fmt, zl, kind):
    raw = struct.pack("<%d%s" % (len(vals), fmt), *vals)
    data = base64.b64encode(zlib.compress(raw) if zl else raw).decode()
    acc = {"d": ("MS:1000523", "64-bit float"), "f": ("MS:1000521", "32-bit float")}[fmt]
    cv = [f'<cvParam cvRef="MS" accession="{acc[0]}" name="{acc[1]}"/>', '<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression"/>' if zl else '<cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>',
          '<cvParam cvRef="MS" accession="MS:1000514" name="m/z array"/>' if kind == "mz" else '<cvParam cvRef="MS" accession="MS:1000515" name="intensity array"/>']
    return f'<binaryDataArray encodedLength="{len(data)}">{"".join(cv)}<binary>{data}</binary></binaryDataArray>'

def mzml():
    """A small high-resolution profile run: MS1 profile scans with 64-bit m/z, three MS2 after each; some arrays without zlib, some 64-bit intensities."""
    rnd = random.Random(7); feats = [(100 + 900 * rnd.random(), 1e4 + 1e6 * rnd.random()) for _ in range(60)]; sp = []
    for i in range(80):
        mz = [m + k * 0.001 for m, _ in feats for k in range(-4, 5)]; it = [h * rnd.random() for _, h in feats for _ in range(9)]
        order = sorted(range(len(mz)), key=mz.__getitem__); mz = [mz[j] for j in order]; it = [it[j] for j in order]
        sp.append((1, i * 0.05, None, True, mz, it, "d" if i % 7 == 3 else "f", i % 5 != 2))
        if i % 3 == 0:
            for _ in range(3):
                m2 = sorted(50 + 600 * rnd.random() for _ in range(40)); sp.append((2, i * 0.05, 150 + 700 * rnd.random(), False, m2, [1e5 * rnd.random() for _ in m2], "f", True))
    out = ['<?xml version="1.0" encoding="utf-8"?>\n<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r"><spectrumList count="%d">' % len(sp)]
    for n, (lvl, rt, prec, prof, mz, it, f, zl) in enumerate(sp):
        pr = f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{prec:.5f}"/></selectedIon></selectedIonList></precursor></precursorList>' if prec else ""
        out.append(f'<spectrum index="{n}" id="scan={n + 1}" defaultArrayLength="{len(mz)}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{lvl}"/>'
                   f'<cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><cvParam cvRef="MS" accession="{"MS:1000128" if prof else "MS:1000127"}" name="{"profile spectrum" if prof else "centroid spectrum"}"/>'
                   f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt:.4f}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>{pr}'
                   f'<binaryDataArrayList count="2">{arr(mz, "d", zl, "mz")}{arr(it, f, zl, "it")}</binaryDataArrayList></spectrum>')
    out.append("</spectrumList></run></mzML>\n")
    return "".join(out).encode()

PAGE = b"""<!doctype html><meta charset="utf-8"><title>idx</title><script type="module">
import init, { prepare_upload } from "./wasm/mzlab_wasm.js";
import { aiutiPer, poolSize } from "./idxpool.js";
window.R = { runs: [] };
try {
  await init();
  R.isolated = self.crossOriginIsolated;
  // the helpers of a builder: one per core beside its own (at most 4) when alone, the free places otherwise, none below 3 cores or without isolation
  R.per = [[2, 1, true], [4, 1, true], [8, 1, true], [16, 1, true], [8, 2, true], [8, 3, true], [8, 1, false]].map(([c, u, i]) => aiutiPer(poolSize(false, c), u, i));
  const name = "par.mzML", bytes = new Uint8Array(await (await fetch(name)).arrayBuffer());
  const root = await navigator.storage.getDirectory();
  const files = await (await root.getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("files", { create: true });
  const w = await (await files.getFileHandle(name, { create: true })).createWritable(); await w.write(bytes); await w.close();
  const blob = await (await files.getFileHandle(name)).getFile();
  const inputs = Array.from(prepare_upload(bytes.length, (o, n) => bytes.slice(o, o + n), name, undefined, undefined, 0)).slice(0, 8);
  const module = await WebAssembly.compileStreaming(fetch("wasm/mzlab_wasm_bg.wasm"));
  for (const budget of [64, 1]) for (const aiuti of [0, 1, 3]) {
    const wk = new Worker("idx-worker.js", { type: "module" }), part = `p${budget}-${aiuti}.mzidx.part`;
    const res = await new Promise(ok => {
      wk.onmessage = e => { const m = e.data; if (m.op === "ready") wk.postMessage({ blob, inputs, part, budget, cap: 0, name, aiuti }); else if (m.op === "done" || m.op === "error") ok(m); };
      wk.onerror = e => ok({ op: "error", text: String(e.message) });
      wk.postMessage({ op: "module", module });
    });
    wk.terminate();
    const f = await (await (await (await root.getDirectoryHandle("mzlab")).getDirectoryHandle("idx")).getFileHandle(part)).getFile();
    const h = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", await f.arrayBuffer())), b => b.toString(16).padStart(2, "0")).join("");
    R.runs.push({ budget, aiuti, op: res.op, text: res.text || "", passes: res.stats && res.stats.passes, sha: h });
  }
} catch (e) { R.err = String(e && e.stack || e); }
window.DONE = true;
</script>"""

def serve(src, isolated):
    class H(http.server.SimpleHTTPRequestHandler):
        extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}
        def end_headers(self):
            if isolated: self.send_header("Cross-Origin-Opener-Policy", "same-origin"); self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
            super().end_headers()
        def do_GET(self):
            if self.path.split("?")[0] == "/t.html":
                self.send_response(200); self.send_header("Content-Type", "text/html"); self.send_header("Content-Length", str(len(PAGE))); self.end_headers(); self.wfile.write(PAGE); return
            if self.path.split("?")[0] == "/par.mzML":
                self.send_response(200); self.send_header("Content-Type", "application/xml"); self.send_header("Content-Length", str(src.stat().st_size)); self.end_headers(); self.wfile.write(src.read_bytes()); return
            super().do_GET()
        def log_message(self, *a): pass
    s = socketserver.ThreadingTCPServer(("127.0.0.1", 0), partial(H, directory=str(STATIC))); s.daemon_threads = True
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return s

def native(src, tmp):
    """The .mzidx of the app's engine (mzlab-core, the same builder as the desktop app) for each budget, or None without cargo."""
    if not shutil.which("cargo"): return None
    out = {}
    for budget in (64, 1):
        o = tmp / f"n{budget}.mzidx"
        r = subprocess.run(["cargo", "run", "-q", "--release", "-p", "mzlab-core", "--example", "mzidx", "--", str(src), str(o), str(budget)], cwd=ROOT, capture_output=True, text=True)
        if r.returncode: raise AssertionError("native build: " + r.stderr[-300:])
        out[budget] = hashlib.sha256(o.read_bytes()).hexdigest()
    return out

tmp = Path(tempfile.mkdtemp())
try:
    src = tmp / "par.mzML"; src.write_bytes(mzml())
    got = {}
    with sync_playwright() as p:
        b = getattr(p, BROWSER).launch()
        for isolated in (True, False):
            s = serve(src, isolated)
            ctx = b.new_context(); pg = ctx.new_page(); pg.on("pageerror", lambda e: errs.append("pageerror " + str(e)))
            pg.goto(f"http://127.0.0.1:{s.server_address[1]}/t.html"); pg.wait_for_function("window.DONE === true", timeout=300000)
            got[isolated] = pg.evaluate("window.R"); ctx.close(); s.shutdown()
        b.close()
    def isolated_helpers():
        r = got[True]; assert not r.get("err"), r.get("err"); assert r["isolated"] is True, r
        assert all(x["op"] == "done" for x in r["runs"]), r["runs"]
        for budget in (64, 1):
            shas = {x["sha"] for x in r["runs"] if x["budget"] == budget}; assert len(shas) == 1, (budget, r["runs"])
        assert any(x["passes"] for x in r["runs"] if x["budget"] == 1)
    step("with helpers (1 and 3) the .mzidx is the one built alone, at 64 MB and 1 MB of budget", isolated_helpers)
    def not_isolated():
        r = got[False]; assert not r.get("err"), r.get("err"); assert r["isolated"] is False, r
        assert all(x["op"] == "done" for x in r["runs"]), r["runs"]
        for x in r["runs"]: assert x["sha"] == next(y["sha"] for y in got[True]["runs"] if y["budget"] == x["budget"]), x
    step("without isolation the helpers asked for are not started: same bytes, no error", not_isolated)
    def helper_count():
        assert got[True]["per"] == [0, 3, 4, 4, 2, 1, 0], got[True]["per"]
    step("helpers per builder: min(4, cores - 1) when alone, the free places otherwise, none below 3 cores or without isolation", helper_count)
    def same_as_native():
        n = native(src, tmp)
        if n is None: print("   no cargo: native comparison skipped"); return
        for budget in (64, 1):
            assert n[budget] == next(y["sha"] for y in got[True]["runs"] if y["budget"] == budget), (budget, n)
    step("the same bytes as the native engine of the app", same_as_native)
finally:
    shutil.rmtree(tmp, ignore_errors=True)
print("FAILED:", errs) if errs else print("ALL OK")
sys.exit(1 if errs else 0)
