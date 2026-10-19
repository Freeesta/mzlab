// @ts-check
// @ts-strict
"use strict";
// Search box, results part (the palette is paletta.js): reads what was typed and says what it is compatible with, never a verdict.
// A number -> neutral loss, adduct shift or contaminant (Rust, ricerca/1); a formula -> its mass and m/z of the common adducts (Rust, formula);
// a word -> pages of the Teoria. The tables are the ones of the panels (LOSSES, QADD, QADD_VIEW, api/contaminants): nothing is copied here.

/** @typedef {{ic: string, chip: string, html: string, detail: string, aria: string, run: () => void}} RicRow */
/** @typedef {{id: string, title: string, rows: RicRow[]}} RicGroup */

const ricT = (/** @type {string} */ k, /** @type {any} */ p = {}) => I18N.t(k, p);
const ricEsc = (/** @type {any} */ s) => String(s == null ? "" : s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const RIC_IC = (/** @type {string} */ d) => `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${d}</svg>`;
const RIC_ICON = {
  loss: RIC_IC('<path d="M3 5h10M3 8h6M3 11h3"/>'),
  adduct: RIC_IC('<path d="M8 3v10M3 8h10"/>'),
  cont: RIC_IC('<path d="M6 2.5h4M7 2.5v4L3.5 12.5a1 1 0 0 0 .9 1.5h7.2a1 1 0 0 0 .9-1.5L9 6.5v-4"/>'),
  formula: RIC_IC('<circle cx="8" cy="8" r="2"/><path d="M8 2v3M8 11v3M2.5 5l2.6 1.5M10.9 9.5l2.6 1.5M2.5 11l2.6-1.5M10.9 6.5l2.6-1.5"/>'),
  theory: RIC_IC('<path d="M3 3.5h4.5a1 1 0 0 1 1 1V13a1 1 0 0 0-1-1H3zM13 3.5H8.5v9.5a1 1 0 0 1 1-1H13z"/>')
};
const RIC_MAX_ADDUCTS = 4;
const RIC_MAX_THEORY = 5;

const RIC_NUMBER = /^[+\-−]?\s*\d+(?:[.,]\d+)?$/;
// strict formula (element symbols with their digits, groups in brackets) with an optional adduct: C6H12O6, C6H12O6+Na, C9H8O4 [M-H]-
const RIC_FORMULA = /^(?:[A-Z][a-z]?\d*|\([A-Za-z0-9()]*\)\d*)+(?:\s*\[?M?[+-][A-Za-z0-9]+\]?[+-]?)?$/;
const ricFold = (/** @type {string} */ s) => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");

/** @param {string} q  @returns {"number" | "formula" | "word" | "empty"} */
function ricClassify(q) {
  const s = q.trim();
  if (!s) return "empty";
  if (RIC_NUMBER.test(s)) return "number";
  if (RIC_FORMULA.test(s) || (/\d/.test(s) && /^[a-z0-9()+\-\s[\]]+$/i.test(s) && /^[a-z]/i.test(s))) return "formula";
  return "word";
}

const ricHrNow = () => (typeof window.QHR_ON === "function" ? !!window.QHR_ON() : true);
const ricSub = (/** @type {string} */ f) => ricEsc(f).replace(/(\d+)/g, "<sub>$1</sub>");
const ricSg = (/** @type {number} */ x) => (x >= 0 ? "+" : "−") + Math.abs(x).toFixed(4);
const ricLang = () => (I18N.lang === "it" ? "it" : "en");

// ------------------------------------------------------------------ the contaminant list (api/contaminants), read once
/** @type {Promise<any[]> | null} */
let ricContP = null;
function ricContaminants() {
  if (!ricContP) {
    ricContP = fetch("api/contaminants").then(r => r.json())
      .then(j => (j.lists || []).flatMap((/** @type {any} */ l) => l.items || []).filter((/** @type {any} */ i) => Number.isFinite(i.mz)))
      .catch(() => []);
  }
  return ricContP;
}

// ------------------------------------------------------------------ number
const RIC_GO = {
  loss: (/** @type {number} */ x) => { if (window.BARRA && BARRA.isDataView()) BARRA.setTab("losses", { q: String(x) }); else if (window.QQQRef) QQQRef.open("ls", { q: String(x) }); },
  adduct: () => { if (window.BARRA && BARRA.isDataView()) BARRA.setTab("adducts"); else if (window.QQQRef) QQQRef.open("ad"); },
  cont: (/** @type {number} */ x, /** @type {any} */ it) => { if (window.LISTE) LISTE.open({ mz: x, polarity: it && it.pol > 0 ? "positive" : "negative", hr: ricHrNow() }); }
};

/** @param {number} x  @returns {Promise<RicGroup[]>} */
async function ricForNumber(x) {
  const hr = ricHrNow();
  const items = await ricContaminants();
  const body = {
    x, hr, tol_mda: 3, tol_ppm: 5,
    losses: (typeof LOSSES !== "undefined" ? LOSSES : []).map((/** @type {any} */ l) => ({ id: l.f, f: l.f })),
    adducts: (window.QADD || []).map((/** @type {any} */ a) => ({ id: a.n, k: a.k, add: a.add || "", sub: a.sub || "", z: a.z })),
    contam: items.map((i, n) => ({ id: String(n), mz: i.mz, pol: i.pol || 0 }))
  };
  const r = await (await fetch("api/ricerca", { method: "POST", body: JSON.stringify(body) })).json();
  if (!r || r.error || !Array.isArray(r.hits)) return [];
  const view = window.QADD_VIEW ? window.QADD_VIEW(hr, null) : [];
  const lossName = (/** @type {string} */ f) => { const l = (typeof LOSSES !== "undefined" ? LOSSES : []).find((/** @type {any} */ z) => z.f === f); return l ? ricT(l.name) : ""; };
  const detail = (/** @type {any} */ h) => (hr ? ricT("pal.d.hr", { d: ricSg(h.delta_da), ppm: (h.ppm >= 0 ? "+" : "−") + Math.abs(h.ppm).toFixed(1) }) : ricT("pal.d.lr"));
  /** @type {RicRow[]} */
  const rows = r.hits.map((/** @type {any} */ h) => {
    const d = detail(h);
    if (h.kind === "loss") {
      const nm = lossName(h.id);
      return { ic: RIC_ICON.loss, chip: ricT("pal.k.loss"), html: ricSub(h.id) + (nm ? ` <span class="pal-d">${ricEsc(nm)}</span>` : ""), detail: d, aria: `${ricT("pal.k.loss")}: ${h.id} ${nm}, ${d}`, run: () => RIC_GO.loss(x) };
    }
    if (h.kind === "adduct") {
      const v = view.find((/** @type {any} */ a) => a.n === h.id);
      const sh = v ? ` <span class="pal-d">${v.shift}</span>` : "";
      return { ic: RIC_ICON.adduct, chip: ricT("pal.k.adduct"), html: ricEsc(h.id) + sh, detail: d, aria: `${ricT("pal.k.adduct")}: ${h.id}, ${d}`, run: () => RIC_GO.adduct() };
    }
    const it = items[+h.id] || {};
    const nm = (ricLang() === "en" && it.name_en) || it.name || "";
    const mz = hr ? Number(it.mz).toFixed(4) : String(Math.round(it.mz));
    return { ic: RIC_ICON.cont, chip: ricT("pal.k.cont"), html: ricEsc(nm) + ` <span class="pal-d">${ricEsc(it.adduct || "")} m/z ${mz}</span>`, detail: d, aria: `${ricT("pal.k.cont")}: ${nm} ${it.adduct || ""}, ${d}`, run: () => RIC_GO.cont(x, it) };
  });
  if (r.more > 0) rows.push({ ic: RIC_ICON.cont, chip: ricT("pal.k.cont"), html: ricEsc(ricT("pal.more", { n: r.more })), detail: "", aria: ricT("pal.more", { n: r.more }), run: () => RIC_GO.cont(x, null) });
  return rows.length ? [{ id: "compat", title: ricT("pal.res.compat"), rows }] : [];
}

// ------------------------------------------------------------------ formula
/** @param {string} q  @returns {Promise<RicGroup[]>} */
async function ricForFormula(q) {
  const hr = ricHrNow();
  const text = q.trim().replace(/\s+/g, "");
  // an adduct written after the formula (C6H12O6+Na, C9H8O4-H, C9H8O4 [M-H]-): the text is already an ion
  const embedded = /\[M[^\]]*\][+-]?$/.test(text) || /[A-Za-z0-9)][+-][A-Za-z0-9]+$/.test(text);
  const call = async (/** @type {string} */ ad) => {
    const r = await fetch(`api/formula?f=${encodeURIComponent(text)}&adduct=${encodeURIComponent(ad)}`);
    const j = await r.json();
    return j && !j.error ? j : null;
  };
  const go = (/** @type {string} */ f) => () => {
    if (window.BARRA && BARRA.isDataView()) BARRA.setTab("adducts"); else if (window.QQQRef) QQQRef.open("ad");
    setTimeout(() => { const i = /** @type {HTMLInputElement | null} */ (document.querySelector("#ad-in")); if (i) { i.value = f; i.dispatchEvent(new Event("input", { bubbles: true })); } }, 120);
  };
  /** @type {RicRow[]} */
  const rows = [];
  if (embedded) {
    const j = await call("none");
    if (j && j.adduct) {
      const mz = hr ? Number(j.mz).toFixed(4) : String(j.nominal);
      const d = ricT("pal.formula.ion", { adduct: j.adduct });
      rows.push({ ic: RIC_ICON.formula, chip: ricT("pal.k.ion"), html: `${ricSub(j.formula)} <span class="pal-d">${ricEsc(j.adduct)}</span> · m/z <b>${mz}</b>`, detail: d, aria: `${ricT("pal.k.ion")}: ${j.formula} ${j.adduct}, m/z ${mz}`, run: go(j.formula) });
      return [{ id: "formula", title: ricT("pal.res.formula"), rows }];
    }
  }
  const j = await call("");
  if (!j) return [];
  const M = hr ? Number(j.neutral).toFixed(4) : String(j.nominal_neutral);
  rows.push({ ic: RIC_ICON.formula, chip: ricT("pal.k.formula"), html: `${ricSub(j.formula)} <span class="pal-d">${ricEsc(ricT("pal.formula.mass", { m: M }))}</span>`, detail: "", aria: `${ricT("pal.k.formula")}: ${j.formula}, ${ricT("pal.formula.mass", { m: M })}`, run: go(j.formula) });
  const view = window.QADD_VIEW ? window.QADD_VIEW(hr, Number(j.neutral)) : [];
  const pol = typeof E !== "undefined" && E.files.some((/** @type {any} */ f) => !f.gone && f.polarity === "negative") && !E.files.some((/** @type {any} */ f) => !f.gone && f.polarity === "positive") ? -1 : 1;
  view.filter((/** @type {any} */ a) => a.common && a.pol === pol).slice(0, RIC_MAX_ADDUCTS).forEach((/** @type {any} */ a) => {
    rows.push({ ic: RIC_ICON.adduct, chip: ricT("pal.k.ion"), html: `${ricEsc(a.n)} · m/z <b>${a.mz}</b>`, detail: "", aria: `${a.n}, m/z ${a.mz}`, run: go(j.formula) });
  });
  return [{ id: "formula", title: ricT("pal.res.formula"), rows }];
}

// ------------------------------------------------------------------ Teoria (the generated index, read on first use)
/** @type {Promise<string[][]> | null} */
let ricTheoryP = null;
function ricTheoryIndex() {
  if (!ricTheoryP) {
    ricTheoryP = new Promise(res => {
      const s = document.createElement("script");
      s.src = "static/teoria/indice-ricerca.js";
      s.onload = () => res(typeof INDICE_RICERCA !== "undefined" ? INDICE_RICERCA : []);
      s.onerror = () => res([]);
      document.head.appendChild(s);
    });
  }
  return ricTheoryP;
}
// every typed word must start a word of the entry; the title counts more than the rest
const ricWords = (/** @type {string} */ s) => " " + ricFold(s).replace(/[^a-z0-9+/.]+/g, " ").trim() + " ";
/** @param {string} q  @param {string[][]} index  @returns {string[][]} */
function ricTheoryMatch(q, index) {
  const toks = ricFold(q).split(/[^a-z0-9+/.]+/).filter(Boolean);
  if (!toks.length || toks.join("").length < 3) return [];
  const hits = [];
  for (const e of index) {
    const title = ricWords(e[0]), all = " " + e[4] + " ";
    let s = 0;
    for (const k of toks) {
      if (title.includes(" " + k)) s += 2; else if (ricWords(all).includes(" " + k)) s += 1; else { s = 0; break; }
    }
    if (s) hits.push({ s: s + (e[1] === "glossario" ? 0.5 : e[1] === "capitolo" ? 0.3 : 0), e });
  }
  hits.sort((a, b) => b.s - a.s);
  return hits.slice(0, RIC_MAX_THEORY).map(h => h.e);
}
const ricOpenTheory = (/** @type {string} */ url) => {
  const f = /** @type {HTMLIFrameElement | null} */ (document.getElementById("tframe"));
  if (f) f.src = "static/teoria/" + url;
  if (typeof setView === "function") setView("theory");
};

/** @param {string} q  @returns {Promise<RicGroup[]>} */
async function ricForWord(q) {
  const hits = ricTheoryMatch(q, await ricTheoryIndex());
  /** @type {RicRow[]} */
  const rows = hits.map(e => ({ ic: RIC_ICON.theory, chip: ricT("pal.k.theory"), html: ricEsc(e[0]) + ` <span class="pal-d">${ricEsc(e[3])}</span>`, detail: "", aria: `${ricT("pal.k.theory")}: ${e[0]}, ${e[3]}`, run: () => ricOpenTheory(e[2]) }));
  return rows.length ? [{ id: "theory", title: ricT("pal.res.theory"), rows }] : [];
}

/** What the text is compatible with, as groups of rows; the palette shows them above or below the commands. A failure is an empty answer.
 * @param {string} q  @returns {Promise<{kind: string, groups: RicGroup[]}>} */
async function ricRisultati(q) {
  const kind = ricClassify(q);
  try {
    if (kind === "number") {
      const x = Number(q.trim().replace(/\s+/g, "").replace(",", ".").replace("−", "-"));
      return { kind, groups: Number.isFinite(x) ? await ricForNumber(Math.abs(x)) : [] };
    }
    if (kind === "formula") return { kind, groups: [...await ricForFormula(q), ...await ricForWord(q)] };
    if (kind === "word") return { kind, groups: await ricForWord(q) };
  } catch (_) { /* no engine for this route (reference engine of the tests): no results, the commands remain */ }
  return { kind, groups: [] };
}

const RICERCA = { classify: ricClassify, risultati: ricRisultati, theoryMatch: ricTheoryMatch };

if (typeof window !== "undefined") window.RICERCA = RICERCA;
