// @ts-strict
// @ts-noany
"use strict";
// Lists of reference m/z (known contaminants: Keller 2008, laboratory contaminants, user suspect lists).
// ONE engine for every list of reference: unit resolution (nominal m/z ±0.5 Da, ordered by proximity) and high resolution (ppm tolerance).
// Pure functions first (also run by node in tests/test_liste.py); page UI (mount, dialogs, IndexedDB storage) below.
// Principle 1 of CLAUDE.md: in low resolution, coincidences are often accidental. The program NEVER shows automatic annotations on LR spectra.
const LISTE = (() => {
  // ---------------------------------------------------------------- pure part
  const tolDa = (/** @type {number} */ mz, /** @type {number} */ ppm) => Math.max(mz * ppm * 1e-6, mz < 200 ? 0.001 : 0);

  // index: the active lists, one array per polarity sorted by m/z (a list item with pol 0 goes in both)
  function buildIndex(/** @type {Bag[] | null | undefined} */ lists, /** @type {Bag | null | undefined} */ active) {
    const /** @type {Bag[]} */ pos = [], /** @type {Bag[]} */ neg = [];
    (lists || []).forEach((/** @type {Bag} */ L) => {
      if (!L) return;
      if (active && active[L.id] === false) return;
      (L.items || []).forEach((/** @type {Bag} */ it) => {
        const /** @type {Bag} */ x = { ...it, clsKey: it.cls, list: L.id, listName: L.name || L.id, listBuiltin: !!L.builtin };
        if (it.series && it.series.n != null && typeof I18N !== "undefined") { x.baseName = String(x.name).replace(/ n=\d+$/, ""); x.name = I18N.t(it.series.n === 1 ? "lst.unitName.one" : "lst.unitName.many", { name: x.baseName, n: it.series.n }); }    // a polymer series is named by its repeat units, not by the bare «n=3» of the data
        if (it.pol !== -1) pos.push(x);
        if (it.pol !== 1) neg.push(x);
      });
    });
    const by = (/** @type {Bag} */ a, /** @type {Bag} */ b) => a.mz - b.mz;
    pos.sort(by); neg.sort(by);
    return { pos, neg };
  }

  const lower = (/** @type {Bag[]} */ a, /** @type {number} */ v) => {
    let lo = 0, hi = a.length;
    while (lo < hi) {
      const m = (lo + hi) >> 1;
      if (a[m].mz < v) lo = m + 1;
      else hi = m;
    }
    return lo;
  };

  // polarity of a file/search: "positive" / 1, "negative" / -1, anything else = both
  const arrays = (/** @type {Bag} */ idx, /** @type {string | number | null | undefined} */ pol) => (pol === "positive" || pol === 1 ? [idx.pos] : pol === "negative" || pol === -1 ? [idx.neg] : [idx.pos, idx.neg]);

  // High-resolution search by ppm, or nominal search by Da if isNominal / da tolerance is passed
  function find(/** @type {Bag | null} */ idx, /** @type {string | number | null | undefined} */ pol, /** @type {number} */ mz, /** @type {number | Bag | null | undefined} */ tol, isNominal = false) {
    if (!idx) return [];
    const nominal = isNominal || (typeof tol === "object" && tol && tol.nominal);
    const t = nominal ? (typeof tol === "number" ? tol : (tol && tol.da) || 0.5) : tolDa(mz, typeof tol === "number" ? tol : (tol && tol.ppm) || 5);
    const /** @type {Bag[]} */ out = [], seen = new Set();
    arrays(idx, pol).forEach(a => {
      for (let i = lower(a, mz - t); i < a.length && a[i].mz <= mz + t; i++) {
        const k = a[i].list + ":" + a[i].id + ":" + a[i].adduct + ":" + a[i].mz;
        if (!seen.has(k)) {
          seen.add(k);
          const diff = mz - a[i].mz;
          out.push({ ...a[i], diffDa: diff, err: (diff / a[i].mz) * 1e6 });
        }
      }
    });
    return nominal ? out.sort((p, q) => Math.abs(p.diffDa) - Math.abs(q.diffDa)) : out.sort((p, q) => Math.abs(p.err) - Math.abs(q.err));
  }

  // true when an entry of the list `listId` lies within the ppm window of an m/z: the same window as find(), nothing allocated
  function hasMatch(/** @type {Bag | null} */ idx, /** @type {string | number | null | undefined} */ pol, /** @type {number} */ mz, /** @type {number} */ ppm, /** @type {string} */ listId) {
    const t = tolDa(mz, typeof ppm === "number" ? ppm : 5);
    for (const a of idx ? arrays(idx, pol) : []) for (let i = lower(a, mz - t); i < a.length && a[i].mz <= mz + t; i++) if (a[i].list === listId) return true;
    return false;
  }
  // The table of the panel asks Rust (reflist-match/1, `api/reflist_match`): the active ions go as three columns (m/z, polarity, one group per
  // distinct list:id:adduct:mz), the answer is positions in that list. Same entries and order as find(), which the synchronous hover and menu callers keep.
  const flatCache = new WeakMap();
  const flatOf = (/** @type {Bag} */ idx) => {
    let f = flatCache.get(idx);
    if (!f) {
      const items = [...new Set([...idx.pos, ...idx.neg])], groups = new Map();
      f = { items, cols: { mz: items.map(i => i.mz), pol: items.map(i => i.pol ?? 0), group: items.map(i => { const k = i.list + ":" + i.id + ":" + i.adduct + ":" + i.mz; return groups.get(k) ?? (groups.set(k, groups.size), groups.size - 1); }) } };
      flatCache.set(idx, f);
    }
    return f;
  };
  async function lookup(/** @type {Bag} */ idx, /** @type {number} */ mz, /** @type {boolean} */ byDa, /** @type {number} */ tol) {
    const f = flatOf(idx), r = await fetch("api/reflist_match", { method: "POST", body: JSON.stringify({ mz, unit: byDa ? "da" : "ppm", tol, items: f.cols }) });
    const j = await r.json();
    if (!r.ok) throw new Error(j.error || r.statusText);
    return j.hits.map((/** @type {Bag} */ h) => ({ ...f.items[h.i], diffDa: h.diff, err: h.err }));
  }
  // Nominal search at unit resolution (default ±0.5 Da, ordered by proximity |Δ m/z|)
  function findNominal(/** @type {Bag} */ idx, /** @type {string | number | null | undefined} */ pol, /** @type {number} */ mz, tolDaVal = 0.5) {
    return find(idx, pol, mz, tolDaVal, true);
  }

  // series: three or more consecutive members (n, n+1, n+2...) of the same polymer and adduct among the peaks of the spectrum.
  function seriesRuns(/** @type {Bag} */ idx, /** @type {string | number | null | undefined} */ pol, /** @type {number[]} */ mzs, /** @type {number} */ ppm, minRun = 3) {
    const found = new Map(), arrs = arrays(idx, pol), /** @type {Bag[]} */ hit = [];
    // same entries and same order as find() would give for each peak (by |error|, ties in list order), without cloning or deduplicating what is not a series member
    for (const m of mzs) {
      const t = tolDa(m, typeof ppm === "number" ? ppm : 5);
      hit.length = 0; for (const a of arrs) for (let i = lower(a, m - t); i < a.length && a[i].mz <= m + t; i++) if (a[i].series && a[i].series.n != null) hit.push(a[i]);
      if (hit.length > 1) hit.sort((p, q) => Math.abs((m - p.mz) / p.mz * 1e6) - Math.abs((m - q.mz) / q.mz * 1e6));
      for (const it of hit) {
        const k = it.list + ":" + it.series.id;
        found.get(k)?.set(it.series.n, it) ?? found.set(k, new Map([[it.series.n, it]]));
      }
    }
    const out = new Map();
    found.forEach((byN, k) => {
      const ns = [...byN.keys()].sort((a, b) => a - b);
      let i = 0;
      while (i < ns.length) {
        let j = i;
        while (j + 1 < ns.length && ns[j + 1] === ns[j] + 1) j++;
        if (j - i + 1 >= minRun) {
          const nm = String(byN.get(ns[i]).baseName || byN.get(ns[i]).name).split(" (")[0];
          const txt = `serie ${nm} (n = ${ns[i]}-${ns[j]})`;
          for (let q = i; q <= j; q++) out.set(k + "#" + ns[q], txt);
        }
        i = j + 1;
      }
    });
    return out;
  }

  const MAXROWS = 3;
  // the lines of the box that opens on the peak (HR only): nothing when nothing matches
  /** @param {Bag[] | null | undefined} matches @param {Map<string, string> | null} [runs] @param {(s: any) => string} [esc] */
  function tipHtml(matches, runs, esc) {
    if (!matches || !matches.length) return "";
    const e = esc || ((/** @type {unknown} */ s) => String(s).replace(/[&<>"]/g, c => (/** @type {Record<string, string>} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }))[c]));
    const row = (/** @type {Bag} */ it) => {
      const ser = it.series && runs && runs.get(it.list + ":" + it.series.id + "#" + it.series.n);
      return `<div class="cont-row"><b>${e(it.name)}</b>${it.mzonly ? ` <span class="sm">${I18N.t("lst.mzOnly")}</span>` : ` <span class="sm">${e(it.adduct || "")}${it.formula ? ", " + e(it.formula.replace(/x(\d+)$/, " ×$1")) : ""}</span>`}<div class="sm">${I18N.t("lst.theoMz")} ${it.mz.toFixed(4)} · ${it.err >= 0 ? "+" : ""}${it.err.toFixed(1)} ppm · ${e(noRef(it.listName || it.list))}${ser ? ` · <b>${e(ser)}</b>` : ""}</div></div>`;
    };
    const shown = matches.slice(0, MAXROWS), more = matches.length - shown.length;
    return `<div class="cont"><div class="sm cont-h">${I18N.t("lst.compatible")}</div>${shown.map(row).join("")}${more > 0 ? `<div class="sm">${I18N.t("lst.more", { n: more })}</div>` : ""}</div>`;
  }

  // The CSV of a list is read in Rust (reflist/1, `mzlab-cli db ref-list`): the same items and the same line-numbered errors on the site, in the app and in the CLI.
  /** @param {string} text @param {string} [name] @returns {Promise<Bag>} */
  async function readCsv(text, name) {
    const r = await fetch("api/reflist", { method: "POST", body: JSON.stringify({ csv: String(text), name: name || "" }) });
    const j = await r.json();
    if (!j || j.error || !Array.isArray(j.items)) throw Object.assign(new Error("reflist"), { key: (j && j.error && j.error.key) || "err.reflist.request" });
    return j;
  }
  // the text of a row error: «Line 7: …»
  const rowErr = (/** @type {Bag} */ e) => I18N.t("lst.err.line", { line: e.line, text: I18N.t(e.key, { value: e.value, field: e.field }) });
  // the template people start from: parsed by the same Rust reader (test_liste_csv)
  const TEMPLATE = "mz,name,formula,polarity,adduct,note\n100.1234,Example A,,+,,only the m/z is known\n,Example B,C6H12O6,,[M+H]+,the m/z is computed from the formula\n,Example C,C6H12O6,-,,negative: [M-H]- is used\n";

  const csvCell = (/** @type {unknown} */ s) => (/[",;\n]/.test(String(s)) ? '"' + String(s).replace(/"/g, '""') + '"' : String(s));
  const csvOut = (/** @type {Bag[]} */ rows) => "mz,name,formula,polarity,note\n" + rows.map((/** @type {Bag} */ r) => [r.mz, r.name, r.formula, r.polarity === 1 ? "positive" : r.polarity === -1 ? "negative" : "both", r.note].map(csvCell).join(",")).join("\n") + "\n";

  // Creates the list object for laboratory contaminants
  const labList = (/** @type {Bag[] | null | undefined} */ rows) => ({
    id: "laboratorio",
    name: I18N.t("lst.lab.name"),
    source: I18N.t("lst.user.of"),
    license: I18N.t("lst.user.of"),
    builtin: false,
    items: (rows || []).map((/** @type {Bag} */ r, /** @type {number} */ i) => ({
      id: "lab" + i,
      name: r.name || `m/z ${r.mz}`,
      cls: I18N.t("lst.lab.cls"),
      formula: r.formula || null,
      adduct: r.adduct || I18N.t("lst.lab.observed"),
      z: 1,
      mz: +r.mz,
      pol: r.polarity || 0,
      series: null,
      source: r.note || "laboratorio",
      mzonly: !r.formula
    }))
  });

  // Creates the list object for an uploaded user suspect list
  function userList(/** @type {string} */ id, /** @type {string} */ name, /** @type {string} */ filename, /** @type {Bag[] | null | undefined} */ rows, /** @type {number} */ compounds) {
    const items = (rows || []).map((/** @type {Bag} */ r, /** @type {number} */ i) => {
      let mz = r.mz;
      // Default to [M+H]+ for positive or [M-H]- for negative if formula is known and adduct is not
      let adduct = r.adduct;
      if (!adduct) adduct = "";
      return {
        id: (id || "usr") + "_" + i,
        name: r.name || (r.formula ? r.formula : `m/z ${mz}`),
        cls: I18N.t("lst.user.source"),
        formula: r.formula || null,
        adduct,
        z: 1,
        mz: mz ? +mz : 0,
        pol: r.pol || r.polarity || 0,
        series: null,
        source: name || filename || I18N.t("lst.user.name"),
        mzonly: !r.formula
      };
    }).filter((/** @type {Bag} */ x) => x.mz > 0);

    return {
      id: id || ("usr_" + Date.now()),
      name: name || filename || I18N.t("lst.user.name"),
      filename: filename || "",
      source: I18N.t("lst.user.source"),
      license: I18N.t("lst.user.of"),
      builtin: false,
      info: { url: null, doi: null, version: null, date: new Date().toISOString().slice(0, 10), source_known: true, provenance: filename || "", compounds, classes: 0 },
      items
    };
  }

  const KEY_ROWS = NOMI.KEY("contaminanti"),
    KEY_ON = NOMI.KEY("contaminanti.on"),
    KEY_LISTS = NOMI.KEY("liste.attive");
  const IDB_STORE = "liste_utente";

  const ST = /** @type {{ lists: any[], userLists: any[], idx: any, loading: Promise<void> | null, err: any, runs: WeakMap<object, any> }} */ ({
    lists: [],           // built-in lists (/api/contaminants: Keller 2008)
    userLists: [],       // lists uploaded by the user from CSV, persisted in IndexedDB
    idx: null,
    loading: null,
    err: null,
    runs: new WeakMap()
  });

  const rd = (/** @type {string} */ k, /** @type {unknown} */ d) => { try { const v = localStorage.getItem(k); return v == null ? d : JSON.parse(v); } catch (e) { return d; } };
  const wr = (/** @type {string} */ k, /** @type {unknown} */ v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) { /* private mode / quota */ } };
  const isOn = () => rd(KEY_ON, false) === true;

  // Exclusions (HR only): hide a compatible peak from the lists and the tables, never change the data. Stored in the browser, the same for every file.
  const KEY_EXCL = NOMI.KEY("contaminanti.esclusi");
  const exclKinds = ["entry", "cls", "list"];
  // names of the stored object (user data: do not rename)
  const FLD = /** @type {Record<string, string>} */ ({ entry: "voc" + "i", cls: "class" + "i", list: "list" + "e" });
  let /** @type {Record<string, string[]> | null} */ exclCache = null;
  const exclRead = () => {
    if (exclCache) return exclCache;
    const o = rd(KEY_EXCL, {}) || {}, /** @type {Record<string, string[]>} */ r = {};
    exclKinds.forEach(k => { r[k] = Array.isArray(o[FLD[k]]) ? o[FLD[k]].filter((/** @type {unknown} */ x) => typeof x === "string") : []; });
    return (exclCache = r);
  };
  const exclSave = (/** @type {Record<string, string[]>} */ e) => { const /** @type {Bag} */ o = {}; exclKinds.forEach(k => { o[FLD[k]] = e[k]; }); wr(KEY_EXCL, o); if (typeof redrawAll === "function") redrawAll(); };
  const voceKey = (/** @type {Bag} */ it) => it.list + ":" + it.id + (it.series ? "#" + it.series.n : "");
  const classKey = (/** @type {Bag} */ it) => it.clsKey || it.cls || "";
  const keyFor = (/** @type {string} */ kind, /** @type {Bag} */ it) => kind === "entry" ? voceKey(it) : kind === "cls" ? classKey(it) : it.list;
  const isExcluded = (/** @type {Bag} */ it) => { const e = exclRead(); return e.entry.includes(voceKey(it)) || (!!classKey(it) && e.cls.includes(classKey(it))) || e.list.includes(it.list); };
  function exclSet(/** @type {string} */ kind, /** @type {string} */ key, /** @type {boolean} */ on) {
    if (!exclKinds.includes(kind) || !key) return;
    const e = exclRead(), a = e[kind], i = a.indexOf(key);
    if (on && i < 0) a.push(key); else if (!on && i >= 0) a.splice(i, 1); else return;
    exclSave(e);
  }
  // saved with the session: the stored object as is; restoring adds what the browser does not have yet
  const exclExport = () => { const e = exclRead(), /** @type {Bag} */ o = {}; exclKinds.forEach(k => { o[FLD[k]] = e[k].slice(); }); return o; };
  function exclImport(/** @type {Bag | null | undefined} */ o) {
    if (!o || typeof o !== "object") return;
    const e = exclRead(); let ch = false;
    exclKinds.forEach(k => (Array.isArray(o[FLD[k]]) ? o[FLD[k]] : []).forEach((/** @type {unknown} */ x) => { if (typeof x === "string" && !e[k].includes(x)) { e[k].push(x); ch = true; } }));
    if (ch) exclSave(e);
  }
  function exclReset() { exclCache = { entry: [], cls: [], list: [] }; exclSave(exclCache); }
  const exclItems = () => { const e = exclRead(); return exclKinds.flatMap(k => e[k].map((/** @type {string} */ key) => ({ kind: k, key }))); };
  // the compatible entries of a peak: vis = those not excluded (the peak keeps its mark); hid = there are some, and all are excluded (the peak is hidden)
  const split = (/** @type {Bag[]} */ ms) => { const vis = ms.filter((/** @type {Bag} */ x) => !isExcluded(x)); return { vis, hid: ms.length > 0 && !vis.length }; };
  function setOn(/** @type {unknown} */ v) { wr(KEY_ON, !!v); document.querySelectorAll("#lst-cont-signs").forEach(el => { el.checked = !!v; }); if (typeof redrawAll === "function") redrawAll(); }
  const LAB_MAX = 5000;                                              // rows taken from one imported CSV: the rest is named in the toast, never dropped silently
  const labRows = () => { const r = rd(KEY_ROWS, []); return Array.isArray(r) ? r.filter(x => x && x.mz > 0) : []; };
  const saveRows = (/** @type {Bag[]} */ rows) => { wr(KEY_ROWS, rows); reindex(); if (typeof redrawAll === "function") redrawAll(); };

  // One short connection per operation, closed when its transaction ends: an idle one would block the reset's deleteDatabase.
  function idbOpen() {
    return new Promise(async res => {
      if (typeof indexedDB === "undefined") return res(null);
      try {
        // the schema is written once (qqq-db.js)
        const { DB_NAME, DB_VER, upgrade } = await import("./qqq-db.js");
        const req = indexedDB.open(DB_NAME, DB_VER);
        req.onupgradeneeded = () => upgrade(req.result);
        req.onsuccess = () => { const db = req.result; db.onversionchange = () => db.close(); res(db); };
        req.onerror = () => res(null);
        req.onblocked = () => window.dispatchEvent(new Event("mzlab:db-blocked"));     // browser.js shows the same message as for the engine's own open
      } catch (_) { res(null); }
    });
  }

  async function idbRun(/** @type {IDBTransactionMode} */ mode, /** @type {(s: IDBObjectStore) => IDBRequest} */ fn) {
    const db = await idbOpen();
    if (!db) return undefined;
    try {
      return await new Promise(res => {
        try {
          const t = db.transaction(IDB_STORE, mode), r = fn(t.objectStore(IDB_STORE));
          t.oncomplete = () => res(r.result);
          t.onerror = t.onabort = () => res(undefined);
        } catch (_) { res(undefined); }
      });
    } finally { db.close(); }
  }
  const idbGetUserLists = async () => (await idbRun("readonly", s => s.getAll())) || [];
  const idbSaveUserList = (/** @type {Bag} */ list) => idbRun("readwrite", s => s.put(list));
  const idbDeleteUserList = (/** @type {string} */ id) => idbRun("readwrite", s => s.delete(id));

  // Per-list switches exist only when there are two or more lists (the built-in ones are Keller and the pesticides); a single list is always on, whatever an old stored choice says
  const activeMap = () => (ST.lists.length + ST.userLists.length < 2 ? {} : rd(KEY_LISTS, {}));

  function reindex() {
    const act = activeMap();
    const all = [...ST.lists, labList(labRows()), ...ST.userLists];
    ST.idx = buildIndex(all, act);
    ST.runs = new WeakMap();
  }

  // Loads the built-in lists; a failed read is not final (the index stays usable with the user lists and the read is tried again on the next
  // ready(), or from ensure() at most every RETRY_MS). IndexedDB never holds the built-in lists back: past IDB_WAIT_MS they go ahead without the user ones.
  const RETRY_MS = 5000, IDB_WAIT_MS = 4000;
  let errAt = 0;
  function load() {
    const p1 = (typeof J === "function") ? J("api/contaminants").then((/** @type {Bag} */ j) => { ST.lists = j.lists || []; ST.err = null; }).catch((/** @type {unknown} */ e) => { ST.err = e; errAt = Date.now(); }) : Promise.resolve();
    const p2 = Promise.race([idbGetUserLists(), new Promise(res => setTimeout(() => res(null), IDB_WAIT_MS))]).then(u => { if (u) ST.userLists = u; }).catch(() => {});
    ST.loading = Promise.all([p1, p2]).then(() => { reindex(); ST.loading = null; });
  }
  function ensure() {
    if (ST.loading) return ST.idx;
    if (!ST.idx || (ST.err && Date.now() - errAt > RETRY_MS)) load();
    return ST.idx;
  }

  const ready = async () => {
    if (!ST.loading && ST.err) load();               // the caller waits for an answer: read again now
    else ensure();
    if (ST.loading) await ST.loading;
  };
  /** True when the built-in lists could not be read (the index then holds the user lists only). */
  const failed = () => !!ST.err;

  // For high resolution spectra only: tooltip box on hover. null if switch is off or NOT high resolution.
  function forSpec(/** @type {number[]} */ mzs, /** @type {number[]} */ ys, /** @type {string | number | null | undefined} */ polarity, /** @type {number} */ ppm) {
    if (!isOn()) return null;
    return {
      tip(/** @type {number} */ m) {
        const idx = ensure(); if (!idx) return "";
        let runs = ST.runs.get(mzs);
        if (!runs || runs.ppm !== ppm) {
          const top = maxOf(ys, 0), /** @type {number[]} */ sel = [];
          mzs.forEach((/** @type {number} */ x, /** @type {number} */ i) => { if (ys[i] >= top * 0.01) sel.push(x); });
          runs = { ppm, map: seriesRuns(idx, polarity, sel, ppm) };
          ST.runs.set(mzs, runs);
        }
        return tipHtml(find(idx, polarity, m, ppm), runs.map);
      },
      match: (/** @type {number} */ m) => (ST.idx ? find(ST.idx, polarity, m, ppm) : []),
      state(/** @type {number} */ m) { const ms = ST.idx ? find(ST.idx, polarity, m, ppm) : []; const s = split(ms); return { vis: s.vis, hid: s.hid, all: ms }; }
    };
  }

  // Entries of the lists the user uploaded (never the built-in ones) that match an m/z; null when none was uploaded (the table of the reference difference shows its list column only then)
  const hasUser = () => ST.userLists.length > 0;
  function userMatches(/** @type {number} */ mz, /** @type {string | number | null | undefined} */ polarity, /** @type {number} */ ppm) {
    if (!ST.userLists.length) return null;
    const idx = ST.idx || ensure(); if (!idx) return null;
    const ids = new Set(ST.userLists.map(u => u.id));
    return find(idx, polarity, mz, ppm).filter(x => ids.has(x.list));
  }

  // m => true when the m/z matches a Keller 2008 entry; null while the lists load. Independent of the global contaminant switch: it serves "remove contaminants" of a spectrum.
  function keller(/** @type {string | number | null | undefined} */ polarity, /** @type {number} */ ppm) {
    const idx = ST.idx || ensure(); if (!idx) return null;
    return (/** @type {number} */ m) => hasMatch(idx, polarity, m, ppm, "keller");
  }

  // Peak tables (HR): compatible entries of an m/z, split into shown and hidden-by-exclusion; null if the switch is off
  function compat(/** @type {number} */ mz, /** @type {string | number | null | undefined} */ polarity, /** @type {number} */ ppm) {
    const idx = ST.idx || ensure(); if (!idx || !isOn()) return null;
    const ms = find(idx, polarity, mz, ppm), s = split(ms);
    return { vis: s.vis, hid: s.hid, all: ms };
  }

  // Summary text for formulas / library matches: "name (adduct, ±ppm)"
  function textFor(/** @type {number} */ mz, /** @type {string | number | null | undefined} */ polarity, /** @type {number} */ ppm) {
    const idx = ST.idx || ensure(); if (!idx || !isOn()) return "";
    const m = find(idx, polarity, mz, ppm).filter(x => !isExcluded(x)); if (!m.length) return "";
    return m.slice(0, 2).map(x => `${x.name} (${x.adduct || I18N.t("lst.mzOnly")}${x.formula ? ", " + x.formula.replace(/x(\d+)$/, " ×$1") : ""}; ${x.err >= 0 ? "+" : ""}${x.err.toFixed(1)} ppm)`).join("; ") + (m.length > 2 ? " " + I18N.t("lst.textMore", { n: m.length - 2 }) : "");
  }

  function addLab(/** @type {number} */ mz, /** @type {Bag} */ o = {}) {
    const rows = labRows();
    rows.push({ mz: +mz, name: o.name || "", formula: o.formula || "", polarity: o.polarity || 0, note: o.note || "" });
    saveRows(rows);
  }

  // Quick dialog to add peak to laboratory contaminants
  async function askAdd(/** @type {number} */ mz, /** @type {string | number | null | undefined} */ polarity) {
    const name = typeof ask === "function" ? await ask(I18N.t("lst.askAdd", { mz: (+mz).toFixed(4) }), `m/z ${(+mz).toFixed(4)}`) : "";
    if (name === null || name === undefined) return;
    addLab(mz, { name, polarity: polarity === "positive" ? 1 : polarity === "negative" ? -1 : 0 });
    if (typeof toast === "function") toast(I18N.t("lst.added"));
  }

  // Dialog for laboratory contaminants
  function openLab() {
    const draw = () => {
      const rows = labRows();
      if (typeof big === "function") {
        big(I18N.t("lst.lab.name"), `<div class="muted sm" style="margin-bottom:6px">${I18N.t("lst.lab.intro")}</div>
          <div class="bar"><button id="lab-xl" type="button">${I18N.t("lst.lab.export")}</button><button id="lab-im" type="button">${I18N.t("lst.lab.import")}</button><button id="lab-add" type="button" class="go">${I18N.t("lst.lab.add")}</button><input type="file" id="lab-f" accept=".csv,.txt" hidden></div>
          <table class="lct"><tr><th class="num"><i>m/z</i></th><th>${I18N.t("lst.col.name")}</th><th>${I18N.t("lst.col.formula")}</th><th>${I18N.t("lst.col.polarity")}</th><th>${I18N.t("lst.col.note")}</th><th></th></tr>
          ${rows.map((r, i) => `<tr><td><input data-i="${i}" data-c="mz" data-num=">0" value="${r.mz}" style="width:84px"></td><td><input data-i="${i}" data-c="name" value="${EH(r.name)}"></td><td><input data-i="${i}" data-c="formula" value="${EH(r.formula)}" style="width:110px"></td>
            <td><select data-i="${i}" data-c="polarity"><option value="0"${!r.polarity ? " selected" : ""}>${I18N.t("lst.pol.both")}</option><option value="1"${r.polarity === 1 ? " selected" : ""}>${I18N.t("lst.pol.pos")}</option><option value="-1"${r.polarity === -1 ? " selected" : ""}>${I18N.t("lst.pol.neg")}</option></select></td>
            <td><input data-i="${i}" data-c="note" value="${EH(r.note)}"></td><td><button data-del="${i}" type="button" title="${I18N.t("lst.lab.removeRow")}">×</button></td></tr>`).join("") || `<tr><td colspan="6" class="muted">${I18N.t("lst.lab.empty")}</td></tr>`}</table>`, () => {
          document.querySelectorAll("#bigbody [data-c]").forEach(inp => inp.onchange = () => {
            const r = labRows(), x = r[+inp.dataset.i]; if (!x) return;
            const c = inp.dataset.c; x[c] = c === "mz" ? numIt(inp.value) || x.mz : c === "polarity" ? +inp.value : inp.value; saveRows(r);
          });
          document.querySelectorAll("#bigbody [data-del]").forEach(b => b.onclick = () => { const r = labRows(); r.splice(+b.dataset.del, 1); saveRows(r); draw(); });
          const bAdd = document.querySelector("#lab-add"); if (bAdd) bAdd.onclick = () => { addLab(100.0, { name: I18N.t("lst.lab.newName") }); draw(); };
          const bXl = document.querySelector("#lab-xl"); if (bXl) bXl.onclick = () => { if (typeof dl === "function") dl("contaminanti_laboratorio.csv", csvOut(labRows()), "text/csv", { table: "lab-contaminants" }); };
          const bIm = document.querySelector("#lab-im"); const fIm = document.querySelector("#lab-f");
          if (bIm && fIm) {
            bIm.onclick = () => fIm.click();
            fIm.onchange = async (/** @type {Bag} */ e) => { const f = e.target.files[0]; if (!f) return; let n, all, nErr = 0; try { const j = await readCsv(await FILESICURI.readText(f)); nErr = j.errors.length; all = j.items.map((/** @type {Bag} */ r) => ({ mz: r.mz, name: r.name, formula: r.formula, polarity: r.pol, note: r.note })); n = all.slice(0, LAB_MAX); } catch (x_) { const x = /** @type {Lanciato} */ (x_); if (typeof toast === "function") toast(I18N.t(x.key || "workflow.err.invalidFormat", x.params)); return; } saveRows([...labRows(), ...n]); draw(); if (nErr && typeof toast === "function") toast(I18N.t("lst.user.skipped", { n: nErr })); else if (typeof toast === "function") toast(all.length > LAB_MAX ? I18N.t("lst.lab.importedCut", { max: LAB_MAX, m: all.length, rest: all.length - LAB_MAX }) : I18N.t("lst.lab.imported", { n: n.length })); };
          }
        });
      }
    };
    draw();
  }

  // Adds a user list from CSV text. Rows with an error are never imported silently: without `partial` a CSV with errors adds nothing and answers its errors.
  /** @param {string} filename @param {string} text @param {string} [customName] @param {boolean} [partial] @returns {Promise<{list: Bag | null, errors: Bag[], valid: number}>} */
  async function addUserListFromCsv(filename, text, customName, partial) {
    const j = await readCsv(text);
    if ((j.errors.length && !partial) || !j.items.length) return { list: null, errors: j.errors, valid: j.items.length };
    const name = customName || (filename ? filename.replace(/\.[^/.]+$/, "") : I18N.t("lst.user.name"));
    const id = "usr_" + Date.now() + "_" + Math.random().toString(36).slice(2, 7);
    const uList = userList(id, name, filename, j.items, j.compounds);
    ST.userLists.push(uList);
    await idbSaveUserList(uList);
    reindex();
    if (typeof toast === "function") toast(I18N.t("lst.user.added", { name, n: uList.items.length }));
    return { list: uList, errors: j.errors, valid: j.items.length };
  }

  // Renames a user list (the name is the list's label in the chips, the table and the peak boxes)
  async function renameUserList(/** @type {string} */ id, /** @type {string} */ name) {
    const L = ST.userLists.find(x => x.id === id), n = String(name || "").trim();
    if (!L || !n) return;
    L.name = n;
    L.items.forEach((/** @type {Bag} */ it) => { it.source = n; });
    await idbSaveUserList(L);
    reindex();
  }

  // Delete user list
  async function deleteUserList(/** @type {string} */ id) {
    ST.userLists = ST.userLists.filter(x => x.id !== id);
    await idbDeleteUserList(id);
    reindex();
    if (typeof toast === "function") toast(I18N.t("lst.user.removed"));
  }

  // Toggle active list
  function toggleListActive(/** @type {string} */ id, /** @type {boolean} */ active) {
    const act = rd(KEY_LISTS, {});
    act[id] = active;
    wr(KEY_LISTS, act);
    reindex();
    if (typeof redrawAll === "function") redrawAll();
  }

  // ---------------------------------------------------------------- UI component: mount(container, opt) & open(opt)
  const EH = (/** @type {unknown} */ s) => String(s == null ? "" : s).replace(/[&<>"]/g, c => (/** @type {Record<string, string>} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }))[c]);
  const fmtSub = (/** @type {string} */ f) => EH(f).replace(/([A-Z][a-z]?|\))(\d+)/g, "$1<sub>$2</sub>").replace(/(\d?[+-])$/, "<sup>$1</sup>");

  // the markup of the panel: search bar, active lists, user lists, exclusions and the table (the handlers are bound in mount())
  // the panel shows list names without a trailing bibliographic reference, e.g. «(Keller 2008)»
  // the short LR table names a list without its reference: "(Keller et al.)", "(… 2008)"
  let short = false;
  const noRef = (/** @type {unknown} */ n) => String(n).replace(short ? /\s*\([^)]*(?:\d{4}|et al\.?)[^)]*\)\s*$/ : /\s*\([^)]*\d{4}\)\s*$/, "");
  // the dormant HRMS Target Screening tab: shown only behind FEATURES.targetScreening («?targetscreening=1» or localStorage «qqq.dev.targetscreening» = «1», features.js)
  const targetOn = () => typeof FEATURES !== "undefined" && !!FEATURES.targetScreening;
  // the short name of the pesticides chip; any other list keeps its own name (the Keller list is named in its data: «Known contaminants (Keller et al.)»)
  const GROUP_KEY = /** @type {Record<string, string>} */ ({ pesticides: "lst.grp.pesticides" });
  const grpName = (/** @type {Bag} */ L) => (GROUP_KEY[L.id] ? I18N.t(GROUP_KEY[L.id]) : noRef(L.name_en || L.name || L.id));
  const srcName = (/** @type {string} */ id, /** @type {string} */ fallback) => { const L = [...ST.lists, ...ST.userLists].find(x => x.id === id); return L ? grpName(L) : noRef(fallback); };
  function panelHtml(/** @type {Bag} */ state) {
    const compact = state.mode === "da";                      // unit resolution: the short table
    return `
      <div class="liste-panel" style="display:flex;flex-direction:column;gap:6px;padding:6px 8px;height:100%;box-sizing:border-box;overflow:auto">
        ${targetOn() ? `<div style="display:${compact ? "none" : "flex"};align-items:center;gap:6px;border-bottom:1px solid var(--line,#cbd5e1);padding-bottom:4px;margin-bottom:2px">
          <span class="seg" id="lst-view-seg">
            <button type="button" data-v="ref" class="on">${I18N.t("lst.title")}</button>
            <button type="button" data-v="target">${I18N.t("lst.view.target")}</button>
          </span>
        </div>` : ""}
        <div id="lst-ref-box" style="display:flex;flex-direction:column;gap:6px;flex:1">
        <div class="liste-bar">
          <div class="lb-row">
            <label class="lb-grow"><span class="sr-only">${I18N.t("lst.qPh")}</span><input type="search" id="lst-q" value="${EH(state.q)}" placeholder="${I18N.t("lst.qPh")}" aria-label="${I18N.t("lst.qPh")}" autocomplete="off"></label>
          </div>
          <div id="lst-ctl"></div>
        </div>

        <div role="group" aria-labelledby="lst-src-lbl" style="display:flex;align-items:center;gap:6px;font-size:12px;flex-wrap:wrap">
          <span class="muted" id="lst-src-lbl">${I18N.t("lst.active")}</span>
          <span id="lst-src-checks" class="lst-chips"></span>
        </div>

        <div style="display:${compact ? "none" : "flex"};align-items:center;font-size:12px">
          <label style="display:inline-flex;align-items:center;gap:5px;cursor:pointer" title="${I18N.t("lst.contSigns.title")}">
            <input type="checkbox" id="lst-cont-signs" ${isOn() ? "checked" : ""}>
            <span>${I18N.t("lst.contSigns")}</span>
          </label>
        </div>

        <div id="lst-add" style="display:flex;flex-wrap:wrap;align-items:center;gap:6px;font-size:12px;padding:4px 8px;border:1px dashed var(--line,#cbd5e1);border-radius:6px">
          <button type="button" id="lst-add-btn">${I18N.t("lst.add.btn")}</button>
          <button type="button" id="lst-add-tpl">${I18N.t("lst.add.tpl")}</button>
          <span class="muted">${I18N.t("lst.add.drop")}</span>
          <input type="file" id="lst-add-file" accept=".csv,.tsv,.txt,text/csv,text/plain" hidden aria-label="${I18N.t("lst.add.btn")}">
          <div id="lst-add-err" role="alert" style="flex:1 1 100%" hidden></div>
        </div>
        <details id="lst-about" style="font-size:12px"><summary>${I18N.t("lst.info.h")}</summary><div id="lst-lists"></div></details>
        <div id="lst-excl" class="sm" style="padding:4px 8px;border:1px dashed var(--line,#cbd5e1);border-radius:6px" hidden></div>
        <span id="lst-count" class="sm muted" role="status" aria-live="polite"></span>

        <div id="lst-table-wrap" role="region" tabindex="0" aria-label="${I18N.t("lst.title")}" style="flex:1;min-height:220px;overflow:auto;border:1px solid var(--line,#e2e8f0);border-radius:6px;background:var(--panel,#fff)">
          <table class="lct" style="width:100%;border-collapse:collapse;font-size:12px">
            <caption class="sr-only">${I18N.t("lst.title")}</caption>
            <thead>
              <tr style="position:sticky;top:0;background:var(--soft,#f1f5f9);z-index:2">
                <th scope="col" class="num" style="width:${compact ? 76 : 84}px" title="${I18N.t("lst.th.theo")}"><i>m/z</i></th>
                ${compact ? "" : `<th scope="col" class="num" style="width:70px">${I18N.t("lst.th.err")}</th>`}
                <th scope="col">${I18N.t("lst.col.name")}</th>
                ${compact ? "" : `<th scope="col">${I18N.t("lst.th.series")}</th>`}
                <th scope="col">${I18N.t("lst.th.ion")}</th>
                ${compact ? "" : `<th scope="col">${I18N.t("lst.col.formula")}</th>`}
                <th scope="col">${I18N.t("lst.th.pol")}</th>
                ${compact ? "" : `<th scope="col">${I18N.t("lst.th.class")}</th><th scope="col" class="lst-src-col">${I18N.t("lst.th.source")}</th>`}
              </tr>
            </thead>
            <tbody id="lst-tbody"></tbody>
          </table>
        </div>
        </div>
        <div id="lst-target-box" style="display:none;flex-direction:column;flex:1;overflow:auto"></div>
      </div>
  `;
  }

  function setupViewSwitcher(/** @type {HTMLElement} */ container, /** @type {Bag} */ opt = {}) {
    const elViewSeg = container.querySelector("#lst-view-seg");
    const elRefBox = container.querySelector("#lst-ref-box");
    const elTargetBox = container.querySelector("#lst-target-box");
    async function switchToView(/** @type {string | undefined} */ viewName) {
      const isTarget = viewName === "target";
      if (elViewSeg) {
        elViewSeg.querySelectorAll("button").forEach((/** @type {HTMLElement} */ b) => b.classList.toggle("on", (b.dataset.v === "target") === isTarget));
      }
      if (elRefBox) elRefBox.style.display = isTarget ? "none" : "flex";
      if (elTargetBox) {
        elTargetBox.style.display = isTarget ? "flex" : "none";
        if (isTarget && !elTargetBox.dataset.mounted) {
          elTargetBox.dataset.mounted = "1";
          try {
            await import("./liste-target-core.js"); if (!window.LISTE_TARGET) await import("./liste-target.js");
            const mod = window.LISTE_TARGET;      // liste-target.js is a classic script: importing it only sets window.LISTE_TARGET (the namespace has no mount)
            if (mod && typeof mod.mount === "function") mod.mount(elTargetBox);
          } catch (_) {}
        }
      }
    }
    if (elViewSeg) {
      elViewSeg.querySelectorAll("button").forEach((/** @type {HTMLElement} */ b) => { b.onclick = () => switchToView(b.dataset.v); });
    }
    if (opt.targetScreening && elViewSeg) switchToView("target");       // without the flag there is no tab bar: the lists stay
  }

  // a CSV from the file picker or a drop: read in Rust; rows with errors are listed with their line, never skipped silently
  async function addFile(/** @type {File} */ f, /** @type {HTMLElement} */ elAddErr, /** @type {() => void} */ redraw) {
    elAddErr.hidden = true;
    let text;
    try { text = await FILESICURI.readText(f); } catch (x_) { const x = /** @type {Lanciato} */ (x_); elAddErr.hidden = false; elAddErr.textContent = I18N.t(x.key || "workflow.err.invalidFormat", x.params); return; }
    let r;
    try { r = await addUserListFromCsv(f.name, text, undefined, false); } catch (x_) { const x = /** @type {Lanciato} */ (x_); elAddErr.hidden = false; elAddErr.textContent = I18N.t(x.key || "err.reflist.request"); return; }
    if (r.list) { redraw(); return; }
    elAddErr.hidden = false;
    const shown = r.errors.slice(0, 20).map(rowErr), more = r.errors.length - shown.length;
    elAddErr.innerHTML = `<div><b>${r.errors.length ? I18N.t("lst.add.errs", { n: r.errors.length }) : I18N.t("lst.add.none")}</b></div><ul class="lst-add-errs" style="margin:2px 0 4px 16px;padding:0">${shown.map(t => `<li>${EH(t)}</li>`).join("")}</ul>${more > 0 ? `<div class="muted">${I18N.t("lst.more", { n: more })}</div>` : ""}`
      + (r.valid ? `<button type="button" id="lst-add-valid">${I18N.t("lst.add.importValid", { n: r.valid })}</button> ` : "") + `<button type="button" id="lst-add-cancel">${I18N.t("lst.add.cancel")}</button>`;
    const bv = elAddErr.querySelector("#lst-add-valid");
    if (bv) /** @type {HTMLElement} */ (bv).onclick = async () => { await addUserListFromCsv(f.name, text, undefined, true); elAddErr.hidden = true; redraw(); };
    /** @type {HTMLElement} */ (elAddErr.querySelector("#lst-add-cancel")).onclick = () => { elAddErr.hidden = true; };
  }

  // one row of the results table (compact: the short unit-resolution table)
  const listRowHtml = (/** @type {Bag} */ it, /** @type {boolean} */ compact, /** @type {boolean} */ hasMz, /** @type {boolean} */ byDa) => {
    const delta = hasMz ? (byDa ? (it.diffDa != null ? (it.diffDa >= 0 ? "+" : "") + it.diffDa.toFixed(3) + " Da" : "")
                                : (it.err != null ? (it.err >= 0 ? "+" : "") + it.err.toFixed(1) + " ppm" : "")) : "";
    const polBadge = it.pol === 1 ? `<span class="pol pos" title="${I18N.t("lst.pol.pos")}" style="font-size:12px">+</span>` : it.pol === -1 ? `<span class="pol neg" title="${I18N.t("lst.pol.neg")}" style="font-size:12px">&minus;</span>` : `<span class="pol both" title="${I18N.t("lst.pol.both")}" style="font-size:12px">&plusmn;</span>`;
    const seriesInfo = it.series ? `<span class="sm" title="${I18N.t("lst.member", { n: it.series.n })}">${I18N.t(it.series.n === 1 ? "lst.unitShort.one" : "lst.unitShort.many", { n: it.series.n })}</span>` : "";
    if (compact) return `<tr>
      <td class="num font-mono"><b>${it.mz.toFixed(4)}</b></td>
      <td class="lst-name" title="${EH(it.name)}"><b>${EH(it.name)}</b></td>
      <td>${EH(it.adduct || I18N.t("lst.mzOnly"))}</td>
      <td>${polBadge}</td>
    </tr>`;
    return `<tr>
      <td class="num font-mono"><b>${it.mz.toFixed(4)}</b></td>
      <td class="num sm muted">${delta}</td>
      <td class="lst-name" title="${EH(it.name)}"><b>${EH(it.name)}</b></td>
      <td>${seriesInfo}</td>
      <td>${EH(it.adduct || I18N.t("lst.mzOnly"))}</td>
      <td>${it.formula ? fmtSub(it.formula) : "<span class='muted'>&ndash;</span>"}</td>
      <td>${polBadge}</td>
      <td class="sm muted">${EH(it.cls || "")}</td>
      <td class="sm lst-src-col">${EH(srcName(it.list, it.listName || it.list))}</td>
    </tr>`;
  };

  function mount(/** @type {HTMLElement} */ container, /** @type {Bag} */ opt = {}) {
    if (!container) return;
    ensure();

    // Initial state of controls
    const /** @type {Bag} */ state = {
      q: opt.mz != null ? String(opt.mz) : "",
      mode: opt.hr ? "ppm" : (opt.mz != null ? "da" : (window.TBL_HR ? (TBL_HR() ? "ppm" : "da") : "ppm"))     // only the layout: the short table at unit resolution
    };

    short = state.mode === "da";
    container.innerHTML = panelHtml(state);
    setupViewSwitcher(container, opt);

    const elQ = container.querySelector("#lst-q");
    // the tolerance (number + Da/ppm) is the one of the Adducts & losses panel: same component, same remembered choice (calc-qui-ui.js, controls only)
    const elCtl = /** @type {HTMLElement | null} */ (container.querySelector("#lst-ctl"));
    const tolView = elCtl && typeof CALCQUI !== "undefined" && CALCQUI.mount ? CALCQUI.mount({ ctl: elCtl, out: document.createElement("div"), idp: "lst-", controls: true, noPol: true, arg: () => ({ text: state.q, ans: 0 }), change: () => render() }) : null;
    const elTbody = container.querySelector("#lst-tbody");
    const elWrap = /** @type {HTMLElement} */ (container.querySelector("#lst-table-wrap"));
    /** @type {IntersectionObserver | null} */ let moreObs = null;
    const CHUNK = 100;
    const elCount = container.querySelector("#lst-count");
    const elChecks = container.querySelector("#lst-src-checks");
    const elChecksLbl = container.querySelector("#lst-src-lbl");
    const elLists = container.querySelector("#lst-lists");
    const elAbout = /** @type {HTMLElement} */ (container.querySelector("#lst-about"));
    const elAddErr = /** @type {HTMLElement} */ (container.querySelector("#lst-add-err"));
    const elContSigns = container.querySelector("#lst-cont-signs");
    if (elContSigns) elContSigns.onchange = (/** @type {Event} */ e) => { setOn(/** @type {HTMLInputElement} */ (e.target).checked); };

    const /** @type {Record<string, () => string>} */ KIND_LBL = { entry: () => I18N.t("cont.excl.k.entry"), cls: () => I18N.t("cont.excl.k.cls"), list: () => I18N.t("cont.excl.k.list") };
    const elExcl = container.querySelector("#lst-excl");
    function renderExcl() {
      const ex = exclItems();
      elExcl.hidden = !ex.length;
      if (!ex.length) return;
      elExcl.innerHTML = `<b>${I18N.t("cont.excl.h")}</b> ` + ex.map((q, i) => `<span style="display:inline-flex;align-items:center;gap:4px;background:var(--soft,#f1f5f9);padding:2px 6px;border-radius:4px;margin:2px 6px 2px 0">${KIND_LBL[q.kind]()}: <b>${EH(q.key)}</b> <button type="button" data-ex="${i}" aria-label="${EH(I18N.t("cont.excl.back"))}: ${EH(q.key)}" title="${EH(I18N.t("cont.excl.back"))}" style="border:none;background:none;cursor:pointer;color:var(--muted);padding:0 2px">&times;</button></span>`).join("") + ` <button type="button" id="lst-excl-reset">${I18N.t("cont.excl.reset")}</button>`;
      elExcl.querySelectorAll("[data-ex]").forEach((/** @type {HTMLElement} */ b) => { b.onclick = () => { const q = ex[Number(b.dataset.ex)]; exclSet(q.kind, q.key, false); renderExcl(); }; });
      elExcl.querySelector("#lst-excl-reset").onclick = () => { exclReset(); renderExcl(); };
    }
    function renderSourceChecks() {
      renderExcl();
      const act = rd(KEY_LISTS, {});
      const allLists = [...ST.lists, ...ST.userLists];
      const lbl = (/** @type {Bag} */ L) => `<span>${EH(grpName(L))}</span> <span class="muted">(${(L.items || []).length})</span>`;
      if (allLists.length < 2) {                              // one list only: a fixed chip, no switch
        const L = allLists[0];
        const one = L ? `<span class="lst-chip">${lbl(L)}</span>` : "";
        elChecks.innerHTML = one;
        elChecksLbl.hidden = true;
      } else {
        elChecksLbl.hidden = false;
        const chips = allLists.map(L => `<label class="lst-chip"><input type="checkbox" data-lid="${EH(L.id)}" ${act[L.id] !== false ? "checked" : ""}> ${lbl(L)}</label>`).join("");
        elChecks.innerHTML = chips;
        elChecks.querySelectorAll("input[data-lid]").forEach((/** @type {HTMLInputElement} */ chk) => {
          chk.onchange = () => { toggleListActive(/** @type {string} */ (chk.dataset.lid), chk.checked); render(); };
        });
      }

      renderLists();
    }

    // what each list is: name (with its link), how many, which version or date, where it comes from; the user's lists can be renamed and removed
    function renderLists() {
      const link = (/** @type {Bag} */ L) => { const i = L.info || {}; const n = EH(L.builtin ? L.name_en || L.name : L.name); return i.url ? `<a href="${EH(i.url)}" target="_blank" rel="noopener noreferrer">${n}</a>` : n; };
      // the short LR table carries no references: there only the user's own lists (rename, remove) are listed
      const shown = state.mode === "da" ? ST.userLists : [...ST.lists, ...ST.userLists];
      elAbout.hidden = !shown.length;
      elLists.innerHTML = shown.map(L => {
        const i = L.info || {}, mine = !L.builtin;
        const when = i.version || i.date ? [i.version, i.date].filter(Boolean).map(EH).join(" · ") : I18N.t("lst.info.noDate");
        const src = mine ? I18N.t("lst.info.local") + (L.filename ? " · " + I18N.t("lst.info.file", { file: L.filename }) : "")
          : i.source_known === false ? `<b>${I18N.t("lst.info.unknown")}</b>. ${EH(i.provenance || "")}` : I18N.t("lst.info.source", { source: EH(L.source || "") }) + (i.doi ? ` · DOI <a href="https://doi.org/${EH(i.doi)}" target="_blank" rel="noopener noreferrer">${EH(i.doi)}</a>` : "");
        const cov = I18N.t("lst.info.count", { n: (L.items || []).length, m: i.compounds || 0 }) + (i.classes ? " · " + I18N.t("lst.info.classes", { n: i.classes }) : "");
        return `<div class="lst-info" data-lid="${EH(L.id)}" style="padding:4px 0;border-top:1px solid var(--line,#e2e8f0)">
          <div><b class="lst-info-name">${link(L)}</b>${mine ? ` <button type="button" data-ren="${EH(L.id)}" class="sm">${I18N.t("lst.info.rename")}</button> <button type="button" data-del-ul="${EH(L.id)}" title="${I18N.t("lst.userDelete")}" class="sm">${I18N.t("lst.info.remove")}</button>` : ""}</div>
          <div class="muted">${cov}</div><div class="muted">${when}</div><div class="muted lst-info-src">${src}</div></div>`;
      }).join("");
      elLists.querySelectorAll("[data-del-ul]").forEach((/** @type {HTMLElement} */ b) => {
        b.onclick = async () => { await deleteUserList(/** @type {string} */ (b.dataset.delUl)); renderSourceChecks(); render(); };
      });
      elLists.querySelectorAll("[data-ren]").forEach((/** @type {HTMLElement} */ b) => {
        b.onclick = () => {
          const L = ST.userLists.find(x => x.id === b.dataset.ren); if (!L) return;
          const row = /** @type {HTMLElement} */ (b.closest(".lst-info")), nm = /** @type {HTMLElement} */ (row.querySelector(".lst-info-name"));
          nm.innerHTML = `<input type="text" class="lst-ren" value="${EH(L.name)}" aria-label="${I18N.t("lst.info.rename")}" style="width:60%">`;
          const inp = /** @type {HTMLInputElement} */ (nm.querySelector("input")); inp.focus(); inp.select();
          const done = async (/** @type {boolean} */ save) => { if (save) await renameUserList(L.id, inp.value); renderSourceChecks(); render(); };
          inp.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") done(true); else if (e.key === "Escape") done(false); };
          inp.onblur = () => done(true);
        };
      });
    }

    const redrawAll_ = () => { renderSourceChecks(); render(); };
    const elAdd = /** @type {HTMLElement} */ (container.querySelector("#lst-add")), elFile = /** @type {HTMLInputElement} */ (container.querySelector("#lst-add-file"));
    /** @type {HTMLElement} */ (container.querySelector("#lst-add-btn")).onclick = () => elFile.click();
    elFile.onchange = () => { const f = elFile.files && elFile.files[0]; elFile.value = ""; if (f) addFile(f, elAddErr, redrawAll_); };
    /** @type {HTMLElement} */ (container.querySelector("#lst-add-tpl")).onclick = () => { if (typeof dl === "function") dl("known_mz_template.csv", TEMPLATE, "text/csv", { table: "reference-list-template" }); };
    elAdd.ondragover = (/** @type {DragEvent} */ e) => { e.preventDefault(); elAdd.style.background = "var(--soft,#f1f5f9)"; };
    elAdd.ondragleave = () => { elAdd.style.background = ""; };
    elAdd.ondrop = (/** @type {DragEvent} */ e) => { e.preventDefault(); elAdd.style.background = ""; const f = e.dataTransfer && e.dataTransfer.files[0]; if (f) addFile(f, elAddErr, redrawAll_); };

    let renderSeq = 0;
    async function render() {
      const mine = ++renderSeq;
      container.classList.toggle("lst-onelist", ST.lists.length + ST.userLists.length < 2);
      const idx = ST.idx;
      if (!idx) {
        elCount.textContent = I18N.t("lst.loadingLists");
        return;
      }

      // one field: a plain number (comma or dot) ranks by m/z, anything else filters by name, formula, class or source
      const qRaw = state.q.trim();
      const hasMz = /^\d+(?:[.,]\d*)?$|^[.,]\d+$/.test(qRaw) && numIt(qRaw) > 0;
      const qMz = hasMz ? numIt(qRaw) : NaN;
      const qTxt = hasMz ? "" : qRaw.toLowerCase();
      const compact = state.mode === "da";
      const o = CALCQUI.opts(CALCQUI.arg(state.q, 0)), byDa = o.unit === "da", tolVal = o.tol;     // the choice of the Adducts & losses panel: Da or ppm

      /** @type {Bag[]} */ let items = [];
      if (hasMz) {
        try { items = await lookup(idx, qMz, byDa, tolVal); } catch (e) { if (mine === renderSeq) { moreObs?.disconnect(); elTbody.innerHTML = ""; elWrap.scrollTop = 0; elCount.textContent = I18N.t("lst.lookupFailed", { why: String(/** @type {Error} */ (e).message || e) }); } return; }
        if (mine !== renderSeq) return;       // a newer render started while Rust answered
      } else {
        // Collect all items from active lists for browsing
        const act = activeMap();
        const pool = arrays(idx, 0);
        const seen = new Set();
        pool.forEach(arr => {
          arr.forEach((/** @type {Bag} */ it) => {
            const k = it.list + ":" + it.id + ":" + it.adduct + ":" + it.mz;
            if (!seen.has(k)) { seen.add(k); items.push(it); }
          });
        });
        items.sort((a, b) => a.mz - b.mz);
      }

      if (qTxt) {
        items = items.filter(it => {
          const t = ((it.name || "") + " " + (it.formula || "") + " " + (it.cls || "") + " " + (it.source || "")).toLowerCase();
          return t.includes(qTxt);
        });
      }

      moreObs?.disconnect(); elTbody.innerHTML = ""; elWrap.scrollTop = 0;     // before the empty return: no stale observer or rows
      if (!items.length) {
        if (elCount) elCount.textContent = I18N.t("lst.count", { n: 0 });
        elTbody.innerHTML = `<tr><td colspan="${compact ? 4 : 9}" class="muted">${/** @type {any} */ (window).EMPTYLEAF("empty.lists")}</td></tr>`;
        return;
      }

      // infinite scroll: a chunk of rows at a time, the next one when the end of the table comes into view; the count follows what is on screen
      let n = 0;
      const more = () => {
        const next = items.slice(n, n + CHUNK);
        n += next.length;
        elTbody.querySelector("#lst-more")?.remove();
        const rows = next.map((/** @type {Bag} */ it) => listRowHtml(it, compact, hasMz, byDa)).join(""), cut = n < items.length;
        const tail = cut ? `<tr id="lst-more" aria-hidden="true"><td colspan="${compact ? 4 : 9}" class="sm muted">&hellip;</td></tr>` : "";
        elTbody.insertAdjacentHTML("beforeend", rows + tail);
        if (elCount) elCount.textContent = cut ? I18N.t("lst.count.cut", { n, m: items.length }) : I18N.t("lst.count", { n: items.length });
        const sentinel = elTbody.querySelector("#lst-more");
        if (sentinel) { moreObs?.disconnect(); moreObs = new IntersectionObserver(es => { if (es.some(e => e.isIntersecting)) more(); }, { root: elWrap }); moreObs.observe(sentinel); }
      };
      more();
    }

    // Bind event listeners
    elQ.oninput = (/** @type {Event} */ e) => { state.q = /** @type {HTMLInputElement} */ (e.target).value; if (tolView) CALCQUI.sync(tolView); render(); };

    ready().then(() => {
      renderSourceChecks();
      render();
    });

    return { render, state };
  }

  // Opens the reference lists: the Liste panel of the drawer in the Dati view, the large reference dialog #refdlg elsewhere
  function open(/** @type {Bag} */ opt = {}) {
    ensure();
    if (window.BARRA && BARRA.isDataView()) {
      BARRA.setTab("lists", opt);
      return;
    }
    // outside the Dati view the lists open in the large reference dialog (tables.js), like the Periodic table
    QQQRef.open("lists", opt);
  }

  // the actions of the spectrum menu (registry azioni/1): the lists of contaminants, the known ones at this m/z (high resolution), add a laboratory contaminant
  if (typeof AZIONI !== "undefined") {
    const S = ["spec"], T = I18N.t.bind(I18N);
    AZIONI.registra({ id: "spec.contLists", gruppo: "cont", viste: S, menu: false, disponibile: (/** @type {Bag} */ c) => c.has, nome: () => T("ctx.spec.contLists"), esegui: (/** @type {Bag} */ c) => open({ mz: c.m, polarity: c.pol, hr: !!c.a?.hrp }) });
    AZIONI.registra({ id: "spec.known", gruppo: "cont", viste: S, voci: (/** @type {Bag} */ c) => {
      const kn = c.a?.hrp && c.has && isOn() && c.a.cont ? c.a.cont.match(c.m) : [];
      return kn.length ? [{ nome: T("ctx.spec.known", { name: kn[0].name, adduct: kn[0].adduct || T("ctx.spec.knownOnlyMz"), err: (kn[0].err >= 0 ? "+" : "") + kn[0].err.toFixed(1), more: kn.length > 1 ? T("ctx.spec.knownMore", { n: kn.length - 1 }) : "" }), dim: true, info: true, esegui: () => {} }] : [];
    } });
    AZIONI.registra({ id: "spec.addLab", gruppo: "cont", viste: S, menu: false, visibile: (/** @type {Bag} */ c) => !!c.a?.hrp, disponibile: (/** @type {Bag} */ c) => c.has, nome: () => T("ctx.spec.addLab"), esegui: (/** @type {Bag} */ c) => askAdd(c.m, c.pol) });
  }
  return {
    tolDa,
    buildIndex,
    find,
    findNominal,
    hasMatch,
    seriesRuns,
    tipHtml,
    readCsv,
    renameUserList,
    csvOut,
    labList,
    userList,
    addUserListFromCsv,
    deleteUserList,
    toggleListActive,
    forSpec, keller, hasUser, userMatches,
    textFor,
    compat, isExcluded, keyFor, exclSet, exclReset, exclExport, exclImport, exclItems, split,
    ensure,
    ready,
    failed,
    isOn,
    setOn,
    openLab,
    askAdd,
    addLab,
    labRows,
    mount,
    open,
    openSearch: open,
    openTargetScreening: (/** @type {Bag} */ opt = {}) => open({ ...opt, targetScreening: true }),
    target: () => (typeof window !== "undefined" && window.LISTE_TARGET) || (typeof globalThis !== "undefined" && /** @type {Bag} */ (globalThis)["require"] ? /** @type {Bag} */ (globalThis)["require"]("./liste-target.js") : null)
  };
})();

if (typeof module !== "undefined") module.exports = LISTE;
if (typeof window !== "undefined") window.LISTE = LISTE;
