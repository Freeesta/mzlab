// Criterion 7 of the MSn graph v2 on the synthetic 80-ion graph (tests_e2e/fixtures/msngrafo_80.json, made by tools/genera_msngrafo_80.py): in both views every ion has a
// box and no two boxes overlap. The crossings of the tree view against the barycentre order are checked in Rust (tests_vista.rs, 80 ions). Exit code 1 on the first failure.
const fs = require("fs"), path = require("path"), vm = require("vm"), assert = require("assert");
const src = fs.readFileSync(path.resolve(__dirname, "../mzlab/web/msngrafo-vista.js"), "utf8").split("\n").map(l => l.replace(/^export /, "")).join("\n");
const mk = vm.runInContext(`(() => { ${src}\nreturn { layout, visible }; })()`, vm.createContext({ document: {}, DOMParser: class {} }));
const j = JSON.parse(fs.readFileSync(path.resolve(__dirname, "../tests_e2e/fixtures/msngrafo_80.json"), "utf8"));
assert.strictEqual(j.nodi.length, 80);
const o = { t: k => k };
const vis = mk.visible(j, o);
assert.strictEqual(vis.ids.size, 80, "all the ions are visible");

const overlap = (a, b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
for (const mode of ["tree", "radial"]) {
  const L = mk.layout(j, Object.assign({ mode }, o), vis);
  assert.strictEqual(L.mode, mode);
  const ids = [...L.boxes.keys()];
  assert.strictEqual(ids.length, 80, mode + ": a box per ion");
  let clashes = 0;
  for (let i = 0; i < ids.length; i++) for (let k = i + 1; k < ids.length; k++) if (overlap(L.boxes.get(ids[i]), L.boxes.get(ids[k]))) clashes++;
  console.error(`${mode}: ${ids.length} boxes, ${clashes} overlaps`);
  assert.strictEqual(clashes, 0, mode + ": no overlapping boxes");
}

console.log("ok");
