// @ts-check
// Flow (administrator mode): result views of the identification steps. JSON files (candidates/1, annot/1, formulas/1) become tables, and an MGF file of MS2 spectra
// gets a list, a stick chart with its data table. Pure readers first (F.read*), then the views registered with F.registerView.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const isJson = (/** @type {any} */ item) => /\.json$/i.test(item.label);
  const fx = (/** @type {any} */ n, d = 4) => n === null || n === undefined || n === "" || !Number.isFinite(Number(n)) ? "" : String(Math.round(Number(n) * 10 ** d) / 10 ** d);
  const txt = (/** @type {any} */ v) => v === null || v === undefined ? "" : String(v);

  /** candidates/1 → table @param {any} j @returns {{head: string[], rows: string[][]}} */
  F.readCandidates = j => ({
    head: ["Query", "Name", "Formula", "Score", "Shared peaks", "Mass error (ppm)", "Source", "Algorithm"],
    rows: (j.hits || []).map((/** @type {any} */ h) => [txt(h.query), txt(h.name), txt(h.formula), fx(h.score), txt(h.shared), fx(h.mass_ppm, 2), txt(h.source), txt(h.algorithm)]),
  });

  /** annot/1 → table; a gate reads «value / threshold» and a failed one is marked @param {any} j @returns {{head: string[], rows: string[][]}} */
  F.readAnnot = j => ({
    head: ["Query", "Level", "Why", "Name", "Analogue", "Gates (value / threshold)"],
    rows: (j.rows || []).map((/** @type {any} */ r) => [txt(r.id), txt(r.level), txt(r.why), txt(r.name), txt(r.analog),
      (r.gates || []).map((/** @type {any} */ g) => `${g.id} ${fx(g.value) || "–"} / ${fx(g.threshold)}${g.ok ? "" : " (not met)"}`).join("; ")]),
  });

  /** formulas/1 → the best candidate of each feature, or each spectrum candidate; `fragments` lists the explained peaks @param {any} j @returns {{head: string[], rows: string[][], fragments: {head: string[], rows: string[][]}}} */
  F.readFormulas = j => {
    const frag = { head: ["Query", "Formula", "Peak m/z", "Intensity", "Fragment", "Theoretical m/z", "Error (ppm)", "Neutral loss"], rows: /** @type {string[][]} */ ([]) };
    if (Array.isArray(j.spectra)) {
      const rows = /** @type {string[][]} */ ([]);
      for (const s of j.spectra) for (const c of s.candidates || []) {
        rows.push([txt(s.query), fx(s.precursor, 4), txt(s.peaks), txt(c.formula), txt(c.adduct), fx(c.ppm_ion, 2), txt(c.n_explained), fx(c.frac_explained), fx(c.score)]);
        for (const f of c.fragments || []) frag.rows.push([txt(s.query), txt(c.formula), fx(f.mz, 4), fx(f.intensity, 1), txt(f.formula), fx(f.mz_th, 4), fx(f.ppm, 2), txt(f.loss)]);
      }
      return { head: ["Query", "Precursor m/z", "Peaks", "Formula", "Adduct", "Error (ppm)", "Fragments explained", "Share explained", "Score"], rows, fragments: frag };
    }
    const rows = (j.features || []).map((/** @type {any} */ f) => {
      const r = f.result || {}, list = r.candidates || [], best = list.find((/** @type {any} */ c) => c.formula === r.chosen) || list[0] || {};
      return [txt(f.id), fx(f.mz, 4), fx(f.rt, 2), txt(f.file), txt(f.scan), txt(best.formula), fx(best.error_ppm, 2), fx(best.score), txt(best.confidence), txt(f.error), txt(list.length)];
    });
    return { head: ["Feature", "m/z", "RT", "File", "Scan", "Best formula", "Error (ppm)", "Score", "Confidence", "Note", "Candidates"], rows, fragments: frag };
  };

  /** MGF text → spectra with title, precursor, charge, RT and peak pairs @param {string} text @returns {{title: string, pepmass: number, charge: string, rt: number, peaks: [number, number][]}[]} */
  F.readMgf = text => {
    const out = [];
    let cur = /** @type {any} */ (null);
    for (const raw of text.split(/\r?\n/)) {
      const line = raw.trim();
      if (line === "BEGIN IONS") cur = { title: "", pepmass: NaN, charge: "", rt: NaN, peaks: [] };
      else if (line === "END IONS") { if (cur) out.push(cur); cur = null; }
      else if (cur && line) {
        const eq = line.indexOf("=");
        if (eq > 0 && /^[A-Z]/.test(line)) {
          const k = line.slice(0, eq).toUpperCase(), v = line.slice(eq + 1).trim();
          if (k === "TITLE") cur.title = v; else if (k === "PEPMASS") cur.pepmass = Number(v.split(/\s+/)[0]); else if (k === "CHARGE") cur.charge = v; else if (k === "RTINSECONDS") cur.rt = Number(v);
        } else {
          const [a, b] = line.split(/\s+/).map(Number);
          if (Number.isFinite(a) && Number.isFinite(b)) cur.peaks.push([a, b]);
        }
      }
    }
    return out;
  };

  /** the JSON table views: the file is parsed, a wrong shape says so @param {string} id @param {string} title @param {string[]} types @param {(j: any) => any} read */
  const jsonView = (id, title, types, read) => F.registerView({ id, title, types, accepts: isJson, open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, j = JSON.parse(await item.text()), t = read(j);
    if (!t.rows.length) { host.append(el("p", { class: "fl-note", text: `${item.label} has no rows to show.` })); return; }
    F.drawTable(host, { caption: item.label, head: t.head, rows: t.rows, label: `Table ${item.label}` });
    if (t.fragments && t.fragments.rows.length) F.drawTable(host, { caption: `${item.label}, explained fragments`, head: t.fragments.head, rows: t.fragments.rows, label: `Fragments of ${item.label}` });
  } });
  jsonView("candidates", "Library hits", ["candidates"], F.readCandidates);
  jsonView("annot", "Confidence levels", ["annot"], F.readAnnot);
  jsonView("formulas", "Formulas", ["formulas"], F.readFormulas);

  /** a stick chart of one spectrum: colours from CSS variables; the data table sits beside it @param {{peaks: [number, number][], pepmass: number}} sp @param {string} label */
  F.sticks = (sp, label) => {
    const ns = "http://www.w3.org/2000/svg", W = 640, H = 220, L = 44, B = 28, T = 10, R = 10, peaks = sp.peaks;
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("viewBox", `0 0 ${W} ${H}`); svg.setAttribute("class", "flx-sticks"); svg.setAttribute("role", "img"); svg.setAttribute("aria-label", label);
    if (!peaks.length) return svg;
    const lo = Math.min(...peaks.map(p => p[0]), sp.pepmass || Infinity), hi = Math.max(...peaks.map(p => p[0]), sp.pepmass || 0), top = Math.max(...peaks.map(p => p[1])) || 1;
    const x = (/** @type {number} */ m) => L + (hi === lo ? 0.5 : (m - lo) / (hi - lo)) * (W - L - R), y = (/** @type {number} */ v) => T + (1 - v / top) * (H - T - B);
    const line = (/** @type {number[]} */ a, /** @type {string} */ cls) => { const e = document.createElementNS(ns, "line"); a.forEach((v, i) => e.setAttribute(["x1", "y1", "x2", "y2"][i], String(v))); e.setAttribute("class", cls); svg.append(e); };
    line([L, H - B, W - R, H - B], "flx-ax"); line([L, T, L, H - B], "flx-ax");
    for (const [m, v] of peaks) line([x(m), y(0), x(m), y(v)], "flx-stick");
    if (Number.isFinite(sp.pepmass)) line([x(sp.pepmass), T, x(sp.pepmass), H - B], "flx-prec");
    const tick = (/** @type {string} */ s, /** @type {number} */ px, /** @type {number} */ py, /** @type {string} */ anchor) => { const t = document.createElementNS(ns, "text"); t.setAttribute("x", String(px)); t.setAttribute("y", String(py)); t.setAttribute("text-anchor", anchor); t.setAttribute("class", "flx-tick"); t.textContent = s; svg.append(t); };
    tick(fx(lo, 2), L, H - 10, "start"); tick(fx(hi, 2), W - R, H - 10, "end"); tick(fx(top, 0), L - 4, T + 8, "end"); tick("0", L - 4, H - B, "end");
    return svg;
  };

  F.registerView({ id: "spectra", title: "MS2 spectra", types: ["spectra"], accepts: (/** @type {any} */ item) => /\.mgf$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, list = F.readMgf(await item.text());
    if (!list.length) { host.append(el("p", { class: "fl-note", text: `${item.label} has no spectra.` })); return; }
    F.drawTable(host, { caption: item.label, head: ["#", "Title", "Precursor m/z", "Charge", "RT (s)", "Peaks"], rows: list.map((s, i) => [String(i + 1), s.title, fx(s.pepmass, 4), s.charge, fx(s.rt, 1), String(s.peaks.length)]), label: `Spectra of ${item.label}` });
    const show = el("div", { class: "flx-one" }), pick = /** @type {HTMLSelectElement} */ (el("select", { "aria-label": "Spectrum to draw", onchange: () => draw(Number(pick.value)) }, ...list.slice(0, 500).map((s, i) => el("option", { value: String(i), text: `${i + 1}. ${s.title || fx(s.pepmass, 4)}` }))));
    /** @param {number} i */
    const draw = i => {
      const s = list[i];
      show.replaceChildren(F.sticks(s, `Spectrum ${i + 1}: ${s.peaks.length} peaks, precursor ${fx(s.pepmass, 4)}`));
      F.drawTable(show, { caption: `Peaks of spectrum ${i + 1}`, head: ["m/z", "Intensity"], rows: s.peaks.map(p => [fx(p[0], 4), fx(p[1], 1)]), label: `Peaks of spectrum ${i + 1}` });
    };
    host.append(el("label", { class: "fl-note" }, "Spectrum ", pick), show); draw(0);
  } });
})();
