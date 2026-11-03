// @ts-check
// The SIM table of the Data view (#simtab, between the sample table and the panels): with SIM files loaded (kind «mrm» with Q1-only transitions, e.g. a SCIEX
// .wiff in SIM), every SIM channel of every file integrated by the engine (`api/simtab`, mzlab_core::simtab: every resolved peak of the trace), as a pivot
// (one row per sample, one column per channel, or swapped) or as the peak table; each view is asked once, its CSV = `mzlab-cli sim-table` after the
// provenance line. Called by syncWork (area-lavoro.js) whenever the work area may have changed. docs/agenti/transizioni.md.
const G = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => G.I18N.t(k, p);

/** The outputs of simtab::OUTPUTS with their labels (literal keys: tools/controlla_i18n.py reads them). */
const OUTS = () => /** @type {[string, string][]} */ ([["area_sum", t("canali.simtab.o.area_sum")], ["area_max", t("canali.simtab.o.area_max")], ["height_max", t("canali.simtab.o.height_max")],
  ["apex_rt", t("canali.simtab.o.apex_rt")], ["n_peaks", t("canali.simtab.o.n_peaks")], ["peaks", t("canali.simtab.o.peaks")]]);
/** The fixed column names of the CSV shown in the user's language; sample names and channel m/z stay as they are. */
const HEADS = () => /** @type {Record<string, string>} */ ({ sample: t("canali.simtab.h.sample"), channel: t("canali.simtab.h.channel"), q1: t("canali.simtab.h.channel"), name: t("canali.simtab.h.name"),
  peak: t("canali.simtab.h.peak"), apex_rt: t("canali.simtab.h.apex_rt"), area: t("canali.simtab.h.area"), height: t("canali.simtab.h.height"), snr: t("canali.simtab.h.snr"), fwhm_s: t("canali.simtab.h.fwhm_s") });

/** @type {{ sig: string, ks: number[], out: string, layout: "samples" | "channels", csv: Record<string, string>, size: { channels: number, samples: number } | null, busy: number }} */
const st = { sig: "", ks: [], out: "area_sum", layout: "samples", csv: {}, size: null, busy: 0 };
const view = () => `${st.out}|${st.out === "peaks" ? "samples" : st.layout}`;

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

/** A number of the CSV in the user's locale with the decimals the engine wrote; any other text as it is. @param {string} v */
const fmt = v => {
  const m = /^-?\d+(?:\.(\d+))?$/.exec(v);
  if (!m) return v;
  const d = m[1] ? m[1].length : 0;
  return Number(v).toLocaleString(G.I18N.lang === "it" ? "it-IT" : "en-GB", { minimumFractionDigits: d, maximumFractionDigits: d });
};

/** @returns {HTMLElement} */
const card = () => /** @type {HTMLElement} */ (document.getElementById("simtab"));

/** The files the table reads: loaded files of an experiment without spectra (transitions, esperimenti.js), in the order of the list (the injection order
 * the user loaded); the engine keeps their SIM channels. */
const candidates = () => (typeof E !== "undefined" ? E.files : []).filter((/** @type {any} */ f) => f && !f.gone && ESP.voce(f.kind)?.spettri === false).map((/** @type {any} */ f) => /** @type {number} */ (f.k));

function draw() {
  const el = card(), csv = st.csv[view()];
  if (!el) return;
  if (!st.size) { el.hidden = true; el.innerHTML = ""; return; }
  const open = !el.querySelector("details") || /** @type {HTMLDetailsElement} */ (el.querySelector("details")).open;
  const r = csv ? rows(csv) : [], head = r[0] || [], body = r.slice(1), heads = HEADS(), lead = st.out === "peaks" ? 4 : st.layout === "channels" ? 2 : 1;
  const label = (OUTS().find(o => o[0] === st.out) || ["", ""])[1];
  const opts = OUTS().map(([o, l]) => h`<option value="${o}"${raw(o === st.out ? " selected" : "")}>${l}</option>`);
  const ths = head.map((x, i) => h`<th scope="col"${raw(i >= lead ? ' class="num"' : "")}>${heads[x] || x}</th>`);
  const td = (/** @type {string} */ v, /** @type {number} */ i) => v === "" ? h`<td class="num" aria-label="${t("canali.simtab.none")}">—</td>` : h`<td${raw(i >= lead || /^-?\d/.test(v) ? ' class="num"' : "")}>${fmt(v)}</td>`;
  const trs = body.map(c => h`<tr><th scope="row">${c[0]}</th>${raw(c.slice(1).map((v, i) => td(v, i + 1)).join(""))}</tr>`);
  el.innerHTML = h`<details${raw(open ? " open" : "")}><summary><h3 id="simtab-h">${t("canali.simtab.title", st.size)}</h3></summary>
<div class="simtab-bar"><label>${t("canali.simtab.show")} <select data-o>${raw(opts.join(""))}</select></label>
<button type="button" class="sec sm" data-flip aria-pressed="${String(st.layout === "channels")}"${raw(st.out === "peaks" ? " disabled" : "")}>${t("canali.simtab.flip")}</button><button type="button" class="sec sm" data-dl${raw(csv ? "" : " disabled")}>${t("canali.simtab.csv")}</button></div>
<p class="muted sm">${t("canali.simtab.note")}</p>
<div class="simtab-wrap" tabindex="0" role="region" aria-labelledby="simtab-h"><table class="qq-tab"><caption>${label}</caption>
<thead><tr>${raw(ths.join(""))}</tr></thead><tbody>${raw(trs.join(""))}</tbody></table></div></details>`;
  el.hidden = false;
  const sel = /** @type {HTMLSelectElement} */ (el.querySelector("[data-o]"));
  sel.addEventListener("change", () => { st.out = sel.value; show("[data-o]"); });
  el.querySelector("[data-flip]")?.addEventListener("click", () => { st.layout = st.layout === "samples" ? "channels" : "samples"; show("[data-flip]"); });
  el.querySelector("[data-dl]")?.addEventListener("click", () => { G.dl(`sim-table-${view().replace("|", "-")}.csv`, st.csv[view()], "text/csv", { table: "simtab" }); });
}

/** Draws the view, asking the engine for it first when it was never asked; focus back on the control `q`. @param {string} q */
async function show(q) {
  if (!st.csv[view()]) await ask(st.ks, view());
  draw();
  /** @type {HTMLElement | null} */ (card().querySelector(q))?.focus();
}

/** The CSV of view `v` («output|layout») of the files `ks`, kept in `st.csv`; false when the engine refused it. @param {number[]} ks @param {string} v */
async function ask(ks, v) {
  const [output, layout] = v.split("|");
  try {
    const r = await fetch(`api/simtab?k=${ks.join(",")}&output=${output}&layout=${layout}`);
    if (!r.ok) return false;
    const text = await r.text();
    if (ks.join(",") === st.sig) st.csv[v] = text;
    return true;
  } catch (_) { return false; }
}

/** The first view of the files `ks` (sum of the areas, one row per sample) gives the size of the table; a 400 (no SIM channel) or no engine hides the card. @param {number[]} ks */
async function compute(ks) {
  const n = ++st.busy, ok = await ask(ks, "area_sum|samples");
  if (n !== st.busy) return;
  const r = ok ? rows(st.csv["area_sum|samples"] || "") : [];
  st.size = r.length ? { channels: r[0].length - 1, samples: r.length - 1 } : null;
  if (st.size && !st.csv[view()]) await ask(ks, view());
  if (n === st.busy) draw();
}

/** The table follows the loaded files; nothing is asked when they did not change. */
function sync() {
  const ks = candidates(), w = document.getElementById("v-workspace"), shown = !!w && (w.dataset.work === "panels" || w.dataset.work === "table");
  const sig = ks.join(",");
  if (sig !== st.sig) {
    st.sig = sig; st.ks = ks; st.csv = {}; st.size = null;
    if (ks.length) compute(ks); else { st.busy++; draw(); }
  }
  const el = card();
  if (el && st.size) el.hidden = !shown;
}

G.SIMTAB = { sync, state: () => ({ ks: st.ks, out: st.out, layout: st.layout, size: st.size, csv: { ...st.csv } }), ask: (/** @type {string} */ v) => ask(st.ks, v).then(() => st.csv[v]) };
sync();
