// @ts-check
// The SIM table of the Data view (#simtab, between the sample table and the panels): with SIM files loaded (kind «mrm» with Q1-only transitions, e.g. a SCIEX
// .wiff in SIM), every SIM channel of every file integrated by the engine (`api/simtab`, mzlab_core::simtab: one column per m/z, every resolved peak), as a
// pivot (one row per sample, rows by sidebar group with a folding header row) or swapped, or as the peak table; its CSV = `mzlab-cli sim-table` after the
// provenance line. Columns picked with their chart button draw a kinetic plot (QAQC.chart over the «Tempo» of the file table, one line per column and group,
// blanks dashed grey). Areas and heights in I18N.sciE. Called by syncWork (area-lavoro.js) whenever the work area may have changed. docs/agenti/transizioni.md.
const G = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => G.I18N.t(k, p);

/** The outputs of simtab::OUTPUTS with their labels (literal keys: tools/controlla_i18n.py reads them). */
const OUTS = () => /** @type {[string, string][]} */ ([["area_sum", t("canali.simtab.o.area_sum")], ["area_max", t("canali.simtab.o.area_max")], ["height_max", t("canali.simtab.o.height_max")],
  ["apex_rt", t("canali.simtab.o.apex_rt")], ["n_peaks", t("canali.simtab.o.n_peaks")], ["peaks", t("canali.simtab.o.peaks")]]);
/** The fixed column names of the CSV shown in the user's language; sample names and channel m/z stay as they are. */
const HEADS = () => /** @type {Record<string, string>} */ ({ sample: t("canali.simtab.h.sample"), group: t("canali.simtab.h.group"), channel: t("canali.simtab.h.channel"), q1: t("canali.simtab.h.channel"), name: t("canali.simtab.h.name"),
  peak: t("canali.simtab.h.peak"), apex_rt: t("canali.simtab.h.apex_rt"), area: t("canali.simtab.h.area"), height: t("canali.simtab.h.height"), snr: t("canali.simtab.h.snr"), fwhm_s: t("canali.simtab.h.fwhm_s") });

/** @typedef {{ k: number, g: string, f: any }} Entry */
/** @type {{ sig: string, files: Entry[], out: string, layout: "samples" | "channels", csv: Record<string, string>, size: { channels: number, samples: number } | null, busy: number, sel: Set<string>, shut: Set<string>, ksig: string }} */
const st = { sig: "", files: [], out: "area_sum", layout: "samples", csv: {}, size: null, busy: 0, sel: new Set(), shut: new Set(), ksig: "" };
const view = () => `${st.out}|${st.out === "peaks" ? "samples" : st.layout}`;
/** The pivot the kinetic plot reads: the chosen value, the sum of the areas for the peak table. */
const kview = () => `${st.out === "peaks" ? "area_sum" : st.out}|samples`;

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

/** An area or height of the CSV (7.84077E+04) in the one form of the page (7.841E+04); any other cell as the engine wrote it. @param {string} v */
const fmt = v => /^-?\d(\.\d+)?E[+-]\d+$/.test(v) ? G.I18N.sciE(Number(v)) : v;
/** A channel column of the CSV as shown: the m/z, or «ione N» (`ion N`, ` [group]` when kept apart per group). @param {string} c */
const shown = c => c.replace(/^ion (\d+)/, (_, n) => t("canali.sim.ion", { n: +n })).replace(/ \[(.*)\]( #\d+)?$/, (_, g, k) => `${k || ""} · ${g}`);

/** @returns {HTMLElement} */
const card = () => /** @type {HTMLElement} */ (document.getElementById("simtab"));

/** The files the table reads: loaded files of an experiment without spectra (transitions, esperimenti.js), the sidebar groups first in their order
 * (`E.ugOrder`, then as they appear), then the files without one, each in the order of the list (the injection order the user loaded). */
function candidates() {
  const all = (typeof E !== "undefined" ? E.files : []).filter((/** @type {any} */ f) => f && !f.gone && ESP.voce(f.kind)?.spettri === false);
  const order = /** @type {string[]} */ ([...(E.ugOrder || [])]);
  for (const f of all) if (f.ugrp && !order.includes(f.ugrp)) order.push(f.ugrp);
  const at = (/** @type {string} */ g) => g ? order.indexOf(g) : order.length;
  return all.map((/** @type {any} */ f) => ({ k: /** @type {number} */ (f.k), g: f.ugrp || "", f })).sort((a, b) => at(a.g) - at(b.g));
}

/** The time of a file in the unit of the file table, null when none is set; its role for the marks (QAQC.roleOf). @param {any} f */
const timeOf = f => (f.time == null || !isFinite(+f.time) ? null : typeof timeShown === "function" ? timeShown(+f.time, tunitGet()) : +f.time);
const roleOf = (/** @type {any} */ f) => (G.QAQC ? G.QAQC.roleOf(f) : f.role === "blank" ? "blank" : "sample");

/** The table body with a folding header row before each group. @param {string[][]} body @param {boolean} gcol @param {number} lead @param {number} ncol */
function bodyHtml(body, gcol, lead, ncol) {
  const pick = st.layout === "channels" && st.out !== "peaks";
  const td = (/** @type {string} */ v, /** @type {number} */ i) => v === "" ? h`<td class="num" aria-label="${t("canali.simtab.none")}">—</td>` : h`<td${raw(i >= lead || /^-?\d/.test(v) ? ' class="num"' : "")}>${fmt(v)}</td>`;
  /** @type {string[]} */ const out = []; let prev = null;
  for (const c of body) {
    const g = gcol ? c[1] : "", cells = gcol ? [c[0], ...c.slice(2)] : c, shut = gcol && st.shut.has(g);
    if (gcol && g !== prev) {
      const n = body.filter(r => r[1] === g).length, name = g || t("canali.simtab.nogroup");
      out.push(h`<tr class="simtab-g"><th scope="rowgroup" colspan="${ncol}"><button type="button" class="sec sm" data-g="${g}" aria-expanded="${String(!shut)}">${name} (${n})</button></th></tr>`);
      prev = g;
    }
    const head = pick ? h`<th scope="row">${shown(cells[0])}${raw(pickBtn(cells[0]))}</th>` : h`<th scope="row">${shown(cells[0])}</th>`;
    out.push(h`<tr${raw(shut ? " hidden" : "")}>${raw(head)}${raw(cells.slice(1).map((v, i) => td(v, i + 1)).join(""))}</tr>`);
  }
  return out.join("");
}

/** The button that puts column `c` (its CSV label) in the kinetic plot. @param {string} c */
const pickBtn = c => h`<button type="button" class="simtab-pick" data-c="${c}" aria-pressed="${String(st.sel.has(c))}" aria-label="${t("canali.simtab.pick", { c: shown(c) })}" title="${t("canali.simtab.pick", { c: shown(c) })}"><svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 13.5h12M3 11l3.5-4 3 2.5L13.5 4"/></svg></button>`;

/** The kinetic plot of the picked columns over the «Tempo» of the files and its data table; a one-line hint when nothing can be drawn. */
function kinetic() {
  const csv = st.csv[kview()];
  if (!st.sel.size) return { html: h`<p class="muted sm" data-kin-hint>${t("canali.simtab.kin.pick")}</p>`, data: null };
  if (!csv) return { html: "", data: null };
  const r = rows(csv), head = r[0] || [], body = r.slice(1), gcol = head[1] === "group";
  const fs = st.files.slice(0, body.length), xs = fs.map(e => timeOf(e.f)), roles = fs.map(e => roleOf(e.f));
  if (!xs.some((x, i) => x != null && roles[i] !== "blank")) return { html: h`<p class="muted sm" data-kin-hint>${t("canali.simtab.kin.notime")}</p>`, data: null };
  const cols = head.map((c, j) => [c, j]).filter(([c, j]) => +j >= (gcol ? 2 : 1) && st.sel.has(/** @type {string} */ (c)));
  const groups = gcol ? [...new Set(body.map(c => c[1]))] : [""], pal = /** @type {string[]} */ (typeof PAL !== "undefined" && PAL.length ? PAL : ["var(--lv1)", "var(--lv3)", "var(--lv4)"]);
  const num = (/** @type {string} */ v) => (v === "" ? null : Number(v));
  const xv = xs.filter((x, i) => x != null && roles[i] !== "blank").map(Number), x0 = minOf(xv), x1 = maxOf(xv);
  // a blank without a time is drawn as a dashed level across the time span: two points at the ends of the axis
  const X = [...xs, x0, x1], R = [...roles, "blank", "blank"];
  /** @type {any[]} */ const lines = []; let s = 0;
  for (const [c, j] of cols) {
    for (const g of groups) {
      const vals = body.map((row, i) => (roles[i] !== "blank" && (!gcol || row[1] === g) ? num(row[+j]) : null));
      if (vals.some(v => v != null)) lines.push({ label: shown(/** @type {string} */ (c)) + (g ? ` · ${g}` : ""), vals: [...vals, null, null], color: pal[s++ % pal.length] });
    }
    const bl = body.map((row, i) => (roles[i] === "blank" ? num(row[+j]) : null)), timed = bl.some((v, i) => v != null && xs[i] != null);
    const lone = bl.filter((v, i) => v != null && xs[i] == null), lvl = lone.length ? lone.reduce((a, b) => a + /** @type {number} */ (b), 0) / lone.length : null;
    if (timed || lvl != null) lines.push({ label: `${shown(/** @type {string} */ (c))} · ${t("qaqc.role.blank")}`, vals: timed ? [...bl.map((v, i) => (xs[i] == null ? null : v)), null, null] : [...bl.map(() => null), lvl, lvl], color: "var(--muted)", dash: "4 3" });
  }
  if (!lines.some(l => !l.dash && l.vals.some((/** @type {number | null} */ v, /** @type {number} */ i) => v != null && X[i] != null))) return { html: h`<p class="muted sm" data-kin-hint>${t("canali.simtab.kin.notime")}</p>`, data: null };
  const area = /^(area|height)/.test(st.out === "peaks" ? "area_sum" : st.out), yl = (OUTS().find(o => o[0] === kview().split("|")[0]) || ["", ""])[1];
  const xlab = `${t("load.table.time.col")} (${typeof tunitGet === "function" ? tunitGet() : "min"})`, title = t("canali.simtab.kin.title", { value: yl });
  const svg = G.QAQC ? G.QAQC.chart(title, lines, R, yl, { xs: X, xlab, yfmt: area ? (/** @type {number} */ v) => G.I18N.sciE(v) : (/** @type {number} */ v) => G.I18N.num(v, 2) }) : "";
  const dhead = [t("canali.simtab.h.sample"), ...(gcol ? [t("canali.simtab.h.group")] : []), xlab, t("load.table.type"), ...cols.map(([c]) => shown(/** @type {string} */ (c)))];
  const drows = body.map((row, i) => [row[0], ...(gcol ? [row[1]] : []), xs[i] == null ? "" : G.I18N.num(xs[i]), G.QAQC ? G.QAQC.roleName(roles[i]) : roles[i], ...cols.map(([, j]) => fmt(row[+j]))]);
  const legend = lines.map(l => h`<li><svg width="18" height="8" aria-hidden="true"><line x1="0" y1="4" x2="18" y2="4" stroke="${l.color}" stroke-width="2" stroke-dasharray="${l.dash || ""}"/></svg> ${l.label}</li>`).join("");
  return { html: h`<div class="simtab-kin-c">${raw(svg)}</div><ul class="simtab-leg">${raw(legend)}</ul><button type="button" class="sec sm" data-kdata title="${t("tdati.btn.title")}">${t("tdati.title")}</button>`, data: { title, head: dhead, rows: drows } };
}

function draw() {
  const el = card(), csv = st.csv[view()];
  if (!el) return;
  if (!st.size) { el.hidden = true; el.innerHTML = ""; return; }
  const open = !el.querySelector("details") || /** @type {HTMLDetailsElement} */ (el.querySelector("details")).open;
  const r = csv ? rows(csv) : [], head = r[0] || [], body = r.slice(1), heads = HEADS(), gcol = head[1] === "group";
  const lead = st.out === "peaks" ? 4 : st.layout === "channels" ? 2 : 1, cols = gcol ? [head[0], ...head.slice(2)] : head, pick = st.out !== "peaks" && st.layout === "samples";
  const label = (OUTS().find(o => o[0] === st.out) || ["", ""])[1];
  const opts = OUTS().map(([o, l]) => h`<option value="${o}"${raw(o === st.out ? " selected" : "")}>${l}</option>`);
  const ths = cols.map((x, i) => i >= lead && pick ? h`<th scope="col" class="num">${shown(x)}${raw(pickBtn(x))}</th>` : h`<th scope="col"${raw(i >= lead ? ' class="num"' : "")}>${heads[x] || shown(x)}</th>`);
  const kin = kinetic();
  el.innerHTML = h`<details${raw(open ? " open" : "")}><summary><h3 id="simtab-h">${t("canali.simtab.title", st.size)}</h3></summary>
<div class="simtab-bar"><label>${t("canali.simtab.show")} <select data-o>${raw(opts.join(""))}</select></label>
<button type="button" class="sec sm" data-flip aria-pressed="${String(st.layout === "channels")}"${raw(st.out === "peaks" ? " disabled" : "")}>${t("canali.simtab.flip")}</button><button type="button" class="sec sm" data-dl${raw(csv ? "" : " disabled")}>${t("canali.simtab.csv")}</button></div>
<div class="simtab-wrap" tabindex="0" role="region" aria-labelledby="simtab-h"><table class="qq-tab"><caption>${label}</caption>
<thead><tr>${raw(ths.join(""))}</tr></thead><tbody>${raw(bodyHtml(body, gcol, lead, cols.length))}</tbody></table></div>
<section class="simtab-kin" aria-label="${t("canali.simtab.kin.aria")}">${raw(kin.html)}</section></details>`;
  el.hidden = false;
  const sel = /** @type {HTMLSelectElement} */ (el.querySelector("[data-o]"));
  sel.addEventListener("change", () => { st.out = sel.value; show("[data-o]"); });
  el.querySelector("[data-flip]")?.addEventListener("click", () => { st.layout = st.layout === "samples" ? "channels" : "samples"; show("[data-flip]"); });
  el.querySelector("[data-dl]")?.addEventListener("click", () => { G.dl(`sim-table-${view().replace("|", "-")}.csv`, st.csv[view()], "text/csv", { table: "simtab" }); });
  el.querySelectorAll("[data-g]").forEach(b => b.addEventListener("click", () => {
    const g = /** @type {HTMLElement} */ (b).dataset.g || "";
    if (st.shut.has(g)) st.shut.delete(g); else st.shut.add(g);
    draw(); /** @type {HTMLElement | null} */ (card().querySelector(`[data-g="${CSS.escape(g)}"]`))?.focus();
  }));
  el.querySelectorAll("[data-c]").forEach(b => b.addEventListener("click", () => {
    const c = /** @type {HTMLElement} */ (b).dataset.c || "";
    if (st.sel.has(c)) st.sel.delete(c); else st.sel.add(c);
    show(`[data-c="${CSS.escape(c)}"]`);
  }));
  const svg = /** @type {HTMLElement | null} */ (el.querySelector(".simtab-kin svg"));
  if (kin.data && G.TDATI) {
    const d = kin.data;
    el.querySelector("[data-kdata]")?.addEventListener("click", () => G.TDATI.table(d));
    if (svg) { svg.tabIndex = 0; G.TDATI.attach(svg, () => d); }
  }
}

/** Draws the view, asking the engine for it (and for the pivot of the kinetic plot) first when never asked; focus back on the control `q`. @param {string} q */
async function show(q) {
  if (!st.csv[view()]) await ask(st.files, view());
  if (st.sel.size && !st.csv[kview()]) await ask(st.files, kview());
  draw();
  /** @type {HTMLElement | null} */ (card().querySelector(q))?.focus();
}

/** The key of a set of files and groups. @param {Entry[]} fs */
const sigOf = fs => fs.map(e => `${e.k}:${e.g}`).join(",");

/** The CSV of view `v` («output|layout») of the files `fs` (with their groups), kept in `st.csv`; false when the engine refused it. @param {Entry[]} fs @param {string} v */
async function ask(fs, v) {
  const [output, layout] = v.split("|");
  try {
    const g = fs.some(e => e.g) ? `&g=${encodeURIComponent(JSON.stringify(fs.map(e => e.g)))}` : "";
    const r = await fetch(`api/simtab?k=${fs.map(e => e.k).join(",")}${g}&output=${output}&layout=${layout}`);
    if (!r.ok) return false;
    const text = await r.text();
    if (sigOf(fs) === st.sig) st.csv[v] = text;
    return true;
  } catch (_) { return false; }
}

/** The first view of the files `fs` (sum of the areas, one row per sample) gives the size of the table; a 400 (no SIM channel) or no engine hides the card. @param {Entry[]} fs */
async function compute(fs) {
  const n = ++st.busy, ok = await ask(fs, "area_sum|samples");
  if (n !== st.busy) return;
  const r = ok ? rows(st.csv["area_sum|samples"] || "") : [], gcol = r[0] && r[0][1] === "group";
  st.size = r.length ? { channels: r[0].length - (gcol ? 2 : 1), samples: r.length - 1 } : null;
  if (st.size && !st.csv[view()]) await ask(fs, view());
  if (st.size && st.sel.size && !st.csv[kview()]) await ask(fs, kview());
  if (n === st.busy) draw();
}

/** The table follows the loaded files and their groups (asked again only when they change); the kinetic plot follows their times and roles. */
function sync() {
  const fs = candidates(), w = document.getElementById("v-workspace"), shown = !!w && (w.dataset.work === "panels" || w.dataset.work === "table");
  const sig = sigOf(fs), ksig = fs.map(e => `${timeOf(e.f)}:${roleOf(e.f)}`).join(",") + (typeof tunitGet === "function" ? tunitGet() : "");
  if (sig !== st.sig) {
    st.sig = sig; st.files = fs; st.csv = {}; st.size = null; st.ksig = ksig;
    if (fs.length) compute(fs); else { st.busy++; draw(); }
  } else if (ksig !== st.ksig) { st.ksig = ksig; st.files = fs; if (st.size) draw(); }
  const el = card();
  if (el && st.size) el.hidden = !shown;
}

G.SIMTAB = { sync, state: () => ({ ks: st.files.map(e => e.k), groups: st.files.map(e => e.g), out: st.out, layout: st.layout, size: st.size, sel: [...st.sel], csv: { ...st.csv } }),
  ask: (/** @type {string} */ v) => ask(st.files, v).then(() => st.csv[v]) };
sync();
