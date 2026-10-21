// The MSn graph view of Flow (flow/lazy/flow-views-msn.js without the DOM): the modifications and the ions read from a msngrafo/1 graph. Exit code 1 on the first failure
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ window: {}, document: {} });
ctx.FLOW = {};
ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-views-msn.js"), "utf8"), ctx);
const F = ctx.FLOW;
const g = {
  formato: "msngrafo/1",
  nodi: [
    { id: "n0", mz: 305.1, livello: 1, formula: { scelta: "C10H20N2O5S", candidati: [{ formula: "C10H20N2O5S", ppm: 0.4 }] } },
    { id: "n1", mz: 261.1, livello: 2, formula: { scelta: null, candidati: [{ formula: "C9H16N2O3S", ppm: 1.2 }, { formula: "C8H12N4O3S", ppm: 2 }], fonte: "mass" } },
  ],
  archi: [
    { da: "n0", a: "n1", isolamento: true, perdita: { da_massa: 43.98983, formula: "CO2", ppm: 0.8 }, misure: [{ albero: "305@cid35", livello: 2, att: "cid", ce: 35, n_scansioni: 12, rel: 0.5 }, { albero: "305@hcd30", livello: 2, att: "hcd", ce: 30, n_scansioni: 8, rel: 0.25 }] },
    { da: "n1", a: "n9", perdita: {} },
  ],
};
const t = F.readMsnGraph(g);
eq(t.modifications.head.length, 16);
// one row per measure, a measure-less arc keeps one row with empty cells
eq(t.modifications.rows.length, 3);
eq(t.modifications.rows[0], ["n0", "305.1", "C10H20N2O5S", "n1", "261.1", "", "43.98983", "CO2", "0.8", "yes", "305@cid35", "2", "cid", "35", "12", "0.5"]);
eq(t.modifications.rows[2].slice(0, 6), ["n1", "261.1", "", "n9", "", ""]);
eq(t.modifications.rows[2].slice(10), ["", "", "", "", "", ""]);
// a node with no chosen formula has none shown, but its candidates are counted
eq(t.nodes.rows, [["n0", "1", "305.1", "C10H20N2O5S", "0.4", "", "1"], ["n1", "2", "261.1", "", "", "mass", "2"]]);
eq(F.readMsnGraph({}).modifications.rows, []);
console.log("ok");
