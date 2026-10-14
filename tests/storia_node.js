// Node test of the pure undo/redo stack in mzlab/web/storia.js (StoriaPila): order, filter by tab, limits, redo cut by a new action.
const fs = require("fs"), vm = require("vm"), assert = require("assert");
const ctx = vm.createContext({});
vm.runInContext(fs.readFileSync(__dirname + "/../mzlab/web/storia.js", "utf8"), ctx);
const P = vm.runInContext("StoriaPila", ctx);
const mk = (n, tab, w) => ({ tab, w, prima: "x".repeat(n || 1), dopo: null });
const st = (p, e, s) => { p.log.push([e.w, s]); return "now-" + e.w; };
let p = new P(); p.log = [];
["a", "b", "c"].forEach(w => p.push(mk(1, "full", w)));
assert.ok(p.v.every((e, i) => i === 0 || e.n > p.v[i - 1].n), "increasing order");
let e = p.annulla(() => true, (e, s) => st(p, e, s)); assert.strictEqual(e.w, "c"); assert.strictEqual(e.dopo, "now-c", "state replaced is kept for redo");
// filter by tab: the entries of another tab wait
p = new P(); p.log = []; p.push(mk(1, "full", "f1")); p.push(mk(1, "mrm", "m1")); p.push(mk(1, "full", "f2"));
assert.strictEqual(p.annulla(x => x.tab === "mrm", (e, s) => st(p, e, s)).w, "m1");
assert.strictEqual(p.annulla(x => x.tab === "mrm", () => 0), null, "nothing left in that tab");
assert.strictEqual(p.v.length, 2);
// redo and its cut
e = p.annulla(x => x.tab === "full", (e, s) => st(p, e, s)); assert.strictEqual(e.w, "f2"); assert.strictEqual(p.r.length, 2);
assert.strictEqual(p.ripeti(x => x.tab === "full", (e, s) => st(p, e, s)).w, "f2"); assert.strictEqual(p.v[p.v.length - 1].w, "f2");
p.annulla(() => true, () => 0); p.push(mk(1, "full", "new")); assert.strictEqual(p.r.length, 0, "a new action cuts the redo");
assert.strictEqual(p.ripeti(() => true, () => 0), null);
// limits: 100 entries and ~2 MB, the oldest go
p = new P(); for (let i = 0; i < 150; i++) p.push(mk(1, "full", "e" + i));
assert.strictEqual(p.v.length, 100); assert.strictEqual(p.v[0].w, "e50");
p = new P(); for (let i = 0; i < 10; i++) p.push(mk(5e5, "full", "g" + i));
assert.ok(p.bytes <= 2e6 && p.v.length === 4 && p.v[0].w === "g6", `size cap: ${p.v.length} ${p.bytes}`);
p.svuota(); assert.strictEqual(p.v.length + p.r.length + p.bytes, 0);
console.log("ok");
