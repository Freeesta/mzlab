// @ts-check
// The card «Manual integrations» of the Data view (#manint, under the panels): every window drawn by hand on an ion or transition trace (the `p.ints`
// of the transition panels, kept in the session), measured again by the engine (`api/manint`, mzlab_core::manint) as one row per window grouped by
// channel, with the name the user gives it; the CSV export is the engine's bytes, as `mzlab-cli manual-table`. Loaded by grafici.js on the first integration.
const G = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => G.I18N.t(k, p);

/** The columns of the engine (manint::HEAD) with their labels (literal keys: tools/controlla_i18n.py reads them). */
const HEADS = () => /** @type {Record<string, string>} */ ({ channel: t("canali.manint.h.channel"), name: t("canali.manint.h.name"), sample: t("canali.manint.h.sample"),
  start_min: t("canali.manint.h.start"), end_min: t("canali.manint.h.end"), apex_rt_min: t("canali.manint.h.apex"), area: t("canali.manint.h.area"),
  height: t("canali.manint.h.height"), snr: t("canali.manint.h.snr"), baseline: t("canali.manint.h.base"), label: t("canali.manint.h.label") });
const BASES = () => /** @type {Record<string, string>} */ ({ zero: t("int.base.zero"), edges: t("int.base.edges"), line: t("int.base.line") });
/** The longest name of an integration (manint::LABEL_MAX). */
const NM_MAX = 40;
/** The columns as shown: the name the user gives right after the sample. @param {string[]} head */
const order = head => { const o = head.map((_, i) => i).filter(i => head[i] !== "label"), l = head.indexOf("label"), s = o.indexOf(head.indexOf("sample")); if (l >= 0) o.splice(s + 1, 0, l); return o; };
const ICON = {
  dl: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M8 2.5v7.5M4.8 7L8 10.2 11.2 7M3 12.5v1h10v-1"/></svg>',
  copy: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><rect x="5.5" y="5.5" width="8" height="8" rx="1.3"/><path d="M10.5 5.5V3.8a1.3 1.3 0 0 0-1.3-1.3H3.8a1.3 1.3 0 0 0-1.3 1.3v5.4a1.3 1.3 0 0 0 1.3 1.3h1.7"/></svg>',
};

/** @typedef {{ p: any, it: any, k: number, tr: number, label: string }} Rec */
/** `had`: rows at the last draw (null before the first), so an integration after none opens the card. */
/** @type {{ sig: string, recs: Rec[], ks: number[], windows: string, csv: string, busy: number, focus: any, had: number | null }} */
const st = { sig: "", recs: [], ks: [], windows: "[]", csv: "", busy: 0, focus: null, had: null };

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

/** The windows JSON of the records, files numbered in the order of `ks`; a name only when the user gave one. @param {Rec[]} recs @param {number[]} ks */
const windowsOf = (recs, ks) => JSON.stringify(recs.map(r => ({ file: ks.indexOf(r.k), tr: r.tr, a: Math.min(r.it.a, r.it.b), b: Math.max(r.it.a, r.it.b), ...baseOf(r.it), ...(r.it.nm ? { label: r.it.nm } : {}) })));

/** The record of a row: same sample, channel and edges (the engine writes the edges at 0.001 min). @param {string[]} row */
function recOf(row) {
  return st.recs.find(r => sampleOf(E.files[r.k].file || "") === row[2] && r.label === row[0] && Math.min(r.it.a, r.it.b).toFixed(3) === row[3] && Math.max(r.it.a, r.it.b).toFixed(3) === row[4]) || null;
}

/** An area or height of the CSV (7.84077E+04) in the one form of the page; a baseline by its name. @param {string} v @param {string} col */
const fmt = (v, col) => col === "baseline" ? BASES()[v] || v : /^-?\d(\.\d+)?E[+-]\d+$/.test(v) ? G.I18N.sciE(Number(v)) : v;
/** A channel as shown: the m/z, «q1 > q3», or «ione N». @param {string} c */
const chShown = c => c.replace(/^ion (\d+)$/, (_, n) => t("canali.sim.ion", { n: +n })).replace(">", " > ");
const COPY = ["apex_rt_min", "area", "height"];
/** A cell: its value as shown; apex, area and height are buttons that copy the CSV value (full precision); the name is an input. @param {string} v @param {string} col @param {number} i */
const cell = (v, col, i) => col === "label"
  ? h`<input type="text" class="manint-nm" maxlength="${NM_MAX}" data-nm="${i}" value="${v}" aria-label="${t("canali.manint.nm.aria")}" autocomplete="off" spellcheck="false">`
  : COPY.includes(col) && v !== ""
    ? h`<span class="manint-cp" role="button" tabindex="0" data-cp="${v}" aria-label="${t("canali.manint.cp.aria", { c: HEADS()[col] || col, v: fmt(v, col) })}" title="${t("canali.manint.cp")}">${fmt(v, col)}</span>`
    : h`${fmt(v, col)}`;

/** Open or closed as the user left it (NOMI key, this browser only). */
const OPEN = () => NOMI.KEY("manint.open");
const isOpen = () => { try { return localStorage.getItem(OPEN()) !== "0"; } catch (_) { return true; } };
/** @param {boolean} v */
const setOpen = v => { try { localStorage.setItem(OPEN(), v ? "1" : "0"); } catch (_) { /* this session only */ } };

/** A name typed in a row: kept on the integration (undoable, saved with the session), then the engine writes it. @param {Rec} rec @param {string} v */
function rename(rec, v) {
  const nm = v.trim().slice(0, NM_MAX);
  if (nm === (rec.it.nm || "")) return false;
  G.pushIh(rec.p);
  if (nm) rec.it.nm = nm; else delete rec.it.nm;
  G.draw(rec.p); G.uiSave(); return true;
}

function draw() {
  const el = card(); if (!el) return;
  const all = rows(st.csv), head = all[0] || [], body = all.slice(1).filter(r => r[2] !== "sum" || r[3] !== "");
  const had = st.had; st.had = body.length;
  if (!body.length) { el.hidden = true; el.innerHTML = ""; return; }
  if (had === 0) setOpen(true);      // an integration after none opens the card; otherwise it stays as the user left it (a reload included)
  const H = HEADS(), cols = order(head), open = isOpen(), plain = ["channel", "name", "sample", "baseline", "label"];
  const ths = cols.map(c => h`<th scope="col"${raw(plain.includes(head[c]) ? "" : ' class="num"')}>${H[head[c]] || head[c]}</th>`).join("");
  const trs = body.map((r, i) => {
    const rec = recOf(r), who = { ch: chShown(r[0]), s: r[2] };
    const act = rec ? h`<button type="button" class="manint-x" data-del="${i}" aria-label="${t("canali.manint.del.aria", who)}" title="${t("canali.manint.del")}">×</button>` : "";
    const tds = cols.slice(1).map(c => h`<td${raw(plain.includes(head[c]) ? "" : ' class="num"')}>${raw(head[c] === "label" && !rec ? h`${r[c]}` : cell(r[c], head[c], i))}</td>`).join("");
    return h`<tr${raw(rec ? h` class="manint-go" data-go="${i}" tabindex="0" aria-label="${t("canali.manint.show.aria", who)}"` : "")}><th scope="row">${chShown(r[cols[0]])}</th>${raw(tds)}<td>${raw(act)}</td></tr>`;
  }).join("");
  el.innerHTML = h`<div class="manint-head"><h3><button type="button" class="manint-tg" id="manint-h" aria-expanded="${open ? "true" : "false"}" aria-controls="manint-b" aria-describedby="manint-d" title="${t("canali.manint.note")}"><svg class="manint-chev" width="12" height="12" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true" focusable="false"><path d="M1.5 3.5h7L5 7.5z"/></svg>${t("canali.manint.title", { n: body.length })}</button></h3>
<button type="button" class="manint-ic" data-dl aria-label="${t("canali.manint.csv")}" title="${t("canali.manint.csv")}">${raw(ICON.dl)}</button><button type="button" class="manint-ic" data-copy aria-label="${t("canali.manint.copy")}" title="${t("canali.manint.copy")}">${raw(ICON.copy)}</button><span id="manint-d" class="sr-only">${t("canali.manint.note")}</span></div>
<div id="manint-b" class="manint-wrap" tabindex="0" role="region" aria-labelledby="manint-h" aria-describedby="manint-d"${raw(open ? "" : " hidden")}><table class="qq-tab">
<thead><tr>${raw(ths)}<th scope="col"><span class="sr-only">${t("canali.manint.h.act")}</span></th></tr></thead><tbody>${raw(trs)}</tbody></table></div>`;
  const ws = document.getElementById("v-workspace");
  el.hidden = !(ws && (ws.dataset.work === "panels" || ws.dataset.work === "table"));
  const tg = /** @type {HTMLElement} */ (el.querySelector(".manint-tg")), bx = /** @type {HTMLElement} */ (el.querySelector("#manint-b"));
  tg.addEventListener("click", () => { const v = bx.hidden; bx.hidden = !v; tg.setAttribute("aria-expanded", String(v)); setOpen(v); });
  el.querySelector("[data-dl]")?.addEventListener("click", () => G.dl("manual-integrations.csv", st.csv, "text/csv", { table: "manint" }));
  el.querySelector("[data-copy]")?.addEventListener("click", e => copy(all, /** @type {HTMLElement} */ (e.currentTarget)));
  /** A row shows its integration on the chart (click, Enter or Space); its X deletes it. @param {Element} b */
  const show = b => { const rec = recOf(body[Number(/** @type {HTMLElement} */ (b).dataset.go)]); if (rec) { G.reveal(rec.p); G.draw(rec.p); } };
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
    const inp = /** @type {HTMLInputElement} */ (n), rec = recOf(body[Number(inp.dataset.nm)]), was = inp.value;
    if (rec && st.focus === rec.it) { st.focus = null; inp.focus(); }
    inp.addEventListener("keydown", e => {
      e.stopPropagation();
      if (e.key === "Escape") { e.preventDefault(); inp.value = was; inp.blur(); }
      else if (e.key === "Enter") { e.preventDefault(); if (rec && rename(rec, inp.value)) st.focus = rec.it; }
    });
    inp.addEventListener("blur", () => { if (rec) rename(rec, inp.value); });
  });
  el.querySelectorAll("[data-del]").forEach(b => b.addEventListener("click", () => {
    const rec = recOf(body[Number(/** @type {HTMLElement} */ (b).dataset.del)]); if (rec) G.delInts(rec.p, [rec.it]);
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
    const r = await fetch(`api/manint?k=${st.ks.join(",")}&sums=0&w=${encodeURIComponent(st.windows)}`);
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

/** @type {WeakMap<any, { v: string | undefined }>} */ const SN = new WeakMap();
/** The S/N of a window that follows the sample (integra-segue.js), by the engine as a table row would have it, without adding a row; undefined until it
 * answers (the panel is then drawn again), "" when there is no peak. @param {any} p @param {any} f */
function snr(p, f) {
  const c = SN.get(f); if (c) return c.v;
  const e = { v: /** @type {string | undefined} */ (undefined) }, m = /^m\|(\d+)\|(.+)$/.exec(f.key || ""), b = baseOf(f); SN.set(f, e);
  if (!m || !b) { e.v = ""; return e.v; }
  getMrm(+m[1]).then(async (/** @type {any[]} */ list) => {
    const tr = list.findIndex(x => String(x.id) === m[2]); if (tr < 0) { e.v = ""; return; }
    const w = JSON.stringify([{ file: 0, tr, a: f.a, b: f.b, ...b }]), r = await fetch(`api/manint?k=${m[1]}&sums=0&w=${encodeURIComponent(w)}`);
    const all = r.ok ? rows(await r.text()) : [], i = (all[0] || []).indexOf("snr");
    e.v = i >= 0 && all[1] ? all[1][i] : ""; G.draw(p);
  }).catch(() => { e.v = ""; });
  return undefined;
}

let timer = /** @type {ReturnType<typeof setTimeout> | undefined} */ (undefined);
/** Every redraw of a panel calls this: one sync after the last of a burst (a dragged edge redraws at every move). */
const soon = () => { clearTimeout(timer); timer = setTimeout(sync, 150); };

G.MANINT = { sync: soon, now: sync, snr, state: () => ({ ks: [...st.ks], windows: st.windows, csv: st.csv, names: st.ks.map(k => E.files[k].file) }) };
soon();
