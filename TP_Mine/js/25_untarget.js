// TPMINE-PRIVATE  Untargeted tables: ONE model of the features of a LC-HRMS table (id, m/z, RT, areas per sample, MS2, external annotations with their source).
// The readers (mzMine full table, annotations, FBMN quant + MGF + IIN edges, MS-DIAL, GNPS2 library and network) are Rust: crates/mzlab-feat/src/import, the .wasm «import» of wasm-feat/manifest.json
// (crates/mzlab-feat-import-wasm, docs/agenti/mzfinder-import.md). Here: loading it, the calls (the arguments in, the table as JSON out; no number is calculated here) and the rest of the model
// that does not read files (mzFinder rows) and the external evidence read with LIVELLI. The match with the libraries and the levels are in 26_untarget_match.js, the window in 27_untarget_ui.js.
// The external annotations are an open structure {source, type, score, shared, ppm, name, ik, smiles, ...}: another program (SIRIUS) enters later without changing the model.
// The key of a feature is (polarity, id) plus the SHA-256 of the file of the table it comes from. The rules of the readers are those of Attributio 1.12.0 (readers/mzmine.py, gnps.py, ms2.py), by Federico Cristaudo.
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const LV = () => G.LIVELLI;
  const txt = v => (v == null ? "" : String(v).trim());
  const lc = s => String(s).toLowerCase();

  // ------------------------------------------------------------ the readers: the .wasm «import» (fetched when a table of another program is opened; node tests give it with use())
  /** @type {any} */ let W = null;
  const fail = (key, message) => Object.assign(new Error(message || key), { key });
  /** the module of the readers (the one of wasm-bindgen: add, build, detect, ...) */
  function use(mod) { W = mod; return W; }
  /** loads the .wasm «import» once: manifest.json of wasm-feat/, SHA-256 checked */
  async function ready() {
    if (W) return W;
    const u = p => new URL("wasm-feat/" + p, G.QTOOLS.ctx.base).href, g = (await (await fetch(u("manifest.json"))).json()).import;
    if (!g) throw fail("error.featFailed");
    const bytes = await (await fetch(u(g.wasm))).arrayBuffer(), hex = b => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
    if (g.sha256 && hex(await crypto.subtle.digest("SHA-256", bytes)) !== g.sha256) throw fail("error.featWasmHash");
    const m = await import(u(g.js)); await m.default({ module_or_path: bytes }); return use(m);
  }
  if (typeof process !== "undefined" && process.env && process.env.MZLAB_IMPORT_PKG) use(require(process.env.MZLAB_IMPORT_PKG + "/mzlab_feat_import_wasm.js"));
  const need = () => { if (!W) throw fail("error.featFailed", "the readers of the tables are not loaded (UNT.ready())"); return W; };
  const nan = x => (Number.isNaN(x) ? null : x);
  /** an error of the .wasm is a JSON string {error_key, params: {message}} */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? fail(j.error_key, (j.params && j.params.message) || j.error_key) : e; };

  const num = v => (v == null ? null : nan(need().num(String(v))));
  const formulaMass = f => nan(need().formula_mass(txt(f)));
  const normAdduct = a => need().norm_adduct(txt(a));
  const cleanName = name => need().clean_name(txt(name));
  const isPropagated = n => need().is_propagated(String(n == null ? "" : n));
  /** Does the library structure of a match agree with the precursor of its entry? → {ok: true|false|null (nothing to check), ppm, adduct} */
  const checkStructure = (formula, precursor, adduct, pol, tolPpm = 20) => JSON.parse(need().check_structure(txt(formula), precursor == null ? NaN : +precursor, txt(adduct), pol || 0, tolPpm));
  /** the kind of a file from its columns: mgf, msdial, gnps-library, gnps-network, mzmine-edges, mzmine-quant, mzmine-annotations, mzmine-full or "" */
  const detect = (name, head) => need().detect(String(name || ""), String(head || ""));

  // ------------------------------------------------------------ model
  const polOf = v => { const s = lc(txt(v)); if (/^(pos|positive|\+|1)$/.test(s)) return 1; if (/^(neg|negative|-|-1)$/.test(s)) return -1; return 0; };
  function newTable(program, name, pol) {
    return { program, file: { name: name || "", sha256: "" }, pol: pol || 0, features: [], byId: new Map(), samples: [], edges: [], warnings: [], stats: {} };
  }
  const keyOf = (pol, id) => (pol > 0 ? "POS" : pol < 0 ? "NEG" : "UNK") + ":" + id;
  function addFeature(t, id, mz, rt) {
    const f = { key: keyOf(t.pol, id), id: String(id), pol: t.pol, mz, rt, areas: {}, detected: null, ms2: null, ann: [], group: null, iin: null, component: null, nScans: null, program: t.program };
    t.features.push(f); t.byId.set(f.id, f); return f;
  }

  // ------------------------------------------------------------ mzFinder HR (internal features): the rows of an HR result
  function fromMzfinder(res, o = {}) {
    const t = newTable("mzfinder", o.name || "Tabui", polOf(o.pol)); const files = (res.files || []).map(f => (typeof f === "string" ? f : f.name || f.label || ""));
    t.samples = files;
    for (const r of res.rows || []) {
      const f = addFeature(t, r.id, r.mz, r.rt); (r.series || []).forEach((a, i) => { if (a) f.areas[files[i] || "file" + i] = a; });
      if (r.formula) f.ann.push({ source: "Tabui", type: "formula", score: null, shared: null, ppm: r.ppm == null ? null : r.ppm, name: r.name || r.formula, cleanName: r.name || r.formula, formula: r.formula, ik: "", smiles: "", external: true, analog: false, propagated: false, notes: [] });
      f.nScans = r.ms2_scans || null;
    }
    t.stats.features = t.features.length; return t;
  }

  // ------------------------------------------------------------ mzFinder pipeline (crates/mzlab-feat): the rows of an aligned series {id, mz, rt, areas[], detected[]} (indices of samples), program «mzlab»
  function fromRows(rows, samples, o = {}) {
    const t = newTable("mzlab", o.name || "Tabui pipeline", polOf(o.pol)); t.samples = samples.slice();
    for (const r of rows) {
      const f = addFeature(t, r.id, r.mz, r.rt); r.areas.forEach((a, i) => { if (a > 0) f.areas[samples[i] || "file" + i] = a; });
      f.detected = (r.detected || []).map(i => samples[i] || "file" + i);
    }
    t.stats.features = t.features.length; return t;
  }

  // ------------------------------------------------------------ SHA-256 of the file of the table (hex)
  async function sha256Hex(data) {
    const buf = typeof data === "string" ? new TextEncoder().encode(data) : data;
    const d = await G.crypto.subtle.digest("SHA-256", buf); return [...new Uint8Array(d)].map(b => b.toString(16).padStart(2, "0")).join("");
  }

  // ------------------------------------------------------------ build: from a list of files {name, text, sha256?} → ONE table (the main table + MS2 + annotations + network)
  function build(files, o = {}) {
    const w = need(); w.clear();
    for (const f of files) w.add(f.name, f.sha256 || "", f.text);
    const pol = o.pol != null ? o.pol : polOf(o.polarity);
    let t; try { t = JSON.parse(w.build(pol, o.maxFeatures || 0)); } catch (e) { throw asError(e); }
    t.byId = new Map(t.features.map(f => [f.id, f]));          // by id, as the page looks the features up
    return t;
  }

  // ------------------------------------------------------------ external evidence read with a profile of livelli/1 (profile «attributio»: a library source of mzMine or GNPS2 that passes the gates is level 2a)
  const bestKey = c => (LV() ? LV().ikKey(c) : lc(c.name));
  function assessExternal(f, profile = "attributio") {
    const L = LV(), cands = (f.ann || []).filter(a => !a.propagated && (a.type === "cosine" || a.type === "MQScore")).map(a => ({ name: a.cleanName || a.name, ik: a.ik, ent: a.score, cos: a.score, shared: a.shared, dprec: a.ppm, lib: a.source, nlib: 30, _a: a }));
    const one = c => L.assess({ cands: [c] }, profile).proposed === "2a";
    const valid = cands.filter(one);
    const res = L.assess({ cands: valid.length ? valid : cands }, profile);
    let base = res.proposed === "2a" ? "spectral library" : res.why === "confirm" ? "name conflict" : "none";
    return { level: res.proposed, base, decisive: res.decisive, best: res.best, valid: valid.length, res };
  }
  /** the same library compound matched at two or more RT (beyond the group tolerance) → level 3, «isomers not discriminated» */
  function markIsomers(table, results, profile = "attributio") {
    const tol = LV().thresholds(profile).rt_gruppo_tol_min, by = new Map();
    for (const f of table.features) { const r = results.get(f.key); if (!r || r.level !== "2a" || !r.best) continue; const k = f.pol + "|" + bestKey(r.best); if (!by.has(k)) by.set(k, []); by.get(k).push(f); }
    for (const fs of by.values()) {
      if (fs.length < 2) continue; const rts = fs.map(f => f.rt), span = Math.max(...rts) - Math.min(...rts); if (span <= tol) continue;
      for (const f of fs) { const r = results.get(f.key); r.level = "3"; r.base = "isomers not discriminated"; r.decisive = "The same compound is matched at two or more retention times: the spectrum does not tell the isomers apart."; }
    }
  }
  function assessAllExternal(table, profile = "attributio") {
    const out = new Map(); for (const f of table.features) out.set(f.key, assessExternal(f, profile)); markIsomers(table, out, profile); return out;
  }

  const UNT = { use, ready, num, formulaMass, normAdduct, checkStructure, cleanName, isPropagated, polOf, keyOf, detect, fromMzfinder, fromRows, sha256Hex, build, assessExternal, assessAllExternal };
  G.UNT = UNT;
  if (typeof module !== "undefined") module.exports = UNT;
})();
