// Warm-up of the folder, engine side (rust-scalda.js), with a fake OPFS archive; argv[2] = folder with the copied module. Exit code 1 on the first failure.
// A file copied by the warm-up must be hidden (marker in "warm") from the moment it is in "files": a reload or a closed tab during the index build left it listed as a session file.
import assert from "assert";
import { pathToFileURL } from "url";
const { createScalda } = await import(pathToFileURL(process.argv[2]).href + "/rust-scalda.js");
const kinds = { files: new Map(), warm: new Map(), idx: new Map() };
const ARCH = {
  ROOM: 3, estimate: async () => ({ quota: 0, usage: 0 }),
  get: async (k, n) => kinds[k].get(n) || null,
  list: async k => [...kinds[k].keys()].map(name => ({ name })),
  put: async (k, n, data) => { const f = new File([data], n); kinds[k].set(n, f); return f; },
  remove: async (k, n) => { kinds[k].delete(n); },
  dropLease: async () => {}, release: () => {},
};
// what lr_boot lists: the files of "files" that no marker of "warm" hides
const listed = async () => { const w = new Set((await ARCH.list("warm")).map(e => e.name.replace(/\.json$/, ""))); return (await ARCH.list("files")).map(e => e.name).filter(n => !w.has(n)); };
let seen = null;
const blob = Object.assign(new Blob(["<mzML/>"]), { lastModified: 5 });
const sc = createScalda({
  ARCH, closeSlot: () => {}, lrFile: new Map(), lrOpen: async () => true, builtFrom: () => true,
  lrPut: async m => { await ARCH.put("files", m.name, m.blob); return { info: JSON.stringify({ size: m.blob.size }) }; },
  lrIndex: async m => { seen = await listed(); await ARCH.put("idx", m.name + ".mzidx", new Uint8Array(4)); return { info: "{}" }; },    // the slow part: a reload here is the failure
});
const r = await sc.lr_warm({ name: "a.mzML", blob });
assert.equal(JSON.parse(r.info).warmed, true);
assert.deepEqual(seen, [], "during the index build the copy is not listed as a session file");
assert.deepEqual(await listed(), [], "after the warm-up it is still hidden");
// a build that fails leaves nothing behind (no file, no marker)
kinds.files.clear(); kinds.warm.clear(); kinds.idx.clear();
const bad = createScalda({ ARCH, closeSlot: () => {}, lrFile: new Map(), lrOpen: async () => true, builtFrom: () => true,
  lrPut: async m => { await ARCH.put("files", m.name, m.blob); return { info: JSON.stringify({ size: 1 }) }; }, lrIndex: async () => { throw new Error("boom"); } });
assert.equal(JSON.parse((await bad.lr_warm({ name: "b.mzML", blob })).info).skipped, "failed");
assert.deepEqual([kinds.files.size, kinds.warm.size], [0, 0]);
console.log("ok");
