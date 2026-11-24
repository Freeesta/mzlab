"""Shared by the e2e of mzFinder in the browser (e2e_feat_worker.py, e2e_feat_run.py): the two .wasm of crates/mzlab-feat-wasm built into a folder with the workers, and a server for it."""
import shutil, subprocess, sys, threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from lib import ROOT, free_port
sys.path.insert(0, str(ROOT / "tests")); sys.path.insert(0, str(ROOT / "tools"))
from cargo_util import wasm_module

def sh(*a, cwd=ROOT):
    return subprocess.run(a, cwd=cwd, check=True, capture_output=True, encoding="utf-8").stdout

def build_wasm(tmp: Path):
    """feat/ and proofs/ (wasm-bindgen --target web), feat-worker.js and feat-run.js in `tmp`; the sizes after wasm-opt (empty without it)."""
    sizes = {}
    for which in ("detect", "proofs"):
        module = wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", which)   # its own copy: other e2e build other features
        out = tmp / ("feat" if which == "detect" else "proofs")
        sh("wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(out), str(module))
        bg = out / "mzlab_feat_wasm_bg.wasm"
        if shutil.which("wasm-opt"):
            sh("wasm-opt", "-Oz", "--enable-bulk-memory", "--enable-sign-ext", "--enable-mutable-globals", "--enable-nontrapping-float-to-int", "-o", str(bg), str(bg))
            sizes[which] = bg.stat().st_size
    for n in ("feat-worker.js", "feat-run.js"):
        shutil.copy(ROOT / "TP_Mine/files" / n, tmp / n)
    (tmp / "index.html").write_text("<!doctype html><meta charset=utf-8><title>feat</title><body>feat</body>")
    return sizes

class Quiet(SimpleHTTPRequestHandler):
    def log_message(self, *a): pass

def serve(tmp: Path):
    port = free_port()
    srv = ThreadingHTTPServer(("127.0.0.1", port), partial(Quiet, directory=str(tmp)))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, f"http://127.0.0.1:{port}/"
