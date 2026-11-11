// @ts-check
// The card «Manual integrations» of the Data view (#manint): every window drawn by hand on an ion or transition trace (the `p.ints` of the transition
// panels, kept in the session), measured again by the engine (`api/manint`, mzlab_core::manint) as one row per window grouped by channel, an optional
// sum row per channel; the CSV export is the engine's bytes, as `mzlab-cli manual-table`. Loaded by grafici.js on the first integration of such a panel.
const G = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => G.I18N.t(k, p);

/** The columns of the engine (manint::HEAD) with their labels (literal keys: tools/controlla_i18n.py reads them). */
const HEADS = () => /** @type {Record<string, string>} */ ({ channel: t("canali.manint.h.channel"), name: t("canali.manint.h.name"), sample: t("canali.manint.h.sample"),
  start_min: t("canali.manint.h.start"), end_min: t("canali.manint.h.end"), apex_rt_min: t("canali.manint.h.apex"), area: t("canali.manint.h.area"),
  height: t("canali.manint.h.height"), snr: t("canali.manint.h.snr"), baseline: t("canali.manint.h.base") });
const BASES = () => /** @type {Record<string, string>} */ ({ zero: t("int.base.zero"), edges: t("int.base.edges"), line: t("int.base.line") });

/** @typedef {{ p: any, it: any, k: number, tr: number, label: string }} Rec */
/** @type {{ sig: string, recs: Rec[], ks: number[], windows: string, csv: string, sums: boolean, busy: number, tm: any }} */
const st = { sig: "", recs: [], ks: [], windows: "[]", csv: "", sums: false, busy: 0, tm: 0 };

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

const card = () => /** @type {HTMLElement | null} */ (document.getElementById("manint"));
/** The sample name the engine writes (manint::sample_of). @param {string} n */
const sampleOf = n => n.replace(/\.mzml$/i, "");
/** The channel as the engine names it (manint::label_of), to find the record of a row. @param {any} tr @param {number} i */
const labelOf = (tr, i) => tr.q1 != null && tr.q3 != null ? `${tr.q1}>${tr.q3}` : tr.q1 != null ? `${tr.q1}` : `ion ${i + 1}`;
/** The window of a record as the engine reads it; null for a record drawn with the older window baseline (no `bm`). @param {any} it */
const baseOf = it => it.bm === "line" && it.bl ? { baseline: "line", line: it.bl } : it.bm === "edges" && it.base ? { baseline: "line", line: it.base }
  : it.bm === "zero" || it.bm === "edges" ? { baseline: it.bm } : null;

/** The records of the transition panels with the transition each names (`m|<file>|<id>`), one per trace and window, in the order of the files. */
async function records() {
  /** @type {Rec[]} */ const out = []; const seen = new Set();
  for (const p of (typeof E !== "undefined" ? E.panels : [])) {
    if (!p || !ESP.mrm(p) || !p.ints || !p.ints.length) continue;
    for (const it of p.ints) {
      const m = /^m\|(\d+)\|(.+)$/.exec(it.key || ""), sig = `${it.key}|${it.a}|${it.b}`, f = m && E.files[+m[1]];
      if (!m || !f || f.gone || !baseOf(it) || seen.has(sig)) continue;
      const list = await getMrm(+m[1]), tr = list.findIndex((/** @type {any} */ x) => String(x.id) === m[2]);
      if (tr < 0) continue;
      seen.add(sig); out.push({ p, it, k: +m[1], tr, label: labelOf(list[tr], tr) });
    }
  }
  return out;
}

/** The windows JSON of the records, files numbered in the order of `ks`. @param {Rec[]} recs @param {number[]} ks */
const windowsOf = (recs, ks) => JSON.stringify(recs.map(r => ({ file: ks.indexOf(r.k), tr: r.tr, a: Math.min(r.it.a, r.it.b), b: Math.max(r.it.a, r.it.b), ...baseOf(r.it) })));

/** The record of a row: same sample, channel and edges (the engine writes the edges at 0.001 min). @param {string[]} row */
function recOf(row) {
  return st.recs.find(r => sampleOf(E.files[r.k].file || "") === row[2] && r.label === row[0] && Math.min(r.it.a, r.it.b).toFixed(3) === row[3] && Math.max(r.it.a, r.it.b).toFixed(3) === row[4]) || null;
}

/** An area or height of the CSV (7.84077E+04) in the one form of the page; a baseline by its name. @param {string} v @param {string} col */
const fmt = (v, col) => col === "baseline" ? BASES()[v] || v : /^-?\d(\.\d+)?E[+-]\d+$/.test(v) ? G.I18N.sciE(Number(v)) : v;
/** A channel as shown: the m/z, «q1 > q3», or «ione N». @param {string} c */
const chShown = c => c.replace(/^ion (\d+)$/, (_, n) => t("canali.sim.ion", { n: +n })).replace(">", " > ");

function draw() {
  const el = card(); if (!el) return;
  const all = rows(st.csv), head = all[0] || [], body = all.slice(1);
  if (!body.length) { el.hidden = true; el.innerHTML = ""; return; }
  const H = HEADS(), n = body.filter(r => r[2] !== "sum" || r[3] !== "").length;
  const ths = head.map(c => h`<th scope="col"${raw(["channel", "name", "sample", "baseline"].includes(c) ? "" : ' class="num"')}>${H[c] || c}</th>`).join("");
  const trs = body.map((r, i) => {
    if (r[2] === "sum" && r[3] === "") return h`<tr class="manint-sum"><th scope="row">${chShown(r[0])}</th><td>${r[1]}</td><td>${t("canali.manint.sum")}</td><td></td><td></td><td></td><td class="num">${fmt(r[6], "area")}</td><td></td><td></td><td></td><td></td></tr>`;
    const rec = recOf(r), who = { ch: chShown(r[0]), s: r[2] };
    const act = rec ? h`<button type="button" class="sec sm" data-go="${i}" aria-label="${t("canali.manint.show.aria", who)}">${t("canali.manint.show")}</button> <button type="button" class="sec sm" data-del="${i}" aria-label="${t("canali.manint.del.aria", who)}">${t("canali.manint.del")}</button>` : "";
    return h`<tr><th scope="row">${chShown(r[0])}</th>${raw(r.slice(1).map((v, j) => h`<td${raw(j >= 2 && j <= 7 ? ' class="num"' : "")}>${fmt(v, head[j + 1])}</td>`).join(""))}<td>${raw(act)}</td></tr>`;
  }).join("");
  const open = !el.querySelector("details") || /** @type {HTMLDetailsElement} */ (el.querySelector("details")).open;
  el.innerHTML = h`<details${raw(open ? " open" : "")}><summary><h3 id="manint-h">${t("canali.manint.title", { n })}</h3></summary>
<div class="manint-bar"><label title="${t("canali.manint.sums.title")}"><input type="checkbox" data-sums${raw(st.sums ? " checked" : "")}> ${t("canali.manint.sums")}</label>
<button type="button" class="sec sm" data-dl>${t("canali.manint.csv")}</button>
<button type="button" class="sec sm" data-copy>${t("canali.manint.copy")}</button><span class="sm" role="status" aria-live="polite" data-ok></span></div>
<div class="manint-wrap" tabindex="0" role="region" aria-labelledby="manint-h"><table class="qq-tab"><caption>${t("canali.manint.caption")}</caption>
<thead><tr>${raw(ths)}<th scope="col">${t("canali.manint.h.act")}</th></tr></thead><tbody>${raw(trs)}</tbody></table></div>
<p class="muted sm">${t("canali.manint.note")}</p></details>`;
  const ws = document.getElementById("v-workspace");
  el.hidden = !(ws && (ws.dataset.work === "panels" || ws.dataset.work === "table"));
  el.querySelector("[data-sums]")?.addEventListener("change", e => { st.sums = /** @type {HTMLInputElement} */ (e.target).checked; ask().then(() => { draw(); /** @type {HTMLElement | null} */ (card()?.querySelector("[data-sums]"))?.focus(); }); });
  el.querySelector("[data-dl]")?.addEventListener("click", () => G.dl(st.sums ? "manual-integrations-sums.csv" : "manual-integrations.csv", st.csv, "text/csv", { table: "manint" }));
  el.querySelector("[data-copy]")?.addEventListener("click", () => copy(all));
  el.querySelectorAll("[data-go]").forEach(b => b.addEventListener("click", () => {
    const rec = recOf(body[Number(/** @type {HTMLElement} */ (b).dataset.go)]); if (!rec) return;
    G.reveal(rec.p); G.draw(rec.p);
  }));
  el.querySelectorAll("[data-del]").forEach(b => b.addEventListener("click", () => {
    const rec = recOf(body[Number(/** @type {HTMLElement} */ (b).dataset.del)]); if (rec) G.delInts(rec.p, [rec.it]);
  }));
}

/** The engine rows as TSV (raw numbers, so a spreadsheet reads them) after the provenance line; the outcome in the status line. @param {string[][]} all */
async function copy(all) {
  let ok = true;
  try { await G.clipboardCopy({ "text/plain": await PROV.tsv(all.map(r => r.join("\t")).join("\n") + "\n", { table: "manint" }) }); } catch (_) { ok = false; }
  const m = card()?.querySelector("[data-ok]"); if (!m) return;
  m.textContent = t(ok ? "canali.manint.copied" : "canali.manint.copy.fail"); clearTimeout(st.tm); st.tm = setTimeout(() => { m.textContent = ""; }, 3000);
}

/** Asks the engine for the table of the records; false when it refused or is missing. */
async function ask() {
  const n = ++st.busy;
  if (!st.recs.length) { st.csv = ""; return true; }
  try {
    const r = await fetch(`api/manint?k=${st.ks.join(",")}&sums=${st.sums ? 1 : 0}&w=${encodeURIComponent(st.windows)}`);
    const text = r.ok ? await r.text() : "";
    if (n === st.busy) st.csv = text;
    return r.ok;
  } catch (_) { return false; }
}

/** The table follows the integrations (asked again only when a window, a baseline or a file changes) and the work area. */
async function sync() {
  const recs = await records(), ks = [...new Set(recs.map(r => r.k))].sort((a, b) => a - b), windows = windowsOf(recs, ks), sig = ks.join(",") + windows;
  if (sig !== st.sig) {
    st.sig = sig; st.recs = recs; st.ks = ks; st.windows = windows;
    await ask();
    if (sig === st.sig) draw();
  } else {
    st.recs = recs;
    const el = card(), ws = document.getElementById("v-workspace");
    if (el && st.csv) el.hidden = !(ws && (ws.dataset.work === "panels" || ws.dataset.work === "table"));
  }
}

let timer = /** @type {ReturnType<typeof setTimeout> | undefined} */ (undefined);
/** Every redraw of a panel calls this: one sync after the last of a burst (a dragged edge redraws at every move). */
const soon = () => { clearTimeout(timer); timer = setTimeout(sync, 150); };

G.MANINT = { sync: soon, now: sync, state: () => ({ ks: [...st.ks], windows: st.windows, sums: st.sums, csv: st.csv, names: st.ks.map(k => E.files[k].file) }) };
soon();
