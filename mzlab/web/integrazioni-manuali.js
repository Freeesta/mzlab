// @ts-check
// The card «Manual integrations» of the Data view (#manint, under the panels): every window drawn by hand on an ion or transition trace (the `p.ints`
// of the transition panels, kept in the session), measured again by the engine (`api/manint`, mzlab_core::manint) as one row per window grouped by
// channel, with the name the user gives it; the CSV export is the engine's bytes, as `mzlab-cli peak table`. Loaded by grafici.js on the first integration.
const G = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => G.I18N.t(k, p);

/** The columns of the engine (manint::HEAD) with their labels (literal keys: tools/controlla_i18n.py reads them). */
const HEADS = () => /** @type {Record<string, string>} */ ({ channel: t("canali.manint.h.channel"), sample: t("canali.manint.h.sample"), time_min: t("canali.manint.h.time"),
  start_min: t("canali.manint.h.start"), end_min: t("canali.manint.h.end"), apex_rt_min: t("canali.manint.h.apex"), area: t("canali.manint.h.area"),
  height: t("canali.manint.h.height"), snr: t("canali.manint.h.snr"), baseline: t("canali.manint.h.base"), overlap: t("canali.manint.h.overlap"),
  pv: t("canali.manint.h.pv"), label: t("canali.manint.h.label"), origin: t("canali.manint.h.origin"),
  sn_zone: t("canali.manint.h.snz"), w_half_min: t("canali.manint.h.w50"), plates: t("int.tbl.plates"), tailing: t("int.tbl.tail"),
  gaussian: t("int.tbl.gauss"), asymmetry: t("int.tbl.asym"), noise_score: t("int.tbl.nscore") });
/** The tooltips of the columns of a split window. */
const TIPS = () => /** @type {Record<string, string>} */ ({ overlap: t("canali.manint.h.overlap.tip"), pv: t("canali.manint.h.pv.tip"), label: t("canali.manint.nm.tip"),
  area: t("canali.manint.h.area.tip"), below: t("canali.manint.h.below.tip"), sn_zone: t("int.tbl.sn.title"), gaussian: t("canali.manint.h.gauss.tip"), asymmetry: t("canali.manint.h.asym.tip"), noise_score: t("canali.manint.h.nscore.tip") });
/** The optional column groups the user switched on (INTOPT of picchi.js, one undoable setting), as the engine names them. */
const optCols = () => typeof INTOPT === "undefined" ? [] : [INTOPT.sn && "sn", INTOPT.par && "fig", INTOPT.q && "quality"].filter(Boolean);
/** The windows document the engine reads: the windows alone, or with the optional columns asked for. @param {string} w */
const withCols = w => optCols().length ? `{"windows":${w},"columns":${JSON.stringify(optCols())}}` : w;
/** The stretch a window's noise is measured on (the user's pick, picchi.js `noiseZone`), as the engine's `noise` field. @param {any} p @param {any} it */
const noiseOf = (p, it) => { const z = typeof noiseZone === "function" ? noiseZone(p, it) : null; return z ? { noise: [Math.min(z[0], z[1]), Math.max(z[0], z[1])] } : {}; };
/** How a row came to be (manint.rs, column `origin`), as shown. */
const ORIGINS = () => /** @type {Record<string, string>} */ ({ manual: t("canali.manint.org.manual"), applied: t("canali.manint.org.applied"), shifted: t("canali.manint.org.shifted"), no_peak: t("canali.manint.org.nopeak"), auto: t("canali.manint.org.auto"), unmatched: t("canali.manint.org.unmatched") });
const BASES = () => /** @type {Record<string, string>} */ ({ zero: t("int.base.zero"), edges: t("int.base.edges"), line: t("int.base.line") });
/** The longest name of an integration (manint::LABEL_MAX). */
const NM_MAX = 40;
/** The place of a column in the engine's header (the columns are shown in the engine's order: channel, sample, time, name, apex RT, area, ...). @param {string[]} head @param {string} c */
const col = (head, c) => head.indexOf(c);
const ICON = {
  dl: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M8 2.5v7.5M4.8 7L8 10.2 11.2 7M3 12.5v1h10v-1"/></svg>',
  copy: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><rect x="5.5" y="5.5" width="8" height="8" rx="1.3"/><path d="M10.5 5.5V3.8a1.3 1.3 0 0 0-1.3-1.3H3.8a1.3 1.3 0 0 0-1.3 1.3v5.4a1.3 1.3 0 0 0 1.3 1.3h1.7"/></svg>',
};

/** @typedef {{ p: any, it: any, k: number, tr: number, label: string, fol?: boolean, src?: boolean }} Rec */
/** `had`: rows at the last draw (null before the first), so an integration after none opens the card; `say`: the text for the live status at the next draw; `log`: the
 * per-file lines of the last automatic integration (this session only). */
/** @type {{ sig: string, recs: Rec[], ks: number[], windows: string, times: string, csv: string, busy: number, focus: any, had: number | null, reach: number, say: string, log: string[] }} */
const st = { sig: "", recs: [], ks: [], windows: "[]", times: "[]", csv: "", busy: 0, focus: null, had: null, reach: 0, say: "", log: [] };

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

/** The records of the transition panels with the transition each names (`m|<file>|<id>`), one per trace and window, in the order of the files; then the
 * windows that follow the sample shown (integra-segue.js: `fol`, one per shown sample without a window of its own on the channel), which the engine
 * writes as rows too. */
async function records() {
  /** @type {Rec[]} */ const out = [], fols = []; const seen = new Set();
  for (const p of (typeof E !== "undefined" ? E.panels : [])) {
    if (!p || !ESP.mrm(p) || !p.ints || !p.ints.length) continue;
    const src = G.SEGUE ? new Set(G.SEGUE.sources(p)) : new Set();
    for (const it of p.ints) {
      const m = /^m\|(\d+)\|(.+)$/.exec(it.key || ""), sig = `${it.key}|${it.a}|${it.b}`, f = m && E.files[+m[1]];
      if (!m || !f || f.gone || !baseOf(it) || seen.has(sig)) continue;
      const list = await getMrm(+m[1]), tr = list.findIndex((/** @type {any} */ x) => String(x.id) === m[2]);
      if (tr < 0) continue;
      seen.add(sig); out.push({ p, it, k: +m[1], tr, label: labelOf(list[tr], tr), ...(src.has(it) ? { src: true } : {}) });
    }
    if (G.SEGUE && p._a && p._a.sr) for (const it of G.SEGUE.all(p, p._a.sr)) {
      if (!it.fol || !baseOf(it)) continue;
      const list = await getMrm(it.k), tr = list.findIndex((/** @type {any} */ x) => String(x.id) === it.key.split("|")[2]);
      if (tr >= 0) fols.push({ p, it, k: it.k, tr, label: labelOf(list[tr], tr), fol: true });
    }
  }
  return out.concat(fols);
}

/** The window of a record as `api/manint` reads it: a name only when the user gave one, the drop lines of a split one. @param {any} it @param {number} file @param {number} tr */
const windowOf = (p, it, file, tr, ks, follow) => ({ file, tr, a: Math.min(it.a, it.b), b: Math.max(it.a, it.b), ...baseOf(it), ...noiseOf(p, it), ...(it.nm ? { label: it.nm } : {}), ...(it.splits && it.splits.length ? { splits: it.splits } : {}),
  ...(it.org ? { origin: it.org, grp: it.grp } : {}), ...(follow ? { follow: true } : {}), ...(follow && it.fn ? { names: Object.fromEntries(Object.entries(it.fn).map(([key, n]) => [ks.indexOf(+String(key).split("|")[1]), n]).filter(([i]) => i >= 0)) } : {}) });
/** The windows JSON of the records (the followers are the engine's: only the windows drawn), files numbered in the order of `ks`. @param {Rec[]} recs @param {number[]} ks */
const windowsOf = (recs, ks) => JSON.stringify(recs.filter(r => !r.fol).map(r => windowOf(r.p, r.it, ks.indexOf(r.k), r.tr, ks, !!r.src)));

/** The record of a row: same sample and channel, the row inside its edges (the engine writes the edges at 0.001 min; a part of a split window lies
 * between them). @param {string[]} row @param {string[]} head */
function recOf(row, head) {
  const iS = col(head, "sample"), iA = col(head, "start_min"), iB = col(head, "end_min");
  const mine = st.recs.filter(r => sampleOf(E.files[r.k].file || "") === row[iS] && r.label === row[0]);
  return live(mine.find(r => !r.fol && +Math.min(r.it.a, r.it.b).toFixed(3) <= +row[iA] && +row[iB] <= +Math.max(r.it.a, r.it.b).toFixed(3))
    || mine.filter(r => r.fol).sort((x, y) => Math.abs(x.it.a - +row[iA]) - Math.abs(y.it.a - +row[iA]))[0] || null);   // a window applied from another sample: the nearest one of the channel
}
/** The same window of two copies of a record: an undo or redo rebuilds p.ints from JSON, so the objects differ but the id stays. @param {any} a @param {any} b */
const sameInt = (a, b) => a === b || (a.id != null && a.id === b.id);
/** The record with the integration the panel holds now (delInts, noise picks and renames act by identity): the records of the last sync name the objects an undo replaced;
 * null when the window is gone. A window that follows the sample shown is not in p.ints, and stays as it is. @param {Rec | null} rec */
function live(rec) {
  if (!rec || rec.fol) return rec;
  const it = rec.p.ints.find((/** @type {any} */ i) => sameInt(i, rec.it));
  return !it ? null : it === rec.it ? rec : { ...rec, it };
}

/** An area or height of the CSV (7.84077E+04) in the one form of the page; a baseline by its name; the time of a sample (minutes) as its tag («3h»). @param {string} v @param {string} col */
const fmt = (v, col) => col === "time_min" ? G.timeTag(v === "" ? null : +v) : col === "origin" ? ORIGINS()[v] || v : col === "baseline" ? BASES()[v] || v : col === "overlap" ? (v === "drop" ? t("canali.manint.drop") : v) : /^-?\d(\.\d+)?E[+-]\d+$/.test(v) ? G.I18N.sciE(Number(v)) : v;
/** A channel as shown: the m/z, «q1 > q3», or «ione N». @param {string} c */
const chShown = c => c.replace(/^ion (\d+)$/, (_, n) => t("canali.sim.ion", { n: +n })).replace(">", " > ");
const COPY = ["apex_rt_min", "area", "height"];
/** The two columns the eye looks for (the apex RT and the area): semi-bold on a light tint, header included (app.css `.manint-key`). */
const KEY = ["apex_rt_min", "area"];
/** The colour of a sample as the Files list and the chart draw it (`f.color`), by the sample name the engine writes; "" when no file has that name. @param {string} name */
const colorOfSample = name => { const f = (typeof E !== "undefined" ? E.files : []).find((/** @type {any} */ x) => x && !x.gone && sampleOf(x.file || "") === name); return f && f.color ? String(f.color) : ""; };
/** A cell: its value as shown; apex, area and height are buttons that copy the CSV value (full precision); the name is an input. @param {string} v @param {string} col @param {number} i */
const cell = (v, col, i) => col === "sn_zone" && v === ""
  ? h`<button type="button" class="manint-nz" data-nz="${i}" title="${t("int.tbl.pickNoise.title")}">${t("int.tbl.pickNoise")}</button>`
  : col === "sample" && colorOfSample(v)
  ? h`<span class="manint-sw" aria-hidden="true" style="background:${colorOfSample(v)}"></span>${v}`
  : col === "label"
  ? h`<input type="text" class="manint-nm" maxlength="${NM_MAX}" data-nm="${i}" value="${v}" aria-label="${t("canali.manint.nm.aria")}" autocomplete="off" spellcheck="false">`
  : COPY.includes(col) && v !== ""
    ? h`<span class="manint-cp" role="button" tabindex="0" data-cp="${v}" aria-label="${t("canali.manint.cp.aria", { c: HEADS()[col] || col, v: fmt(v, col) })}" title="${t("canali.manint.cp")}">${fmt(v, col)}</span>`
    : h`${fmt(v, col)}`;

/** Open or closed as the user left it (NOMI key, this browser only). */
const OPEN = () => NOMI.KEY("manint.open");
const isOpen = () => { try { return localStorage.getItem(OPEN()) !== "0"; } catch (_) { return true; } };
/** @param {boolean} v */
const setOpen = v => { try { localStorage.setItem(OPEN(), v ? "1" : "0"); } catch (_) { /* this session only */ } };

/** A name typed in a row. By default it names the whole group (the window drawn and what follows it in the other samples), kept on the window drawn
 * (undoable, saved with the session); `only` (Alt+Enter) gives it to this sample alone (`fn` of the window drawn). @param {Rec} rec @param {string} v @param {boolean} [only] */
function rename(rec, v, only = false) {
  const nm = v.trim().slice(0, NM_MAX), p = rec.p, tgt = rec.fol ? p.ints.find((/** @type {any} */ i) => i.id === rec.it.srcId) : rec.it;
  if (!tgt) return false;
  const now = rec.fol ? G.SEGUE.name(tgt, rec.it.key) : rec.it.nm || "";
  if (nm === now) return false;
  G.pushIh(p);
  if (rec.fol && only) { tgt.fn = { ...(tgt.fn || {}) }; if (nm) tgt.fn[rec.it.key] = nm; else delete tgt.fn[rec.it.key]; if (!Object.keys(tgt.fn).length) delete tgt.fn; }
  else {
    if (nm) tgt.nm = nm; else delete tgt.nm;
    if (rec.fol && tgt.fn) { delete tgt.fn[rec.it.key]; if (!Object.keys(tgt.fn).length) delete tgt.fn; }
  }
  G.draw(p); G.uiSave(); return true;
}

/** The switches of the optional columns (S/N, chromatographic parameters, quality): INTOPT of picchi.js, one undoable setting. */
const optsHtml = () => typeof INTOPT === "undefined" ? "" : h`<span class="manint-opt" role="group" aria-label="${t("canali.manint.opt")}">${raw([["sn", "S/N", "int.tbl.sn.title"], ["par", t("int.tbl.par"), "int.tbl.par.title"], ["q", t("int.tbl.q"), "int.tbl.q.title"]]
  .map(([k, l, tip]) => h`<label title="${t(tip)}"><input type="checkbox" data-opt="${k}"${raw(INTOPT[k] ? " checked" : "")}> ${l}</label>`).join(""))}</span>`;
/** The role of a sample's file (sample, blank, standard...) as the sheets of the Excel file write it. @param {string} name */
const roleOf = name => { const f = (typeof E !== "undefined" ? E.files : []).find((/** @type {any} */ x) => x && !x.gone && sampleOf(x.file || "") === name), k = f ? CAMPIONI.kindOf(f) : "sample";
  return t(`export.int.role.${k === "calibration" ? "std" : ["sample", "blank", "dark", "infusion", "std", "qc"].includes(k) ? k : "sample"}`); };
const TEXT_COLS = ["channel", "sample", "label", "baseline", "origin", "overlap", "panel", "correction", "conc_unit", "area_unit"];
/** One sheet of the Excel file from a table of the engine: its header as shown, the numbers as numbers, the role of the file last. @param {string} text @param {string} name @param {Record<string, string>} labels */
function sheetOf(text, name, labels) {
  const all = rows(text), head = all[0] || [], iS = col(head, "sample"), iA = col(head, "start_min"), body = all.slice(1).filter(r => r[iS] !== "sum" || r[iA] !== "");
  return { name, head: [...head.map(c => labels[c] || HEADS()[c] || c), t("export.int.role")],
    rows: body.map(r => [...r.map((v, i) => v !== "" && !TEXT_COLS.includes(head[i]) && Number.isFinite(+v) ? +v : v), roleOf(r[iS])]), widths: [...head.map(c => TEXT_COLS.includes(c) ? 20 : 14), 12] };
}
/** The Excel file of the card: one sheet per table (the files' transitions, the traces the page makes), the engine's numbers. */
function xlsx() {
  const X = G.MANTR, sheets = [];
  if (st.csv) sheets.push(sheetOf(st.csv, t("int.xlsx.sheet"), {}));
  if (X && X.count()) sheets.push(sheetOf(X.csv(), sheets.length ? t("canali.manint.xlsx.traces") : t("int.xlsx.sheet"), X.labels()));
  if (sheets.length) G.dlx(t("int.xlsx.file"), sheets, { table: "integration" });
}
/** The switches: each one is an undoable setting (`peaks.settings`), then the table is asked again with the columns. @param {HTMLElement} el */
function wireOpts(el) {
  el.querySelectorAll("input[data-opt]").forEach(i => i.addEventListener("change", () => {
    const k = /** @type {HTMLInputElement} */ (i).dataset.opt || "";
    COMANDI.fai("peaks.settings", COMANDI.GLOBALE, () => { INTOPT[k] = /** @type {HTMLInputElement} */ (i).checked; }); st.sig = ""; soon();
  }));
  el.querySelectorAll("button[data-nz]").forEach(b => b.addEventListener("click", e => {
    e.stopPropagation();
    const rec = recOf(rowsOf()[Number(/** @type {HTMLElement} */ (b).dataset.nz)], rows(st.csv)[0] || []); if (!rec) return;
    rec.p._pickNoise = rec.it; G.reveal(rec.p); G.stepMsg(rec.p, t("int.pickNoise.msg"), true);
  }));
}
const rowsOf = () => { const all = rows(st.csv), head = all[0] || [], iS = col(head, "sample"), iA = col(head, "start_min"); return all.slice(1).filter(r => r[iS] !== "sum" || r[iA] !== ""); };

function draw() {
  const el = card(); if (!el) return;
  const all = rows(st.csv), head = all[0] || [], iS = col(head, "sample"), iA = col(head, "start_min"), iO = col(head, "origin"), body = all.slice(1).filter(r => r[iS] !== "sum" || r[iA] !== "");
  const X = G.MANTR, xn = X ? X.count() : 0, total = body.length + xn;      // + the windows on the traces the page makes itself (integrazioni-tracce.js)
  const had = st.had; st.had = total;
  if (!total) { el.hidden = true; el.innerHTML = ""; return; }
  if (had === 0) setOpen(true);      // an integration after none opens the card; otherwise it stays as the user left it (a reload included)
  const H = HEADS(), tips = TIPS(), cols = head.map((_, i) => i).filter(c => head[c] === "origin" ? body.some(r => r[c] !== "manual") : !["overlap", "pv"].includes(head[c]) || body.some(r => r[c] !== "")), open = isOpen(), plain = ["channel", "sample", "time_min", "baseline", "overlap", "label", "origin"];
  const cls = (/** @type {string} */ c) => [plain.includes(c) ? "" : "num", KEY.includes(c) ? "manint-key" : ""].filter(Boolean).join(" "), clsAttr = (/** @type {string} */ c) => cls(c) ? ` class="${cls(c)}"` : "";
  const ths = cols.map(c => h`<th scope="col"${raw(clsAttr(head[c]))}${raw(tips[head[c]] ? h` title="${tips[head[c]]}"` : "")}>${H[head[c]] || head[c]}</th>`).join("");
  const trs = body.map((r, i) => {
    const rec = recOf(r, head), who = { ch: chShown(r[0]), s: r[iS] };
    const act = rec && !rec.fol ? h`<button type="button" class="manint-x" data-del="${i}" aria-label="${t("canali.manint.del.aria", who)}" title="${t("canali.manint.del")}">×</button>` : "";
    const tds = cols.slice(1).map(c => h`<td${raw(clsAttr(head[c]))}${raw(head[c] === "origin" && r[c] === "unmatched" ? h` title="${t("canali.manint.unmatched.tip")}"` : "")}>${raw(head[c] === "label" && !rec ? h`${r[c]}` : cell(r[c], head[c], i))}</td>`).join("");
    return h`<tr data-org="${r[iO]}"${raw(rec ? h` class="manint-go" data-go="${i}" tabindex="0" aria-label="${t("canali.manint.show.aria", who)}"` : "")}><th scope="row">${chShown(r[cols[0]])}</th>${raw(tds)}<td>${raw(act)}</td></tr>`;
  }).join("");
  el.innerHTML = h`<div class="manint-head"><h3><button type="button" class="manint-tg" id="manint-h" aria-expanded="${open ? "true" : "false"}" aria-controls="manint-b" aria-describedby="manint-d" title="${t("canali.manint.note")}"><svg class="manint-chev" width="12" height="12" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true" focusable="false"><path d="M1.5 3.5h7L5 7.5z"/></svg>${t("canali.manint.title", { n: total })}</button></h3>
<button type="button" class="manint-ic" data-dl aria-label="${t("canali.manint.csv")}" title="${t("canali.manint.csv")}">${raw(ICON.dl)}</button><button type="button" class="manint-ic" data-copy aria-label="${t("canali.manint.copy")}" title="${t("canali.manint.copy")}">${raw(ICON.copy)}</button><button type="button" class="manint-ic manint-xl" data-xlsx aria-label="${t("canali.manint.xlsx")}" title="${t("canali.manint.xlsx")}">${raw(ICON.dl)}<span aria-hidden="true">xlsx</span></button>${raw(optsHtml())}<span id="manint-d" class="sr-only">${t("canali.manint.note")}</span><span id="manint-live" class="sr-only" role="status" aria-live="polite"></span></div>
<div id="manint-b" class="manint-wrap" tabindex="0" role="region" aria-labelledby="manint-h" aria-describedby="manint-d"${raw(open ? "" : " hidden")}>${raw(body.length ? h`<table class="qq-tab">
<thead><tr>${raw(ths)}<th scope="col"><span class="sr-only">${t("canali.manint.h.act")}</span></th></tr></thead><tbody>${raw(trs)}</tbody></table>` : "")}${raw(X ? X.html() : "")}</div>${raw(st.log.length && body.some(r => r[iO] === "auto" || r[iO] === "unmatched") ? h`<details class="manint-log"><summary>${t("canali.manint.log.h")}</summary><ul>${raw(st.log.map(l => h`<li>${l}</li>`).join(""))}</ul></details>` : "")}`;
  const ws = document.getElementById("v-workspace");
  el.hidden = !(ws && ws.dataset.work === "panels");
  // told when integrations reach more samples than before (the rows of another sample that the engine applied)
  const reach = new Set(body.filter(r => r[col(head, "origin")] !== "manual").map(r => r[iS])).size, lv = el.querySelector("#manint-live");
  if (lv && reach > st.reach) lv.textContent = t("canali.manint.applied", { n: reach });
  st.reach = reach;
  if (lv && st.say) { lv.textContent = st.say; st.say = ""; }
  const tg = /** @type {HTMLElement} */ (el.querySelector(".manint-tg")), bx = /** @type {HTMLElement} */ (el.querySelector("#manint-b"));
  tg.addEventListener("click", () => { const v = bx.hidden; bx.hidden = !v; tg.setAttribute("aria-expanded", String(v)); setOpen(v); });
  wireOpts(el);
  el.querySelector("[data-xlsx]")?.addEventListener("click", xlsx);
  el.querySelector("[data-dl]")?.addEventListener("click", () => G.dl("manual-integrations.csv", st.csv, "text/csv", { table: "manint" }));
  el.querySelector("[data-copy]")?.addEventListener("click", e => copy(all, /** @type {HTMLElement} */ (e.currentTarget)));
  /** A row shows its integration on the chart (click, Enter or Space); its X deletes it. @param {Element} b */
  const show = b => { const rec = recOf(body[Number(/** @type {HTMLElement} */ (b).dataset.go)], head); if (rec) { G.reveal(rec.p); G.draw(rec.p); } };
  el.querySelectorAll("tr[data-go]").forEach(b => {
    b.addEventListener("click", e => { if (!/** @type {Element} */ (e.target).closest("button, input")) show(b); });
    b.addEventListener("keydown", e => { const k = /** @type {KeyboardEvent} */ (e); if (k.target === b && (k.key === "Enter" || k.key === " ")) { k.preventDefault(); show(b); } });
  });
  // a copyable cell: click (by the pointer), Enter or Space (by the cell); never shows the row's integration
  el.querySelectorAll("[data-cp]").forEach(c => {
    const go = (/** @type {number} */ x, /** @type {number} */ y) => G.AREACOPY && G.AREACOPY.put(/** @type {HTMLElement} */ (c).dataset.cp || "", x, y);
    c.addEventListener("click", e => { e.stopPropagation(); const m = /** @type {MouseEvent} */ (e); go(m.clientX, m.clientY); });
    c.addEventListener("keydown", e => {
      const k = /** @type {KeyboardEvent} */ (e); if (k.key !== "Enter" && k.key !== " ") return;
      k.preventDefault(); k.stopPropagation(); const b = c.getBoundingClientRect(); go(b.left + b.width / 2, b.bottom);
    });
  });
  // the name of a row: kept on blur or Enter, Esc gives back the name it had
  el.querySelectorAll("input[data-nm]").forEach(n => {
    const inp = /** @type {HTMLInputElement} */ (n), at = () => recOf(body[Number(inp.dataset.nm)], head), was = inp.value, first = at();      // `at` at the event: an undo in between replaces the integration
    if (first && st.focus === first.it) { st.focus = null; inp.focus(); }
    inp.addEventListener("keydown", e => {
      e.stopPropagation();
      if (e.key === "Escape") { e.preventDefault(); inp.value = was; inp.blur(); }
      else if (e.key === "Enter") { e.preventDefault(); const rec = at(); if (rec && rename(rec, inp.value, e.altKey)) st.focus = rec.it; }
    });
    inp.addEventListener("blur", () => { const rec = at(); if (rec) rename(rec, inp.value); });
  });
  if (X) X.bind(el);
  el.querySelectorAll("[data-del]").forEach(b => b.addEventListener("click", () => {
    const rec = recOf(body[Number(/** @type {HTMLElement} */ (b).dataset.del)], head); if (rec) G.delInts(rec.p, [rec.it]);
  }));
}

/** The engine rows as TSV (raw numbers, so a spreadsheet reads them) after the provenance line; «Copiato!» or the failure by the button. @param {string[][]} all @param {HTMLElement} b */
async function copy(all, b) {
  let ok = true;
  try { await G.clipboardCopy({ "text/plain": await PROV.tsv(all.map(r => r.join("\t")).join("\n") + "\n", { table: "manint" }) }); } catch (_) { ok = false; }
  const r = b.getBoundingClientRect(); if (G.AREACOPY) G.AREACOPY.say(r.left + r.width / 2, r.bottom, ok);
}

/** Asks the engine for the table of the records; false when it refused or is missing. */
async function ask() {
  const n = ++st.busy;
  if (!st.recs.length) { st.csv = ""; return true; }
  try {
    const r = await fetch(`api/manint?k=${st.ks.join(",")}&sums=0&tm=${encodeURIComponent(st.times)}&w=${encodeURIComponent(withCols(st.windows))}`);
    const text = r.ok ? await r.text() : "";
    if (n === st.busy) st.csv = text;
    return r.ok;
  } catch (_) { return false; }
}

/** The table follows the integrations (asked again only when a window, a baseline or a file changes) and the work area. */
async function sync() {
  if (!G.MANTR && E.panels.some(p => p && p.ints && p.ints.length && !ESP.mrm(p))) await carica("m:integrazioni-tracce.js");
  const xch = G.MANTR ? await G.MANTR.sync() : false;
  const recs = await records(), ks = [...new Set(recs.map(r => r.k))].sort((a, b) => a - b), windows = windowsOf(recs, ks);
  const times = JSON.stringify(ks.map(k => E.files[k].time ?? null)), colors = JSON.stringify(ks.map(k => E.files[k].color || "")), sig = ks.join(",") + times + colors + windows + optCols();   // the time points of the samples as the table of the files has them
  if (sig !== st.sig || xch) {
    st.sig = sig; st.recs = recs; st.ks = ks; st.windows = windows; st.times = times;
    await ask();
    if (sig === st.sig) { draw(); for (const r of recs) if (r.it.splits) G.draw(r.p); }   // a split window's labels come from the new rows
  } else {
    st.recs = recs;
    const el = card(), ws = document.getElementById("v-workspace");
    if (el && (st.csv || (G.MANTR && G.MANTR.count()))) el.hidden = !(ws && ws.dataset.work === "panels");
  }
}

/** Where a record splits at `depth` (0..1): the times of the engine (`api/manvalley`, manint::valleys_text), null when it refused. @param {any} it @param {number} depth */
async function valleys(it, depth) {
  const m = /^m\|(\d+)\|(.+)$/.exec(it.key || ""), b = baseOf(it); if (!m || !b) return null;
  try {
    const list = await getMrm(+m[1]), tr = list.findIndex((/** @type {any} */ x) => String(x.id) === m[2]); if (tr < 0) return null;
    const w = JSON.stringify({ file: 0, tr, a: Math.min(it.a, it.b), b: Math.max(it.a, it.b), ...b });
    const r = await fetch(`api/manvalley?k=${m[1]}&depth=${depth}&w=${encodeURIComponent(w)}`);
    return r.ok ? JSON.parse(await r.text()).splits : null;
  } catch (_) { return null; }
}

/** The parts of a split record as the engine measured them (the rows of the table: apex, area, height), left to right; null when the record is not split or the
 * table does not have its parts yet. Drawn as one label each (grafici.js). @param {any} it */
function parts(it) {
  const rec = st.recs.find(r => sameInt(r.it, it)); if (!rec || !it.splits || !it.splits.length) return null;
  const all = rows(st.csv), head = all[0] || [], c = (/** @type {string} */ n) => head.indexOf(n), f = E.files[rec.k], lo = +Math.min(it.a, it.b).toFixed(3), hi = +Math.max(it.a, it.b).toFixed(3);
  const mine = all.slice(1).filter(r => r[c("sample")] === sampleOf(f && f.file || "") && r[0] === rec.label && +r[c("start_min")] >= lo && +r[c("end_min")] <= hi);
  const cuts = new Set(it.splits.filter((/** @type {number} */ x) => x > lo && x < hi)).size;
  return mine.length > 1 && mine.length === cuts + 1 ? mine.map(r => ({ rt: +r[c("apex_rt_min")], area: +r[c("area")], height: +r[c("height")], wait: false })) : null;
}

/** @type {WeakMap<any, { sig: string, by: Map<number, { a: number, b: number, origin: string, shift: number }>, ask: Set<number> }>} */ const FW = new WeakMap();
/** Where a window drawn on one sample lands on sample `k` (api/manfollow, manint::follow_text: the apex searched within the window's width on each side),
 * undefined until the engine answers (the panel is then drawn again); asked for the samples shown, and again only for one that comes later.
 * @param {any} p @param {any} src @param {number} k @param {any[]} sr */
function follow(p, src, k, sr) {
  const a = Math.min(src.a, src.b), b = Math.max(src.a, src.b), sig = `${src.key}|${a}|${b}|${JSON.stringify(src.bl || src.bm || "")}`;
  let c = FW.get(src); if (!c || c.sig !== sig) { c = { sig, by: new Map(), ask: new Set() }; FW.set(src, c); }
  const got = c.by.get(k); if (got) return got;
  const want = [...new Set(sr.map(s => s.k))].filter(x => x !== src.k && !c.by.has(x) && !c.ask.has(x));
  if (want.length) { want.forEach(x => c.ask.add(x)); askFollow(p, src, a, b, want, c); }
  return undefined;
}
/** @param {any} p @param {any} src @param {number} a @param {number} b @param {number[]} ks @param {any} c */
async function askFollow(p, src, a, b, ks, c) {
  const m = /^m\|(\d+)\|(.+)$/.exec(src.key || ""); if (!m) return;
  try {
    const list = await getMrm(+m[1]), tr = list.findIndex((/** @type {any} */ x) => String(x.id) === m[2]); if (tr < 0) return;
    const all = [+m[1], ...ks], r = await fetch(`api/manfollow?k=${all.join(",")}&w=${encodeURIComponent(JSON.stringify({ file: 0, tr, a, b, ...baseOf(src) }))}`);
    if (!r.ok) return;
    for (const e of JSON.parse(await r.text())) c.by.set(all[e.file], { a: e.a, b: e.b, origin: e.origin, shift: e.shift });
    G.draw(p);
  } catch (_) { /* the window stays where the source put it */ }
}

let timer = /** @type {ReturnType<typeof setTimeout> | undefined} */ (undefined);
/** Every redraw of a panel calls this: one sync after the last of a burst (a dragged edge redraws at every move). */
const soon = () => { clearTimeout(timer); timer = setTimeout(sync, 150); };

/** The notice and the per-file log of an automatic integration, shown at the next draw of the card. @param {string} say @param {string[]} log */
const report = (say, log) => { st.say = say; st.log = log; st.sig = ""; };

G.MANINT = { rows, report, opts: optCols, wrap: withCols, sync: soon, now: sync, valleys, parts, follow, state: () => ({ ks: [...st.ks], windows: st.windows, csv: st.csv, names: st.ks.map(k => E.files[k].file), times: Object.fromEntries(st.ks.map(k => [E.files[k].label, E.files[k].time ?? null])) }) };
document.addEventListener("samples", soon);      // a time point typed in the sample table changes the column
soon();
