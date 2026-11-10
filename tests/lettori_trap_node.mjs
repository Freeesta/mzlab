// The readers worker (mzlab/web/lettori-worker.js) after a wasm trap, run by node on copies of the modules; argv[2] = folder with them, argv[3] = the stand-in module's glue.
// Stand-in `agilent` and `thermo` modules trap on demand: a trapped instance is never asked again (a fresh glue loads), a plain error keeps it; the same for the engine's sessions (lettore-esterno.js). Prints "ok".
import assert from "assert";
import { pathToFileURL, fileURLToPath } from "url";
import { createHash } from "crypto";
import { mkdirSync, writeFileSync, readFileSync } from "fs";
const root = process.argv[2], wasm = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]), sha256 = createHash("sha256").update(wasm).digest("hex");
for (const fam of ["agilent", "thermo"]) {
  const modDir = root + "/wasm-lettori/" + fam;
  mkdirSync(modDir, { recursive: true });
  writeFileSync(modDir + "/mzlab_lettori_wasm_bg.wasm", wasm);
  writeFileSync(modDir + "/mzlab_lettori_wasm.js", readFileSync(process.argv[3]));
}
writeFileSync(root + "/wasm-lettori/manifest.json", JSON.stringify({ agilent: { sha256 }, thermo: { sha256 } }));
globalThis.self = globalThis;
globalThis.FileReaderSync = class { readAsArrayBuffer() { return new ArrayBuffer(0); } };
globalThis.fetch = async u => { const b = readFileSync(fileURLToPath(u)); return { json: async () => JSON.parse(b.toString()), arrayBuffer: async () => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) }; };
const answers = [];
globalThis.postMessage = x => answers.push(x);
await import(pathToFileURL(root + "/lettori-worker.js").href);
let id = 0;
const probe = async mode => {
  globalThis.fakeMode = mode; const n = answers.length;
  self.onmessage({ data: { op: "probe", id: ++id, fam: "agilent", members: [{ name: "a.d", blob: new Blob([]) }] } });
  while (answers.length === n) await new Promise(r => setTimeout(r, 1));
  return answers[n];
};
assert.deepStrictEqual((await probe("ok")).info, { instance: 1 });
assert.ok((await probe("error")).error);                       // a plain error from the bridge: the instance stays
assert.deepStrictEqual((await probe("ok")).info, { instance: 1 });
const t = await probe("trap");
assert.ok(t.error && t.error.error_key, JSON.stringify(t));    // the trap is told to the page
assert.deepStrictEqual((await probe("ok")).info, { instance: 2 }); // and the next message runs on a fresh instance
assert.deepStrictEqual((await probe("ok")).info, { instance: 2 });

// the engine's sessions of a `.raw` (lettore-esterno.js): a trap in a call drops the module, the next `prepara` loads a fresh one
const { createEsterno } = await import(pathToFileURL(root + "/lettore-esterno.js").href);
let open = null;
const ext = createEsterno(o => { open = o; }), raw = ["a.raw"], sizes = Float64Array.from([1]), rd = () => new Uint8Array(0);
await ext.prepara(raw);
const before = JSON.parse(open(raw, sizes, rd).samples()).instance;
globalThis.fakeMode = "error";
assert.throws(() => open(raw, sizes, rd).samples());
await ext.prepara(raw); globalThis.fakeMode = "ok";
assert.strictEqual(JSON.parse(open(raw, sizes, rd).samples()).instance, before, "a plain error keeps the module");
globalThis.fakeMode = "trap";
assert.throws(() => open(raw, sizes, rd).samples(), WebAssembly.RuntimeError);
assert.deepStrictEqual(ext.loaded(), [], "the trapped module is dropped");
await ext.prepara(raw); globalThis.fakeMode = "ok";
assert.strictEqual(JSON.parse(open(raw, sizes, rd).samples()).instance, before + 1, "a fresh instance after the trap");
console.log("ok");
