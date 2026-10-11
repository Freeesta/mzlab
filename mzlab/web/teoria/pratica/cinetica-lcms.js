/* mzLab - Pratica: data and generator of the game "lc-cin" (kinetic profiles of transformation products; classic script, also loadable by node).
   ALL DATA ARE SYNTHETIC: invented scaffolds (esercizi-lcms.js), first-order kinetics A -> B -> C plus noise, never measurements of a laboratory experience.
   Every candidate is a "feature" of an XIC table: m/z, RT, blank area, areas at t = 0 and at the irradiation times. The student discards the false
   ones (present at t0 or in the blank, profile with no trend, RT incoherent with the mass shift), separates primary from secondary products and
   chooses the time of the MS2 experiment. Masses come from LCMS (ELEMENTS through PAL), never typed by hand; pure function of (level, seed). */
"use strict";
const LCCIN = (() => {
  const P = typeof PAL !== "undefined" ? PAL : require("./motore.js");
  const L = typeof LCMS !== "undefined" ? LCMS : require("./esercizi-lcms.js");
  const TIMES = [0, 10, 20, 40, 80, 160];                    // min of irradiation; index 0 is t0 (no light)
  const LOQ = 15;                                            // same (arbitrary) units as the areas: thousands of counts
  const FRAC = 0.85;                                         // MS2 time: area at least this fraction of the maximum of the product
  // teaching rule for reversed phase: more polar products (oxidised, hydrated, shorter chains) elute BEFORE the precursor, the others after
  const POLAR = new Set(["O", "2O", "3O", "+H2O", "-CH2", "-C2H4", "CH3>COOH", "-Cl+OH"]);
  const APOLAR = new Set(["+CH2", "-H2O", "-Cl+H"]);
  const LEVELS = {
    1: { sigma: 0.03, kinds: () => ["primary", "secondary", "blank", "rt"] },
    2: { sigma: 0.07, kinds: (r) => ["primary", "secondary", ...P.shuffle(["blank", "t0", "flat", "rt"], r).slice(0, 3)] },
    3: { sigma: 0.11, kinds: (r) => ["primary", "primary", "secondary", "rt", r() < 0.5 ? "blank" : "t0", "flat"], isomer: true },
  };
  const KIND_TRUTH = { primary: "tengo", secondary: "tengo", blank: "t0bianco", t0: "t0bianco", rt: "rt", flat: "piatto" };

  const pick = (r, a) => a[Math.floor(r() * a.length)];
  const fx = L.fx;
  const round1 = v => Math.round(v * 10) / 10;

  /** noiseless shapes (0..1 scale) of the first-order chain A -> B -> C with rate constants k1, k2 */
  const shapeB = (k1, k2, t) => k1 / (k2 - k1) * (Math.exp(-k1 * t) - Math.exp(-k2 * t));
  const shapeC = (k1, k2, t) => 1 - Math.exp(-k1 * t) - shapeB(k1, k2, t);

  function profile(kind, r, k1) {
    const k2 = kind === "secondary" ? 0.006 + r() * 0.012 : 0.004 + r() * 0.016;
    let w, ymax = 150 + r() * 450, blank = 0;
    if (kind === "primary" || kind === "rt") w = TIMES.map(t => shapeB(k1, k2, t));
    else if (kind === "secondary") w = TIMES.map(t => shapeC(k1, k2, t));
    else if (kind === "t0") w = TIMES.map(t => 0.55 + 0.45 * Math.exp(-k1 * t));
    else if (kind === "blank") { w = TIMES.map(() => 1); blank = ymax * (0.7 + r() * 0.4); }
    else { ymax = 28 + r() * 20; w = TIMES.map((t, i) => i === 0 ? 0 : 1 + (2 * r() - 1) * 0.2); }
    const mx = Math.max(...w);
    const areas = w.map(v => ymax * v / (mx || 1));
    if (kind === "primary" || kind === "secondary" || kind === "rt") { areas[0] = ymax * r() * 0.04; blank = ymax * r() * 0.04; }
    return { areas, blank };
  }

  function metrics(areas, blank) {
    const mx = Math.max(...areas, blank), irr = areas.slice(1);
    return { blankFrac: blank / mx, t0Frac: areas[0] / mx, max: mx, ratio: Math.max(...irr) / Math.max(Math.min(...irr), 0.5),
      r1: areas[1] / mx, tmaxIdx: areas.indexOf(Math.max(...areas)) };
  }
  /** is the noisy table consistent with the declared kind? (rejection sampling keeps every exercise unambiguous) */
  function consistent(kind, m) {
    if (kind === "primary") return m.blankFrac <= 0.1 && m.t0Frac <= 0.1 && m.r1 >= 0.3 && m.ratio >= 1.9;
    if (kind === "rt") return m.blankFrac <= 0.1 && m.t0Frac <= 0.1 && m.r1 >= 0.3 && m.ratio >= 1.9;
    if (kind === "secondary") return m.blankFrac <= 0.1 && m.t0Frac <= 0.1 && m.r1 <= 0.2 && m.ratio >= 3 && m.tmaxIdx >= 4;
    if (kind === "t0") return m.blankFrac <= 0.1 && m.t0Frac >= 0.75;
    if (kind === "blank") return m.blankFrac >= 0.4 && m.t0Frac >= 0.4;
    return m.blankFrac <= 0.05 && m.t0Frac <= 0.2 && m.ratio <= 1.7 && m.max <= 70;               // flat
  }

  function genCin(level, seed) {
    const cfg = LEVELS[level] || LEVELS[1];
    for (let attempt = 0; attempt < 120; attempt++) {
      const r = P.rng((seed * 2749 + attempt * 15485863) >>> 0);
      const sc = pick(r, L.SCAFFOLDS), k1 = 0.02 + r() * 0.03, R0 = Math.round((6.5 + r() * 5) * 100) / 100;
      const mzOf = f => Math.round((P.exact(f) + L.SHIFT["[M+H]+"]) * 1e4) / 1e4;
      const mzP = mzOf(sc.p);
      const usable = P.shuffle(L.TRANS.filter(t => (POLAR.has(t.id) || APOLAR.has(t.id)) && t.needs.every(g => sc.groups.includes(g)) && L.valid(L.apply(sc.p, t.delta))), r);
      const kinds = P.shuffle(cfg.kinds(r), r);
      const polar = usable.filter(t => POLAR.has(t.id)), rest = usable.slice();
      const needed = kinds.length - (cfg.isomer ? 1 : 0);
      if (polar.length < 1 || rest.length < needed) continue;
      // assign a transformation to every feature; the RT-incoherent one (and its isomer twin at level 3) needs a polar one
      const taken = new Set(), feats = [];
      const take = pool => { const t = pool.find(x => !taken.has(x.id)); if (t) taken.add(t.id); return t; };
      let ok = true;
      kinds.forEach(kind => {
        if (!ok) return;
        let t;
        if (kind === "rt") t = take(polar); else t = take(rest);
        if (!t) { ok = false; return; }
        feats.push({ kind, t });
      });
      if (!ok) continue;
      if (cfg.isomer) {                                      // level 3: the RT-incoherent feature is an isomer of a real primary product (same m/z)
        const prim = feats.find(f => f.kind === "primary" && POLAR.has(f.t.id)), bad = feats.find(f => f.kind === "rt");
        if (!prim) continue;
        bad.t = prim.t;
      }
      const sig = cfg.sigma;
      for (const f of feats) {
        const p = profile(f.kind, r, k1);
        f.areas = p.areas.map(v => Math.max(0, Math.round(v * (1 + (2 * r() - 1) * sig) + (2 * r() - 1) * 1.5)));
        f.blank = Math.max(0, Math.round(p.blank * (1 + (2 * r() - 1) * sig)));
        if (!consistent(f.kind, metrics(f.areas, f.blank))) { ok = false; break; }
        const pol = POLAR.has(f.t.id);
        f.rt = Math.round((f.kind === "rt" ? R0 + 0.5 + r() * 1.8 : pol ? R0 - 0.8 - r() * 2.7 : R0 + 0.6 + r() * 1.9) * 100) / 100;
        f.prod = L.apply(sc.p, f.t.delta); f.mz = mzOf(f.prod); f.dm = Math.round((f.mz - mzP) * 1e4) / 1e4;
      }
      if (!ok) continue;
      feats.sort((a, b) => a.mz - b.mz || a.rt - b.rt);
      feats.forEach((f, i) => { f.id = "F" + (i + 1); f.truth = KIND_TRUTH[f.kind]; f.role = f.kind === "primary" ? "primario" : f.kind === "secondary" ? "secondario" : null; f.polar = POLAR.has(f.t.id); });
      const precursor = { f: sc.f, mz: mzP, rt: R0, areas: TIMES.map(t => Math.round(3200 * Math.exp(-k1 * t) * (1 + (2 * r() - 1) * sig))), blank: 0 };
      feats.forEach(f => { const m = metrics(f.areas, f.blank), top = Math.max(...f.areas); f.ms2Times = f.truth === "tengo" ? f.areas.map((a, i) => a / top >= FRAC && i > 0 ? i : -1).filter(i => i > 0) : []; f.max = top; f.m = m; });
      const e = { game: "lc-cin", level, seed, scaffold: sc.id, desc: sc.desc, times: TIMES, loq: LOQ, precursor, features: feats, k1,
        id: `L${level}-${sc.id}-${feats.map(f => f.kind[0] + (f.kind === "t0" ? "0" : "")).join("")}`, d0: level === 1 ? 0 : level === 2 ? 0.6 : 1.2 };
      e.help = helpCin(e);
      return e;
    }
    throw new Error("genCin: no exercise for seed " + seed);
  }

  /** Numbers for one feature (never "right"/"wrong"): what the student should have compared. */
  function coherenceCin(e, id) {
    const f = e.features.find(x => x.id === id), m = f.m;
    return { id, blankPct: Math.round(m.blankFrac * 100), t0Pct: Math.round(m.t0Frac * 100), r1Pct: Math.round(m.r1 * 100), ratio: round1(m.ratio),
      max: f.max, tmax: TIMES[m.tmaxIdx], dRT: Math.round((f.rt - e.precursor.rt) * 100) / 100, dm: f.dm };
  }
  const pctAtTime = (f, i) => Math.round(f.areas[i] / f.max * 100);

  function helpCin(e) {
    const iso = e.features.filter((f, i) => e.features.some((g, j) => j !== i && g.dm === f.dm));
    const ids = [...new Set(iso.map(f => f.id))];
    return [
      `Osservazione. Per ogni candidato mettete a confronto tre numeri: l'area nel bianco, l'area a t0 e l'area massima tra i campioni irradiati. Un prodotto della fotocatalisi nasce dalla reazione: senza luce (t0) e nel bianco dovrebbe esserci poco o niente. Guardate anche il RT rispetto a quello del precursore (${fx(e.precursor.rt, 2)} min).`,
      `Famiglie. I falsi sono di tre tipi: presenti già a t0 o nel bianco (non nascono dalla reazione); con un profilo senza andamento, vicino al limite di quantificazione (${e.loq}); con un RT incoerente con il Δm (regola didattica: in fase inversa un prodotto più polare, per esempio ossidato, eluisce prima del precursore). Tra i veri, un primario compare già al primo prelievo e poi può calare; un secondario parte con ritardo, perché si forma da un altro prodotto.`,
      ids.length ? `Zona. I candidati ${ids.join(" e ")} hanno lo stesso Δm: sono isomeri e li separa il RT. Confrontate il RT di ciascuno con quello del precursore e poi guardate, riga per riga, i valori a t0 e nel bianco.`
        : `Zona. La differenza tra falsi e veri sta nelle prime due colonne di area (bianco e t0) e nel primo prelievo irradiato (${e.times[1]} min): lì si vede cosa era già presente e cosa si forma davvero. Per i secondari guardate quanto l'area al primo prelievo è piccola rispetto al massimo.`,
    ];
  }

  return { TIMES, LOQ, FRAC, POLAR, APOLAR, LEVELS, KIND_TRUTH, genCin, coherenceCin, pctAtTime, shapeB, shapeC, fx };
})();
if (typeof module !== "undefined") module.exports = LCCIN;
