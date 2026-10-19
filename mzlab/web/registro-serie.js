// @ts-check
// The instrument log of a Thermo .raw as chromatogram kind «Registro dello strumento» (p.kind = "reg", p.reg = id of the series): the status log of the instrument (temperatures, voltages,
// vacuum, gas) and the numeric fields of the scan trailer that change (injection time, AGC, lock mass), each one drawn like an analog trace. Loaded as a module after the classic scripts; pannelli.js
// and grafici.js use window.REGISTRO. The list of the series comes in `metodo/1.serie_registro` (api/metodo); the points are asked one series at a time (api/registro, engine worker `natregistro`).
// A file with no such list (an mzML, a .wiff) has no entry: the kind is offered only when a file of the tab has series.

/** @typedef {{ id: string, gruppo?: string, nome: string, unita?: string, n: number, min: number, max: number }} Voce */

// files of a tab asked for their list (the others are mzML or other makers' files with no log)
const MAX_FILES = 8;
const MAX_POINTS = 2000;                               // min/max pairs kept per series on screen
/** @type {Map<string, Voce[] | null | undefined>} file name -> its list (null: none; undefined: being asked) */
const LISTS = new Map();

/** @param {{ k: number, file: string }} f */
async function ask(f) {
  LISTS.set(f.file, undefined);
  await CHROM1.ready;
  /** @type {Voce[] | null} */ let list = null;
  try {
    const d = ((await J(`api/metodo?k=${f.k}`)) || {}).metodo;
    const l = d && Array.isArray(d.serie_registro) ? d.serie_registro : [];
    if (l.length) {
      // the list is in the document of any file that came from a .raw; the series are served only when the file is read by the engine as such: one request tells
      await J(`api/registro?k=${f.k}&id=${encodeURIComponent(l[0].id)}`);
      list = l;
    }
  } catch (_) { list = null; }
  LISTS.set(f.file, list);
  for (const p of E.panels) if (p.type === "chrom" && p.el && p.el.isConnected) { ctl(p); if (p.kind === "reg") draw(p); }
}

/** the list of a file when it is known, `undefined` while it is asked for (and starts asking) */
const listOf = (/** @type {any} */ f) => {
  if (!LISTS.has(f.file)) ask(f);
  return LISTS.get(f.file);
};

/** true: some file of `files` has series; false: none has; undefined: some are still being asked */
function has(/** @type {any[]} */ files) {
  let waiting = false;
  for (const f of files.slice(0, MAX_FILES)) {
    const l = listOf(f);
    if (l && l.length) return true;
    if (l === undefined) waiting = true;
  }
  return waiting ? undefined : false;
}

/** the series of the files of a tab, once each id, in the order of the files */
function union(/** @type {any[]} */ files) {
  /** @type {Map<string, Voce>} */ const m = new Map();
  for (const f of files.slice(0, MAX_FILES)) for (const v of LISTS.get(f.file) || []) if (!m.has(v.id)) m.set(v.id, v);
  return [...m.values()];
}

const groupOf = (/** @type {Voce} */ v) => (v.id.startsWith("trailer:") ? I18N.t("reg.group.trailer") : v.gruppo || I18N.t("reg.group.none"));
const label = (/** @type {Voce} */ v) => v.nome + (v.unita ? ` (${v.unita})` : "");

/** the options of the select, in a group each (`<optgroup>`), keeping only those whose name has `q` */
function options(/** @type {Voce[]} */ list, /** @type {string} */ cur, /** @type {string} */ q) {
  const w = q.trim().toLowerCase();
  /** @type {Map<string, string[]>} */ const g = new Map();
  for (const v of list) {
    if (w && !(`${groupOf(v)} ${label(v)}`.toLowerCase().includes(w))) continue;
    if (!g.has(groupOf(v))) g.set(groupOf(v), []);
    /** @type {string[]} */ (g.get(groupOf(v))).push(`<option value="${EH(v.id)}"${v.id === cur ? " selected" : ""}>${EH(label(v))}</option>`);
  }
  return [...g].map(([name, o]) => `<optgroup label="${EH(name)}">${o.join("")}</optgroup>`).join("") || `<option value="" disabled>${EH(I18N.t("reg.none"))}</option>`;
}

/** the controls of the panel when its kind is «Registro»: a search box and the select of the channels */
function picker(/** @type {any} */ p, /** @type {any[]} */ files) {
  const list = union(files);
  if (!list.length) return "";
  if (!list.some(v => v.id === p.reg)) p.reg = list[0].id;
  return `<span class="regpick"><input type="search" data-regq class="mzf" style="width:9em" autocomplete="off" aria-label="${EH(I18N.t("reg.search"))}" placeholder="${EH(I18N.t("reg.search"))}">`
    + `<select data-o="reg" style="max-width:18em" aria-label="${EH(I18N.t("reg.pick"))}" title="${EH(I18N.t("reg.pick"))}">${options(list, p.reg, "")}</select></span>`;
}

/** the search box filters the channels of the select without redrawing the row of controls (it would lose the focus) */
function wire(/** @type {any} */ p, /** @type {HTMLElement} */ c) {
  const q = /** @type {HTMLInputElement | null} */ (c.querySelector("[data-regq]")), s = /** @type {HTMLSelectElement | null} */ (c.querySelector('select[data-o="reg"]'));
  if (!q || !s) return;
  const list = union(tabFiles(p.tab || E.tab));
  q.oninput = () => { s.innerHTML = options(list, p.reg, q.value); };
}

/** a series cut to at most MAX_POINTS min/max pairs (the extremes are kept, as for the pressure) */
function decimate(/** @type {number[]} */ t, /** @type {number[]} */ y) {
  if (t.length <= MAX_POINTS * 2) return { t, y };
  const b = Math.ceil(t.length / MAX_POINTS), ox = [], oy = [];
  for (let i = 0; i < t.length; i += b) {
    let lo = i, hi = i;
    for (let j = i; j < Math.min(i + b, t.length); j++) { if (y[j] < y[lo]) lo = j; if (y[j] > y[hi]) hi = j; }
    for (const j of lo < hi ? [lo, hi] : [hi, lo]) { ox.push(t[j]); oy.push(y[j]); if (lo === hi) break; }
  }
  return { t: ox, y: oy };
}

/** the series of the panel for the files that have the channel (one curve per file, the colours of the files) */
async function series(/** @type {any} */ p, /** @type {any[]} */ files) {
  const list = union(files);
  if (!list.length) return [];
  if (!list.some(v => v.id === p.reg)) { p.reg = list[0].id; ctl(p); }
  const v = /** @type {Voce} */ (list.find(x => x.id === p.reg));
  const out = await Promise.all(files.filter(f => (LISTS.get(f.file) || []).some(x => x.id === v.id)).map(f =>
    memo(`reg${f.k}|${v.id}`, () => J(`api/registro?k=${f.k}&id=${encodeURIComponent(v.id)}`)).then((/** @type {any} */ d) => {
      const r = decimate(d.time, d.intensity);
      return { x: r.t, y: r.y, color: f.color, dash: fdash(f), name: f.label, k: f.k, key: `r|${f.k}|${v.id}`, ion: v.nome, time: f.time, unit: d.unit || v.unita || "" };
    })));
  return out.filter(s => s.x.length);
}

/** the title of the y axis of the panel */
const title = (/** @type {any} */ p, /** @type {any[]} */ sr) => {
  const v = union(tabFiles(p.tab || E.tab)).find(x => x.id === p.reg);
  return v ? label(v) : sr[0] && sr[0].unit || "";
};

/** @type {any} */ (window).REGISTRO = { has, picker, wire, series, title, union };
