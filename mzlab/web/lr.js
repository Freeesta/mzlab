// The flow of the low-resolution files WITHOUT Pyodide (opt-in: ?lr=1 or localStorage qqq.lr = "1", with the Rust engine). browser.js asks this module
// first: upload, the list of the files, the session (explore, meta, remove, new), the state and the notebook are answered here, from the
// scan table and the index that the Rust engine reads (rust-worker.js, `lr_*` operations; files and indexes live in OPFS, archivio.js).
// What this module cannot answer it returns null for: browser.js then starts Pyodide (once), replays the session into it (`replay()`)
// and from then on everything goes the classic way. Python and this module never answer for the same session at the same time.
//
// The answers have the JSON of mzlab/api.py (tests_e2e/e2e_lr.py compares them with Python's).
const VERSION = "0.1.0";                 // mzlab.__version__ (tests/test_lr_version.py keeps the two equal)
const WORKDIR = "/work/sessione";
const ACCEPT = /\.(mzml|dam)$/i;

// the developer text that Python's UserError carries for the errors of /api/formula (the page translates the key)
const formulaText = ({ error_key: k, params: p = {} }) => ({
  "err.formula.empty": "empty formula",
  "err.formula.unreadable": `cannot read the formula '${p.formula}'`,
  "err.formula.element": `unknown element '${p.element}' in '${p.formula}'`,
  "err.formula.parens": `unbalanced parentheses in '${p.formula}'`,
  "err.adduct.unknown": `unknown adduct '${p.adduct}'`,
}[k] || k);

export function createLr({ rb }) {
  const files = new Map();               // name -> row of App.files() (as an object)
  let samples = [];                      // the session as the page asked for it, with the later edits (what replay() sends to Python)
  let items = null;                      // Item.info() of every item (objects), null = no session
  let hr = false, lr = false;            // the kinds of file in the session: the two cannot be mixed
  let foreign = false;                   // something in OPFS that only Python reads (a .dam, a file without index): every request goes to Python
  const api = { files, get foreign() { return foreign; }, session: () => ({ samples }), freeze: () => { foreign = true; } };
  const rpc = async (msg) => {
    const tr = window.MZLAB_TRACE, t0 = tr ? performance.now() : 0;      // tests: where the time goes
    const r = await rb.rpc(msg);
    if (tr) tr.push(`rpc ${msg.op} ${msg.name || ""} ${Math.round(performance.now() - t0)}ms${r.error ? " ERR " + r.error : ""}`);
    if (r.dead || r.error) throw Object.assign(new Error(r.error || "engine lost"), { raw: r.error });
    return r;
  };
  const json = (obj, status = 200) => ({ status, text: JSON.stringify(obj) });
  const fail = (key, params = {}, text = key, status = 400) => json({ error: text, error_key: key, params }, status);
  const sorted = () => [...files.values()].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  const state = () => ({ error: null, phase: "start", app: true, workdir: WORKDIR, files: sorted(), methods: [], version: VERSION });
  const session = () => ({ session: items, app: true, workdir: WORKDIR, files: sorted(), methods: [], version: VERSION, commit: "dev" });
  const base = n => String(n || "").split(/[\\/]/).pop();
  // App.safe_name: the name of a file we accept, or null
  const safeName = name => {
    const n = base(name).trim();
    if (!n || n.startsWith(".") || !ACCEPT.test(n)) return null;
    return n.replace(/[\x00-\x1f<>:"|?*]/g, "_");
  };
  const num = v => (v === null || v === undefined || v === "" ? null : Number(v));

  // «start from scratch» (?riparti=1) when the page could not delete the database itself: files, indexes, notebook
  api.reset = async () => { await rpc({ op: "lr_clear" }); files.clear(); items = null; samples = []; hr = lr = false; foreign = false; };

  // the files of an earlier visit (OPFS): the list, the index of every one, the File for the bridge
  api.boot = async () => {
    const r = await rpc({ op: "lr_boot" });
    const list = JSON.parse(r.info);
    for (const [name, file] of r.files || []) rb.noteUpload(name, file);
    for (const f of list) {
      if (!/\.mzml$/i.test(f.name) || !f.indexed) { foreign = true; continue; }
      try { files.set(f.name, JSON.parse((await rpc({ op: "lr_entry", name: f.name })).info)); } catch (_) { foreign = true; }
    }
  };

  async function upload(name, blob) {
    const n = safeName(name);
    if (!n) return fail("err.file.notAccepted", { name }, `file type not accepted: ${name}`);
    if (/\.dam$/i.test(n)) { foreign = true; return null; }               // the methods are read by Python
    if (blob.size > 4 * 1024 * 1024 * 1024) return fail("err.memory.over4gb", {}, "file over 4 GB: too big", 500);
    try {
      const put = await rpc({ op: "lr_put", name: n, blob });
      for (const [nm, file] of put.files || []) rb.noteUpload(nm, file);
      await rpc({ op: "lr_index", name: n });
      files.set(n, JSON.parse((await rpc({ op: "lr_entry", name: n })).info));
    } catch (e) { foreign = true; return null; }                         // any trouble: Python takes the file as it always did
    return json({ files: sorted(), methods: [] });
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
    const add = [];
    let h = body.append && items ? hr : false, l = body.append && items ? lr : false;
    for (const r of rows) {
      const out = JSON.parse((await rpc({ op: "lr_items", name: r.file, sample: { label: r.label, type: r.type, time: r.time, conc: r.conc, cunit: r.cunit, rep: r.rep } })).info);
      for (const it of out) add.push(it.info);
      if (out.some(it => it.hr)) h = true; else l = true;
      if (h && l) return fail("err.hr.mix", {}, "low- and high-resolution files cannot be mixed");
    }
    if (body.append && items) { items = items.concat(add); samples = samples.concat(rows); } else { items = add; samples = rows; }
    hr = h; lr = l;
    rb.noteExplore({ samples: items.map(it => ({ file: it.file })) });            // one slot per ITEM (a mixed file has several), in the order of the session
    return json(session());
  }

  function meta(body) {
    if (!items) return fail("err.session.none", {}, "no session open");
    let n = 0;
    for (const e of body.files || []) {
      const name = String(e.file || "");
      for (const it of items) {
        if (name !== it.file && name !== it.file.split("#")[0]) continue;
        if (["sample", "blank", "standard"].includes(e.type)) it.type = e.type;
        if ("time" in e) it.time = num(e.time);
        if ("conc" in e) it.conc = num(e.conc);
        if ("cunit" in e) it.cunit = e.cunit || null;
        if ("rep" in e) it.rep = String(e.rep || "").trim() || null;
        n++;
      }
      for (const s of samples) {
        if (name !== s.file && name !== s.file.split("#")[0]) continue;
        if (["sample", "blank", "standard"].includes(e.type)) s.type = e.type;
        if ("time" in e) s.time = num(e.time);
        if ("conc" in e) s.conc = num(e.conc);
        if ("cunit" in e) s.cunit = e.cunit || null;
        if ("rep" in e) s.rep = String(e.rep || "").trim() || null;
      }
    }
    return json({ changed: n });
  }

  async function remove(body) {
    const n = safeName(body.name);
    if (!n) return fail("err.file.notAccepted", { name: body.name }, `file type not accepted: ${body.name}`);
    await rpc({ op: "lr_remove", name: n });
    files.delete(n);
    if (items) {
      items = items.filter(it => base(it.file) !== n);
      samples = samples.filter(s => s.file !== n);
      if (!items.length) { items = null; hr = lr = false; }
    }
    rb.noteRemove(n); rb.noteExplore(null);
    return json({ files: sorted(), methods: [] });
  }

  async function fresh(body) {
    items = null; samples = []; hr = lr = false;
    if (body.fresh) { await rpc({ op: "lr_clear" }); files.clear(); foreign = false; }
    rb.noteNew();
    return json(state());
  }

  // Returns { status, text } or null (Python answers).
  api.handle = async (method, url, init) => {
    if (foreign) return null;
    const u = new URL(url, "http://x/"), path = u.pathname.replace(/^\/?(\.\/)?/, "/");
    const text = () => (typeof init.body === "string" ? init.body : null);
    try {
      if (method === "GET") {
        if (path === "/api/state") return json(state());
        if (path === "/api/session") return json(session());
        if (path === "/api/gradient") return json({ gradient: null });                      // none typed, none in a .dam (a POST would have sent everything to Python)
        if (path === "/api/formula") {
          try { return { status: 200, text: (await rpc({ op: "lr_formula", f: u.searchParams.get("f") || "", adduct: u.searchParams.get("adduct") || "" })).info }; }
          catch (e) { let j = null; try { j = JSON.parse(e.raw); } catch (_) { /* not an engine error */ } if (j && j.error_key) return json({ error: formulaText(j), error_key: j.error_key, params: j.params || {} }, 400); throw e; }
        }
        if (path === "/api/method_warnings") return json({ warnings: [] });                // no .dam in OPFS (foreign would be set): nothing to compare
        if (path === "/api/notebook") { const v = JSON.parse((await rpc({ op: "lr_kv_get", key: "notebook" })).info).value; return { status: 200, text: v || "{}" }; }
        if (path === "/api/spazio") return { status: 200, text: (await rpc({ op: "lr_space" })).info };
        return null;
      }
      if (method !== "POST") return null;
      if (path === "/api/upload") return init.body instanceof Blob ? await upload(u.searchParams.get("name") || "", init.body) : null;
      const t = text();
      if (path === "/api/spazio") return { status: 200, text: (await rpc({ op: "lr_space", free: true })).info };
      if (t === null) return null;
      if (path === "/api/notebook") { await rpc({ op: "lr_kv_set", key: "notebook", value: t }); return json({ ok: true }); }
      const body = JSON.parse(t || "{}");
      if (path === "/api/explore") return await explore(body);
      if (path === "/api/meta") return meta(body);
      if (path === "/api/remove") return await remove(body);
      if (path === "/api/new") return await fresh(body);
      return null;
    } catch (e) { foreign = true; return null; }
  };
  return api;
}
