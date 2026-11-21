// @ts-strict
// TPMINE-PRIVATE  Recipe of the pipeline in the .mzflow (window.PIPEREC): the record of the operation «feature_pipeline» (schema docs/schemi/feature_pipeline-3.json; the reader and the check of
// the record are in mzlab/web/workflow.js) and the comparison of a re-application with the table that was recorded. Pure but for UNT.sha256Hex (a global of the page, injected by the node tests).
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const sha = d => G.UNT.sha256Hex(d);
  const bytesOf = d => (typeof d === "string" ? new TextEncoder().encode(d) : d);
  const verOf = (text, re) => { const m = re.exec(String(text || "")); return m ? +m[1] : null; };

  /** the record of a run. a: { recipe (text), proofs, parent ({formula, mz, rt, minutes, smiles?} | null), out (result of runFeatPipeline), data (PIPEV.readFeat of the table), crate, rules (bytes|text|null), levels (text|null), suspects ({version, sha256}|null) } */
  async function record(a) {
    const rulesText = a.rules == null ? null : typeof a.rules === "string" ? a.rules : new TextDecoder().decode(a.rules);
    const reg = a.out.registry == null ? null : typeof a.out.registry === "string" ? a.out.registry : new TextDecoder().decode(a.out.registry);
    return {
      op: "feature_pipeline", t: new Date().toISOString(), schema: "feature_pipeline/3", crate_version: a.crate, recipe: a.recipe, params: JSON.parse(a.recipe), proofs: a.proofs || null,
      parent: a.parent ? { formula: a.parent.formula, mz: Number.isFinite(a.parent.mz) ? a.parent.mz : null, rt: Number.isFinite(a.parent.rt) ? a.parent.rt : null, minutes: a.parent.minutes.map(m => (Number.isFinite(m) ? m : null)), ...(a.parent.smiles ? { smiles: a.parent.smiles } : {}) } : null,
      inputs: (a.data.meta.inputs || []).map(i => ({ name: i.name, sha256: i.sha256 })),
      table_sha256: await sha(a.out.evid || a.out.table),
      rules: rulesText == null ? null : { name: "regole_tp/3", version: verOf(rulesText, /"version"\s*:\s*(\d+)/), sha256: await sha(bytesOf(rulesText)) },
      levels: a.levels == null ? null : { name: "levels/1", version: verOf(a.levels, /"version"\s*:\s*(\d+)/), sha256: await sha(bytesOf(a.levels)) },
      suspects: a.suspects || null,
      hypotheses: { tried: a.out.tried || 0, found: a.out.found || 0, registry_sha256: reg == null ? null : await sha(bytesOf(reg)) },
      runs: 1,
    };
  }

  /** the operation «samples» of the log (next to «feature_pipeline»): the canonical samples/1 of the run and its SHA-256, both of the Rust (finder-ruoli.js `canonical`) */
  function samplesOp(c) { const d = JSON.parse(c.json); return { schema: d.schema, samples: d.samples, sha256: c.sha256 }; }

  /** the last operation feature_pipeline of the log that can be re-applied (null when none) */
  function lastOp(ops, check) {
    for (let i = (ops || []).length - 1; i >= 0; i--) if (ops[i] && ops[i].op === "feature_pipeline" && !check(ops[i]).length) return ops[i];
    return null;
  }

  /** what a re-application gave against the record: same table (SHA-256), same files, same engine; `lines` in words */
  function compare(op, now) {
    const cur = new Map(now.inputs.map(i => [i.name, i.sha256])), missing = op.inputs.filter(i => cur.get(i.name) !== i.sha256).map(i => i.name);
    // a record of another version of the format has no such field: not comparable, never a crash
    const old = typeof op.table_sha256 !== "string" || typeof now.table_sha256 !== "string", same = !old && now.table_sha256 === op.table_sha256, lines = [];
    if (missing.length) lines.push(`the files are not the ones of the recipe: ${missing.join(", ")}`);
    if (now.crate_version !== op.crate_version) lines.push(`the engine is another version (${now.crate_version}, the recipe was made with ${op.crate_version})`);
    if (old) lines.push("the record has no table hash: it cannot be compared");
    else if (!same) lines.push(`the table differs (${now.table_sha256.slice(0, 12)}, recorded ${op.table_sha256.slice(0, 12)})`);
    return { same, filesSame: !missing.length, crateSame: now.crate_version === op.crate_version, lines };
  }

  G.PIPEREC = { record, lastOp, compare, samplesOp };
  if (typeof module !== "undefined") module.exports = G.PIPEREC;
})();
