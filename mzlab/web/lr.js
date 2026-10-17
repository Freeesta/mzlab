// @ts-strict
// The files and the session of the browser version, answered by the Rust engine. browser.js asks this module: upload, the list of the files, the session (explore, meta, remove, new), the state and the notebook are answered here, from the
// scan table and the index that the Rust engine reads (rust-worker.js, `lr_*` operations; files and indexes live in OPFS, archivio.js).
// What this module cannot answer it returns null for: browser.js then answers with the error `err.route.unavailable`.
//
// The answers have the JSON of mzlab/api.py, the reference of the tests (tests_e2e/e2e_lr.py, tests/golden/).
const VERSION = "1.0.1";                 // mzlab.__version__ (tests/test_lr_version.py keeps the two equal)
const WORKDIR = "/work/sessione";
const ACCEPT = /\.(mzml|dam)$/i;

// the developer text that Python's UserError carries for the errors of /api/formula (the page translates the key)
const formulaText = ({ error_key: k, params: p = /** @type {any} */ ({}) }) => ({
  "err.formula.empty": "empty formula",
  "err.formula.unreadable": `cannot read the formula '${p.formula}'`,
  "err.formula.element": `unknown element '${p.element}' in '${p.formula}'`,
  "err.formula.parens": `unbalanced parentheses in '${p.formula}'`,
  "err.adduct.unknown": `unknown adduct '${p.adduct}'`,
}[k] || k);

import { originRoute } from "./lr-origin.js";
import { engineRoute, statRoute } from "./lr-map.js";
import { createMethodRoutes } from "./lr-method.js";
import { mixesResolutions, sessionFiles } from "./lr-sessione.js";
import { W } from "./nativo-fasi.js";

/** the page shows how far the loading of `name` has got (event mzlab-index, shell.js upload); `p` = { done, total } scans of the sample, `wait` = queued for a free builder */
const tellIndex = (/** @type {string} */ name, /** @type {number} */ frac, /** @type {string | undefined} */ phase, /** @type {any} */ p = {}) => window.dispatchEvent(new CustomEvent("mzlab-index", { detail: { name, frac, phase, done: p.done || 0, total: p.total || 0, wait: !!p.wait } }));
export function createLr({ rb }) {
  const files = new Map();               // name -> row of App.files() (as an object)
  let items = null;                      // Item.info() of every item (objects), null = no session
  let hr = false, lr = false;            // the kinds of file in the session: the two cannot be mixed
  const mr = createMethodRoutes({ rpc: m => rpc(m), json: (o, st) => json(o, st), items: () => items });      // the methods (.dam) and how the items were acquired
  const missing = new Map();             // name -> scans its index lacks (a truncated data file, `build_mzidx_native`): for the row of the loading, not part of the row of App.files()
  const api = { files, missing };
  const rpc = async (msg, onp) => {
    const tr = /** @type {any} */ (window).MZLAB_TRACE, t0 = tr ? performance.now() : 0;      // tests: where the time goes
    const r = await rb.rpc(msg, onp);
    if (tr) tr.push(`rpc ${msg.op} ${msg.name || ""} ${Math.round(performance.now() - t0)}ms${r.error ? " ERR " + r.error : ""}`);
    if (r.dead || r.error) throw Object.assign(new Error(r.error || "engine lost"), { raw: r.error });
    return r;
  };
  const json = (obj, status = 200) => ({ status, text: JSON.stringify(obj) });
  const fail = (key, params = {}, text = key, status = 400) => json({ error: text, error_key: key, params }, status);
  const sorted = () => [...files.values()].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  const state = () => ({ error: null, phase: "start", app: true, workdir: WORKDIR, files: sorted(), methods: mr.list(), version: VERSION });
  const session = () => ({ session: items, app: true, workdir: WORKDIR, files: sorted(), methods: mr.list(), version: VERSION, commit: "dev" });
  const base = n => String(n || "").split(/[\\/]/).pop() || "";
  // App.safe_name: the name of a file we accept, or null
  const safeName = name => {
    const n = base(name).trim();
    if (!n || n.startsWith(".") || !ACCEPT.test(n)) return null;
    return n.replace(/[\x00-\x1f<>:"|?*]/g, "_");
  };
  const num = v => (v === null || v === undefined || v === "" ? null : Number(v));

  api.reset = async () => { await rpc({ op: "lr_clear" }); files.clear(); mr.clear(); items = null; hr = lr = false; if (rb.clearNative) rb.clearNative(); };

  api.boot = async (fresh) => {                                          // fresh: «start from scratch» (the database may be held by another tab: it is not read)
    const r = await rpc({ op: "lr_boot", fresh });
    const list = JSON.parse(r.info);
    for (const [name, file] of r.files || []) rb.noteUpload(name, file);
    for (const f of list) {
      if (/\.dam$/i.test(f.name)) continue;                                           // a method: read on demand
      if (!/\.mzml$/i.test(f.name)) continue;
      try {                                                                           // a file whose index was lost (a stopped upload) is indexed again
        if (f.nat && rb.markNative) rb.markNative(f.name);                            // a sample of a maker's file: «Download .mzML» makes its mzML
        if (!f.indexed) await rpc({ op: "lr_index", name: f.name });
        files.set(f.name, JSON.parse((await rpc({ op: "lr_entry", name: f.name })).info));
      } catch (_) { /* unreadable: it is left out of the list */ }
    }
    await mr.refresh();
    if (fresh) return;
    try {
      const nb = JSON.parse(JSON.parse((await rpc({ op: "lr_kv_get", key: "notebook" })).info).value || "{}");
      const list = sessionFiles(nb.session, f => safeName(f) && files.has(safeName(f)), nb.ui && nb.ui.files);
      if (list.length) await explore({ samples: list });
    } catch (_) { items = null; hr = lr = false; }                    // a broken notebook never stops the program
  };

  async function upload(name, blob) {
    const n = safeName(name);
    if (!n) return fail("err.file.notAccepted", { name }, `file type not accepted: ${name}`);
    const nat = /** @type {any} */ (blob).mzlabNat || null;                   // a sample of a maker's file (nativo.js): the members go to the browser storage with it
    if ((nat ? nat.size : blob.size) > 4 * 1024 * 1024 * 1024) return fail("err.memory.over4gb", {}, "file over 4 GB: too big", 500);
    let fase = "copy";                                                 // where it fails, for the diagnosis (diagnosi.js): copy → index → open
    try {                                                              // lr_put and lr_index report how far they have got: the page shows it (event mzlab-index, shell.js upload)
      const put = await rpc({ op: "lr_put", name: n, blob, nat }, nat ? p => tellIndex(n, W.copy * p.frac, "read") : undefined);
      if (nat && rb.markNative) rb.markNative(n);
      for (const [nm, file] of put.files || []) rb.noteUpload(nm, file);
      if (/\.dam$/i.test(n)) await mr.refresh();                          // a method: no index, the engine reads it when asked
      else { fase = "index"; const ix = JSON.parse((await rpc({ op: "lr_index", name: n, cen: !!window.MZLAB_CEN_IDX }, p => tellIndex(n, p.frac, p.phase, p))).info || "{}"); fase = "open"; files.set(n, JSON.parse((await rpc({ op: "lr_entry", name: n })).info)); missing.set(n, ix.missing || 0); }
    } catch (e) {
      let key = "err.engine.index", params = {}; try { const j = JSON.parse(e.raw); key = j.error_key || key; params = j.params || {}; } catch (_) { /* not an engine error */ }
      const raw = String(e && (e.raw || e.message || e) || "").slice(0, 600);      // the original text for the diagnosis (the page cuts paths out)
      if (key === "err.load.cancelled") { try { await rpc({ op: "lr_remove", name: n }); } catch (_) { /* nothing was kept */ } files.delete(n); }      // «Cancel»: no half file stays to be indexed at the next start
      return fail(key, { name: n, ...params, fase, raw }, `cannot read the file: ${n}`);
    }
    return json({ files: sorted(), methods: mr.list() });
  }

  async function explore(body) {
    const list = Array.isArray(body.samples) ? body.samples : [];
    const rows = [];
    for (const s of list) {
      const n = safeName(s.file);
      if (!n) return fail("err.file.notAccepted", { name: s.file }, `file type not accepted: ${s.file}`);
      if (!files.has(n)) return fail("err.session.fileNotFound", { name: n }, `file not found: ${n}`);
      rows.push({ file: n, label: s.label ?? null, type: s.type ?? "sample", time: num(s.time), conc: num(s.conc), cunit: s.cunit ?? null, rep: s.rep ?? null });
    }
    if (!rows.length) return fail("err.session.noFiles", {}, "add at least one file");
    if (Array.isArray(body.all) && body.all.length > rows.length && !body.append && await mixesResolutions(rpc, body.all.map(safeName), n => n && files.has(n))) return fail("err.hr.mix", {}, "low- and high-resolution files cannot be mixed");
    const add = [], appended = !!(body.append && items);
    let h = appended ? hr : false, l = appended ? lr : false;
    for (const r of rows) {
      const out = JSON.parse((await rpc({ op: "lr_items", name: r.file, sample: { label: r.label, type: r.type, time: r.time, conc: r.conc, cunit: r.cunit, rep: r.rep } })).info);
      for (const it of out) add.push(it.info);
      if (out.some(it => it.hr)) h = true; else l = true;
      if (h && l) return fail("err.hr.mix", {}, "low- and high-resolution files cannot be mixed");
    }
    items = appended ? items.concat(add) : add;
    hr = h; lr = l;
    rb.noteExplore({ append: appended, samples: (appended ? add : items).map(it => ({ file: it.file })) });     // one slot per ITEM (a mixed file has several), in the order of the session; a file added keeps the open indexes of the others
    return json(session());
  }

  function meta(body) {
    if (!items) return fail("err.session.none", {}, "no session open");
    let n = 0;
    for (const e of body.files || []) {
      const name = String(e.file || "");
      for (const it of items) {
        if (name !== it.file && name !== it.file.split("#")[0]) continue;
        if (["sample", "blank", "standard"].includes(e.type)) it.type = e.type;      // the roles of campioni/2 (role, sub, tconf) travel with the item: the page reads them back with the session; for (const k of ["role", "sub", "tconf"]) if (k in e) it[k] = e[k];
        if ("time" in e) it.time = num(e.time);
        if ("conc" in e) it.conc = num(e.conc);
        if ("cunit" in e) it.cunit = e.cunit || null;
        if ("rep" in e) it.rep = String(e.rep || "").trim() || null;
        n++;
      }
    }
    return json({ changed: n });
  }

  async function remove(body) {
    const n = safeName(body.name);
    if (!n) return fail("err.file.notAccepted", { name: body.name }, `file type not accepted: ${body.name}`);
    await rpc({ op: "lr_remove", name: n });
    files.delete(n); await mr.refresh();
    if (items) {
      items = items.filter(it => base(it.file) !== n);
      if (!items.length) { items = null; hr = lr = false; }
    }
    rb.noteRemove(n); rb.noteExplore(null);
    return json({ files: sorted(), methods: mr.list() });
  }

  async function fresh(body) {
    items = null; hr = lr = false;
    if (body.fresh) { await rpc({ op: "lr_clear" }); files.clear(); mr.clear(); if (rb.clearNative) rb.clearNative(); }
    rb.noteNew();
    return json(state());
  }

  // Returns { status, text } or null (the route is unavailable: browser.js answers err.route.unavailable).
  api.handle = async (method, url, init) => {
    const u = new URL(url, "http://x/"), path = u.pathname.replace(/^\/?(\.\/)?/, "/");
    const text = () => (typeof init.body === "string" ? init.body : null);
    try {
      if (method === "GET") {
        if (path === "/api/state") return json(state());
        if (path === "/api/session") return json(session());
        { const r = await mr.get(path, u.searchParams); if (r !== undefined) return r; }          // method, analog traces, delay, PDA, gradient
        if (path === "/api/formula") {
          try { return { status: 200, text: (await rpc({ op: "lr_formula", f: u.searchParams.get("f") || "", adduct: u.searchParams.get("adduct") || "" })).info }; }
          catch (e) { let j = null; try { j = JSON.parse(e.raw); } catch (_) { /* not an engine error */ } if (j && j.error_key) return json({ error: formulaText(j), error_key: j.error_key, params: j.params || {} }, 400); throw e; }
        }
        if (path === "/api/origin") return await originRoute(u.searchParams, { rb, rpc, json, items });
        { const r = await engineRoute(path, u.searchParams, { rb, rpc, items }); if (r !== undefined) return r; }          // maps, composition, sub-formulas: the Rust engine from the indexes (null = unavailable)
        if (path === "/api/notebook") { const v = JSON.parse((await rpc({ op: "lr_kv_get", key: "notebook" })).info).value; return { status: 200, text: v || "{}" }; }
        if (path === "/api/spazio") return { status: 200, text: (await rpc({ op: "lr_space" })).info };
        return null;
      }
      if (method !== "POST") return null;
      if (path === "/api/upload") return init.body instanceof Blob ? await upload(u.searchParams.get("name") || "", init.body) : null;
      const t = text();
      if (path === "/api/spazio") return { status: 200, text: (await rpc({ op: "lr_space", free: u.searchParams.get("derivati") ? "derived" : true })).info };
      if (t === null) return null;
      { const r = await statRoute(path, t, { rpc }); if (r !== undefined) return r; }                       // the statistics of the series: the Rust engine, no file needed
      if (path === "/api/notebook") { await rpc({ op: "lr_kv_set", key: "notebook", value: t }); return json({ ok: true }); }
      const body = JSON.parse(t || "{}");
      { const r = await mr.post(path, body); if (r !== undefined) return r; }
      if (path === "/api/explore") return await explore(body);
      if (path === "/api/meta") return meta(body);
      if (path === "/api/remove") return await remove(body);
      if (path === "/api/new") return await fresh(body);
      return null;
    } catch (e) {
      let j = null; try { j = JSON.parse(e.raw); } catch (_) { /* not an engine error */ }
      return j && j.error_key ? json({ error: j.error || j.error_key, error_key: j.error_key, params: j.params || {} }, 400) : fail("err.route.unavailable", {}, String(e && e.message || e), 500);
    }
  };
  return api;
}
