// TPMINE-PRIVATE  Untargeted identities: the features of a table that are the same compound, grouped with the evidence of every link. The graph is built by the Python engine of mzFinder
// (tpmine/hr/identity.py, api «identity_graph»: co-eluting, correlated over the shared samples and a mass relation explained by iimn.relations; the ion identities of the mzMine table enter as
// evidence «mzMine IIN»); this file prepares the payload, adds the singletons, takes the level of an identity from its best member and draws the small network. Pure but for the globals
// UNT / UNTM / LIVELLI (node tests inject them) and the `call` of the engine, which is passed in.
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const PROTON = 1.00727646688, MIN_COMMON = 3;
  const OPT = { ppm: 5, r_min: 0.85, apex_scans: 2, cycle_s: 1.0, min_common: MIN_COMMON };
  const fmt = (x, n) => (x == null || !isFinite(x) ? "" : Number(x).toFixed(n));

  /** first block of an InChIKey (14 characters, upper case): the connectivity, whatever the stereochemistry or the protonation */
  const structKey = ik => { const s = String(ik || "").trim().toUpperCase(); return s.length >= 14 && /^[A-Z]{14}/.test(s) ? s.slice(0, 14) : ""; };

  /** the strongest MS2_PEAKS peaks of the fragmentation spectrum of a feature, for the evidence of its links (null without MS2) */
  const MS2_PEAKS = 40;
  function ms2Peaks(f) {
    if (!f.ms2 || !f.ms2.mz || !f.ms2.mz.length) return null;
    const ix = f.ms2.mz.map((_, i) => i).sort((a, b) => f.ms2.it[b] - f.ms2.it[a]).slice(0, MS2_PEAKS);
    return { mz: ix.map(i => f.ms2.mz[i]), it: ix.map(i => f.ms2.it[i]) };
  }

  /** table → payload of the engine. Only the features that can be linked by shape (>= min_common samples) or already carry an ion identity are sent; the others are singletons. */
  function payload(table, o = {}) {
    const opt = { ...OPT, ...(o.opt || {}) }, sidx = new Map(table.samples.map((s, i) => [s, i])), send = [], rest = [];
    for (const f of table.features) {
      const a = []; for (const s in f.areas) { const i = sidx.get(s); if (i != null && f.areas[s] > 0) a.push([i, f.areas[s]]); }
      if (a.length >= opt.min_common || f.iin) send.push({ k: f.key, id: f.id, mz: f.mz, rt: f.rt, a, ms2: ms2Peaks(f), iin: f.iin ? { id: f.iin.id, ions: f.iin.ions || "", neutral: f.iin.neutral } : null }); else rest.push(f);
    }
    return { send, rest, body: { pol: table.pol > 0 ? 1 : -1, samples: table.samples, features: send, edges: table.edges || [], opt } };
  }

  /** the neutral-mass hypothesis of a feature alone: it is read as the protonated (or deprotonated) ion */
  function single(f, pol) {
    const sh = pol > 0 ? PROTON : -PROTON, form = pol > 0 ? "[M+H]+" : "[M-H]-";
    return { id: "", members: [f.key], rt: f.rt, neutral: { mass: +(f.mz - sh).toFixed(5), explained: 1, anchor: f.key, form, from_mzmine: false, roles: { [f.key]: { label: form, kind: "adduct", dmz: 0, ppm: 0 } } }, alternatives: [], edges: [], mzmine_groups: [] };
  }

  /** runs the engine and returns {identities, edges, stats, byKey}; call(fn, payload) → parsed JSON (the engine of mzFinder in its worker) */
  async function compute(table, call, o = {}) {
    if (!table.pol) throw Object.assign(new Error("Choose the polarity (positive or negative) before grouping."), { key: "pol" });
    const p = payload(table, o);
    const g = p.send.length ? await call("identity_graph", p.body) : { identities: [], edges: [], stats: {} };
    const ids = g.identities.concat(p.rest.map(f => single(f, table.pol)));
    ids.sort((a, b) => a.rt - b.rt || (a.members[0] < b.members[0] ? -1 : 1)); ids.forEach((x, i) => { x.id = "ID" + (i + 1); });
    const byKey = new Map(); for (const x of ids) for (const k of x.members) byKey.set(k, x);
    const st = g.stats || {};
    return { identities: ids, edges: g.edges, byKey, stats: { ...st, features: table.features.length, identities: ids.length, grouped: ids.filter(x => x.members.length > 1).length, samples: table.samples.length, opt: p.body.opt },
      warnings: table.samples.length < p.body.opt.min_common ? [`Fewer than ${p.body.opt.min_common} samples: the shape cannot be compared, only the ion identities of the table are used.`] : [] };
  }

  /** level and candidate of every identity = those of its best member (the 2b only if the chemist decided it); returns the identities (same objects) */
  function annotate(graph, table, results, LV = G.LIVELLI, M = G.UNTM) {
    const byKey = new Map(table.features.map(f => [f.key, f]));
    for (const x of graph.identities) {
      let best = null;
      for (const k of x.members) {
        const r = results.get(k); if (!r) continue;
        const lv = M.shown(r), rk = LV.rank(lv);
        if (!best || rk < best.rk || (rk === best.rk && (r.ent ?? -1) > (best.r.ent ?? -1))) best = { k, r, lv, rk };
      }
      const feats = x.members.map(k => byKey.get(k)).filter(Boolean);
      x.feature = null; x.level = best ? best.lv : "5"; x.via = best ? best.k : x.members[0]; x.best = best ? best.r : null; x.decided = !!(best && best.r.decided);
      x.maxArea = Math.max(0, ...feats.map(f => Math.max(0, ...Object.values(f.areas))));
      x.hasMs2 = feats.some(f => f.ms2 && f.ms2.mz.length);
      x.mzRep = byKey.get(x.neutral ? x.neutral.anchor : x.members[0]) || feats[0];
      const keys = new Set(); for (const k of x.members) { const r = results.get(k); const s = r && r.best ? structKey(r.ik) : ""; if (s) keys.add(s); }
      x.structKeys = [...keys]; x.structConflict = keys.size > 1;
      x.external = [...new Set(x.members.flatMap(k => (results.get(k) || { external: [] }).external))];
    }
    return graph.identities;
  }

  /** the label of the link with its evidence («adduct (evidence: ...)», «in-source fragment? (evidence: ...)», «not distinguishable»), from the engine; "" for an isotope or an unexplained step */
  const evidenceText = e => (e.evidence && e.evidence.text) || "";
  const edgeText = e => {
    const base = e.source === "computed"
      ? `${e.label} · Δm/z ${fmt(e.dmz, 4)}${e.relations && e.relations[0] ? ` (${fmt(e.relations[0].ppm, 1)} ppm)` : ""} · r ${fmt(e.r, 2)} over ${e.n_common} samples · ΔRT ${fmt(Math.abs(e.drt) * 60, 1)} s (${fmt(e.dscans, 1)} scans)`
      : `mzMine IIN · ${e.label}${e.neutral ? ` · neutral mass ${fmt(e.neutral, 4)}` : ""}${e.score != null ? ` · score ${e.score}` : ""}${e.note ? e.note : ""}`;
    const ev = evidenceText(e);
    return ev ? `${ev} · ${base}` : base;
  };
  const edgeShort = e => (e.source === "computed" ? e.label.split(" | ")[0].replace(/ \/ \[M[+-]H\][+-]$/, "") : "mzMine IIN");

  /** the neutral-mass hypothesis in words (a hypothesis with its evidence, never a verdict) */
  function hypothesisText(h, table) {
    if (!h) return "";
    const fk = table ? table.features.find(f => f.key === h.anchor) : null;
    return `Neutral mass ${fmt(h.mass, 4)} · ${h.form} of feature ${fk ? fk.id : h.anchor.split(":").pop()} · explains ${h.explained} member${h.explained === 1 ? "" : "s"}${h.from_mzmine ? " · same mass as the mzMine ion identity" : ""}`;
  }

  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  /** small network of one identity as SVG: nodes on a circle (m/z), edges labelled with the relation; solid = adduct, dashed = isotope, dotted = in-source loss, thick = mzMine IIN; colours from CSS variables */
  function networkSvg(x, graph, table, size = 320) {
    const keys = x.members.slice(0, 24), pos = new Map(), cx = size / 2, cy = size / 2, R = keys.length > 1 ? size / 2 - 52 : 0, byKey = new Map(table.features.map(f => [f.key, f]));
    keys.forEach((k, i) => { const a = -Math.PI / 2 + (2 * Math.PI * i) / keys.length; pos.set(k, [cx + R * Math.cos(a), cy + R * Math.sin(a)]); });
    const es = x.edges.map(i => graph.edges[i]).filter(e => pos.has(e.a) && pos.has(e.b));
    const line = e => { const [x1, y1] = pos.get(e.a), [x2, y2] = pos.get(e.b), t = e.source !== "computed" ? "iin" : e.kind;
      return `<g class="idn-e idn-${esc(t)}"><title>${esc(edgeText(e))}</title><line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/><text x="${((x1 + x2) / 2).toFixed(1)}" y="${((y1 + y2) / 2 - 3).toFixed(1)}" text-anchor="middle">${esc(edgeShort(e))}</text></g>`; };
    const node = k => { const [px, py] = pos.get(k), f = byKey.get(k), main = k === x.neutral.anchor;
      return `<g class="idn-n${main ? " idn-main" : ""}"><title>${esc(`feature ${f ? f.id : k} · m/z ${f ? fmt(f.mz, 4) : ""} · RT ${f ? fmt(f.rt, 2) : ""} min`)}</title><circle cx="${px.toFixed(1)}" cy="${py.toFixed(1)}" r="${main ? 8 : 6}"/><text x="${px.toFixed(1)}" y="${(py + (py < cy ? -12 : 20)).toFixed(1)}" text-anchor="middle">${f ? fmt(f.mz, 4) : esc(k)}</text></g>`; };
    return `<svg viewBox="0 0 ${size} ${size}" width="100%" role="img" aria-label="${esc(`Network of identity ${x.id}: ${x.members.length} features and ${es.length} links`)}">${es.map(line).join("")}${keys.map(node).join("")}</svg>`
      + (x.members.length > keys.length ? `<div class="mut">${x.members.length - keys.length} more features not drawn</div>` : "");
  }

  /** positive and negative tables → pairs that can be the same molecule (same neutral mass under the usual adduct pairs, RT within a window); evidence only */
  async function pairPolarity(pos, neg, call, o = {}) {
    const f = t => t.features.map(x => ({ k: x.key, mz: x.mz, rt: x.rt }));
    return call("polarity_pairs", { pos: f(pos), neg: f(neg), ppm: o.ppm ?? 5, rt_min: o.rtMin ?? 0.15 });
  }

  const UNTID = { OPT, MIN_COMMON, structKey, payload, compute, annotate, single, edgeText, evidenceText, edgeShort, hypothesisText, networkSvg, pairPolarity };
  G.UNTID = UNTID;
  if (typeof module !== "undefined") module.exports = UNTID;
})();
