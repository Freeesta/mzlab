"""Builds the .wasm msngrafo into site/static/wasm-feat (manifest entry with its SHA-256) when the built site does not have it, and copies the page sources the tests change."""
import hashlib, json, shutil, subprocess
from lib import ROOT

PAGINA = ("rust-msngrafo.js", "rust-msngrafo-nativo.js", "rust-msngrafo-pagina.js", "rust-worker.js", "rust-bridge.js", "rust-bridge-info.js", "browser.js", "infusione-grafo-avanza.js", "infusione-grafo-stato.js", "infusione-grafo.js")


def prepara():
    feat = ROOT / "site" / "static" / "wasm-feat"
    mod = feat / "msngrafo"
    if not (mod / "mzlab_msngrafo_wasm_bg.wasm").exists():
        sh = lambda *a: subprocess.run(a, cwd=ROOT, check=True, capture_output=True)
        sh("cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-msngrafo-wasm")
        mod.mkdir(parents=True, exist_ok=True)
        sh("wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(mod), "target/wasm32-unknown-unknown/release/mzlab_msngrafo_wasm.wasm")
        if shutil.which("wasm-opt"):
            bg = str(mod / "mzlab_msngrafo_wasm_bg.wasm")
            sh("wasm-opt", "-Oz", "--enable-bulk-memory", "--enable-sign-ext", "--enable-mutable-globals", "--enable-nontrapping-float-to-int", "-o", bg, bg)
        mp = feat / "manifest.json"
        m = json.loads(mp.read_text()) if mp.exists() else {}
        m["msngrafo"] = {"js": "msngrafo/mzlab_msngrafo_wasm.js", "wasm": "msngrafo/mzlab_msngrafo_wasm_bg.wasm", "sha256": hashlib.sha256((mod / "mzlab_msngrafo_wasm_bg.wasm").read_bytes()).hexdigest()}
        mp.write_text(json.dumps(m, indent=1))
    for f in PAGINA:
        shutil.copy(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)
