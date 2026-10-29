// mixesResolutions (lr-sessione.js): the entry of a plain file answers for its resolution, only a mixed file asks the engine for its items; argv[2] = folder with the copied module.
import assert from "assert";
const { mixesResolutions } = await import(process.argv[2] + "/lr-sessione.js");

const mk = (entries, items = {}) => {
  const calls = [];
  const rpc = async m => { calls.push(m.name); return { info: JSON.stringify(items[m.name] || [{ hr: !!(entries[m.name] || {}).hr }]) }; };
  return { calls, rpc, entry: n => entries[n] || null };
};
const ok = () => true;

// 30 plain files: no items RPC at all
{
  const names = Array.from({ length: 30 }, (_, i) => `s${i}.mzML`), entries = Object.fromEntries(names.map(n => [n, { hr: false, mixed: [] }]));
  const t = mk(entries);
  assert.equal(await mixesResolutions(t.rpc, names, ok, t.entry), false);
  assert.equal(t.calls.length, 0);
}
// one LR and one HR plain file: still a mix, without RPC
{
  const t = mk({ "a.mzML": { hr: false, mixed: [] }, "b.mzML": { hr: true, mixed: [] } });
  assert.equal(await mixesResolutions(t.rpc, ["a.mzML", "b.mzML"], ok, t.entry), true);
  assert.equal(t.calls.length, 0);
}
// a mixed file asks the engine (its parts can differ from the whole); the plain ones do not
{
  const entries = { "m.mzML": { hr: false, mixed: [{ name: "Full Scan" }, { name: "MS2" }] }, "a.mzML": { hr: false, mixed: [] } };
  const t = mk(entries, { "m.mzML": [{ hr: false }, { hr: true }] });
  assert.equal(await mixesResolutions(t.rpc, ["m.mzML", "a.mzML"], ok, t.entry), true);
  assert.deepEqual(t.calls, ["m.mzML"]);
}
// no entry (or the old call without entries): the engine is asked as before
{
  const t = mk({});
  assert.equal(await mixesResolutions(t.rpc, ["a.mzML", "b.mzML"], ok), false);
  assert.deepEqual(t.calls, ["a.mzML", "b.mzML"]);
}
// names not accepted are skipped
{
  const t = mk({ "a.mzML": { hr: true, mixed: [] } });
  assert.equal(await mixesResolutions(t.rpc, ["a.mzML", "x.txt"], n => n.endsWith(".mzML"), t.entry), false);
  assert.equal(t.calls.length, 0);
}
console.log("ok");
