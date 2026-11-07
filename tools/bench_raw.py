#!/usr/bin/env python3
"""Times and measures the memory of the `thermo` module (or another family) of the readers, in Node, step by step: open, samples, index, scan (all the scans), file_header, method, to_mzml.
One JSON line per step: {"step", "ms", "mem" (bytes of the WASM memory after the step), "bytes", "sha256" of the text/bytes the step produced}. Python is only the tool: the module is built by
tools/build_lettori_wasm.sh's own cargo line (without wasm-opt, like the measures of docs/studi/RAW_VELOCE.md §2.3) and wrapped by `wasm-bindgen --target nodejs`.
Use: python3 tools/bench_raw.py FILE.raw [--family thermo] [--steps open,index,...] [--no-build] [--keep DIR]
Needs: cargo with the wasm32-unknown-unknown target, wasm-bindgen (same version as Cargo.lock), node."""
import argparse, os, shutil, subprocess, sys, tempfile

JS = r"""
const fs = require('fs'), crypto = require('crypto');
const m = require(process.argv[2]);
const file = process.argv[3], steps = process.argv[4].split(',');
const fd = fs.openSync(file, 'r'), size = fs.statSync(file).size;
const read = (i, off, n) => { const b = new Uint8Array(n); const got = fs.readSync(fd, b, 0, n, off); if (got !== n) throw new Error('short read'); return b; };
const names = [require('path').basename(file)], sizes = [size];
const mem = () => m.__wasm.memory.buffer.byteLength;
const sha = (x) => crypto.createHash('sha256').update(x).digest('hex');
const out = (o) => console.log(JSON.stringify(o));
let h = null, peak = 0;
const step = (name, f) => {
  if (name !== 'open' && !steps.includes(name)) return;
  const t = process.hrtime.bigint();
  let r;
  try { r = f() || {}; } catch (e) { out({ step: name, error: String(e && e.message || e), mem: mem() }); process.exit(1); }
  const ms = Number(process.hrtime.bigint() - t) / 1e6;
  peak = Math.max(peak, mem());
  out(Object.assign({ step: name, ms: Math.round(ms * 10) / 10, mem: mem() }, r));
};
step('open', () => { h = m.open(names, sizes, read); });
let camp = null;
step('samples', () => { const s = m.samples(h); camp = JSON.parse(s); return { bytes: s.length, sha256: sha(s) }; });
step('index', () => { const s = m.index(h, 0); return { bytes: s.length, sha256: sha(Buffer.from(s.buffer, s.byteOffset, s.byteLength)) }; });
step('scan', () => {
  const n = camp.campioni[0].scans, hash = crypto.createHash('sha256'); let pts = 0, ns = 0n;
  for (let k = 0; k < n; k++) { const t = process.hrtime.bigint(); const r = m.scan(h, 0, k); ns += process.hrtime.bigint() - t; pts += new DataView(r.buffer, r.byteOffset).getUint32(4, true); hash.update(r); }
  return { scans: n, points: pts, us_per_scan: Math.round(Number(ns) / n / 100) / 10, sha256: hash.digest('hex') };
});
step('scan_header', () => { const n = camp.campioni[0].scans, hash = crypto.createHash('sha256'); let b = 0; for (let k = 0; k < n; k += Math.max(1, Math.floor(n / 500))) { const s = m.scan_header(h, 0, k); b += s.length; hash.update(s); } return { bytes: b, sha256: hash.digest('hex') }; });
step('file_header', () => { const s = m.file_header(h, 0); return { bytes: s.length, sha256: sha(s) }; });
step('method', () => { const s = m.method(h, 0); return { bytes: s.length, sha256: sha(s) }; });
step('to_mzml', () => {
  const hash = crypto.createHash('sha256'); let b = 0;
  m.to_mzml(names, sizes, read, 0, (u8) => { b += u8.length; hash.update(u8); }, () => {});
  return { bytes: b, sha256: hash.digest('hex') };
});
out({ step: 'peak', mem: peak });
"""

def run(c, **k):
    print("+", " ".join(c), file=sys.stderr)
    subprocess.run(c, check=True, **k)

def main():
    a = argparse.ArgumentParser()
    a.add_argument("file")
    a.add_argument("--family", default="thermo")
    a.add_argument("--steps", default="samples,index,scan,scan_header,file_header,method,to_mzml")
    a.add_argument("--no-build", action="store_true", help="reuse the module of --keep")
    a.add_argument("--keep", help="folder of the wrapped module (default: a temporary one)")
    o = a.parse_args()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    tmp = o.keep or tempfile.mkdtemp(prefix="bench_raw_")
    os.makedirs(tmp, exist_ok=True)
    if not o.no_build:
        run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", o.family], cwd=root, stdout=sys.stderr)
        wasm = os.path.join(tmp, "in.wasm")
        shutil.copy(os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(root, "target")), "wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm"), wasm)
        run(["wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", os.path.join(tmp, "pkg"), wasm], stdout=sys.stderr, stdin=subprocess.DEVNULL)
        with open(os.path.join(tmp, "pkg/in.js"), "a") as f:  # the generated glue does not export the instance: the memory is read from it
            f.write("\nmodule.exports.__wasm = wasm;\n")
        print("wasm bytes:", os.path.getsize(os.path.join(tmp, "pkg/in_bg.wasm")), file=sys.stderr)
    js = os.path.join(tmp, "bench.js")
    open(js, "w").write(JS)
    run(["node", js, os.path.join(tmp, "pkg/in.js"), os.path.abspath(o.file), o.steps])
    if not o.keep:
        shutil.rmtree(tmp, ignore_errors=True)

main()
