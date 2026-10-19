// @ts-check
// @ts-strict
"use strict";
// Search box, results part (the palette is paletta.js): reads what was typed and says what it is compatible with, never a verdict.
// A number -> neutral loss, adduct shift or contaminant (Rust, ricerca/1); a formula -> its mass and m/z of the common adducts (Rust, formula);
// a word -> pages of the Teoria. The tables are the ones of the panels (LOSSES, QADD, QADD_VIEW, api/contaminants): nothing is copied here.

/** @typedef {{ic: string, chip: string, html: string, detail: string, aria: string, run: () => void}} RicRow */
/** @typedef {{id: string, title: string, rows: RicRow[]}} RicGroup */

const RICERCA = (() => {
  const t = (/** @type {string} */ k, /** @type {any} */ p = {}) => I18N.t(k, p);
  const esc = (/** @type {any} */ s) => String(s == null ? "" : s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  const IC = (/** @type {string} */ d) => `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${d}</svg>`;
  const ICON = {
    loss: IC('<path d="M3 5h10M3 8h6M3 11h3"/>'),
    adduct: IC('<path d="M8 3v10M3 8h10"/>'),
    cont: IC('<path d="M6 2.5h4M7 2.5v4L3.5 12.5a1 1 0 0 0 .9 1.5h7.2a1 1 0 0 0 .9-1.5L9 6.5v-4"/>'),
    formula: IC('<circle cx="8" cy="8" r="2"/><path d="M8 2v3M8 11v3M2.5 5l2.6 1.5M10.9 9.5l2.6 1.5M2.5 11l2.6-1.5M10.9 6.5l2.6-1.5"/>'),
    theory: IC('<path d="M3 3.5h4.5a1 1 0 0 1 1 1V13a1 1 0 0 0-1-1H3zM13 3.5H8.5v9.5a1 1 0 0 1 1-1H13z"/>')
  };
  const MAX_ADDUCTS = 4;
  const MAX_THEORY = 5;

  const NUMBER = /^[+\-−]?\s*\d+(?:[.,]\d+)?$/;
  // strict formula (element symbols with their digits, groups in brackets) with an optional adduct: C6H12O6, C6H12O6+Na, C9H8O4 [M-H]-
  const FORMULA = /^(?:[A-Z][a-z]?\d*|\([A-Za-z0-9()]*\)\d*)+(?:\s*\[?M?[+-][A-Za-z0-9]+\]?[+-]?)?$/;
  const fold = (/** @type {string} */ s) => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");

  /** @param {string} q  @returns {"number" | "formula" | "word" | "empty"} */
  function classify(q) {
    const s = q.trim();
    if (!s) return "empty";
    if (NUMBER.test(s)) return "number";
    if (FORMULA.test(s) || (/\d/.test(s) && /^[a-z0-9()+\-\s[\]]+$/i.test(s) && /^[a-z]/i.test(s))) return "formula";
    return "word";
  }

  const hrNow = () => (typeof window.QHR_ON === "function" ? !!window.QHR_ON() : true);
  const sub = (/** @type {string} */ f) => esc(f).replace(/(\d+)/g, "<sub>$1</sub>");
  const sg = (/** @type {number} */ x) => (x >= 0 ? "+" : "−") + Math.abs(x).toFixed(4);
  const lang = () => (I18N.lang === "it" ? "it" : "en");

  // ------------------------------------------------------------------ the contaminant list (api/contaminants), read once
  /** @type {Promise<any[]> | null} */
  let contP = null;
  function contaminants() {
    if (!contP) {
      contP = fetch("api/contaminants").then(r => r.json())
        .then(j => (j.lists || []).flatMap((/** @type {any} */ l) => l.items || []).filter((/** @type {any} */ i) => Number.isFinite(i.mz)))
        .catch(() => []);
    }
    return contP;
  }

  // ------------------------------------------------------------------ number
  const GO = {
    loss: (/** @type {number} */ x) => { if (window.BARRA && BARRA.isDataView()) BARRA.setTab("losses", { q: String(x) }); else if (window.QQQRef) QQQRef.open("ls", { q: String(x) }); },
    adduct: () => { if (window.BARRA && BARRA.isDataView()) BARRA.setTab("adducts"); else if (window.QQQRef) QQQRef.open("ad"); },
    cont: (/** @type {number} */ x, /** @type {any} */ it) => { if (window.LISTE) LISTE.open({ mz: x, polarity: it && it.pol > 0 ? "positive" : "negative", hr: hrNow() }); }
  };

  /** @param {number} x  @returns {Promise<RicGroup[]>} */
  async function forNumber(x) {
    const hr = hrNow();
    const items = await contaminants();
    const body = {
      x, hr, tol_mda: 3, tol_ppm: 5,
      losses: (typeof LOSSES !== "undefined" ? LOSSES : []).map((/** @type {any} */ l) => ({ id: l.f, f: l.f })),
      adducts: (window.QADD || []).map((/** @type {any} */ a) => ({ id: a.n, k: a.k, add: a.add || "", sub: a.sub || "", z: a.z })),
      contam: items.map((i, n) => ({ id: String(n), mz: i.mz, pol: i.pol || 0 }))
    };
    const r = await (await fetch("api/ricerca", { method: "POST", body: JSON.stringify(body) })).json();
    if (!r || r.error || !Array.isArray(r.hits)) return [];
    const view = window.QADD_VIEW ? window.QADD_VIEW(hr, null) : [];
    const lossName = (/** @type {string} */ f) => { const l = (typeof LOSSES !== "undefined" ? LOSSES : []).find((/** @type {any} */ z) => z.f === f); return l ? t(l.name) : ""; };
    const detail = (/** @type {any} */ h) => (hr ? t("pal.d.hr", { d: sg(h.delta_da), ppm: (h.ppm >= 0 ? "+" : "−") + Math.abs(h.ppm).toFixed(1) }) : t("pal.d.lr"));
    /** @type {RicRow[]} */
    const rows = r.hits.map((/** @type {any} */ h) => {
      const d = detail(h);
      if (h.kind === "loss") {
        const nm = lossName(h.id);
        return { ic: ICON.loss, chip: t("pal.k.loss"), html: sub(h.id) + (nm ? ` <span class="pal-d">${esc(nm)}</span>` : ""), detail: d, aria: `${t("pal.k.loss")}: ${h.id} ${nm}, ${d}`, run: () => GO.loss(x) };
      }
      if (h.kind === "adduct") {
        const v = view.find((/** @type {any} */ a) => a.n === h.id);
        const sh = v ? ` <span class="pal-d">${v.shift}</span>` : "";
        return { ic: ICON.adduct, chip: t("pal.k.adduct"), html: esc(h.id) + sh, detail: d, aria: `${t("pal.k.adduct")}: ${h.id}, ${d}`, run: () => GO.adduct() };
      }
      const it = items[+h.id] || {};
      const nm = (lang() === "en" && it.name_en) || it.name || "";
      const mz = hr ? Number(it.mz).toFixed(4) : String(Math.round(it.mz));
      return { ic: ICON.cont, chip: t("pal.k.cont"), html: esc(nm) + ` <span class="pal-d">${esc(it.adduct || "")} m/z ${mz}</span>`, detail: d, aria: `${t("pal.k.cont")}: ${nm} ${it.adduct || ""}, ${d}`, run: () => GO.cont(x, it) };
    });
    if (r.more > 0) rows.push({ ic: ICON.cont, chip: t("pal.k.cont"), html: esc(t("pal.more", { n: r.more })), detail: "", aria: t("pal.more", { n: r.more }), run: () => GO.cont(x, null) });
    return rows.length ? [{ id: "compat", title: t("pal.res.compat"), rows }] : [];
  }

  // ------------------------------------------------------------------ formula
  /** @param {string} q  @returns {Promise<RicGroup[]>} */
  async function forFormula(q) {
    const hr = hrNow();
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
        const d = t("pal.formula.ion", { adduct: j.adduct });
        rows.push({ ic: ICON.formula, chip: t("pal.k.ion"), html: `${sub(j.formula)} <span class="pal-d">${esc(j.adduct)}</span> · m/z <b>${mz}</b>`, detail: d, aria: `${t("pal.k.ion")}: ${j.formula} ${j.adduct}, m/z ${mz}`, run: go(j.formula) });
        return [{ id: "formula", title: t("pal.res.formula"), rows }];
      }
    }
    const j = await call("");
    if (!j) return [];
    const M = hr ? Number(j.neutral).toFixed(4) : String(j.nominal_neutral);
    rows.push({ ic: ICON.formula, chip: t("pal.k.formula"), html: `${sub(j.formula)} <span class="pal-d">${esc(t("pal.formula.mass", { m: M }))}</span>`, detail: "", aria: `${t("pal.k.formula")}: ${j.formula}, ${t("pal.formula.mass", { m: M })}`, run: go(j.formula) });
    const view = window.QADD_VIEW ? window.QADD_VIEW(hr, Number(j.neutral)) : [];
    const pol = typeof E !== "undefined" && E.files.some((/** @type {any} */ f) => !f.gone && f.polarity === "negative") && !E.files.some((/** @type {any} */ f) => !f.gone && f.polarity === "positive") ? -1 : 1;
    view.filter((/** @type {any} */ a) => a.common && a.pol === pol).slice(0, MAX_ADDUCTS).forEach((/** @type {any} */ a) => {
      rows.push({ ic: ICON.adduct, chip: t("pal.k.ion"), html: `${esc(a.n)} · m/z <b>${a.mz}</b>`, detail: "", aria: `${a.n}, m/z ${a.mz}`, run: go(j.formula) });
    });
    return [{ id: "formula", title: t("pal.res.formula"), rows }];
  }

  // ------------------------------------------------------------------ Teoria (the generated index, read on first use)
  /** @type {Promise<string[][]> | null} */
  let theoryP = null;
  function theoryIndex() {
    if (!theoryP) {
      theoryP = new Promise(res => {
        const s = document.createElement("script");
        s.src = "static/teoria/indice-ricerca.js";
        s.onload = () => res(typeof INDICE_RICERCA !== "undefined" ? INDICE_RICERCA : []);
        s.onerror = () => res([]);
        document.head.appendChild(s);
      });
    }
    return theoryP;
  }
  // every typed word must start a word of the entry; the title counts more than the rest
  const words = (/** @type {string} */ s) => " " + fold(s).replace(/[^a-z0-9+/.]+/g, " ").trim() + " ";
  /** @param {string} q  @param {string[][]} index  @returns {string[][]} */
  function theoryMatch(q, index) {
    const toks = fold(q).split(/[^a-z0-9+/.]+/).filter(Boolean);
    if (!toks.length || toks.join("").length < 3) return [];
    const hits = [];
    for (const e of index) {
      const title = words(e[0]), all = " " + e[4] + " ";
      let s = 0;
      for (const k of toks) {
        if (title.includes(" " + k)) s += 2; else if (words(all).includes(" " + k)) s += 1; else { s = 0; break; }
      }
      if (s) hits.push({ s: s + (e[1] === "glossario" ? 0.5 : e[1] === "capitolo" ? 0.3 : 0), e });
    }
    hits.sort((a, b) => b.s - a.s);
    return hits.slice(0, MAX_THEORY).map(h => h.e);
  }
  const openTheory = (/** @type {string} */ url) => {
    const f = /** @type {HTMLIFrameElement | null} */ (document.getElementById("tframe"));
    if (f) f.src = "static/teoria/" + url;
    if (typeof setView === "function") setView("theory");
  };

  /** @param {string} q  @returns {Promise<RicGroup[]>} */
  async function forWord(q) {
    const hits = theoryMatch(q, await theoryIndex());
    /** @type {RicRow[]} */
    const rows = hits.map(e => ({ ic: ICON.theory, chip: t("pal.k.theory"), html: esc(e[0]) + ` <span class="pal-d">${esc(e[3])}</span>`, detail: "", aria: `${t("pal.k.theory")}: ${e[0]}, ${e[3]}`, run: () => openTheory(e[2]) }));
    return rows.length ? [{ id: "theory", title: t("pal.res.theory"), rows }] : [];
  }

  /** What the text is compatible with, as groups of rows; the palette shows them above or below the commands. A failure is an empty answer.
   * @param {string} q  @returns {Promise<{kind: string, groups: RicGroup[]}>} */
  async function risultati(q) {
    const kind = classify(q);
    try {
      if (kind === "number") {
        const x = Number(q.trim().replace(/\s+/g, "").replace(",", ".").replace("−", "-"));
        return { kind, groups: Number.isFinite(x) ? await forNumber(Math.abs(x)) : [] };
      }
      if (kind === "formula") return { kind, groups: [...await forFormula(q), ...await forWord(q)] };
      if (kind === "word") return { kind, groups: await forWord(q) };
    } catch (_) { /* no engine for this route (reference engine of the tests): no results, the commands remain */ }
    return { kind, groups: [] };
  }

  return { classify, risultati, theoryMatch };
})();

if (typeof window !== "undefined") window.RICERCA = RICERCA;
