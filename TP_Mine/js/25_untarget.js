// TPMINE-PRIVATE  Untargeted tables: ONE model of the features of a LC-HRMS table (id, m/z, RT, areas per sample, MS2, external annotations with their source) and tolerant readers
// that recognise the columns by NAME: mzMine (full table, annotations, FBMN quant + MGF + IIN edges), MS-DIAL (alignment .txt), GNPS2 FBMN (library matches and network), mzFinder HR (internal features).
// Pure functions, no DOM: node tests them (tests/test_untarget.py). The match with the libraries and the levels are in 26_untarget_match.js, the window in 27_untarget_ui.js.
// The external annotations are an open structure {source, type, score, shared, ppm, name, ik, smiles, ...}: another program (SIRIUS) enters later without changing the model.
// The key of a feature is (polarity, id) plus the SHA-256 of the file of the table it comes from. The rules of the readers are those of Attributio 1.12.0 (readers/mzmine.py, gnps.py, ms2.py), by Federico Cristaudo.
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const LV = () => G.LIVELLI;

  // ------------------------------------------------------------ delimited text (streaming over a string; only the wanted columns are kept)
  function splitQuoted(line, d) {
    const out = []; let i = 0, n = line.length;
    while (i <= n) {
      if (line[i] === '"') {
        let j = i + 1, s = "";
        for (;;) { const k = line.indexOf('"', j); if (k < 0) { s += line.slice(j); j = n; break; } if (line[k + 1] === '"') { s += line.slice(j, k + 1); j = k + 2; } else { s += line.slice(j, k); j = k + 1; break; } }
        out.push(s); i = line[j] === d ? j + 1 : n + 1;
      } else { const k = line.indexOf(d, i); if (k < 0) { out.push(line.slice(i)); break; } out.push(line.slice(i, k)); i = k + 1; }
    }
    return out;
  }
  /** header: first row (or the one `findHeader(lines so far)` accepts); onHeader(names) → Set of wanted indices | null; onRow(cells, rowNo). Quoted newlines are joined. */
  function eachRow(text, d, onHeader, onRow, opt = {}) {
    if (text.charCodeAt(0) === 0xFEFF) text = text.slice(1);
    let pos = 0, hdr = null, want = null, no = 0, pend = "";
    const n = text.length;
    while (pos < n) {
      let e = text.indexOf("\n", pos); if (e < 0) e = n;
      let line = text.slice(pos, e); pos = e + 1; if (line.endsWith("\r")) line = line.slice(0, -1);
      if (pend) { line = pend + "\n" + line; pend = ""; }
      if (line.indexOf('"') >= 0 && (line.split('"').length - 1) % 2) { pend = line; continue; }
      if (!line && hdr) continue;
      const cells = line.indexOf('"') >= 0 ? splitQuoted(line, d) : line.split(d);
      if (!hdr) {
        if (opt.isHeader && !opt.isHeader(cells)) continue;
        hdr = cells.map(c => c.trim()); want = onHeader(hdr); continue;
      }
      if (want) { const r = new Array(hdr.length); for (const i of want) r[i] = cells[i]; onRow(r, ++no); } else onRow(cells, ++no);
    }
    if (!hdr) return null;
    return hdr;
  }
  const num = v => { if (v == null) return null; const s = String(v).trim(); if (s === "" || /^(na|nan|null|none|-)$/i.test(s)) return null; const x = Number(s.replace(",", ".")); return Number.isFinite(x) ? x : null; };
  const txt = v => (v == null ? "" : String(v).trim());
  const lc = s => String(s).toLowerCase();
  /** header lookup: names given in order of preference, case-insensitive, exact */
  const colMap = hdr => { const m = new Map(); hdr.forEach((h, i) => { if (!m.has(lc(h))) m.set(lc(h), i); }); return m; };
  const pick = (m, ...names) => { for (const n of names) { const i = m.get(lc(n)); if (i != null) return i; } return -1; };

  // ------------------------------------------------------------ chemistry used by the readers (the formula must explain the precursor of the library entry)
  const MONO = { H: 1.00782503207, C: 12, N: 14.0030740048, O: 15.99491461956, F: 18.99840322, P: 30.97376163, S: 31.972071, Cl: 34.96885268, Br: 78.9183371, I: 126.904473, Na: 22.9897692809, K: 38.96370668, Si: 27.9769265325, B: 11.0093054, Se: 79.9165218, Fe: 55.9349375 };
  const E = 0.00054857990946, PROTON = MONO.H - E;
  function formulaMass(f) {
    const s = txt(f); if (!/^([A-Z][a-z]?\d*)+$/.test(s)) return null;
    let m = 0; for (const [, el, k] of s.matchAll(/([A-Z][a-z]?)(\d*)/g)) { if (!(el in MONO)) return null; m += MONO[el] * (k ? +k : 1); }
    return m;
  }
  const ADDUCTS = {         // m/z of the ion = M + delta (single charge)
    "[M+H]+": PROTON, "[M+Na]+": MONO.Na - E, "[M+K]+": MONO.K - E, "[M+NH4]+": MONO.N + 4 * MONO.H - E, "[M-H2O+H]+": PROTON - (2 * MONO.H + MONO.O),
    "[M-H]-": -PROTON, "[M+Cl]-": MONO.Cl + E, "[M+HCOO]-": MONO.C + MONO.H + 2 * MONO.O + E, "[M+CH3COO]-": 2 * MONO.C + 3 * MONO.H + 2 * MONO.O + E, "[M-H2O-H]-": -PROTON - (2 * MONO.H + MONO.O),
  };
  const COMMON = { 1: ["[M+H]+", "[M+Na]+", "[M+NH4]+", "[M+K]+", "[M-H2O+H]+"], "-1": ["[M-H]-", "[M+Cl]-", "[M+HCOO]-", "[M+CH3COO]-", "[M-H2O-H]-"] };
  function normAdduct(a) {            // «M+H» → «[M+H]+»; unknown stays as written
    let s = txt(a).replace(/\s+/g, ""); if (!s) return "";
    if (!s.startsWith("[")) { const sign = s.endsWith("-") || /^M-H(?!2O)/.test(s) || /^M\+(Cl|HCOO|CH3COO)/.test(s) ? "-" : "+"; s = "[" + s.replace(/[+-]$/, "") + "]" + sign; }
    return s;
  }
  const ppmOf = (a, b) => (b ? (a - b) / b * 1e6 : null);
  /** Does the library structure of a match agree with the precursor of its entry? → {ok: true|false|null (nothing to check), ppm, adduct} */
  function checkStructure(formula, precursor, adduct, pol, tolPpm = 20) {
    const M = formulaMass(formula); if (M == null || !(precursor > 0)) return { ok: null };
    const decl = normAdduct(adduct), list = (decl && decl in ADDUCTS ? [decl] : []).concat(COMMON[pol < 0 ? "-" + 1 : 1].filter(a => a !== decl));
    let best = null;
    for (const a of list) { const p = ppmOf(M + ADDUCTS[a], precursor); if (best == null || Math.abs(p) < Math.abs(best.ppm)) best = { ppm: p, adduct: a }; if (a === decl && Math.abs(p) <= tolPpm) return { ok: true, ppm: p, adduct: a }; }
    return best && Math.abs(best.ppm) <= tolPpm ? { ok: true, ...best } : { ok: false, ...(best || {}) };
  }

  // ------------------------------------------------------------ names of library entries (profile «attributio» of livelli/1: prefixes, «|», suffixes, leading accession, trailing energy)
  function cleanName(name, profile = "attributio") {
    let s = txt(name); if (!s) return "";
    const R = LV() && LV().profile(profile) && LV().profile(profile).pulizia_nomi; if (!R) return s;
    for (const p of R.prefissi) if (lc(s).startsWith(lc(p))) { s = s.slice(p.length); break; }
    const bar = s.indexOf("|"); if (bar >= 0) s = s.slice(0, bar);
    s = s.trim();
    for (const x of R.suffissi) if (lc(s).endsWith(lc(x))) { s = s.slice(0, s.length - x.length); break; }
    s = s.replace(new RegExp(R.accessione_iniziale_regex), "").replace(new RegExp(R.energia_finale_regex), "");
    return s.trim();
  }
  const isPropagated = n => /^candidate\b/i.test(txt(n)) || /\(known structural isomers/i.test(n) || /\(delta mass:/i.test(n);      // GNPS2 propagated names: never a source

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
  const warn = (t, msg) => { if (t.warnings.length < 50) t.warnings.push(msg); };

  // ------------------------------------------------------------ file type recognition (by columns)
  function detect(name, head) {
    const h = String(head || ""), first = h.split(/\r?\n/, 40), n = lc(name || "");
    if (/BEGIN IONS/i.test(h) || /\.mgf$/.test(n)) return "mgf";
    const line = first.find(l => l.trim()) || "", L = lc(line);
    if (first.some(l => /(^|\t)Alignment ID(\t|$)/i.test(l))) return "msdial";
    if (/^"?#scan#"?[\t,]/.test(L) && /mqscore/.test(L)) return "gnps-library";
    if (/cluster index/.test(L) && /component/.test(L)) return "gnps-network";
    if (/(^|,)"?id1"?,"?id2"?,/.test(L) && /edgetype/.test(L)) return "mzmine-edges";
    if (/row id/.test(L) && /row m\/z/.test(L)) return "mzmine-quant";
    if (/(^|,)"?id"?,/.test(L) && /compound_name/.test(L) && /(^|,)"?method"?(,|$)/.test(L)) return "mzmine-annotations";
    if (/(^|,)"?id"?,/.test(L) && /(^|,)"?mz"?,/.test(L) && /(^|,)"?rt"?,/.test(L)) return "mzmine-full";
    return "";
  }

  // ------------------------------------------------------------ mzMine: full table (spectral_db_matches:*, datafile:<F>:area, ion identities)
  function readMzmineFull(text, o = {}) {
    const t = newTable("mzmine", o.name, o.pol), mx = o.maxFeatures || 0;
    let ix = null;
    const hdr = eachRow(text, ",", h => {
      const m = colMap(h); ix = { id: pick(m, "id", "row id"), mz: pick(m, "mz", "row m/z"), rt: pick(m, "rt", "row retention time"), scans: pick(m, "fragment_scans"), grp: pick(m, "feature_group"),
        iin: pick(m, "ion_identities:iin_id"), ion: pick(m, "ion_identities:ion_identities"), nm: pick(m, "ion_identities:neutral_mass"),
        mname: pick(m, "spectral_db_matches:compound_name"), msim: pick(m, "spectral_db_matches:similarity_score"), mn: pick(m, "spectral_db_matches:n_matching_signals"), mexp: pick(m, "spectral_db_matches:explained_intensity_percent"),
        madd: pick(m, "spectral_db_matches:ion_adduct"), mform: pick(m, "spectral_db_matches:mol_formula"), msmi: pick(m, "spectral_db_matches:smiles"), minchi: pick(m, "spectral_db_matches:inchi"), mprec: pick(m, "spectral_db_matches:precursor_mz"),
        mppm: pick(m, "spectral_db_matches:mz_diff_ppm"), mdmz: pick(m, "spectral_db_matches:mz_diff"), mrt: pick(m, "spectral_db_matches:rt_absolute_error") };
      ix.samples = []; ix.states = new Map();
      h.forEach((c, i) => { const mm = /^datafile:(.+):(area|feature_state)$/i.exec(c); if (!mm) return; const sn = mm[1].replace(/\.mzml$/i, ""); if (lc(mm[2]) === "area") ix.samples.push([sn, i]); else ix.states.set(sn, i); });
      if (ix.id < 0 || ix.mz < 0 || ix.rt < 0) throw Object.assign(new Error("mzMine table: columns id, mz and rt not found"), { key: "columns" });
      t.samples = ix.samples.map(s => s[0]);
      return new Set(Object.values(ix).filter(v => typeof v === "number" && v >= 0).concat(ix.samples.map(s => s[1]), [...ix.states.values()]));
    }, r => {
      if (mx && t.features.length >= mx) return;
      const id = txt(r[ix.id]), mz = num(r[ix.mz]), rt = num(r[ix.rt]); if (!id || mz == null || rt == null) return;
      const f = addFeature(t, id, mz, rt);
      for (const [s, i] of ix.samples) { const a = num(r[i]); if (a) f.areas[s] = a; }
      if (ix.states.size) { f.detected = []; for (const [s, i] of ix.states) if (lc(txt(r[i])) === "detected") f.detected.push(s); }
      if (ix.scans >= 0) { const sc = txt(r[ix.scans]); if (sc) f.nScans = sc.split(/[,;\s]+/).filter(Boolean).length; }
      if (ix.grp >= 0 && txt(r[ix.grp])) f.group = txt(r[ix.grp]);
      if (ix.iin >= 0 && txt(r[ix.iin])) f.iin = { id: txt(r[ix.iin]), ions: ix.ion >= 0 ? txt(r[ix.ion]) : "", neutral: ix.nm >= 0 ? num(r[ix.nm]) : null };
      const nm = ix.mname >= 0 ? txt(r[ix.mname]) : "";
      if (nm) {
        const prec = ix.mprec >= 0 ? num(r[ix.mprec]) : null, a = {
          source: "mzMine library", type: "cosine", score: ix.msim >= 0 ? num(r[ix.msim]) : null, shared: ix.mn >= 0 ? num(r[ix.mn]) : null, explained: ix.mexp >= 0 ? num(r[ix.mexp]) : null,
          ppm: ix.mppm >= 0 && num(r[ix.mppm]) != null ? num(r[ix.mppm]) : prec != null ? ppmOf(mz, prec) : null,
          name: nm, cleanName: cleanName(nm), adduct: ix.madd >= 0 ? normAdduct(r[ix.madd]) : "", formula: ix.mform >= 0 ? txt(r[ix.mform]) : "", smiles: ix.msmi >= 0 ? txt(r[ix.msmi]) : "", inchi: ix.minchi >= 0 ? txt(r[ix.minchi]) : "", ik: "",
          precursor: prec, analog: false, propagated: false, notes: [] };
        f.ann.push(a);
      }
    }, { });
    if (!hdr) throw Object.assign(new Error("empty table"), { key: "empty" });
    t.stats.features = t.features.length; t.stats.withMatch = t.features.filter(f => f.ann.length).length;
    return t;
  }

  // ------------------------------------------------------------ mzMine annotations file: completes InChIKey / entry id of the SAME feature and name (else the same InChI); lipid rows are dropped
  function mergeMzmineAnnotations(t, text) {
    let ix = null, dropped = 0, merged = 0, rows = 0;
    eachRow(text, ",", h => {
      const m = colMap(h); ix = { id: pick(m, "id"), name: pick(m, "compound_name"), method: pick(m, "method"), ik: pick(m, "inchi_key", "inchikey"), inchi: pick(m, "inchi"), entry: pick(m, "entry_id"), smiles: pick(m, "smiles", "isomeric_smiles"),
        form: pick(m, "mol_formula"), add: pick(m, "adduct"), prec: pick(m, "precursor_mz") };
      return null;
    }, r => {
      rows++;
      if (/matchedlipid/i.test(txt(r[ix.method]))) { dropped++; return; }
      const f = t.byId.get(txt(r[ix.id])); if (!f) return;
      const nm = txt(r[ix.name]), cn = lc(cleanName(nm)), inchi = ix.inchi >= 0 ? txt(r[ix.inchi]) : "";
      const a = f.ann.find(x => x.source === "mzMine library" && ((nm && (lc(x.name) === lc(nm) || lc(x.cleanName) === cn)) || (inchi && x.inchi === inchi)));
      if (!a) return;
      if (ix.ik >= 0 && txt(r[ix.ik])) a.ik = txt(r[ix.ik]);
      if (ix.entry >= 0 && txt(r[ix.entry])) a.entry = txt(r[ix.entry]);
      if (!a.smiles && ix.smiles >= 0) a.smiles = txt(r[ix.smiles]);
      if (!a.formula && ix.form >= 0) a.formula = txt(r[ix.form]);
      if (!a.adduct && ix.add >= 0) a.adduct = normAdduct(r[ix.add]);
      if (a.precursor == null && ix.prec >= 0) a.precursor = num(r[ix.prec]);
      merged++;
    });
    t.stats.annotationRows = rows; t.stats.lipidRowsDropped = dropped; t.stats.annotationsMerged = merged;
    checkStructures(t);
    return t;
  }
  /** known bug of mzMine exports (keys reused): the structure of an entry counts only if its formula explains the precursor of the entry within 20 ppm */
  function checkStructures(t) {
    let bad = 0;
    for (const f of t.features) for (const a of f.ann) {
      if (a.source !== "mzMine library" || a.structureChecked) continue; a.structureChecked = true;
      const c = checkStructure(a.formula, a.precursor != null ? a.precursor : f.mz, a.adduct, f.pol || t.pol);
      a.structure = c.ok === true ? "ok" : c.ok === false ? "unreliable" : "unverified";
      if (c.ok === false) { a.ik = ""; a.smiles = ""; a.inchi = ""; a.formula = ""; a.notes.push("unreliable library structure: the formula does not explain the precursor"); bad++; }
    }
    t.stats.unreliableStructures = (t.stats.unreliableStructures || 0) + bad;
  }

  // ------------------------------------------------------------ MGF of FBMN: MSLEVEL=2 only, several blocks per FEATURE_ID → the one with most peaks
  function readMgf(text) {
    const out = new Map(); let cur = null, nb = 0, ms1 = 0;
    for (const raw of text.split("\n")) {
      const line = raw.trim(); if (!line) continue;
      if (line === "BEGIN IONS") { cur = { h: {}, mz: [], it: [] }; continue; }
      if (line === "END IONS") {
        if (cur) {
          nb++; const id = cur.h.FEATURE_ID != null ? cur.h.FEATURE_ID : cur.h.SCANS; const lvl = cur.h.MSLEVEL;
          if (lvl != null && String(lvl).trim() !== "2") ms1++;
          else if (id != null && cur.mz.length) { const old = out.get(String(id)); if (!old || cur.mz.length > old.mz.length) out.set(String(id), { mz: cur.mz, it: cur.it, prec: num(String(cur.h.PEPMASS || "").split(/\s+/)[0]), ionmode: cur.h.IONMODE || "", merged: num(cur.h.MERGED_ACROSS_N_SAMPLES), ce: cur.h.COLLISION_ENERGY || "" }); }
        }
        cur = null; continue;
      }
      if (!cur) continue;
      const eq = line.indexOf("=");
      if (eq > 0 && !/^[\d.]+[\s\t]/.test(line)) { cur.h[line.slice(0, eq).trim().toUpperCase()] = line.slice(eq + 1).trim(); continue; }
      const p = line.split(/[\s\t]+/); const a = +p[0], b = +p[1]; if (Number.isFinite(a) && Number.isFinite(b)) { cur.mz.push(a); cur.it.push(b); }
    }
    return { spectra: out, blocks: nb, notMs2: ms1 };
  }
  function attachMs2(t, mgf) {
    let n = 0;
    for (const [id, s] of mgf.spectra) {
      const f = t.byId.get(id); if (!f) continue;
      f.ms2 = { mz: s.mz, it: s.it, prec: s.prec, ce: s.ce }; if (s.merged) f.nScans = Math.max(f.nScans || 0, s.merged);
      if (!t.pol && polOf(s.ionmode)) { t.pol = polOf(s.ionmode); }
      n++;
    }
    t.stats.ms2 = t.features.filter(f => f.ms2).length; t.stats.mgfBlocks = mgf.blocks; t.stats.mgfNotMs2 = mgf.notMs2;
    return n;
  }

  // ------------------------------------------------------------ mzMine FBMN: quant table (areas per sample) and IIN edges
  function readMzmineQuant(text, o = {}) {
    const t = newTable("mzmine", o.name, o.pol); let ix = null;
    eachRow(text, ",", h => {
      const m = colMap(h); ix = { id: pick(m, "row id"), mz: pick(m, "row m/z"), rt: pick(m, "row retention time"), corr: pick(m, "correlation group id"), net: pick(m, "annotation network number"), ion: pick(m, "best ion"), nm: pick(m, "neutral m mass"), s: [] };
      h.forEach((c, i) => { const mm = /^(.*?)\s*Peak area$/i.exec(c); if (mm && mm[1]) ix.s.push([mm[1].replace(/\.mzml$/i, ""), i]); });
      t.samples = ix.s.map(s => s[0]);
      return null;
    }, r => {
      const id = txt(r[ix.id]), mz = num(r[ix.mz]), rt = num(r[ix.rt]); if (!id || mz == null || rt == null) return;
      const f = addFeature(t, id, mz, rt);
      for (const [s, i] of ix.s) { const a = num(r[i]); if (a) f.areas[s] = a; }
      if (ix.corr >= 0 && num(r[ix.corr]) != null && num(r[ix.corr]) >= 0) f.group = txt(r[ix.corr]);
      if (ix.net >= 0 && num(r[ix.net]) != null && num(r[ix.net]) >= 0) f.iin = { id: txt(r[ix.net]), ions: ix.ion >= 0 ? txt(r[ix.ion]) : "", neutral: ix.nm >= 0 ? num(r[ix.nm]) : null };
    });
    t.stats.features = t.features.length; return t;
  }
  function readEdges(t, text) {
    let ix = null, n = 0;
    eachRow(text, ",", h => { const m = colMap(h); ix = { a: pick(m, "id1"), b: pick(m, "id2"), type: pick(m, "edgetype"), sc: pick(m, "score"), an: pick(m, "annotation") }; return null; }, r => {
      const a = txt(r[ix.a]), b = txt(r[ix.b]); if (!a || !b) return;
      t.edges.push({ a, b, type: ix.type >= 0 ? txt(r[ix.type]) : "", score: ix.sc >= 0 ? num(r[ix.sc]) : null, annotation: ix.an >= 0 ? txt(r[ix.an]) : "" }); n++;
    });
    t.stats.edges = n; return n;
  }

  // ------------------------------------------------------------ MS-DIAL alignment (.txt, tab): metadata rows on top, then the header with «Alignment ID»; the MS/MS spectrum is «m/z:int m/z:int ...»
  function parseSpectrumText(s) {
    const mz = [], it = []; for (const p of txt(s).split(/\s+/)) { const k = p.indexOf(":"); if (k <= 0) continue; const a = +p.slice(0, k), b = +p.slice(k + 1); if (Number.isFinite(a) && Number.isFinite(b)) { mz.push(a); it.push(b); } }
    return { mz, it };
  }
  function readMsdial(text, o = {}) {
    const t = newTable("msdial", o.name, o.pol); let ix = null;
    const hdr = eachRow(text, "\t", h => {
      const m = colMap(h); ix = { id: pick(m, "alignment id"), rt: pick(m, "average rt(min)", "average rt (min)", "average rt"), mz: pick(m, "average mz", "average m/z"), name: pick(m, "metabolite name"), add: pick(m, "adduct type"), form: pick(m, "formula"),
        ik: pick(m, "inchikey"), smi: pick(m, "smiles"), dot: pick(m, "dot product"), rev: pick(m, "reverse dot product"), tot: pick(m, "total score"), msms: pick(m, "ms/ms spectrum"), ref: pick(m, "reference m/z"), has: pick(m, "ms/ms assigned"), s: [] };
      if (ix.id < 0 || ix.mz < 0 || ix.rt < 0) throw Object.assign(new Error("MS-DIAL table: columns Alignment ID, Average Rt and Average Mz not found"), { key: "columns" });
      const start = Math.max(ix.msms, pick(m, "spectrum reference file name"), pick(m, "ms1 isotopic spectrum"));
      h.forEach((c, i) => { if (i > start && c) ix.s.push([c, i]); });
      t.samples = ix.s.map(s => s[0]);
      return null;
    }, r => {
      const id = txt(r[ix.id]), mz = num(r[ix.mz]), rt = num(r[ix.rt]); if (!id || mz == null || rt == null) return;
      const f = addFeature(t, id, mz, rt);
      for (const [s, i] of ix.s) { const a = num(r[i]); if (a) f.areas[s] = a; }
      if (ix.msms >= 0) { const sp = parseSpectrumText(r[ix.msms]); if (sp.mz.length) f.ms2 = sp; }
      const nm = ix.name >= 0 ? txt(r[ix.name]) : "";
      if (nm && !/^(unknown|w\/o ms2|low score|no ms2)/i.test(nm) && !/^unknown$/i.test(nm)) {
        const dot = ix.dot >= 0 ? num(r[ix.dot]) : null, ref = ix.ref >= 0 ? num(r[ix.ref]) : null;
        f.ann.push({ source: "MS-DIAL", type: "dot product (0-1000)", score: dot, reverse: ix.rev >= 0 ? num(r[ix.rev]) : null, total: ix.tot >= 0 ? num(r[ix.tot]) : null, shared: null, ppm: ref != null ? ppmOf(mz, ref) : null,
          name: nm, cleanName: cleanName(nm), adduct: ix.add >= 0 ? normAdduct(r[ix.add]) : "", formula: ix.form >= 0 ? txt(r[ix.form]) : "", smiles: ix.smi >= 0 ? txt(r[ix.smi]) : "", ik: ix.ik >= 0 ? txt(r[ix.ik]) : "", analog: false, propagated: false, external: true, notes: [] });
      }
    }, { isHeader: c => c.some(x => lc(x.trim()) === "alignment id") });
    if (!hdr) throw Object.assign(new Error("MS-DIAL header not found"), { key: "columns" });
    t.stats.features = t.features.length; t.stats.ms2 = t.features.filter(f => f.ms2).length; return t;
  }

  // ------------------------------------------------------------ GNPS2 FBMN: library matches (external evidence only) and network components
  const ANALOG_PPM = 15;
  function readGnpsLibrary(t, text) {
    let ix = null, rows = 0, analogs = 0, prop = 0, lost = 0;
    eachRow(text, "\t", h => {
      const m = colMap(h); ix = { id: pick(m, "#scan#", "scan"), name: pick(m, "compound_name"), mq: pick(m, "mqscore"), sh: pick(m, "sharedpeaks"), mzs: pick(m, "specmz"), md: pick(m, "massdiff"), ikp: pick(m, "inchikey-planar"), ik: pick(m, "inchikey"),
        smi: pick(m, "smiles"), inchi: pick(m, "inchi"), lib: pick(m, "libraryname"), add: pick(m, "adduct"), ppm: pick(m, "mzerrorppm") };
      return null;
    }, r => {
      rows++;
      const id = txt(r[ix.id]), f = t.byId.get(id), nm = txt(r[ix.name]), mzs = ix.mzs >= 0 ? num(r[ix.mzs]) : null, md = ix.md >= 0 ? num(r[ix.md]) : null;
      const ppm = mzs && md != null ? md / mzs * 1e6 : ix.ppm >= 0 ? num(r[ix.ppm]) : null, analog = ppm != null && Math.abs(ppm) > ANALOG_PPM;
      if (analog) analogs++;          // every row of the file, as the file reports them
      if (!f) { lost++; return; }
      if (isPropagated(nm)) { prop++; return; }
      f.ann.push({ source: "GNPS2", type: "MQScore", score: ix.mq >= 0 ? num(r[ix.mq]) : null, shared: ix.sh >= 0 ? num(r[ix.sh]) : null, ppm, name: nm, cleanName: cleanName(nm),
        ik: (ix.ikp >= 0 && txt(r[ix.ikp])) || (ix.ik >= 0 ? txt(r[ix.ik]) : ""), smiles: ix.smi >= 0 ? txt(r[ix.smi]) : "", inchi: ix.inchi >= 0 ? txt(r[ix.inchi]) : "", adduct: ix.add >= 0 ? normAdduct(r[ix.add]) : "", library: ix.lib >= 0 ? txt(r[ix.lib]) : "",
        precursor: mzs, analog, propagated: false, notes: analog ? ["analogue: |mass difference| > " + ANALOG_PPM + " ppm"] : [] });
    });
    for (const f of t.features) f.ann.sort((a, b) => (a.source === "GNPS2" && b.source === "GNPS2" ? (a.analog - b.analog) || ((b.score || 0) - (a.score || 0)) : 0));      // per feature the exact ones first, then the score
    t.stats.gnpsRows = rows; t.stats.gnpsAnalogs = analogs; t.stats.gnpsPropagated = prop; t.stats.gnpsUnmatched = lost;
    return t;
  }
  function readGnpsNetwork(t, text) {
    let ix = null, n = 0;
    eachRow(text, "\t", h => { const m = colMap(h); ix = { id: pick(m, "cluster index", "#scan#"), c: pick(m, "component") }; return null; }, r => {
      const f = t.byId.get(txt(r[ix.id])); if (!f || ix.c < 0) return; const c = num(r[ix.c]); f.component = c == null || c < 0 ? null : c; n++;      // −1 = isolated node
    });
    t.stats.networkRows = n; return n;
  }

  // ------------------------------------------------------------ mzFinder HR (internal features): the rows of an HR result
  function fromMzfinder(res, o = {}) {
    const t = newTable("mzfinder", o.name || "mzFinder", polOf(o.pol)); const files = (res.files || []).map(f => (typeof f === "string" ? f : f.name || f.label || ""));
    t.samples = files;
    for (const r of res.rows || []) {
      const f = addFeature(t, r.id, r.mz, r.rt); (r.series || []).forEach((a, i) => { if (a) f.areas[files[i] || "file" + i] = a; });
      if (r.formula) f.ann.push({ source: "mzFinder", type: "formula", score: null, shared: null, ppm: r.ppm == null ? null : r.ppm, name: r.name || r.formula, cleanName: r.name || r.formula, formula: r.formula, ik: "", smiles: "", external: true, analog: false, propagated: false, notes: [] });
      f.nScans = r.ms2_scans || null;
    }
    t.stats.features = t.features.length; return t;
  }

  // ------------------------------------------------------------ mzFinder pipeline (crates/mzlab-feat): the rows of an aligned series {id, mz, rt, areas[], detected[]} (indices of samples), program «mzlab»
  function fromRows(rows, samples, o = {}) {
    const t = newTable("mzlab", o.name || "mzFinder pipeline", polOf(o.pol)); t.samples = samples.slice();
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
  const MAIN = ["mzmine-full", "mzmine-quant", "msdial"];
  function build(files, o = {}) {
    const kinds = files.map(f => ({ ...f, kind: detect(f.name, f.text.slice(0, 20000)) }));
    const main = kinds.find(f => f.kind === "mzmine-full") || kinds.find(f => f.kind === "mzmine-quant") || kinds.find(f => f.kind === "msdial");
    if (!main) throw Object.assign(new Error("No feature table found: expected an mzMine table (id, mz, rt), an FBMN quant table or an MS-DIAL alignment"), { key: "noTable" });
    const pol = o.pol != null ? o.pol : polOf(o.polarity);
    const t = main.kind === "msdial" ? readMsdial(main.text, { name: main.name, pol }) : main.kind === "mzmine-quant" ? readMzmineQuant(main.text, { name: main.name, pol }) : readMzmineFull(main.text, { name: main.name, pol, maxFeatures: o.maxFeatures });
    t.file.sha256 = main.sha256 || ""; t.used = [main.name]; t.ignored = [];
    for (const f of kinds) {
      if (f === main) continue;
      try {
        if (f.kind === "mgf") attachMs2(t, readMgf(f.text));
        else if (f.kind === "mzmine-annotations") mergeMzmineAnnotations(t, f.text);
        else if (f.kind === "mzmine-edges") readEdges(t, f.text);
        else if (f.kind === "gnps-library") readGnpsLibrary(t, f.text);
        else if (f.kind === "gnps-network") readGnpsNetwork(t, f.text);
        else { t.ignored.push(f.name); continue; }
        t.used.push(f.name);
      } catch (e) { warn(t, `${f.name}: ${e.message}`); t.ignored.push(f.name); }
    }
    if (main.kind === "mzmine-full") checkStructures(t);
    if (!t.pol) warn(t, "Polarity not found in the files: choose it before matching.");
    for (const f of t.features) f.key = keyOf(t.pol, f.id);
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

  const UNT = { eachRow, num, formulaMass, normAdduct, checkStructure, cleanName, isPropagated, polOf, keyOf, detect, readMzmineFull, mergeMzmineAnnotations, readMgf, attachMs2, readMzmineQuant, readEdges, readMsdial, parseSpectrumText,
    readGnpsLibrary, readGnpsNetwork, fromMzfinder, fromRows, sha256Hex, build, assessExternal, assessAllExternal, ADDUCTS, ANALOG_PPM, MAIN };
  G.UNT = UNT;
  if (typeof module !== "undefined") module.exports = UNT;
})();
