"""Splits the vendored Ketcher bundle so its Indigo worker is a real file instead of base64 inside the page script.

    python3 tools/ketcher_estrai_worker.py [--dry-run] [--rev REV]

The Vite/rollup build inlines the Indigo web worker as a 21 MB base64 string; the page thread decodes it (atob), makes a Blob and starts the
worker, and inside that worker the 11.8 MB WebAssembly module is base64 again. That costs seconds of main-thread time in the Disegno iframe.
This script rewrites the bundle in place so that:
  - assets/indigo-worker-<hash>.js  is the worker (classic script, the wasm is read from a file by a synchronous XHR in the worker thread),
  - assets/indigo-<hash>.wasm       is the Indigo module as bytes,
  - assets/index-<hash>.js          is the page bundle, now loading the worker by URL (same name: the other chunks import it by that name).
Starting point is the vendored file in git (`--rev`, default the working tree); when the bundle is already split it does nothing.
Nothing is printed beyond a few short lines: the files are one-line megabyte strings.
"""
import argparse
import base64
import hashlib
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KET = ROOT / "mzlab" / "web" / "vendor" / "ketcher"
MARK = "Lyogcm9sbHVwLXBsdWdpbi13ZWItd29ya2VyLWxvYWRlciAq"      # base64 of "/* rollup-plugin-web-worker-loader *"
WORKER_B64 = re.compile(r'"(' + MARK + r'[A-Za-z0-9+/=]+)"')
WASM_B64 = re.compile(r'function findWasmBinary\(\)\{return base64Decode\("([A-Za-z0-9+/=]{100000,})"\)\}')
# the blob factory of the page bundle: base64 -> Blob -> object URL
BLOB_FACTORY = re.compile(r'function (\w+)\(F,Q,U\)\{var B=\w+\(F\),R=B\.indexOf\((?:`\n`|"\\n"),10\)\+1,V=B\.substring\(R\)\+"",n=new Blob\(\[V\],\{type:"application/javascript"\}\);return URL\.createObjectURL\(n\)\}')

# worker thread only: a synchronous request is fine there and returns the module bytes without a string copy
WASM_LOADER = ('function findWasmBinary(){var x=new XMLHttpRequest;x.open("GET",new URL("%s",self.location.href).href,false);x.responseType="arraybuffer";'
               'x.send();if(x.status!==200)throw new Error("Indigo wasm: HTTP "+x.status);return new Uint8Array(x.response)}')


def short(h: bytes) -> str:
    return hashlib.sha256(h).hexdigest()[:8]


def main_bundle() -> Path:
    c = [p for p in (KET / "assets").glob("index-*.js")]
    if len(c) != 1:
        sys.exit(f"expected one assets/index-*.js, found {len(c)}")
    return c[0]


def context(s: str, pat: str, n: int = 90) -> str:
    i = s.find(pat)
    return "not found" if i < 0 else repr(s[max(0, i - n):i + n])


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--dry-run", action="store_true", help="report sizes and the rewrite, write nothing")
    ap.add_argument("--rev", help="read the page bundle from this git revision instead of the working tree")
    a = ap.parse_args()
    path = main_bundle()
    rel = path.relative_to(ROOT).as_posix()
    src = subprocess.run(["git", "show", f"{a.rev}:{rel}"], cwd=ROOT, capture_output=True, check=True).stdout.decode("utf8") if a.rev else path.read_text("utf8")
    m = WORKER_B64.search(src)
    if not m:
        print("already split: no inline worker in", path.name)
        return 0
    worker = base64.b64decode(m.group(1)).decode("utf8")
    first_nl = worker.index("\n", 10) + 1                 # the factory drops the first (comment) line; keep the same source
    worker = worker[first_nl:]
    w = WASM_B64.search(worker)
    if not w:
        print("wasm loader not found in the worker; findWasmBinary:", context(worker, "findWasmBinary"))
        return 1
    wasm = base64.b64decode(w.group(1))
    if wasm[:4] != b"\0asm":
        print("decoded wasm has no magic number")
        return 1
    f = BLOB_FACTORY.search(src)
    if not f:
        print("blob factory not found; atob context:", context(src, "atob(", 120))
        return 1
    wasm_name = f"indigo-{short(wasm)}.wasm"
    worker = worker[:w.start()] + WASM_LOADER % wasm_name + worker[w.end():]
    worker_bytes = worker.encode("utf8")
    worker_name = f"indigo-worker-{short(worker_bytes)}.js"
    # the worker URL is relative to the page bundle (import.meta.url), like its other chunks
    page = src[:m.start()] + '"./' + worker_name + '"' + src[m.end():]
    page = page[:f.start()] + f"function {f.group(1)}(F,Q,U){{return new URL(F,import.meta.url).href}}" + page[f.end():]
    page_bytes = page.encode("utf8")
    print(f"page bundle {len(src.encode('utf8'))} -> {len(page_bytes)} bytes; worker {len(worker_bytes)} bytes; wasm {len(wasm)} bytes")
    print("files:", worker_name, wasm_name)
    if a.dry_run:
        return 0
    assets = KET / "assets"
    (assets / worker_name).write_bytes(worker_bytes)
    (assets / wasm_name).write_bytes(wasm)
    path.write_bytes(page_bytes)
    # hints for the warm-up of first-paint.js (it prefetches the scripts, styles and these links of index.html)
    idx = KET / "index.html"
    hints = f'<link rel="prefetch" href="./assets/{worker_name}" fetchpriority="low"><link rel="prefetch" href="./assets/{wasm_name}" fetchpriority="low">'
    idx.write_text(idx.read_text("utf8").replace("</head>", hints + "</head>", 1), "utf8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
