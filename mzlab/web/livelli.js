"use strict";
// The confidence levels of the identifications (Schymanski et al. 2014; MSI 2007): ONE module used by «Cos'è?», «Identifica tutte le MS2» and mzFinder (same words, same colours).
// The rules and the thresholds are in the data file livelli-1.js (format «livelli/1»); here only the cascade that reads them. Always «candidate, level N, evidence», never «it is X».
// Every result has two distinct fields: `proposed` (what the programme computes) and `decided` (empty until the chemist decides; 2b can only be written there).
// The user's thresholds are in localStorage `qqq.lib.soglie` and in the workflow file. Pure functions: no DOM, so that node can test them (tests/test_livelli.py).
const LIVELLI = (() => {
  const D = (typeof window !== "undefined" ? window : globalThis).LIVELLI_DATI;
  const KEY = "qqq.lib.soglie", ORDER = D.livelli;
  const LEGACY = { ent: "entropia_min", shared: "picchi_min", ppm: "precursore_ppm_max", ent2: "entropia_provvisorio" };      // the keys of the first version of the thresholds
  const t = (k, p) => (typeof I18N !== "undefined" ? I18N.t(k, p) : k);
  const read = () => { try { return JSON.parse(localStorage.getItem(KEY) || "{}") || {}; } catch (e) { return {}; } };
  const write = o => { try { localStorage.setItem(KEY, JSON.stringify(o)); } catch (e) { /* not available */ } };

  // ------------------------------------------------------------ profiles and thresholds
  const names = (all = false) => Object.keys(D.profili).filter(n => all || D.profili[n].visibile);
  function profileName(st = read()) { return D.profili[st.profilo] ? st.profilo : D.predefinito; }
  function profile(name) { return D.profili[name || profileName()]; }
  function thresholds(name, st = read()) {            // data file < old flat keys < the user's values of this profile
    const n = name || profileName(st), base = { ...D.profili[n].soglie }, mine = (st[n] && typeof st[n] === "object") ? st[n] : {};
    if (n === D.predefinito) for (const [o, k] of Object.entries(LEGACY)) if (typeof st[o] === "number") base[k] = st[o];
    for (const [k, v] of Object.entries(mine)) if (k in base && typeof v === "number" && v >= 0) base[k] = v;
    return base;
  }
  function setThreshold(key, v, name) {
    const st = read(), n = name || profileName(st); if (!(key in D.profili[n].soglie) || !(v >= 0)) return false;
    st[n] = { ...(st[n] || {}), [key]: v }; write(st); return true;
  }
  function reset() { write({}); }
  const exportState = () => ({ formato: D.formato, profilo: profileName(), soglie: thresholds() });          // what the workflow file keeps
  function importState(o) {                            // accepts the first format ({ent, shared, ppm, ent2}) and this one
    if (!o || typeof o !== "object") return;
    if (o.formato === D.formato && o.soglie) { const n = D.profili[o.profilo] ? o.profilo : D.predefinito; write({ profilo: n, [n]: o.soglie }); }
    else write(o);
  }

  // ------------------------------------------------------------ the gates
  const num = (v, d) => (v == null || !isFinite(v) ? "–" : Number(v).toLocaleString(typeof I18N !== "undefined" && I18N.lang === "en" ? "en-US" : "it-IT", { maximumFractionDigits: d, minimumFractionDigits: d }));
  const dec = g => (g.id === "score" || g.id === "distinct" || g.id === "stdscore" ? 2 : g.unita === "ppm" || g.unita === "min" ? 1 : 0);
  function evalGate(g, vals, T) {
    let thr = T[g.soglia]; const r = g.ridotta;
    if (r && vals[r.se_campo] != null && vals[r.se_campo] < r.minore_di) thr = T[r.soglia];
    let v = vals[g.campo]; const known = v != null && !Number.isNaN(v);
    if (known && g.assoluto) v = Math.abs(v);
    const ok = known && (g.op === "<=" ? v <= thr : v >= thr), dm = dec(g), inf = v === Infinity;
    const label = t(`lv.gate.${g.id === "score" ? g.campo : g.id}`), unit = g.unita ? " " + g.unita : "";
    const txt = `${label} ${inf ? "∞" : num(v, dm)} vs ${num(thr, dm)}${unit} ${ok ? "✓" : "✗"}`;
    return { id: g.id, ruolo: g.ruolo || "", value: known ? v : null, thr, op: g.op, unit: g.unita || "", ok, txt };
  }
  const ikKey = c => { const k = String(c.ik || "").slice(0, 14).toUpperCase(); return k || ("n:" + String(c.name || "").toLowerCase().replace(/[^a-z0-9]+/g, "")); };      // first InChIKey block (or the name)
  function ranked(cands, field) { return [...(cands || [])].filter(c => c && c[field] != null).sort((a, b) => b[field] - a[field]); }
  function marginOf(list, field) {                     // lead of the first candidate over the first one with another structure (∞ when there is none)
    if (!list.length) return null; const k = ikKey(list[0]), o = list.find(c => ikKey(c) !== k);
    return o ? list[0][field] - o[field] : Infinity;
  }

  /** input: { cands: [{name, ik, ent, cos, shared, dprec (ppm), nlib, lib}], analogs: [{name, mcos, matched, delta}], nScans, std: {drt, ent, dprec, name} | null, formulaUnique }
   *  → { proposed, decided: null, why, best, gates, lines, decisive, std } */
  function assess(input, name, T) {
    const P = profile(name), TT = T || thresholds(name), inp = input || {};
    const sf = (P.cancelli.find(g => g.id === "score") || { campo: "ent" }).campo, list = ranked(inp.cands, sf), best = list[0] || null;
    const res = { proposed: P.senza_prove, decided: null, why: "none", best, gates: [], lines: [], decisive: "", std: null, profile: name || profileName() };
    const fin = (lv, why, decisive) => { res.proposed = lv; res.why = why; res.decisive = decisive; return res; };
    const nm = c => c.name || "?", lb = c => (c.lib ? " " + c.lib : "");
    if (inp.std && P.standard.length) {                // level 1 only with the standard marked by the user
      const vals = { drt: inp.std.drt, ent: inp.std.ent, ppm: inp.std.dprec };
      res.std = P.standard.map(g => evalGate(g, vals, TT));
      if (res.std.every(x => x.ok)) { res.lines = res.std.map(x => x.txt); return fin("1", "standard", t("lv.dec.1", { name: inp.std.name || "" })); }
    }
    if (best) {
      const vals = { ent: best.ent, cos: best.cos, shared: best.shared, ppm: best.dprec, nlib: best.nlib, nScans: inp.nScans ?? 1, margin: marginOf(list, "ent"), marginCos: marginOf(list, "cos") };
      res.gates = P.cancelli.map(g => evalGate(g, vals, TT)); res.lines = res.gates.map(g => g.txt);
      const bad = res.gates.filter(g => !g.ok), srcBad = bad.filter(g => g.ruolo === "source"), conf = bad.filter(g => g.ruolo === "confirm"), fl = g => g.map(x => x.txt).join("; ");
      if (!bad.length) return fin("2a", "2a", t("lv.dec.2a", { name: nm(best), lib: lb(best) }));
      if (!srcBad.length) return fin("3", "confirm", t("lv.dec.3conf", { name: nm(best), lib: lb(best), gates: fl(conf) }));
      if (P.provvisorio && best[P.provvisorio.campo] >= TT[P.provvisorio.soglia]) return fin("3", "provvisorio", t("lv.dec.3prov", { name: nm(best), lib: lb(best), gates: fl(srcBad) }));
    }
    const an = ranked(inp.analogs, "mcos")[0];
    if (an && TT.analogo_mcos_min != null && an.mcos >= TT.analogo_mcos_min) {
      res.analog = an; return fin("3", "analogo", t("lv.dec.3an", { name: nm(an), cos: num(an.mcos, 2), delta: (an.delta >= 0 ? "+" : "−") + num(Math.abs(an.delta), 4) }));
    }
    if (best) {
      const g = res.gates.filter(x => !x.ok).sort((a, b) => (a.ruolo === "source" ? 0 : 1) - (b.ruolo === "source" ? 0 : 1))[0];
      res.nearest = best; res.decisive = t("lv.dec.none", { name: nm(best), gate: g ? g.txt : "" });
    }
    if (inp.formulaUnique && P.senza_prove === "5") return fin("4", "formula", t("lv.dec.4"));
    res.proposed = P.senza_prove; res.why = best ? "none" : "empty";
    if (!best) res.decisive = t(P.senza_prove === "5" ? "lv.dec.5" : "lv.dec.0");
    return res;
  }
  // the chemist's decision: 2b only from 3 or 2a (the programme never assigns it); refused on 4, 5 and 0; 1 only when a standard was marked
  function decide(res, lv) {
    if (lv === "" || lv == null) { res.decided = null; return { ok: true }; }
    if (!ORDER.includes(lv)) return { ok: false, msg: t("lv.refuse.unknown") };
    if (lv === "2b" && !["2a", "3"].includes(res.proposed)) return { ok: false, msg: t("lv.refuse.2b", { lv: res.proposed }) };
    if (lv === "1" && !res.std) return { ok: false, msg: t("lv.refuse.1") };
    res.decided = lv; return { ok: true };
  }
  const shown = res => res.decided || res.proposed;                 // the level written in the lists: the decision when there is one
  const cls = lv => "lv-" + String(lv);
  const name = lv => t(`lv.name.${lv}`);
  const sentence = lv => t("lv.sentence", { lv, name: name(lv) });  // «Candidate · level 2a», never «it is X»
  const rank = lv => ORDER.indexOf(lv);
  return { names, profile, profileName, thresholds, setThreshold, reset, exportState, importState, assess, decide, shown, cls, name, sentence, rank, ikKey, marginOf, data: D, ORDER };
})();
if (typeof window !== "undefined") window.LIVELLI = LIVELLI;
if (typeof module !== "undefined") module.exports = LIVELLI;
