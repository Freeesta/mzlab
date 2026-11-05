// @ts-check
// Fallbacks for APIs some engines lack (Safari/iPadOS first; table in docs/agenti/compat-safari.md). Feature detection only, never the user agent:
// each block patches one missing or failing API, so callers stay the same everywhere. Imported first by capacita.js; installa(g) is exported for the
// Node tests (tests/compat_node.mjs), which pass a fake global.

/** AbortSignal.any (Safari 17.4): a signal that aborts with the first of `signals`. @param {any} g @param {AbortSignal[]} signals */
export function anySignal(g, signals) {
  const ac = new g.AbortController();
  for (const s of signals) if (s.aborted) { ac.abort(s.reason); return ac.signal; }
  for (const s of signals) s.addEventListener("abort", () => ac.abort(s.reason), { once: true, signal: ac.signal });
  return ac.signal;
}

/** The chunks given to write() as one Blob (string, Blob, BufferSource or {type:"write", data}; positions and seek are not offered). @param {any} g @param {any[]} parts */
export function blobOf(g, parts) {
  return new g.Blob(parts.map(p => {
    if (p && typeof p === "object" && p.type === "write" && "data" in p) {
      if (p.position !== undefined && p.position !== null) throw new g.DOMException("write at a position is not available in this browser", "NotSupportedError");
      return p.data;
    }
    if (p && typeof p === "object" && (p.type === "seek" || p.type === "truncate")) throw new g.DOMException(p.type + " is not available in this browser", "NotSupportedError");
    return p;
  }));
}

/** FileSystemFileHandle.createWritable (absent in WebKit before Safari 26): the chunks are kept in memory and written at close() by a small worker with a
 *  synchronous handle (the only way WebKit writes in the private storage). Only for files of the private storage (OPFS), the only ones the site writes from the page.
 *  @param {any} g @param {() => any} worker */
export function writableFallback(g, worker) {
  let seq = 0;
  /** @type {Map<number, {ok: (v: any) => void, no: (e: any) => void}>} */ const wait = new Map();
  /** @type {any} */ let w = null;
  const send = (/** @type {string[]} */ path, /** @type {Blob} */ blob) => new Promise((ok, no) => {
    if (!w) { w = worker(); w.onmessage = (/** @type {MessageEvent} */ e) => { const p = wait.get(e.data.id); if (!p) return; wait.delete(e.data.id); if (e.data.ok) p.ok(undefined); else p.no(new g.DOMException(e.data.message, e.data.name || "UnknownError")); }; }
    const id = ++seq; wait.set(id, { ok, no }); w.postMessage({ id, path, blob });
  });
  /** @this {any} @param {any} opts */
  return async function createWritable(opts) {
    if (opts && opts.keepExistingData) throw new g.DOMException("keepExistingData is not available in this browser", "NotSupportedError");
    const path = await (await g.navigator.storage.getDirectory()).resolve(this);
    if (!path || !path.length) throw new g.DOMException("only files of the private storage of the site can be written in this browser", "NotSupportedError");
    /** @type {any[] | null} */ let parts = [];
    const live = () => { if (!parts) throw new g.DOMException("the stream is closed", "InvalidStateError"); return parts; };
    return {
      async write(/** @type {any} */ data) { live().push(data); },
      async close() { const b = blobOf(g, live()); parts = null; await send(path, b); },
      async abort() { parts = null; },
    };
  };
}

/** Clipboard writeText refused (Safari: the click is spent once an await came between): the old copy command, same text. @param {any} g @param {string} text */
export function copyCommand(g, text) {
  const d = g.document; if (!d || !d.body || typeof d.execCommand !== "function") return false;
  const back = d.activeElement, t = d.createElement("textarea");
  t.value = text; t.setAttribute("readonly", ""); t.setAttribute("aria-hidden", "true");
  t.style.cssText = "position:fixed;top:0;left:0;width:1px;height:1px;opacity:0;font-size:16px";
  d.body.appendChild(t); t.select();
  let ok = false; try { ok = !!d.execCommand("copy"); } catch (_) { ok = false; }
  t.remove(); if (back && typeof back.focus === "function") try { back.focus({ preventScroll: true }); } catch (_) { /* gone */ }
  return ok;
}

/** True on a device whose only pointer is a finger (iPad, Android tablet): its file picker greys out files whose extension the system does not know. @param {any} g */
export const touchOnly = (g) => !!(g.matchMedia && g.matchMedia("(hover: none) and (pointer: coarse)").matches);

/** A file input that lists extensions (".raw,.wiff,…") opens with every file on a touch-only device; the app checks the type of what is chosen itself.
 *  The list stays in data-accept. @param {any} g @param {any} i */
export function freeAccept(g, i) {
  if (!i || i.type !== "file" || !i.hasAttribute || !i.hasAttribute("accept") || !touchOnly(g)) return false;
  if (!/(^|,)\s*\./.test(i.getAttribute("accept") || "")) return false;
  i.dataset.accept = i.getAttribute("accept"); i.removeAttribute("accept");
  return true;
}

/** Installs every fallback the global `g` needs; returns the names installed (for the tests and the diagnosis). @param {any} g @param {{worker?: () => any}} [o] */
export function installa(g, o = {}) {
  /** @type {string[]} */ const done = [];
  if (g.AbortSignal && typeof g.AbortSignal.any !== "function") { g.AbortSignal.any = (/** @type {AbortSignal[]} */ s) => anySignal(g, s); done.push("AbortSignal.any"); }
  const FH = g.FileSystemFileHandle;          // the synchronous handle exists only in workers: the writer worker checks it and answers with its error
  if (FH && FH.prototype && typeof FH.prototype.createWritable !== "function" && g.navigator && g.navigator.storage && typeof g.Worker === "function") {
    FH.prototype.createWritable = writableFallback(g, o.worker || (() => new g.Worker(new URL("compat-scrivi-worker.js", import.meta.url))));
    done.push("createWritable");
  }
  const cb = g.navigator && g.navigator.clipboard;
  if (cb && typeof cb.writeText === "function" && g.document) {
    const orig = cb.writeText.bind(cb);
    try {
      cb.writeText = (/** @type {string} */ text) => orig(text).catch((/** @type {any} */ e) => { if (copyCommand(g, String(text))) return; throw e; });
      done.push("clipboard.writeText");
    } catch (_) { /* read-only clipboard object: callers keep their own message */ }
  }
  const IP = g.HTMLInputElement && g.HTMLInputElement.prototype;
  if (IP && g.document) {
    // a tap on the input or its label (event in capture), and the inputs opened from code, also when they are not in the page
    g.document.addEventListener("click", (/** @type {Event} */ e) => freeAccept(g, e.target), true);
    for (const m of ["click", "showPicker"]) {
      const f = IP[m]; if (typeof f !== "function") continue;
      IP[m] = function (/** @type {any[]} */ ...a) { freeAccept(g, this); return f.apply(this, a); };
    }
    done.push("accept");
  }
  return done;
}

const G = /** @type {any} */ (globalThis);
if (typeof window !== "undefined" && !G.__compat) G.__compat = installa(G);
