// TPMINE-PRIVATE  Recipe of the pipeline in the .mzflow (window.PIPEREC): the record of the operation «feature_pipeline» (schema docs/schemi/feature_pipeline-1.json; the reader and the check of
// the record are in mzlab/web/workflow.js) and the comparison of a re-application with the table that was recorded. Pure but for UNT.sha256Hex (a global of the page, injected by the node tests).
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const sha = d => G.UNT.sha256Hex(d);
  const bytesOf = d => (typeof d === "string" ? new TextEncoder().encode(d) : d);
  const verOf = (text, re) => { const m = re.exec(String(text || "")); return m ? +m[1] : null; };

  /** the record of a run. a: { recipe (text), proofs, parent ({formula, mz, rt, minutes} | null), out (result of runFeatPipeline), data (PIPEV.readFeat of the table), crate, rules (bytes|text|null), levels (text|null), suspects ({versione, sha256}|null) } */
  async function record(a) {
    const rulesText = a.rules == null ? null : typeof a.rules === "string" ? a.rules : new TextDecoder().decode(a.rules);
    const reg = a.out.registry == null ? null : typeof a.out.registry === "string" ? a.out.registry : new TextDecoder().decode(a.out.registry);
    return {
      op: "feature_pipeline", t: new Date().toISOString(), schema: "feature_pipeline/1", crate_version: a.crate, recipe: a.recipe, parametri: JSON.parse(a.recipe), prove: a.proofs || null,
      genitore: a.parent ? { formula: a.parent.formula, mz: Number.isFinite(a.parent.mz) ? a.parent.mz : null, rt: Number.isFinite(a.parent.rt) ? a.parent.rt : null, minuti: a.parent.minuti.map(m => (Number.isFinite(m) ? m : null)) } : null,
      ingressi: (a.data.meta.inputs || []).map(i => ({ nome: i.name, sha256: i.sha256 })),
      tabella_sha256: await sha(a.out.evid || a.out.table),
      regole: rulesText == null ? null : { nome: "regole_tp/1", versione: verOf(rulesText, /"version"\s*:\s*(\d+)/), sha256: await sha(bytesOf(rulesText)) },
      livelli: a.levels == null ? null : { nome: "livelli/1", versione: verOf(a.levels, /"versione"\s*:\s*(\d+)/), sha256: await sha(bytesOf(a.levels)) },
      sospetti: a.suspects || null,
      ipotesi: { cercate: a.out.tried || 0, trovate: a.out.found || 0, registro_sha256: reg == null ? null : await sha(bytesOf(reg)) },
      giri: 1,
    };
  }

  /** the last operation feature_pipeline of the log that can be re-applied (null when none) */
  function lastOp(ops, check) {
    for (let i = (ops || []).length - 1; i >= 0; i--) if (ops[i] && ops[i].op === "feature_pipeline" && !check(ops[i]).length) return ops[i];
    return null;
  }

  /** what a re-application gave against the record: same table (SHA-256), same files, same engine; `lines` in words */
  function compare(op, now) {
    const cur = new Map(now.ingressi.map(i => [i.nome, i.sha256])), missing = op.ingressi.filter(i => cur.get(i.nome) !== i.sha256).map(i => i.nome);
    const same = now.tabella_sha256 === op.tabella_sha256, lines = [];
    if (missing.length) lines.push(`the files are not the ones of the recipe: ${missing.join(", ")}`);
    if (now.crate_version !== op.crate_version) lines.push(`the engine is another version (${now.crate_version}, the recipe was made with ${op.crate_version})`);
    if (!same) lines.push(`the table differs (${now.tabella_sha256.slice(0, 12)}, recorded ${op.tabella_sha256.slice(0, 12)})`);
    return { same, filesSame: !missing.length, crateSame: now.crate_version === op.crate_version, lines };
  }

  G.PIPEREC = { record, lastOp, compare };
  if (typeof module !== "undefined") module.exports = G.PIPEREC;
})();
