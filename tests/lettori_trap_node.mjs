// The readers worker (mzlab/web/lettori-worker.js) after a wasm trap, run by node on copies of the modules; argv[2] = folder with them, argv[3] = the stand-in module's glue.
// A stand-in `agilent` module traps on demand: a trapped instance is never asked again (a fresh glue loads), a plain error keeps it. Prints "ok".
import assert from "assert";
import { pathToFileURL, fileURLToPath } from "url";
import { createHash } from "crypto";
import { mkdirSync, writeFileSync, readFileSync } from "fs";
const root = process.argv[2], modDir = root + "/wasm-lettori/agilent", wasm = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
mkdirSync(modDir, { recursive: true });
writeFileSync(modDir + "/mzlab_lettori_wasm_bg.wasm", wasm);
writeFileSync(root + "/wasm-lettori/manifest.json", JSON.stringify({ agilent: { sha256: createHash("sha256").update(wasm).digest("hex") } }));
writeFileSync(modDir + "/mzlab_lettori_wasm.js", readFileSync(process.argv[3]));
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
console.log("ok");
