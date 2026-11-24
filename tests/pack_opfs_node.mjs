// Runs mzlab/web/rust-pack.js (the page's pack / unpack code) on the nodejs build of the `pack` .wasm over an in-memory stand-in for OPFS, and prints what it checked.
// See tests/test_parita_pack.py. usage: node pack_opfs_node.mjs <pkg dir> <src dir> <app dir> <lost dir> <cli-lost-out dir> <work dir>
//   src: the campaign; app: its pack made by the command line (1 KiB parts); lost: a copy of app without part 5; cli-lost-out: the command line's unpack of lost.
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const [pkg, src, app, lost, cliLost, work] = process.argv.slice(2);
const here = path.dirname(new URL(import.meta.url).pathname);
fs.writeFileSync(path.join(work, "stub.mjs"), `import { createRequire } from "node:module"; export const load = async () => createRequire(import.meta.url)(${JSON.stringify(path.join(pkg, "mzlab_pack_wasm.js"))});`);
const code = fs.readFileSync(path.join(here, "../mzlab/web/rust-pack.js"), "utf8").replace("./rust-msngrafo.js", pathToFileURL(path.join(work, "stub.mjs")).href);
fs.writeFileSync(path.join(work, "rust-pack.mjs"), code);
const { packFiles, unpackPack } = await import(pathToFileURL(path.join(work, "rust-pack.mjs")).href);

// ---- the stand-in: the parts of the File System API the page code uses, with its errors ----
const dom = (name, msg = name) => Object.assign(new Error(msg), { name });
const okName = n => { if (!n || n === "." || n === ".." || n.includes("/") || n.includes("\\")) throw new TypeError(`bad name ${n}`); };
class Node { constructor() { this.data = new Uint8Array(0); this.locked = false; } }
class FileH {
  constructor(parent, name, node) { this.kind = "file"; this.parent = parent; this.name = name; this.node = node; }
  async getFile() { const n = this.node, size = n.fakeSize ?? n.data.length; return { size, arrayBuffer: async () => n.data.slice().buffer }; }
  async createSyncAccessHandle() {
    const n = this.node;
    if (n.failOpen) throw dom("NoModificationAllowedError");
    if (n.locked) throw dom("NoModificationAllowedError");
    n.locked = true;
    return {
      read: (buf, { at }) => { const k = Math.max(0, Math.min(buf.length, n.data.length - at)); buf.set(n.data.subarray(at, at + k)); return k; },
      write: (buf, { at }) => { const end = at + buf.length; if (end > n.data.length) { const d = new Uint8Array(end); d.set(n.data); n.data = d; } n.data.set(buf, at); return buf.length; },
      getSize: () => n.data.length,
      truncate: k => { const d = new Uint8Array(k); d.set(n.data.subarray(0, k)); n.data = d; },
      flush() {}, close: () => { n.locked = false; },
    };
  }
  async move(dir, name) { okName(name); this.parent.map.delete(this.name); dir.map.set(name, this.node); this.parent = dir; this.name = name; }
}
class Dir {
  constructor() { this.kind = "directory"; this.map = new Map(); }
  async getDirectoryHandle(n, { create = false } = {}) {
    okName(n);
    let e = this.map.get(n);
    if (!e) { if (!create) throw dom("NotFoundError"); e = new Dir(); this.map.set(n, e); }
    if (!(e instanceof Dir)) throw dom("TypeMismatchError");
    return e;
  }
  async getFileHandle(n, { create = false } = {}) {
    okName(n);
    let e = this.map.get(n);
    if (!e) { if (!create) throw dom("NotFoundError"); e = new Node(); this.map.set(n, e); }
    if (e instanceof Dir) throw dom("TypeMismatchError");
    return new FileH(this, n, e);
  }
  async removeEntry(n) {
    okName(n);
    const e = this.map.get(n);
    if (!e) throw dom("NotFoundError");
    if (e instanceof Dir && e.map.size) throw dom("InvalidModificationError");
    if (e instanceof Node && e.locked) throw dom("NoModificationAllowedError");
    this.map.delete(n);
  }
  async *keys() { yield* [...this.map.keys()]; }
}
const fromDisk = (d, into = new Dir()) => {
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    if (e.isDirectory()) fromDisk(path.join(d, e.name), (into.map.set(e.name, new Dir()), into.map.get(e.name)));
    else { const n = new Node(); n.data = new Uint8Array(fs.readFileSync(path.join(d, e.name))); into.map.set(e.name, n); }
  }
  return into;
};
/** Every entry as `d:path` or `f:path:sha-less size:first bytes`: folders (empty ones too) and file contents. */
const listFake = (d, pre = "") => [...d.map].flatMap(([n, e]) => e instanceof Dir ? [`d:${pre}${n}`, ...listFake(e, `${pre}${n}/`)] : [`f:${pre}${n}:${Buffer.from(e.data).toString("base64")}`]).sort();
const listDisk = (d, pre = "") => fs.readdirSync(d, { withFileTypes: true }).flatMap(e => e.isDirectory() ? [`d:${pre}${e.name}`, ...listDisk(path.join(d, e.name), `${pre}${e.name}/`)] : [`f:${pre}${e.name}:${fs.readFileSync(path.join(d, e.name)).toString("base64")}`]).sort();
const inputs = d => { const v = []; const walk = (x, pre) => { for (const [n, e] of x.map) e instanceof Dir ? walk(e, `${pre}${n}/`) : v.push({ path: `${pre}${n}`, handle: new FileH(x, n, e) }); }; walk(d, ""); return v; };
const key = async f => { try { await f(); return "no error"; } catch (e) { try { return JSON.parse(e).error_key; } catch (_) { return `not a module error: ${e}`; } } };
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

const checks = {};
const campaign = fromDisk(src), PART = 1024;

// the same parts and manifest as the command line, no temporary file left behind
let dir = new Dir();
const packed = await packFiles({ files: inputs(campaign), dir, name: "pack", partSize: PART });
checks.same_bytes_as_the_app = same(listFake(dir), listDisk(app));
checks.document = JSON.parse(packed.doc).schema;

// existing parts are not replaced without force, and nothing changes
let before = listFake(dir);
checks.exists_refused = await key(() => packFiles({ files: inputs(campaign), dir, name: "pack", partSize: PART }));
checks.exists_changed_nothing = same(listFake(dir), before);

// a failed forced run (an input that grew while it was read) leaves the older pack and no temporary file
const grown = fromDisk(src), first = inputs(grown)[0];
first.handle.node.fakeSize = first.handle.node.data.length + 7;
checks.failure_key = await key(() => packFiles({ files: inputs(grown), dir, name: "pack", partSize: PART, force: true }));
checks.failure_kept_the_old_pack = same(listFake(dir), before);

// force replaces, and the parts of the older, longer pack go
await packFiles({ files: inputs(campaign), dir, name: "pack", partSize: 4096, force: true });
const left = [...dir.map.keys()].sort();
checks.force_replaced = left.filter(n => n.startsWith("pack.part")).length === JSON.parse(Buffer.from(dir.map.get("pack.pack.json").data).toString()).parts.length && left.every(n => n.startsWith("pack."));

// a dry run is refused for the names a real run refuses, and touches nothing
for (const [tag, names] of [["case", ["A.txt", "a.txt"]], ["folder", ["a", "A/b"]], ["colon", ["c:x.txt"]]]) {
  const d = new Dir(), root = new Dir(), fs_ = names.map(n => { const node = new Node(); node.data = new Uint8Array(1); return { path: n, handle: new FileH(root, n.replace("/", "_"), node) }; });
  checks[`dry_run_${tag}`] = await key(() => packFiles({ files: fs_, dir: d, name: "pack", partSize: PART, dryRun: true }));
  checks[`dry_run_${tag}_real`] = await key(() => packFiles({ files: fs_, dir: d, name: "pack", partSize: PART }));
  checks[`dry_run_${tag}_empty`] = d.map.size === 0;
}

// a manifest the module refuses creates no folder and no file, and is its error key, not a TypeError
const manifest = fs.readFileSync(path.join(app, "pack.pack.json"), "utf8");
for (const [tag, bad] of [["stream", "a/b:stream"], ["dotdot", "../x"], ["abs", "/abs"]]) {
  const out = new Dir();
  checks[`bad_manifest_${tag}`] = await key(() => unpackPack({ manifest: manifest.replace("camp/s1.D/AcqData/MSScan.bin", bad), dir: fromDisk(app), out }));
  checks[`bad_manifest_${tag}_empty`] = out.map.size === 0;
}

// an intact unpack gives the campaign back; with a part missing it makes the command line's tree, and no folder for a file that was not written
const out = new Dir();
const done = await unpackPack({ manifest, dir: fromDisk(app), out });
checks.unpack_round_trip = same(listFake(out), listDisk(src)) && JSON.parse(done).schema;
const lostOut = new Dir();
await unpackPack({ manifest, dir: fromDisk(lost), out: lostOut });
checks.lost_part_same_tree_as_the_app = same(listFake(lostOut), listDisk(cliLost));
checks.nothing_left_over = !listFake(out).concat(listFake(lostOut)).some(e => e.includes(".mzpack-tmp"));

// a file where the manifest wants a folder is the module's `err.run.output`, and nothing is left behind
{
  const blocked = new Dir(), node = new Node();
  blocked.map.set("camp", node);
  checks.file_where_a_folder_is_wanted = await key(() => unpackPack({ manifest, dir: fromDisk(app), out: blocked }));
  checks.file_where_a_folder_is_wanted_left = same(listFake(blocked), ["f:camp:"]);
}

// an existing target is refused without force and nothing is created
const again = fromDisk(src);
checks.unpack_exists = await key(() => unpackPack({ manifest, dir: fromDisk(app), out: again }));
process.stdout.write(JSON.stringify(checks));
