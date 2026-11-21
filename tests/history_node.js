// Node test of the pure undo/redo stack in mzlab/web/history.js (HistoryStack): order, filter by tab, limits, redo cut by a new action.
const fs = require("fs"), vm = require("vm"), assert = require("assert");
const ctx = vm.createContext({});
vm.runInContext(fs.readFileSync(__dirname + "/../mzlab/web/history.js", "utf8"), ctx);
const P = vm.runInContext("HistoryStack", ctx);
const mk = (n, tab, w) => ({ tab, w, before: "x".repeat(n || 1), after: null });
const st = (p, e, s) => { p.log.push([e.w, s]); return "now-" + e.w; };
let p = new P(); p.log = [];
["a", "b", "c"].forEach(w => p.push(mk(1, "full", w)));
assert.ok(p.v.every((e, i) => i === 0 || e.n > p.v[i - 1].n), "increasing order");
let e = p.undo(() => true, (e, s) => st(p, e, s)); assert.strictEqual(e.w, "c"); assert.strictEqual(e.after, "now-c", "state replaced is kept for redo");
// filter by tab: the entries of another tab wait
p = new P(); p.log = []; p.push(mk(1, "full", "f1")); p.push(mk(1, "mrm", "m1")); p.push(mk(1, "full", "f2"));
assert.strictEqual(p.undo(x => x.tab === "mrm", (e, s) => st(p, e, s)).w, "m1");
assert.strictEqual(p.undo(x => x.tab === "mrm", () => 0), null, "nothing left in that tab");
assert.strictEqual(p.v.length, 2);
// redo and its cut
e = p.undo(x => x.tab === "full", (e, s) => st(p, e, s)); assert.strictEqual(e.w, "f2"); assert.strictEqual(p.r.length, 2);
assert.strictEqual(p.redo(x => x.tab === "full", (e, s) => st(p, e, s)).w, "f2"); assert.strictEqual(p.v[p.v.length - 1].w, "f2");
p.undo(() => true, () => 0); p.push(mk(1, "full", "new")); assert.strictEqual(p.r.length, 0, "a new action cuts the redo");
assert.strictEqual(p.redo(() => true, () => 0), null);
// limits: 100 entries and ~2 MB, the oldest go
p = new P(); for (let i = 0; i < 150; i++) p.push(mk(1, "full", "e" + i));
assert.strictEqual(p.v.length, 100); assert.strictEqual(p.v[0].w, "e50");
p = new P(); for (let i = 0; i < 10; i++) p.push(mk(5e5, "full", "g" + i));
assert.ok(p.bytes <= 2e6 && p.v.length === 4 && p.v[0].w === "g6", `size cap: ${p.v.length} ${p.bytes}`);
p.clear(); assert.strictEqual(p.v.length + p.r.length + p.bytes, 0);
console.log("ok");
