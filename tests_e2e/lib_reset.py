"""Helpers to measure the browser reset (reset.js `QQRESET.run`, browser.js `qqHardReset`): a timing probe injected before the page scripts,
a seed of synthetic stored files and records, and a snapshot of what the browser storage still holds. Not a test (no `e2e` prefix):
`misura_reset.py` and later e2e use it. The probe only wraps browser APIs to time them; the product code is untouched."""
import json, re
from lib import ROOT

DB_VER = int(re.search(r"DB_VER\s*=\s*(\d+)", (ROOT / "mzlab" / "web" / "qqq-db.js").read_text(encoding="utf-8")).group(1))

# Runs in every page before its own scripts. Marks are absolute ms (timeOrigin + now) in window.__rs.ev as [t, kind, name, detail]; each mark also goes to
# the test through the binding __rsSend (probe()). A page loaded between the reset and the last one (browser.js isolateOnce reloads once after a reset:
# Chrome gives back the sessionStorage the reset cleared, so the harness guard does not hold) is in Marks.loads as well.
PROBE = r"""(() => {
  if (window.__rs) return;
  let rsVals = null; const now = () => Math.round((performance.timeOrigin + performance.now()) * 10) / 10;
  let ls0 = null, kept = null; try { ls0 = Object.keys(localStorage).sort(); rsVals = Object.fromEntries(ls0.map(k => [k, String(localStorage.getItem(k)).slice(0, 40)])); if (window === window.top) { kept = sessionStorage.getItem("__rs.prev"); sessionStorage.removeItem("__rs.prev"); } } catch (_) {}
  const rs = window.__rs = { t0: Math.round(performance.timeOrigin), url: location.href, ls0, vals0: rsVals, ev: [], conn: { open: 0, close: 0 }, lsAfter: [] };
  // a binding call made while the page unloads is lost: the record of the top page is also kept in sessionStorage (same tab), sent again by the next page
  if (kept) setTimeout(() => { try { window.__rsSend(kept); } catch (_) {} }, 0);
  const save = () => { try { rs.nav = (performance.getEntriesByType("navigation")[0] || {}).type || null; const j = JSON.stringify(rs); if (typeof window.__rsSend === "function") window.__rsSend(j); if (window === window.top) sessionStorage.setItem("__rs.prev", j); } catch (_) {} };
  const mark = (k, n, d) => { rs.ev.push([now(), k, n === undefined ? null : n, d === undefined ? null : d]); save(); };
  rs.mark = mark;
  const wrapP = (proto, fn, kind) => { const o = proto && proto[fn]; if (!o) return; proto[fn] = function (...a) { const t = now(), n = typeof a[0] === "string" ? a[0] : null;
    let p; try { p = o.apply(this, a); } catch (e) { mark(kind, n, { t0: t, err: e && e.name }); throw e; }
    return Promise.resolve(p).then(v => { mark(kind, n, { t0: t, ok: v === undefined ? 1 : v }); return v; }, e => { mark(kind, n, { t0: t, err: e && e.name }); throw e; }); }; };
  try { wrapP(FileSystemDirectoryHandle.prototype, "removeEntry", "rm"); } catch (_) {}
  try { wrapP(CacheStorage.prototype, "delete", "cdel"); } catch (_) {}
  try { wrapP(ServiceWorkerRegistration.prototype, "unregister", "swu"); } catch (_) {}
  try { const o = IDBFactory.prototype.databases; if (o) IDBFactory.prototype.databases = function () { mark("dbs"); return o.apply(this, arguments); }; } catch (_) {}
  try {
    const dd = IDBFactory.prototype.deleteDatabase;
    IDBFactory.prototype.deleteDatabase = function (name) { const t = now(), q = dd.call(this, name); mark("dd0", name);
      for (const ev of ["success", "blocked", "error"]) q.addEventListener(ev, () => mark("dd", name, { t0: t, ev })); return q; };
    // connections to mzlab_lab opened and closed by the page (not its workers): an open one blocks deleteDatabase unless it closes on versionchange
    const op = IDBFactory.prototype.open;
    IDBFactory.prototype.open = function (name, ...a) { const q = op.call(this, name, ...a); if (name === "mzlab_lab") q.addEventListener("success", () => { rs.conn.open++; const db = q.result, c = db.close.bind(db); db.close = () => { rs.conn.close++; return c(); }; }); return q; };
  } catch (_) {}
  try {
    const st = Storage.prototype.setItem, cl = Storage.prototype.clear;
    // writes to localStorage once a reset runs, from the page or one of its same-origin frames (they share the storage); the frame's are told apart
    const top = () => { try { return window.top && window.top.__rs ? window.top : window; } catch (_) { return window; } };
    Storage.prototype.setItem = function (k, v) { const w = top(); if (w.QQ_RESETTING && this === window.localStorage) { const n = w === window ? k : k + " (frame " + location.pathname + ")"; w.__rs.lsAfter.push(n); w.__rs.mark("ls", n); } return st.call(this, k, v); };
    Storage.prototype.clear = function () { if (this === window.localStorage) mark("lsclear"); return cl.call(this); };
  } catch (_) {}
  try { const pm = Worker.prototype.postMessage; Worker.prototype.postMessage = function (m, ...a) { if (m && m.op === "lr_clear") mark("lrclear0"); return pm.call(this, m, ...a); }; } catch (_) {}
  try { const rp = History.prototype.replaceState; History.prototype.replaceState = function (s, t, u) { if (/riparti/.test(location.search) && u != null && !/riparti/.test(String(u))) mark("lrclear1"); return rp.call(this, s, t, u); }; } catch (_) {}
  const hook = (name, f) => { let cur; Object.defineProperty(window, name, { configurable: true, get: () => cur, set: v => { cur = f(v); } }); };
  hook("qqEngineStop", f => typeof f === "function" ? function () { mark("stop"); return f.apply(this, arguments); } : f);
  hook("qqHardReset", f => typeof f === "function" ? function () { mark("hard"); return f.apply(this, arguments); } : f);
  hook("QQRESET", o => {
    if (!o || typeof o !== "object") return o;
    const run = o.run; if (typeof run === "function") o.run = function (mode, opt) { mark("run", mode); return run.call(this, mode, opt); };
    let last = o.last; Object.defineProperty(o, "last", { configurable: true, get: () => last, set: v => { last = v; if (v) mark("wipe", v.mode, JSON.parse(JSON.stringify(v))); } });
    return o;
  });
  addEventListener("pagehide", () => { let k = null; try { k = Object.keys(localStorage).sort(); } catch (_) {} mark("hide", null, { ls: k }); });
  const poll = () => { if (window.qqReady === true) { mark("ready"); return; } setTimeout(poll, 20); };
  poll();
})();"""

# `n` filler files (not valid samples: the engine of a page that boots over them tries to index each one and fails) in mzlab/files and mzlab/idx,
# records in the stores of mzlab_lab, a folder, a database and caches invented by the test, the library folder and databases that mode "files" keeps,
# and localStorage keys (as SEED of e2e_reset.py). Returns the seconds it took.
SEED = r"""async ({ n, kb, ver }) => {
  const t = performance.now(), root = await navigator.storage.getDirectory(), mz = await root.getDirectoryHandle("mzlab", { create: true });
  const buf = new Uint8Array(kb * 1024); for (let i = 0; i < buf.length; i++) buf[i] = i & 255;
  const put = async (d, name) => { const w = await (await d.getFileHandle(name, { create: true })).createWritable(); await w.write(buf); await w.close(); };
  const files = await mz.getDirectoryHandle("files", { create: true }), idx = await mz.getDirectoryHandle("idx", { create: true });
  for (let i = 0; i < n; i++) { await put(files, "synt_" + i + ".mzML"); await put(idx, "synt_" + i + ".mzidx"); }
  for (const name of ["synt-invented", "librerie"]) await put(await root.getDirectoryHandle(name, { create: true }), "x.bin");
  const db = await new Promise((ok, no) => { const q = indexedDB.open("mzlab_lab", ver);
    q.onupgradeneeded = () => { const d = q.result; for (const [s, k] of [["files", null], ["kv", null], ["liste_utente", "id"], ["manifest", "path"], ["molecole", "id"]]) if (!d.objectStoreNames.contains(s)) d.createObjectStore(s, k ? { keyPath: k } : undefined); };
    q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); q.onblocked = () => no(new Error("blocked")); });
  const list = typeof LISTE !== "undefined" && LISTE.userList ? LISTE.userList("synt_list", "Synthetic", "synt.csv", []) : { id: "synt_list", name: "Synthetic", items: [] };
  await new Promise(ok => { const x = db.transaction(["liste_utente", "molecole", "kv"], "readwrite");
    x.objectStore("liste_utente").put(list); x.objectStore("molecole").put({ id: "synt_mol", nome: "synthetic", smiles: "CCO" }); x.objectStore("kv").put("1", "synt_kv");
    x.oncomplete = x.onerror = x.onabort = () => ok(); });
  db.close();
  for (const name of ["mzlab-libs", "mzlab-libdir", "synt-invented-db"]) await new Promise(ok => { const q = indexedDB.open(name, 1); q.onupgradeneeded = () => q.result.createObjectStore("s"); q.onsuccess = () => { q.result.close(); ok(); }; q.onerror = ok; });
  for (const name of ["mzlab-big-synt", "mzlab-app-synt", "synt-other-cache"]) await (await caches.open(name)).put("/synt", new Response("1"));
  localStorage.setItem("mzlab.lang", "en"); localStorage.setItem("mzlab.mappa", '{"pal":"cividis"}'); localStorage.setItem("mzlab.synt", "1");
  return Math.round(performance.now() - t) / 1000;
}"""

# What the storage holds (point 4 of the study): OPFS top level and everything under mzlab/, the record count of each store of mzlab_lab (opened only if
# it is listed, at its current version, so the snapshot never creates it), other databases, caches, mzlab/qqq localStorage keys, worker registrations, usage.
STATE = r"""async () => {
  const root = await navigator.storage.getDirectory(), top = [], under = [];
  for await (const [n] of root.entries()) top.push(n);
  const walk = async (d, pre) => { for await (const [n, h] of d.entries()) { if (h.kind === "file") under.push(pre + n); else { under.push(pre + n + "/"); await walk(h, pre + n + "/"); } } };
  try { await walk(await root.getDirectoryHandle("mzlab"), ""); } catch (_) {}
  const dbs = (await indexedDB.databases()).map(d => d.name).sort(), stores = {};
  if (dbs.includes("mzlab_lab")) await new Promise(ok => { const q = indexedDB.open("mzlab_lab");
    q.onsuccess = async () => { const d = q.result; for (const s of d.objectStoreNames) stores[s] = await new Promise(r => { try { const c = d.transaction(s).objectStore(s).count(); c.onsuccess = () => r(c.result); c.onerror = () => r(-1); } catch (_) { r(-1); } });
      d.close(); ok(); };
    q.onerror = q.onblocked = () => ok(); });
  let sw = -1; try { sw = (await navigator.serviceWorker.getRegistrations()).length; } catch (_) {}
  let usage = -1; try { usage = (await navigator.storage.estimate()).usage; } catch (_) {}
  return { opfs: top.sort(), mzlab: under.sort(), idb: dbs, stores, caches: (await caches.keys()).sort(),
    ls: Object.keys(localStorage).filter(k => /^(qqq|mzlab)/i.test(k)).sort(), sw, usage,
    files: typeof E !== "undefined" && E.files ? E.files.length : -1, url: location.search };
}"""

# localStorage keys the harness itself writes (lib.py init scripts): left out of the residues
HARNESS_LS = {"mzlab.tour.lr", "mzlab.barra.hint", "mzlab.pannelli.affianca.hint", "mzlab.safari.seen", "mzlab.disegno.schede", "mzlab.lang"}


class Marks:
    """The marks the probe sends from every page of a context, one record per page load (key: its timeOrigin), in arrival order."""
    def __init__(s): s.loads = {}
    def _got(s, source, text):
        rec = json.loads(text); s.loads[rec["t0"]] = rec
    def pair(s, pg):
        """(page that ran the reset, current page of the tab `pg`, the pages loaded in between) by timeOrigin."""
        cur = json.loads(pg.evaluate("JSON.stringify(__rs)")); t0 = cur["t0"]
        prev = [r for k, r in sorted(s.loads.items()) if k < t0 and any(e[1] in ("run", "hard") for e in r["ev"])]
        p = prev[-1] if prev else None
        mid = [r for k, r in sorted(s.loads.items()) if p and p["t0"] < k < t0 and r["url"].split("/")[2] == cur["url"].split("/")[2]]
        return p, cur, mid


def probe(ctx):
    """Install the timing probe on a context (before any page script); the returned Marks collects what its pages send."""
    m = Marks(); ctx.expose_binding("__rsSend", m._got); ctx.add_init_script(PROBE)
    return m


def seed(pg, n, size_kb=64):
    """Fill the storage of the page's origin: `n` filler files of `size_kb` KB in mzlab/files and mzlab/idx, plus the records of SEED. Seconds taken."""
    return pg.evaluate(SEED, {"n": n, "kb": size_kb, "ver": DB_VER})


def state(pg):
    """The residue snapshot of the page's origin (STATE)."""
    return pg.evaluate(STATE)


def steps(prev, cur, mid=()):
    """Per-step durations (ms) from the marks of the page that ran the reset (`prev`), of the pages loaded in between (`mid`) and of the last one (`cur`)."""
    ev = prev["ev"] if prev else []
    first = lambda k, f=lambda e: True: next((e for e in ev if e[1] == k and f(e)), None)
    t_run = first("run") or first("hard")
    t_stop = first("stop", lambda e: not t_run or e[0] >= t_run[0])
    t0 = (t_run or t_stop or [None])[0]
    after = [e for e in ev if t0 is not None and e[0] >= t0]
    rm = [e for e in after if e[1] == "rm"]
    rm_ok = [e for e in rm if e[3] and "ok" in e[3]]
    nomod = sum(1 for e in rm if e[3] and e[3].get("err") == "NoModificationAllowedError")
    dbs = next((e for e in after if e[1] == "dbs"), None)
    dd = [e for e in after if e[1] == "dd"]
    dd0 = [e for e in after if e[1] == "dd0"]
    wipe = next((e for e in after if e[1] == "wipe"), None)
    # pagehide; its mark can be lost with the page: the last step before location.replace stands in for it
    hide = next((e for e in after if e[1] == "hide"), None) or next((e for e in after if e[1] == "lsclear"), None) or wipe
    cd = [e for e in after if e[1] == "cdel"]; sw = [e for e in after if e[1] == "swu"]
    span = lambda es: round(max(e[0] for e in es) - min(e[3]["t0"] for e in es)) if es else None
    blocked = sorted({e[2] for e in dd if e[3]["ev"] == "blocked"})
    first_ev = {}
    for e in dd: first_ev.setdefault(e[2], e)
    late_ok = sorted({e[2] for e in dd if e[3]["ev"] == "success" and e[2] in blocked})
    ce = [e for r in list(mid) + [cur] if r for e in r["ev"]]          # lr_clear runs in whichever of them had ?riparti=1
    cfirst = lambda k: next((e[0] for e in ce if e[1] == k), None)
    ready, l0, l1 = next((e[0] for e in (cur or {}).get("ev", []) if e[1] == "ready"), None), cfirst("lrclear0"), cfirst("lrclear1")
    out = {
        "stop_to_first_rm": round(rm_ok[0][0] - t_stop[0]) if rm_ok and t_stop else None,
        "opfs_ms": span(rm), "rm_n": len(rm_ok), "nomod_retries": nomod,
        "rm_err": sorted({f"{e[2]}:{e[3]['err']}" for e in rm if e[3] and e[3].get("err") and e[3]["err"] != "NoModificationAllowedError"}),
        "idb_ms": round(max(e[0] for e in dd) - (dbs or dd0[0])[0]) if dd and (dbs or dd0) else None,
        "idb_blocked": blocked, "idb_done_later": late_ok, "idb_no_event": sorted({e[2] for e in dd0} - {e[2] for e in dd}),
        "caches_ms": span(cd), "caches_n": len(cd), "sw_ms": span(sw), "sw_n": len(sw),
        "wipe_ms": round(wipe[0] - t0) if wipe and t0 is not None else None,
        "to_hide_ms": round(hide[0] - t0) if hide and t0 is not None else None,
        "reload_to_ready_ms": round(ready - hide[0]) if ready and hide else None,
        "total_ms": round(ready - t0) if ready and t0 is not None else None,
        "lr_clear_ms": round(l1 - l0) if l0 and l1 else None,
        "pages_between": len(mid), "last_nav": cur.get("nav") if cur else None,
        "hide_to_last_page_ms": round(cur["t0"] - hide[0]) if cur and hide else None,
        "ls_written_after_reset": sorted(set(prev.get("lsAfter", []))) if prev else [],
        "lists_conn_open": (prev["conn"]["open"] - prev["conn"]["close"]) if prev else None,
        "report": wipe[3] if wipe else None,
    }
    return out
