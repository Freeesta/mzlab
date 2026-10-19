// Measure only: runs the InChI wasm built by build.sh (next to this copy) on the molfiles written by confronto.py --molfiles DIR.
// Prints name, InChIKey, InChI and the start of AuxInfo (its /N: layer is the canonical numbering of the heavy atoms).
import { readFileSync, readdirSync } from "node:fs";
import { WASI } from "node:wasi";
import path from "node:path";
const dir = process.argv[2];
const wasi = new WASI({ version: "preview1", args: [], env: {} });
const mod = await WebAssembly.compile(readFileSync(new URL("./inchi.wasm", import.meta.url)));
const inst = await WebAssembly.instantiate(mod, wasi.getImportObject());
wasi.initialize(inst);
const ex = inst.exports, enc = new TextEncoder(), dec = new TextDecoder();
for (const f of readdirSync(dir).sort()) {
  const mf = enc.encode(readFileSync(path.join(dir, f), "utf8") + "\0");
  const p = ex.alloc(mf.length);
  new Uint8Array(ex.memory.buffer, p, mf.length).set(mf);
  const r = ex.inchi_from_molfile(p);
  const u = new Uint8Array(ex.memory.buffer, r);
  const out = dec.decode(u.subarray(0, u.indexOf(0))).split("\n");
  console.log(f.replace(".mol", "") + "\t" + out[2] + "\t" + out[0] + "\t" + (out[1] || "").slice(0, 80));
}
