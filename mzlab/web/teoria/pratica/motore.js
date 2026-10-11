/* mzLab - Pratica: shared engine of the exercises and games (classic script; also loadable by node for the tests).
   - progress kept ONLY in this browser (localStorage "qqq.pratica"): nothing is sent anywhere, no leaderboard;
   - a level per skill with an Elo-like rule (Pelánek 2016): p = 1/(1+e^-(theta-d)), theta += K (s - p), K = 0.4/(1+0.05 n);
   - spaced repetition of the items that went badly (1, 3, 7, 21 days: Leitner boxes);
   - challenge codes: a short code = a seed, so a class plays the same problems without a server;
   - pure chemistry helpers (formula, nominal and exact mass, rings plus double bonds, unit-resolution isotope pattern). */
"use strict";
const PAL = (() => {
  const KEY = "qqq.pratica", DAY = 864e5; // kept from the old name: renaming it would lose the users' data
  const SKILLS = {
    M: "Ione molecolare", iso: "Isotopi ed elementi", formula: "Formula, azoto e RDB", serie: "Serie e ioni caratteristici",
    perdite: "Perdite neutre", meccanismi: "Meccanismi di frammentazione", struttura: "Struttura dallo spettro EI",
    hr: "Massa esatta e alta risoluzione", strumento: "Strumenti e tecniche", quad: "Il quadrupolo", mrm: "Transizioni MRM",
    crom: "Cromatografia (GC e LC)", ion: "Ionizzazione", anal: "Analizzatori e risoluzione", dati: "Dati, quantificazione e TP", tp: "Prodotti di trasformazione",
  };
  // ------------------------------------------------------------------ storage (never throws: private windows, blocked storage)
  function blank() { return { v: 1, skills: {}, items: {}, log: [] }; }
  let mem = null;
  function load() {
    if (mem) return mem;
    try { const s = JSON.parse(localStorage.getItem(KEY) || "null"); mem = s && s.v === 1 ? s : blank(); } catch (_) { mem = blank(); }
    return mem;
  }
  function save() { try { localStorage.setItem(KEY, JSON.stringify(mem || blank())); } catch (_) { /* storage not available: progress lives until the page closes */ } }
  function reset() { mem = blank(); save(); }

  // ------------------------------------------------------------------ Elo per skill and item difficulty
  const prob = (th, d) => 1 / (1 + Math.exp(-(th - d)));
  const LEVELS = [[-Infinity, "da rinforzare"], [-0.6, "in crescita"], [0.4, "solida"], [1.4, "esperta"]];
  function levelOf(th, n) { if (!n) return "da scoprire"; let l = LEVELS[0][1]; LEVELS.forEach(([t, name]) => { if (th >= t) l = name; }); return l; }
  function skill(id) { const s = load().skills[id] || { th: 0, n: 0 }; return { ...s, level: levelOf(s.th, s.n), name: SKILLS[id] || id }; }
  /** Record a result. game: game id; item: item id; d0: starting difficulty (-1, 0, 1, 2); scores: {skill: 0..1}; total 0..100. */
  function record(game, item, d0, scores, total) {
    const st = load(), g = st.items[game] || (st.items[game] = {}), it = g[item] || (g[item] = { d: d0, n: 0, box: 0, due: 0 });
    let dsum = 0, k = 0;
    Object.entries(scores).forEach(([sk, s]) => {
      const S = st.skills[sk] || (st.skills[sk] = { th: 0, n: 0 }), p = prob(S.th, it.d), K = 0.4 / (1 + 0.05 * S.n);
      S.th += K * (s - p); S.n += 1; dsum += K * (s - p); k++;
    });
    if (k) it.d -= dsum / k;
    it.n += 1; it.last = Date.now();
    const ok = total >= 60;
    it.box = ok ? Math.min(4, it.box + 1) : 0;
    it.due = Date.now() + [1, 1, 3, 7, 21][it.box] * DAY;
    st.log.push({ g: game, i: item, s: Math.round(total), t: Date.now() });
    if (st.log.length > 400) st.log.splice(0, st.log.length - 400);
    save();
    return it;
  }
  /** Choose the next item: first the reviews that are due, then the one with success probability closest to 0.7.
      items: [{id, d0, skills:[...], fam}], opt: {avoidFam, rng}. */
  function pick(game, items, opt = {}) {
    if (!items.length) return null;
    const st = load(), g = st.items[game] || {}, now = Date.now(), rnd = opt.rng || Math.random;
    const due = items.filter(i => g[i.id] && g[i.id].box === 0 && g[i.id].due <= now && g[i.id].n > 0);
    if (due.length && rnd() < 0.6) return due[Math.floor(rnd() * due.length)];
    const th = it => { const ss = it.skills.map(s => (st.skills[s] || { th: 0 }).th); return ss.reduce((a, b) => a + b, 0) / ss.length; };
    const scored = items.map(it => {
      const rec = g[it.id], d = rec ? rec.d : it.d0, p = prob(th(it), d);
      let cost = Math.abs(p - 0.7) + (rec ? 0.25 * Math.min(rec.n, 3) : 0) + (rec && rec.box >= 3 ? 0.5 : 0) + (opt.avoidFam && it.fam === opt.avoidFam ? 0.35 : 0);
      return { it, cost: cost + rnd() * 0.12 };
    });
    scored.sort((a, b) => a.cost - b.cost);
    return scored[0].it;
  }
  function stats(game) {
    const st = load(), g = st.items[game] || {}, l = st.log.filter(x => x.g === game);
    return { done: Object.keys(g).length, plays: l.length, mean: l.length ? Math.round(l.reduce((a, x) => a + x.s, 0) / l.length) : 0, last: l.slice(-20) };
  }
  /** "Ready for the oral" (an indication, not a mark): the last 3 results in oral mode of the EI game are >= 70 and from 3 different families,
      and at least one problem of each family solved with >= 60. */
  function oralReady(fams) {
    const st = load(), l = st.log.filter(x => x.g === "ei-orale").slice(-3);
    const fam = id => (fams.find(f => f.id === id) || {}).fam;
    const last3 = l.length === 3 && l.every(x => x.s >= 70) && new Set(l.map(x => fam(x.i))).size === 3;
    const okFam = new Set(st.log.filter(x => (x.g === "ei" || x.g === "ei-orale") && x.s >= 60).map(x => fam(x.i)));
    const all = [...new Set(fams.map(f => f.fam))];
    return { ready: last3 && all.every(f => okFam.has(f)), last3, missing: all.filter(f => !okFam.has(f)) };
  }
  function exportText() { const st = load(); return JSON.stringify({ app: "mzLab Pratica", v: 1, date: new Date().toISOString().slice(0, 10), skills: st.skills, log: st.log.slice(-200) }); }

  // ------------------------------------------------------------------ seeded random numbers and challenge codes
  function rng(seed) { let a = seed >>> 0; return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
  const AL = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  function code(seed) { let s = seed >>> 0, out = ""; for (let i = 0; i < 5; i++) { out += AL[s % 32]; s = Math.floor(s / 32); } return out; }
  function seedOf(c) { c = String(c || "").toUpperCase().replace(/[^A-Z0-9]/g, ""); if (c.length !== 5) return null; let s = 0; for (let i = 4; i >= 0; i--) { const k = AL.indexOf(c[i]); if (k < 0) return null; s = s * 32 + k; } return s; }
  function shuffle(a, r = Math.random) { a = a.slice(); for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a; }

  // ------------------------------------------------------------------ chemistry helpers (pure)
  const NOMINAL = { H: 1, C: 12, N: 14, O: 16, F: 19, Si: 28, P: 31, S: 32, Cl: 35, Br: 79, I: 127 };
  /** "C6H5Cl" -> {C:6, H:5, Cl:1}; accepts lower-case two-letter symbols written properly (Cl, Br, Si); null if invalid. */
  function parse(f) {
    f = String(f || "").replace(/\s+/g, "").replace(/[₀-₉]/g, d => "0123456789"["₀₁₂₃₄₅₆₇₈₉".indexOf(d)]);
    if (!/^([A-Z][a-z]?\d*)+$/.test(f)) return null;
    const out = {}; let ok = true;
    f.replace(/([A-Z][a-z]?)(\d*)/g, (_, e, n) => { if (!(e in NOMINAL)) ok = false; out[e] = (out[e] || 0) + (n === "" ? 1 : +n); return ""; });
    return ok ? out : null;
  }
  const nominal = f => Object.entries(f).reduce((a, [e, n]) => a + NOMINAL[e] * n, 0);
  function rdb(f) {
    const c = (f.C || 0) + (f.Si || 0), x = (f.H || 0) + (f.F || 0) + (f.Cl || 0) + (f.Br || 0) + (f.I || 0), n = (f.N || 0) + (f.P || 0);
    return c - x / 2 + n / 2 + 1;
  }
  const fstr = f => ["C", "H", ...Object.keys(f).filter(e => e !== "C" && e !== "H").sort()].filter(e => f[e]).map(e => e + (f[e] > 1 ? f[e] : "")).join("");
  const fhtml = f => fstr(f).replace(/(\d+)/g, "<sub>$1</sub>");
  /** Exact monoisotopic mass from ELEMENTS (elements.js, the data of the program). */
  function exact(f, EL) {
    EL = EL || (typeof ELEMENTS !== "undefined" ? ELEMENTS : []);
    return Object.entries(f).reduce((a, [e, n]) => { const el = EL.find(x => x.s === e); const top = el.iso.slice().sort((p, q) => q[2] - p[2])[0]; return a + top[1] * n; }, 0);
  }
  /** Unit-resolution isotope pattern of a formula: intensities at M, M+1, M+2, ... relative to M = 100 (convolution by nominal mass). */
  function isoPattern(f, n = 7, EL) {
    EL = EL || (typeof ELEMENTS !== "undefined" ? ELEMENTS : []);
    let dist = [1];
    Object.entries(f).forEach(([e, k]) => {
      const el = EL.find(x => x.s === e), top = el.iso.slice().sort((p, q) => q[2] - p[2])[0];
      const one = []; el.iso.forEach(([A, , ab]) => { const d = A - top[0]; if (d >= 0) one[d] = (one[d] || 0) + ab / 100; });
      for (let i = 0; i < k; i++) {
        const out = new Array(Math.min(dist.length + one.length - 1, n)).fill(0);
        dist.forEach((a, x) => one.forEach((b, y) => { if (b && x + y < n) out[x + y] += a * b; }));
        dist = out;
      }
    });
    const m0 = dist[0] || 1;
    return dist.map(v => v / m0 * 100);
  }
  return { SKILLS, load, save, reset, record, pick, stats, skill, levelOf, prob, oralReady, exportText, rng, code, seedOf, shuffle,
    parse, nominal, rdb, fstr, fhtml, exact, isoPattern, NOMINAL };
})();
if (typeof module !== "undefined") module.exports = PAL;
