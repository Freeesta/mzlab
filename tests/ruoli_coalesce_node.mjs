// RUOLI.coalesce (ruoli-carica.js): the table of a loading repaints once per frame and asks the proposals (`campioni/2 proponi`) once per batch; argv[2] = folder with the copied module.
import assert from "assert";
import fs from "fs";
import vm from "vm";
const posts = [];
const ctx = {
  I18N: { t: k => k }, EH: s => s, darkTime: x => x, CAMPIONI: { apply() {} },
  fetch: async (u, o) => ({ json: async () => { const b = JSON.parse(o.body); posts.push(b.files.map(f => f.name)); return { campioni: b.files.map(f => ({ file: f.name, tipo: "sample", tempo_min: 5 })) }; } }),
};
vm.createContext(ctx);
const RUOLI = vm.runInContext(fs.readFileSync(process.argv[2] + "/ruoli-carica.js", "utf8") + "\n;RUOLI", ctx);

const mkRaf = () => { const q = new Map(); let n = 0; return { raf: f => { q.set(++n, f); return n; }, caf: h => q.delete(h), frame: () => { const l = [...q.values()]; q.clear(); l.forEach(f => f()); }, size: () => q.size }; };
const files = n => Array.from({ length: n }, (_, i) => ({ name: `s${i}.mzML`, kind: "full" }));

// a merge outside a loading paints and proposes at once: one post per merge (the old behaviour of every merge)
{
  posts.length = 0; let paints = 0; const ST = []; const m = RUOLI.coalesce(() => { paints++; }, () => RUOLI.propose(() => ST));
  for (const f of files(30)) { ST.push(f); await m.flush(true); }
  assert.equal(paints, 30); assert.equal(posts.length, 30);
}
// 30 merges of a loading: frames repaint, the end of the loading flushes once
{
  posts.length = 0; let paints = 0; const ST = [], r = mkRaf(); const m = RUOLI.coalesce(() => { paints++; }, () => RUOLI.propose(() => ST), r.raf, r.caf);
  for (const f of files(30)) { ST.push(f); m.soon(); }
  assert.equal(r.size(), 1, "one frame asked for the 30 merges");
  r.frame(); assert.equal(paints, 1);
  for (const f of files(5).map(f => ({ ...f, name: "t" + f.name }))) { ST.push(f); m.soon(); }
  assert.equal(posts.length, 0, "no proposal while the files arrive");
  await m.flush();
  assert.equal(r.size(), 0, "the pending frame was cancelled");
  assert.equal(paints, 2); assert.equal(posts.length, 1); assert.equal(posts[0].length, 35);
  assert.ok(ST.every(f => f.prop === 1 && f.role === "sample" && f.time === 5), "every file has its proposal");
  assert.equal(await m.flush(), undefined, "a second flush with nothing merged does nothing");
  assert.equal(paints, 2);
}
// a hidden tab (no frame ever): the flush paints the final table
{
  let paints = 0; const ST = []; const m = RUOLI.coalesce(() => { paints++; }, () => RUOLI.propose(() => ST), () => 1, () => {});
  ST.push(...files(3)); m.soon(); assert.equal(paints, 0); await m.flush(); assert.equal(paints, 1);
}
console.log("ok");
