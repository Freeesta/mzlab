// wiffToMzml (shell.js) with fake engine and module: how many times the engine parses a pair, when the module probes it, and where a variant or an error goes; argv[2] = shell.js function text file.
const fs = require("fs"), assert = require("assert");
const WEB = process.argv[3];
global.window = {}; global.convName = require(WEB + "/conversione.js").convName;
const NATIVO = require(WEB + "/nativo.js"), CartCarica = require(WEB + "/cartella-carica.js");
global.NATIVO = NATIVO; global.CartCarica = CartCarica;
global.wiffName = (stem, id, several) => several ? `${stem}_${id}.mzML` : `${stem}.mzML`;
global.I18N = { t: k => k, err: j => ({ message: j.error }) };
global.DIAG = { da: () => ({}) };
global.memDesc = () => "";
const held = new Set();      // × on a row of the side list (OPENNOW.hold)
global.OPENNOW = { held: n => held.has(n) };
const rows = [];
global.CONVROWS = { add: (...a) => rows.push(["add", ...a]), finish: (...a) => rows.push(["finish", ...a]), drop: (...a) => rows.push(["drop", ...a]) };
const src = fs.readFileSync(process.argv[2], "utf8");
global.WIFF_WAIT = new Map();
global.ST = { files: [] };
const run = new Function("window", "ST", "WIFF_WAIT", "NATIVO", "CartCarica", "wiffName", "I18N", "DIAG", "memDesc", "CONVROWS", src + "\nreturn wiffToMzml;");

const pairFiles = () => [new File(["w"], "S.wiff"), new File(["s"], "S.wiff.scan")];
const samples2 = [{ id: 1, name: "a", size: 1 }, { id: 2, name: "b", size: 1 }];

async function go({ listed = [], same = [], samples = samples2, nativeError = null, probe = undefined, fam = true, retriable = "" }) {
  const calls = { samples: 0, probe: 0, same: 0, stand: 0, retriable: 0 };
  const rb = {
    nativeSamples: async () => { calls.samples++; if (nativeError) throw nativeError; return { samples }; },
    nativeSame: async (mem, names) => { calls.same++; return same; },
  };
  const M0 = {
    probe: async () => { calls.probe++; if (probe instanceof Error) throw probe; return probe; },
    stand: (nm, mem, i, f) => { calls.stand++; return { name: nm, stood: true }; },
    retriable: () => { calls.retriable++; return retriable; },
  };
  const w = { MZLAB_RUST_READY: Promise.resolve(rb), LETTORI_MOD: probe === undefined ? null : M0, LETTORI_FAM: { sciex: { sonda: fam } } };
  global.window = w; rows.length = 0;
  const wf = run(w, { files: listed }, new Map(), NATIVO, CartCarica, global.wiffName, global.I18N, global.DIAG, global.memDesc, global.CONVROWS);
  const res = await wf(pairFiles(), null, new AbortController().signal);
  return { calls, res, rows: rows.slice() };
}
(async () => {
  const stem = "S";
  // miss with files already listed (one of the two samples is listed): the engine parses the pair once (twice before)
  let r = await go({ listed: [{ name: "a.mzML" }], same: ["a.mzML"], probe: { motivi: [], campioni: [] } });
  assert.equal(r.calls.samples, 1, "one parse of the pair on a miss");
  assert.equal(r.calls.probe, 1);
  assert.deepEqual(r.res.files.map(f => f.name), ["S_1.mzML", "S_2.mzML"]);
  assert.ok(r.res.files.every(f => f.mzlabNat.probed), "the module's answer goes with the samples");
  // a cache hit: the module is not asked, one parse
  r = await go({ listed: [{ name: "a.mzML" }, { name: "b.mzML" }], same: ["a.mzML", "b.mzML"], probe: { motivi: [], campioni: [] } });
  assert.equal(r.calls.probe, 0, "no probe on a cache hit");
  assert.equal(r.calls.samples, 1);
  assert.equal(r.res.files.length, 0);
  // a variant pair goes to the module, even when the engine fails on it
  r = await go({ probe: { motivi: ["variante"], campioni: [{ id: 1, name: "v", size: 2 }] }, nativeError: Object.assign(new Error("x"), { json: { error: "err.file.foreign" } }) });
  assert.equal(r.calls.stand, 1); assert.equal(r.calls.retriable, 0);
  assert.deepEqual(r.res.files.map(f => f.stood), [true]);
  assert.equal(r.res.errs.length, 0);
  // no module: the engine alone, one parse
  r = await go({});
  assert.equal(r.calls.samples, 1); assert.equal(r.res.files.length, 2);
  // an engine error with a module that does not take the pair: retriable decides
  r = await go({ probe: { motivi: [], campioni: [] }, nativeError: Object.assign(new Error("x"), { json: { error: "err.sciex.variant" } }), retriable: "sciex" });
  assert.equal(r.calls.retriable, 1); assert.equal(r.calls.stand, 1);
  // an engine error nobody retries stays on the row
  r = await go({ probe: { motivi: [], campioni: [] }, nativeError: Object.assign(new Error("x"), { json: { error: "err.bad" } }), retriable: "" });
  assert.ok(r.rows.some(x => x[0] === "finish" && x[2] === "fail"));
  // × on the row of the pair while it is parsed: no sample is made, the row ends as cancelled
  held.add("S.wiff"); r = await go({}); held.clear();
  assert.equal(r.res.files.length, 0); assert.ok(r.rows.some(x => x[0] === "finish" && x[1] === "S.wiff" && x[2] === "cancel"));
  console.log("ok");
})().catch(e => { console.error(e); process.exit(1); });
