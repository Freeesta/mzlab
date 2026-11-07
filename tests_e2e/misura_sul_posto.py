"""Times the index of a big mzML two ways (docs/studi/lettura-sul-posto.md, step 1): A = copy into OPFS (8 MiB slices, SyncAccessHandle) then the
index read from the copy; B = the index read straight from the user's File (Blob slices, no copy). Runs interleave A and B, prints median and min-max
of each phase and the SHA-256 of every .mzidx (they must all be equal).

    python3 tests_e2e/misura_sul_posto.py FILE.mzML [--static site/static] [--giri 10] [--aiuti 3] [--out run.json]

The same file is read by both arms, so the page cache of the system serves both alike. Build first: python3 tools/build_site.py (or a static
folder with wasm/mzlab_wasm.js and idx-worker.js)."""
import argparse, http.server, json, shutil, socketserver, statistics, sys, tempfile, threading
from functools import partial
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent

PAGE = b"""<!doctype html><meta charset="utf-8"><title>sul posto</title><input type="file" id="f"><script type="module">
window.giro = async (arm, aiuti) => {
  const file = document.getElementById("f").files[0], name = "big.mzML", part = "big.mzML.mzidx.part";
  const root = await navigator.storage.getDirectory(), mz = await root.getDirectoryHandle("mzlab", { create: true });
  const files = await mz.getDirectoryHandle("files", { create: true }), idx = await mz.getDirectoryHandle("idx", { create: true });
  for (const [d, n] of [[files, name], [idx, part]]) { try { await d.removeEntry(n); } catch (_) { /* not there */ } }
  const module = window.MOD || (window.MOD = await WebAssembly.compileStreaming(fetch("wasm/mzlab_wasm_bg.wasm")));
  const t0 = performance.now();
  let blob = file, tCopy = 0;
  if (arm === "A") {
    const cw = new Worker("copia-w.js");
    await new Promise((ok, ko) => { cw.onmessage = e => e.data.ok ? ok() : ko(new Error(e.data.text)); cw.postMessage({ file, name }); });
    cw.terminate();
    blob = await (await files.getFileHandle(name)).getFile();
    tCopy = performance.now() - t0;
  }
  const t1 = performance.now();
  const wk = new Worker("idx-worker.js", { type: "module" });
  const res = await new Promise(ok => {
    wk.onmessage = e => { const m = e.data; if (m.op === "ready") wk.postMessage({ blob, prep: { size: blob.size, prints: [undefined, undefined] }, part, budget: 64, cap: 0, name, aiuti });
      else if (m.op === "done" || m.op === "error") ok(m); };
    wk.onerror = e => ok({ op: "error", text: String(e.message) });
    wk.postMessage({ op: "module", module });
  });
  wk.terminate();
  const tIdx = performance.now() - t1;
  if (res.op !== "done") return { arm, op: res.op, text: res.text || "", copy_ms: Math.round(tCopy), index_ms: Math.round(tIdx), total_ms: 0, sha: "" };
  const f = await (await idx.getFileHandle(part)).getFile();
  const sha = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", await f.arrayBuffer())), b => b.toString(16).padStart(2, "0")).join("");
  for (const [d, n] of [[files, name], [idx, part]]) { try { await d.removeEntry(n); } catch (_) { /* not there */ } }
  return { arm, op: res.op, text: res.text || "", copy_ms: Math.round(tCopy), index_ms: Math.round(tIdx), total_ms: Math.round(performance.now() - t0), idx_bytes: f.size, sha,
           isolated: self.crossOriginIsolated };
};
window.READY = true;
</script>"""

# the copy as rust-carica.js does it in the worker: 8 MiB slices of the File written through a SyncAccessHandle
COPY = b"""onmessage = async e => {
  try {
    const { file, name } = e.data, fr = new FileReaderSync(), STEP = 8 << 20;
    const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("files");
    const h = await (await d.getFileHandle(name, { create: true })).createSyncAccessHandle();
    h.truncate(0);
    for (let o = 0; o < file.size; o += STEP) h.write(new Uint8Array(fr.readAsArrayBuffer(file.slice(o, o + STEP))), { at: o });
    h.flush(); h.close(); postMessage({ ok: true });
  } catch (err) { postMessage({ ok: false, text: String(err && err.message || err) }); }
};"""


def serve(static):
    extra = {"/t.html": (PAGE, "text/html"), "/copia-w.js": (COPY, "text/javascript")}
    class H(http.server.SimpleHTTPRequestHandler):
        extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}
        def end_headers(self):
            self.send_header("Cross-Origin-Opener-Policy", "same-origin"); self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
            super().end_headers()
        def do_GET(self):
            p = self.path.split("?")[0]
            if p in extra:
                body, ct = extra[p]
                self.send_response(200); self.send_header("Content-Type", ct); self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body); return
            super().do_GET()
        def log_message(self, *a): pass
    s = socketserver.ThreadingTCPServer(("127.0.0.1", 0), partial(H, directory=str(static))); s.daemon_threads = True
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return s


def summary(runs, arm, key):
    v = [r[key] for r in runs if r["arm"] == arm]
    return {"median": statistics.median(v), "min": min(v), "max": max(v), "n": len(v)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("file"); ap.add_argument("--static", default=str(ROOT / "site" / "static"))
    ap.add_argument("--giri", type=int, default=10); ap.add_argument("--aiuti", type=int, default=3); ap.add_argument("--out"); ap.add_argument("--chrome"); ap.add_argument("--solo")
    a = ap.parse_args()
    s = serve(Path(a.static)); runs = []
    with sync_playwright() as p:
        prof = tempfile.mkdtemp(); kw = {"executable_path": a.chrome} if a.chrome else {}
        b = p.chromium.launch_persistent_context(prof, **kw); pg = b.new_page()      # a persistent profile: the ephemeral one keeps OPFS in memory with a small quota
        pg.on("pageerror", lambda e: print("pageerror", e, file=sys.stderr))
        pg.goto(f"http://127.0.0.1:{s.server_address[1]}/t.html"); pg.wait_for_function("window.READY === true")
        pg.set_input_files("#f", a.file)
        for i in range(a.giri):
            if a.solo: arm_list = (a.solo,)
            for arm in ((a.solo,) if a.solo else ("A", "B") if i % 2 == 0 else ("B", "A")):
                r = pg.evaluate("([arm, n]) => window.giro(arm, n)", [arm, a.aiuti]); runs.append(r)
                print(json.dumps(r), flush=True)
        b.close(); shutil.rmtree(prof, ignore_errors=True)
    s.shutdown()
    out = {"file_bytes": Path(a.file).stat().st_size, "aiuti": a.aiuti, "runs": runs,
           "A": {k: summary(runs, "A", k) for k in ("copy_ms", "index_ms", "total_ms")}, "B": {k: summary(runs, "B", k) for k in ("index_ms", "total_ms")},
           "shas": sorted({r["sha"] for r in runs}), "errors": [r for r in runs if r["op"] != "done"]}
    print(json.dumps({k: out[k] for k in ("A", "B", "shas", "errors")}, indent=1))
    if a.out: Path(a.out).write_text(json.dumps(out, indent=1))
    sys.exit(1 if out["errors"] or len(out["shas"]) != 1 else 0)


main()
