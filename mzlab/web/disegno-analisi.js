// @ts-check
// «Formula analysis» card of Draw (`#anal-card`, a closed <details>): formula, exact, nominal and average mass, mass % and, on a click, the theoretical
// isotope pattern of the drawn structure. Every number comes from the Rust `anal/1` (`analysis` of the grafo .wasm, `feat/grafo` with kind "analysis" in
// the app, `mzlab-cli mol-anal`); this module only draws. The pattern is theory for the formula, never compared with a spectrum; Composition does the scoring.
import { carica } from "./disegno-frammenti.js";
import { atomOf, formulaParts } from "./disegno-etichette.js";

/** @typedef {{ off: number, mz: number, rel: number, abund: number }} Stick */
/** @typedef {{ atoms: () => any[] | null, t: (k: string, p?: Record<string, unknown>) => string, copy: (s: string) => void, table: (o: Record<string, unknown>) => void }} Helpers */

const W = 280, H = 132, ML = 34, MR = 10, MT = 10, MB = 30;
const esc = (/** @type {unknown} */ s) => String(s ?? "").replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c] || c);

/** the app asks the native engine; the site the .wasm loaded by `carica` */
async function ask(/** @type {string} */ request) {
  if (/** @type {any} */ (window).MZLAB_DESKTOP) {
    const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
    const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", kind: "analysis", request }) }), t = await r.text();
    if (r.status !== 200) throw t;
    return t;
  }
  return (await carica()).analysis(request);
}

/** formula as HTML: isotope mass numbers as superscripts, counts as subscripts, then the charge */
function formulaHtml(/** @type {any} */ s) {
  const q = s.charge, sign = q ? `<sup>${Math.abs(q) > 1 ? Math.abs(q) : ""}${q > 0 ? "+" : "&minus;"}</sup>` : "";
  return formulaParts(s.formula).map(([x, k]) => k ? `<${k}>${esc(x)}</${k}>` : esc(x)).join("") + sign;
}

const note = (/** @type {string} */ s) => '<div class="muted sm">' + esc(s) + "</div>";
const peakName = (/** @type {number} */ off) => off ? `M+${off}` : "M";

/** the sticks as SVG: one focusable group per stick (roving tabindex), a target at least 24 px wide */
function chartSvg(/** @type {Stick[]} */ p, /** @type {string} */ xl, /** @type {Helpers["t"]} */ t) {
  const lo = p[0].mz, hi = p[p.length - 1].mz, pad = Math.max(0.6, (hi - lo) * 0.08), x0 = lo - pad, x1 = hi + pad;
  const X = (/** @type {number} */ v) => ML + (v - x0) / (x1 - x0) * (W - ML - MR), Y = (/** @type {number} */ r) => MT + (1 - r / 100) * (H - MT - MB);
  const gap = p.length > 1 ? Math.min(...p.slice(1).map((s, i) => X(s.mz) - X(p[i].mz))) : 40, tw = Math.max(24, Math.min(40, gap));
  const axes = `<line x1="${ML}" y1="${Y(0)}" x2="${W - MR}" y2="${Y(0)}" stroke="var(--muted)"/><line x1="${ML}" y1="${MT}" x2="${ML}" y2="${Y(0)}" stroke="var(--muted)"/>`
    + [0, 50, 100].map(r => `<text x="${ML - 4}" y="${Y(r) + 3}" text-anchor="end" font-size="9" fill="var(--muted)">${r}</text>`).join("")
    + `<text x="${(ML + W - MR) / 2}" y="${H - 2}" text-anchor="middle" font-size="10" fill="var(--muted)" font-style="italic">${esc(xl)}</text>`;
  const sticks = p.map((s, i) => {
    const x = X(s.mz), label = t("draw.anal.stick", { peak: peakName(s.off), x: xl, mz: s.mz.toFixed(5), rel: s.rel.toFixed(2) });
    return `<g class="an-st" data-i="${i}" tabindex="${i ? -1 : 0}" role="img" aria-label="${esc(label)}"><title>${esc(label)}</title>`
      + `<rect x="${x - tw / 2}" y="${MT}" width="${tw}" height="${Y(0) - MT + 14}" fill="transparent"/>`
      + `<line x1="${x}" y1="${Y(0)}" x2="${x}" y2="${Y(s.rel)}" stroke="var(--lv1)" stroke-width="3"/>`
      + (p.length <= 10 ? `<text x="${x}" y="${Y(0) + 11}" text-anchor="middle" font-size="9" fill="var(--ink)">${peakName(s.off)}</text>` : "") + "</g>";
  }).join("");
  return `<svg id="anal-chart" viewBox="0 0 ${W} ${H}" width="100%" style="max-width:${W}px;display:block" role="group" aria-label="${esc(t("draw.anal.chart"))}">${axes}${sticks}</svg>`;
}

/** mounts the card (draw.js imports this module when the card is first opened): `refresh` asks the numbers while it is open and the structure changes */
export function mount(/** @type {Helpers} */ h) {
  const card = /** @type {HTMLDetailsElement | null} */ (document.getElementById("anal-card")), body = document.getElementById("anal-body");
  if (!card || !body) return { refresh: () => {} };
  let lastKey = "", run = 0, showPattern = false, /** @type {any} */ cur = null;
  const t = h.t;
  function render() {
    if (!body) return;
    if (!cur) return;
    if (cur.error) { body.innerHTML = note(cur.error); return; }
    const s = cur.structure, ion = s.charge !== 0, xl = ion ? "m/z" : t("draw.anal.mass");
    const row = (/** @type {string} */ k, /** @type {string} */ v) => `<tr><th scope="row" style="text-align:left;font-weight:400">${esc(t(k))}</th><td class="num">${v}</td></tr>`;
    let html = `<div style="display:flex;gap:6px;align-items:center;margin:2px 0 4px"><span id="anal-formula" style="flex:1;font-size:15px">${formulaHtml(s)}</span>`
      + `<button type="button" class="sm" id="anal-copy" data-text="${esc(s.text)}" style="min-height:24px">${esc(t("draw.anal.copy"))}</button></div>`
      + `<table class="sm" id="anal-nums">${row("draw.anal.monoisotopic", s.mass.toFixed(5))}${ion ? row("draw.anal.mz", s.mz.toFixed(5)) : ""}${row("draw.anal.nominal", String(s.nominal))}${row("draw.anal.average", cur.average.toFixed(4))}</table>`
      + `<table class="sm" id="anal-comp" style="margin-top:4px"><caption style="text-align:left;font-weight:600">${esc(t("draw.anal.composition"))}</caption><tr><th scope="col">${esc(t("draw.anal.element"))}</th><th scope="col" class="num">${esc(t("draw.anal.count"))}</th><th scope="col" class="num">${esc(t("draw.anal.pct"))}</th></tr>`
      + cur.composition.map((/** @type {any} */ c) => `<tr><td>${esc(c.el)}</td><td class="num">${c.count}</td><td class="num">${c.pct.toFixed(2)}</td></tr>`).join("") + "</table>"
      + `<button type="button" id="anal-show" aria-expanded="${showPattern}" aria-controls="anal-pattern" style="margin-top:6px;min-height:24px">${esc(t(showPattern ? "draw.anal.hide" : "draw.anal.show"))}</button>`;
    const W_ = /** @type {any} */ (window), hr = !!(W_.HR && W_.COMP && W_.E && W_.HR.anyHr(W_.E.files));
    if (showPattern) {
      const title = t("draw.anal.title", { formula: s.text });
      html += `<div id="anal-pattern" style="margin-top:6px"><div class="sm" style="font-weight:600" id="anal-title">${esc(title)}</div><div class="muted sm">${esc(t("draw.anal.theory"))}</div>`
        + chartSvg(cur.pattern, xl, t)
        + `<div class="bar" style="margin-top:4px"><button type="button" class="sm" id="anal-table" style="min-height:24px">${esc(t("draw.anal.table"))}</button>`
        + (hr ? `<button type="button" class="sm" id="anal-comp-go" style="min-height:24px">${esc(t("draw.anal.composition.go"))}</button>` : "") + "</div></div>";
    }
    body.innerHTML = html;
  }
  /** the data of the chart, for TDATI */
  function tableOf() {
    const s = cur.structure, xl = s.charge ? "m/z" : t("draw.anal.mass");
    return { title: t("draw.anal.title", { formula: s.text }), note: t("draw.anal.theory"), head: [t("draw.anal.peak"), xl, t("draw.anal.rel"), t("draw.anal.abund")], rowHeader: true,
      rows: cur.pattern.map((/** @type {Stick} */ p) => [peakName(p.off), p.mz.toFixed(5), p.rel.toFixed(3), p.abund.toFixed(4)]) };
  }
  body.addEventListener("click", e => {
    const b = /** @type {HTMLElement} */ (e.target).closest("button"); if (!b || !cur || cur.error) return;
    if (b.id === "anal-copy") h.copy(b.dataset.text || "");
    else if (b.id === "anal-show") { showPattern = !showPattern; render(); const n = document.getElementById("anal-show"); if (n) n.focus(); }
    else if (b.id === "anal-table") h.table(tableOf());
    else if (b.id === "anal-comp-go") { const io = cur.ions[0]; /** @type {any} */ (window).COMP.open({ mz: io.mz, ion: io.ion, polarity: /-$/.test(io.ion) ? "negative" : "positive" }); }
  });
  body.addEventListener("keydown", e => {
    const g = /** @type {HTMLElement} */ (e.target).closest(".an-st"); if (!g) return;
    const all = /** @type {HTMLElement[]} */ ([...body.querySelectorAll(".an-st")]), i = all.indexOf(/** @type {HTMLElement} */ (g));
    const j = e.key === "ArrowRight" || e.key === "ArrowUp" ? i + 1 : e.key === "ArrowLeft" || e.key === "ArrowDown" ? i - 1 : e.key === "Home" ? 0 : e.key === "End" ? all.length - 1 : -2;
    if (j === -2) { if (e.key === "t" || e.key === "T") { e.preventDefault(); h.table(tableOf()); } return; }
    e.preventDefault();
    const k = Math.max(0, Math.min(all.length - 1, j));
    all.forEach((x, n) => { x.tabIndex = n === k ? 0 : -1; }); all[k].focus();
  });
  async function refresh() {
    if (!card || !body || !card.open) return;
    const atoms = h.atoms();
    if (!atoms || !atoms.length) { lastKey = ""; cur = null; body.innerHTML = note(t("draw.anal.select")); return; }
    const key = JSON.stringify({ atoms: atoms.map(atomOf) });
    if (key === lastKey && cur) return;
    lastKey = key; const me = ++run;
    let r;
    try { r = JSON.parse(await ask(key)); }
    catch (e) {
      let label = "";
      try { const j = JSON.parse(String(e)); label = (j.params && j.params.label) || ""; } catch (_) { /* not an engine error */ }
      r = { error: label ? t("draw.anal.error", { label }) : t("draw.anal.off") };
      if (!label) lastKey = "";                                                 // the module did not load: ask again at the next change
    }
    if (me !== run) return;
    cur = r; render();
  }
  return { refresh };
}
