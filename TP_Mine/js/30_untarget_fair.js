// @ts-strict
// TPMINE-PRIVATE  Open-data exports of an untargeted table: GNPS2 (FBMN / IIMN: the quantification table in the mzMine format + the MGF with FEATURE_ID + the IIN edges) and SIRIUS 6 (.ms).
// Files only, nothing is sent: the user uploads them where they choose. Spectra and areas, never annotations or names (the programs of the destination do their own search). Pure functions (node tests them,
// reading the files back with the readers of 25_untarget.js) + the buttons added to the export bar of 28_untarget_export.js.
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const q = v => { const s = v == null ? "" : String(v); return /[,"\r\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s; };
  const area = v => (v == null ? 0 : +(+v).toPrecision(8));

  /** quantification table in the mzMine format that FBMN reads: row ID, row m/z, row retention time (min), the ion-identity columns (-1 = none) and «<sample>.mzML Peak area» */
  function quant(table, feats) {
    const head = ["row ID", "row m/z", "row retention time", "correlation group ID", "annotation network number", "best ion", "neutral M mass"].concat(table.samples.map(s => s + ".mzML Peak area"));
    const rows = feats.map(f => [f.id, +f.mz.toFixed(5), +f.rt.toFixed(4), f.group != null ? f.group : -1, f.iin ? f.iin.id : -1, f.iin ? f.iin.ions || "" : "", f.iin && f.iin.neutral != null ? f.iin.neutral : ""].concat(table.samples.map(s => area(f.areas[s]))));
    return [head].concat(rows).map(r => r.map(q).join(",")).join("\n") + "\n";
  }

  /** MGF with the MS2 of the features that have one, one block per FEATURE_ID, as the FBMN workflow expects (FEATURE_ID links it to the table); no names, no levels */
  function mgf(table, feats) {
    const out = [];
    for (const f of feats) {
      if (!f.ms2 || !f.ms2.mz.length) continue;
      out.push("BEGIN IONS", `FEATURE_ID=${f.id}`, `PEPMASS=${(f.ms2.prec > 0 ? f.ms2.prec : f.mz).toFixed(4)}`, `SCANS=${f.id}`, `RTINSECONDS=${(f.rt * 60).toFixed(2)}`, `CHARGE=${f.pol < 0 ? "1-" : "1+"}`, "MSLEVEL=2", `IONMODE=${f.pol < 0 ? "Negative" : "Positive"}`);
      if (f.nScans > 0) out.push(`MERGED_ACROSS_N_SAMPLES=${f.nScans}`);
      for (let i = 0; i < f.ms2.mz.length; i++) out.push(`${(+f.ms2.mz[i]).toFixed(5)} ${(+f.ms2.it[i]).toFixed(1)}`);
      out.push("END IONS");
    }
    return out.length ? out.join("\n") + "\n" : "";
  }

  /** ion-identity edges (IIN) among these features: ID1,ID2,EdgeType,Score,Annotation; "" if the table has none */
  function edges(table, feats) {
    const ids = new Set(feats.map(f => String(f.id))), E = (table.edges || []).filter(e => ids.has(String(e.a)) && ids.has(String(e.b)) && !/^cosine$/i.test(e.type || ""));
    if (!E.length) return "";
    return ["ID1,ID2,EdgeType,Score,Annotation"].concat(E.map(e => [e.a, e.b, e.type || "", e.score == null ? "" : e.score, e.annotation || ""].map(q).join(","))).join("\n") + "\n";
  }

  /** the SIRIUS compounds of the features that have an MS2: name = feature key, no formula (SIRIUS finds it), the isotope pattern of the MS1 when the table has it */
  function sirius(table, feats) {
    return G.FAIR.siriusMs(feats.map(f => ({ name: `feature_${f.key || f.id}`.replace(/:/g, "_"), mass: f.ms2 && f.ms2.prec > 0 ? f.ms2.prec : f.mz, pol: f.pol, rt: f.rt, ms1: f.ms1 || null, ms2: f.ms2 ? [{ mz: f.ms2.mz, it: f.ms2.it, ce: null }] : [] })));
  }

  /** the files of the GNPS2 package: quant + MGF (+ edges when the table has them) */
  function gnps(table, feats) {
    const files = [["quant.csv", quant(table, feats)], ["ms2.mgf", mgf(table, feats)]], e = edges(table, feats);
    if (e) files.push(["iin_edges.csv", e]);
    return files;
  }

  const buttons = () => ` <button type="button" id="unt-x-gnps" title="Quantification table + MGF (+ IIN edges) in a .zip, for FBMN / IIMN on GNPS2. Files only: nothing is sent.">GNPS2 (FBMN)</button><button type="button" id="unt-x-sirius" title="SIRIUS 6 .ms file: MS2 (and MS1) of the features. Files only: nothing is sent.">SIRIUS (.ms)</button>`;
  function bind(dlg, pick) {
    const stem = s => G.UNTX.base(s.table).replace(/^untargeted_/, "");
    dlg.querySelector("#unt-x-gnps").onclick = () => {
      const { s, feats } = pick(), enc = new TextEncoder();
      G.dl("gnps2_" + stem(s) + ".zip", G.zipStore(gnps(s.table, feats).map(([n, t]) => [n, enc.encode(t)])), "application/zip");
    };
    dlg.querySelector("#unt-x-sirius").onclick = () => { const { s, feats } = pick(); G.dl("sirius_" + stem(s) + ".ms", sirius(s.table, feats), "text/plain;charset=utf-8"); };
  }

  G.UNTF = { quant, mgf, edges, sirius, gnps, buttons, bind };
  if (typeof module !== "undefined") module.exports = G.UNTF;
})();
