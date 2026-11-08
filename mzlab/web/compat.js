// @ts-check
// Fallbacks for APIs some engines lack or send differently (Safari/iPadOS first; table in docs/agenti/compat-safari.md). Feature detection only, never the user agent:
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

/** A copy whose content is still being made (the provenance line asks the engine, an image is drawn): Safari writes the clipboard only while the
 *  click runs, so the item is handed over now, as a ClipboardItem of Promises. Without ClipboardItem, or refused, the plain text goes through writeText
 *  once ready (an image alone rejects: the caller saves it). @param {any} g @param {Record<string, string | Blob | Promise<string | Blob>>} parts MIME type -> content */
export function copyNow(g, parts) {
  const cb = g.navigator && g.navigator.clipboard;
  if (!cb) return Promise.reject(new g.DOMException("no clipboard in this browser", "NotAllowedError"));
  const plain = () => Promise.resolve(parts["text/plain"]).then(t => cb.writeText(String(t)));
  if (typeof g.ClipboardItem !== "function" || typeof cb.write !== "function") return "text/plain" in parts ? plain() : Promise.reject(new g.DOMException("no ClipboardItem", "NotAllowedError"));
  /** @type {Promise<void>} */ let w;
  try {
    /** @type {Record<string, Promise<Blob>>} */ const item = {};
    for (const [t, v] of Object.entries(parts)) item[t] = Promise.resolve(v).then(x => (x instanceof g.Blob ? x : new g.Blob([String(x)], { type: t })));
    w = cb.write([new g.ClipboardItem(item)]);
  } catch (e) { w = Promise.reject(e); }
  return w.catch(e => ("text/plain" in parts ? plain() : Promise.reject(e)));
}

/** Safari on a Mac sends a trackpad pinch as gesture* events, never as the Ctrl+wheel that the other browsers send and every chart zoom listens to:
 *  each gesturechange becomes that wheel on the element under the fingers (deltaY = -100·ln of the step, as Chromium). Only over a surface that zooms
 *  itself (touch-action:none) is the page zoom stopped; a gesture a view handles (defaultPrevented) and a pinch of fingers on a screen are left alone.
 *  @param {any} g */
export function pinchWheel(g) {
  const d = g.document; let last = 1, on = false, touches = 0;
  const zoomable = (/** @type {any} */ t) => { for (let e = t; e && e.nodeType === 1; e = e.parentElement) if (g.getComputedStyle(e).touchAction === "none") return true; return false; };
  for (const [t, k] of [["pointerdown", 1], ["pointerup", -1], ["pointercancel", -1]]) d.addEventListener(t, (/** @type {any} */ e) => { if (e.pointerType === "touch") touches = Math.max(0, touches + /** @type {number} */ (k)); }, true);
  d.addEventListener("gesturestart", (/** @type {any} */ e) => { last = 1; on = !e.defaultPrevented && !touches && zoomable(e.target); if (on) e.preventDefault(); });
  d.addEventListener("gesturechange", (/** @type {any} */ e) => {
    if (!on || e.defaultPrevented) return;
    e.preventDefault();
    const s = e.scale > 0 ? e.scale : last, r = s / last; last = s;
    if (r === 1) return;
    e.target.dispatchEvent(new g.WheelEvent("wheel", { bubbles: true, cancelable: true, composed: true, view: g, ctrlKey: true, clientX: e.clientX, clientY: e.clientY, screenX: e.screenX, screenY: e.screenY, deltaY: -100 * Math.log(r), deltaMode: 0 }));
  });
  d.addEventListener("gestureend", (/** @type {any} */ e) => { if (on && !e.defaultPrevented) e.preventDefault(); on = false; });
}

// Safari and iPadOS refuse a canvas over 16 777 216 pixels (toBlob gives null, the drawing is lost): canvasScale is the largest scale <= `s` that keeps
// a w x h image under that area, for every export that rasterises at a multiple of its size. cappedScale also tells the user once per export when it lowered it.
export const MAX_CANVAS_PX = 16777216;
/** @param {number} w @param {number} h @param {number} s */
export const canvasScale = (w, h, s) => (w > 0 && h > 0 ? Math.min(s, Math.sqrt(MAX_CANVAS_PX / (w * h)) * 0.999) : s);
/** canvasScale plus a calm notice (g.toast, text from g.I18N) with the final size when the cap lowered the scale @param {any} g @param {number} w @param {number} h @param {number} s */
export function cappedScale(g, w, h, s) {
  const k = canvasScale(w, h, s);
  if (k < s && typeof g.toast === "function" && g.I18N) g.toast(g.I18N.t("export.canvasCap", { w: Math.round(w * k), h: Math.round(h * k) }));
  return k;
}

/** Safari in a tab of its own (WebKit's GestureEvent, absent in the other engines): "mac", or "ipad" with a touch screen (several touch points, as an iPad with a trackpad; every browser there is WebKit);
 *  "" in the other engines, the desktop app (a WebKit view on a Mac, no 7-day limit), the phone layout and a frame. "mac" and "ipad" get the first-visit
 *  notice (avviso-safari.js, loaded after the page). @param {any} g @returns {"" | "mac" | "ipad"} */
export const safariKind = (g) => (!g.GestureEvent || g.MZLAB_DESKTOP || g.MZLAB_PHONE || g.parent !== g ? "" : touchOnly(g) || (g.navigator && g.navigator.maxTouchPoints > 1) ? "ipad" : "mac");

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
  if (g.GestureEvent && g.document && typeof g.WheelEvent === "function" && typeof g.getComputedStyle === "function" && !touchOnly(g)) { pinchWheel(g); done.push("pinch"); }
  return done;
}

const G = /** @type {any} */ (globalThis);
if (typeof window !== "undefined" && !G.__compat) {
  G.__compat = installa(G); G.clipboardCopy = (/** @type {Record<string, any>} */ parts) => copyNow(G, parts); G.canvasScale = (/** @type {number} */ w, /** @type {number} */ h, /** @type {number} */ s) => cappedScale(G, w, h, s);
  const notice = () => { if (safariKind(G) && typeof G.carica === "function") G.carica("m:avviso-safari.js").catch(() => { /* offline: no notice */ }); };
  if (G.document.readyState === "complete") notice(); else G.addEventListener("load", notice, { once: true });
}
