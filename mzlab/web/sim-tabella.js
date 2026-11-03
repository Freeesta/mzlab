// @ts-check
// The SIM table of the Data view (#simtab, between the sample table and the panels): with SIM files loaded (kind «mrm» with Q1-only transitions, e.g. a SCIEX
// .wiff in SIM), every SIM channel of every file integrated by the engine (`api/simtab`, mzlab_core::simtab: the highest peak of the whole trace, the defaults
// of chrom.transitions), one row per channel and one column per sample, area or apex RT; CSV = `mzlab-cli sim-table` after the provenance line.
// Called by syncWork (area-lavoro.js) whenever the work area may have changed. docs/agenti/transizioni.md.
const G = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => G.I18N.t(k, p);
const esc = (/** @type {unknown} */ s) => String(s == null ? "" : s).replace(/[&<>"]/g, c => (/** @type {Record<string, string>} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }))[c]);

/** @type {{ sig: string, ks: number[], out: "area" | "apex_rt", csv: Record<string, string>, busy: number }} */
const st = { sig: "", ks: [], out: "area", csv: {}, busy: 0 };

/** The rows of a CSV with quoted fields. @param {string} text */
function rows(text) {
  /** @type {string[][]} */ const out = [];
  for (const line of text.split("\n")) {
    if (!line) continue;
    /** @type {string[]} */ const cells = []; let cur = "", q = false;
    for (let i = 0; i < line.length; i++) {
      const c = line[i];
      if (q) { if (c === '"' && line[i + 1] === '"') { cur += '"'; i++; } else if (c === '"') q = false; else cur += c; }
      else if (c === '"') q = true; else if (c === ",") { cells.push(cur); cur = ""; } else cur += c;
    }
    cells.push(cur); out.push(cells);
  }
  return out;
}

/** The label and caption of each output (literal keys: tools/controlla_i18n.py reads them). */
const OUT = { area: () => [t("canali.simtab.area"), t("canali.simtab.area.cap")], apex_rt: () => [t("canali.simtab.apex_rt"), t("canali.simtab.apex_rt.cap")] };

const fmt = (/** @type {string} */ v, /** @type {number} */ d) => v === "" ? "" : Number(v).toLocaleString(G.I18N.lang === "it" ? "it-IT" : "en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });

/** @returns {HTMLElement} */
const card = () => /** @type {HTMLElement} */ (document.getElementById("simtab"));

/** The files the table reads: loaded transition files, in the order of the list. */
const candidates = () => (typeof E !== "undefined" ? E.files : []).filter((/** @type {any} */ f) => f && !f.gone && f.kind === "mrm").map((/** @type {any} */ f) => /** @type {number} */ (f.k));

function draw() {
  const el = card(), csv = st.csv[st.out];
  if (!el) return;
  if (!csv) { el.hidden = true; el.innerHTML = ""; return; }
  const r = rows(csv), head = r[0] || [], body = r.slice(1), samples = head.slice(2), d = st.out === "area" ? 1 : 3;
  const open = !el.querySelector("details") || /** @type {HTMLDetailsElement} */ (el.querySelector("details")).open;
  const btn = (/** @type {"area" | "apex_rt"} */ o) => `<button type="button" class="sec sm" data-o="${o}" aria-pressed="${st.out === o}">${esc(OUT[o]()[0])}</button>`;
  el.innerHTML = `<details${open ? " open" : ""}><summary><h3 id="simtab-h">${esc(t("canali.simtab.title", { rows: body.length, cols: samples.length }))}</h3></summary>
<div class="simtab-bar" role="group" aria-label="${esc(t("canali.simtab.show"))}">${btn("area")}${btn("apex_rt")}<button type="button" class="sec sm" data-dl="1">${esc(t("canali.simtab.csv"))}</button></div>
<p class="muted sm">${esc(t("canali.simtab.note"))}</p>
<div class="simtab-wrap" tabindex="0" role="region" aria-labelledby="simtab-h"><table class="qq-tab"><caption>${esc(OUT[st.out]()[1])}</caption>
<thead><tr><th scope="col">m/z (Q1)</th><th scope="col">${esc(t("canali.simtab.name"))}</th>${samples.map(s => `<th scope="col" class="num">${esc(s)}</th>`).join("")}</tr></thead>
<tbody>${body.map(c => `<tr><th scope="row">${esc(fmt(c[0], 1))}</th><td>${esc(c[1])}</td>${c.slice(2).map(v => `<td class="num"${v === "" ? ` aria-label="${esc(t("canali.simtab.none"))}"` : ""}>${v === "" ? "—" : esc(fmt(v, d))}</td>`).join("")}</tr>`).join("")}</tbody></table></div></details>`;
  el.hidden = false;
  el.querySelectorAll("[data-o]").forEach(b => b.addEventListener("click", () => { st.out = /** @type {"area" | "apex_rt"} */ (/** @type {HTMLElement} */ (b).dataset.o); draw(); /** @type {HTMLElement | null} */ (card().querySelector(`[data-o="${st.out}"]`))?.focus(); }));
  el.querySelector("[data-dl]")?.addEventListener("click", () => { G.dl(`sim-table-${st.out}.csv`, st.csv[st.out], "text/csv", { table: "simtab" }); });
}

/** Asks the engine for both tables of the files `ks`; a 400 (no SIM channel) or no engine hides the card. @param {number[]} ks */
async function compute(ks) {
  const n = ++st.busy, got = /** @type {Record<string, string>} */ ({});
  for (const o of ["area", "apex_rt"]) {
    try {
      const r = await fetch(`api/simtab?k=${ks.join(",")}&output=${o}`);
      if (!r.ok) break;
      got[o] = await r.text();
    } catch (_) { break; }
  }
  if (n !== st.busy) return;
  st.csv = got.area && got.apex_rt ? got : {};
  draw();
}

/** The table follows the loaded files; nothing is asked when they did not change. */
function sync() {
  const ks = candidates(), w = document.getElementById("v-workspace"), shown = !!w && (w.dataset.work === "panels" || w.dataset.work === "table");
  const sig = ks.join(",");
  if (sig !== st.sig) {
    st.sig = sig; st.ks = ks; st.csv = {};
    if (ks.length) compute(ks); else draw();
  }
  const el = card();
  if (el && st.csv[st.out]) el.hidden = !shown;
}

G.SIMTAB = { sync, state: () => ({ ks: st.ks, out: st.out, csv: { ...st.csv } }) };
sync();
