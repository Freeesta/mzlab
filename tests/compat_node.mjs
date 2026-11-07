// Checks the fallbacks of mzlab/web/compat.js on fake globals (what Safari/iPadOS lacks: AbortSignal.any, createWritable, a refused clipboard, the accept
// list of the iPad file picker, a copy handed over inside the click, the gesture* pinch of a Mac trackpad, the canvas area cap); prints one JSON object of named checks. See tests/test_compat.py.
import { installa, anySignal, freeAccept, copyNow, pinchWheel, canvasScale, MAX_CANVAS_PX } from "../mzlab/web/compat.js";

const out = {};
const check = async (name, fn) => { try { out[name] = (await fn()) === true ? true : "false"; } catch (e) { out[name] = "threw: " + (e && e.message || e); } };

// a fake element / document, only what compat.js touches
function el(tag, attrs = {}) {
  const a = { ...attrs };
  return {
    tagName: tag, type: a.type, dataset: {}, style: {}, value: "",
    hasAttribute: k => k in a, getAttribute: k => (k in a ? a[k] : null), setAttribute: (k, v) => { a[k] = v; }, removeAttribute: k => { delete a[k]; },
    select() {}, remove() { this.removed = true; }, focus() { this.focused = true; },
  };
}
function fakeDoc(copyOk) {
  const d = { listeners: [], body: { appendChild(t) { d.last = t; } }, activeElement: el("button"), createElement: t => el(t),
    execCommand: c => { d.copied = c === "copy" ? d.last.value : null; return copyOk; }, addEventListener: (t, f, c) => d.listeners.push([t, f, c]) };
  return d;
}
const media = coarse => q => ({ matches: coarse && q === "(hover: none) and (pointer: coarse)" });

// a fake private storage: path -> bytes, written by the fake worker
function opfs(files) {
  const handle = path => ({ path });
  class FH {}
  FH.prototype.createSyncAccessHandle = function () {};
  const root = { resolve: async h => (h && h.path) || null };
  const worker = () => {
    const w = { posted: [], postMessage(m) { w.posted.push(m); m.blob.text().then(t => { files[m.path.join("/")] = t; w.onmessage({ data: { id: m.id, ok: !m.path.includes("locked"), name: "NoModificationAllowedError", message: "locked" } }); }); } };
    return w;
  };
  return { FH, root, worker, handle };
}

await check("abort: AbortSignal.any installed when missing, native kept", () => {
  const g = { AbortController, AbortSignal: { }, Blob, DOMException };
  const done = installa(g);
  const native = () => 0, g2 = { AbortController, AbortSignal: { any: native }, Blob, DOMException }; installa(g2);
  return done.includes("AbortSignal.any") && typeof g.AbortSignal.any === "function" && g2.AbortSignal.any === native;
});
await check("abort: aborts with the first signal and its reason", () => {
  const a = new AbortController(), b = new AbortController(), s = anySignal({ AbortController }, [a.signal, b.signal]);
  if (s.aborted) return false;
  b.abort("second"); a.abort("first");
  return s.aborted && s.reason === "second";
});
await check("abort: an already aborted signal gives an aborted one", () => {
  const a = new AbortController(); a.abort("x");
  const s = anySignal({ AbortController }, [new AbortController().signal, a.signal]);
  return s.aborted && s.reason === "x";
});

await check("writable: fallback writes the chunks at close, through the worker", async () => {
  const files = {}, o = opfs(files);
  const g = { Blob, DOMException, Worker: function () {}, FileSystemFileHandle: o.FH, navigator: { storage: { getDirectory: async () => o.root } } };
  const done = installa(g, { worker: o.worker });
  const h = Object.assign(Object.create(o.FH.prototype), o.handle(["mzlab", "flow", "a.txt"]));
  const w = await h.createWritable(); await w.write("ab"); await w.write(new Uint8Array([99])); await w.write({ type: "write", data: new Blob(["d"]) });
  if (files["mzlab/flow/a.txt"] !== undefined) return false;          // nothing before close
  await w.close();
  let closed = false; try { await w.write("x"); } catch (e) { closed = e.name === "InvalidStateError"; }
  return done.includes("createWritable") && files["mzlab/flow/a.txt"] === "abcd" && closed;
});
await check("writable: refusals are DOMExceptions (outside OPFS, keepExistingData, position, worker error)", async () => {
  const files = {}, o = opfs(files);
  const g = { Blob, DOMException, Worker: function () {}, FileSystemFileHandle: o.FH, navigator: { storage: { getDirectory: async () => o.root } } };
  installa(g, { worker: o.worker });
  const mk = p => Object.assign(Object.create(o.FH.prototype), o.handle(p));
  const names = [];
  const grab = async f => { try { await f(); names.push("none"); } catch (e) { names.push(e.name); } };
  await grab(() => mk(null).createWritable());
  await grab(() => mk(["a"]).createWritable({ keepExistingData: true }));
  await grab(async () => { const w = await mk(["b"]).createWritable(); await w.write({ type: "write", data: "x", position: 3 }); await w.close(); });
  await grab(async () => { const w = await mk(["locked"]).createWritable(); await w.write("x"); await w.close(); });
  return JSON.stringify(names) === JSON.stringify(["NotSupportedError", "NotSupportedError", "NotSupportedError", "NoModificationAllowedError"]);
});
await check("writable: a native createWritable is kept, and no fallback without workers", () => {
  const native = function () {};
  class A {} A.prototype.createWritable = native; A.prototype.createSyncAccessHandle = function () {};
  class B {}
  const nav = { storage: { getDirectory: async () => ({}) } };
  const ga = { Blob, DOMException, Worker: function () {}, FileSystemFileHandle: A, navigator: nav }, gb = { Blob, DOMException, FileSystemFileHandle: B, navigator: nav };
  return !installa(ga).includes("createWritable") && A.prototype.createWritable === native && !installa(gb).includes("createWritable") && B.prototype.createWritable === undefined;
});

await check("clipboard: a refused writeText falls back to the copy command", async () => {
  const d = fakeDoc(true), back = d.activeElement;
  const g = { document: d, navigator: { clipboard: { writeText: () => Promise.reject(Object.assign(new Error("no"), { name: "NotAllowedError" })) } } };
  installa(g); await g.navigator.clipboard.writeText("m/z 301.1");
  return d.copied === "m/z 301.1" && d.last.removed === true && back.focused === true;
});
await check("clipboard: both refused -> the original error; accepted -> no copy command", async () => {
  const d = fakeDoc(false), err = Object.assign(new Error("no"), { name: "NotAllowedError" });
  const g = { document: d, navigator: { clipboard: { writeText: () => Promise.reject(err) } } };
  installa(g); let same = false; try { await g.navigator.clipboard.writeText("x"); } catch (e) { same = e === err; }
  const d2 = fakeDoc(true), g2 = { document: d2, navigator: { clipboard: { writeText: async () => {} } } };
  installa(g2); await g2.navigator.clipboard.writeText("y");
  return same && d2.copied === undefined;
});

await check("accept: touch-only device frees an extension list and keeps it in data-accept", () => {
  const i = el("input", { type: "file", accept: ".mzml,.raw,.wiff,.scan,.gz" });
  return freeAccept({ matchMedia: media(true) }, i) === true && !i.hasAttribute("accept") && i.dataset.accept === ".mzml,.raw,.wiff,.scan,.gz";
});
await check("accept: kept with a mouse, for a MIME-only list and for other inputs", () => {
  const a = el("input", { type: "file", accept: ".raw" }), b = el("input", { type: "file", accept: "image/*" }), c = el("input", { type: "text", accept: ".raw" });
  return !freeAccept({ matchMedia: media(false) }, a) && a.hasAttribute("accept") && !freeAccept({ matchMedia: media(true) }, b) && b.hasAttribute("accept")
    && !freeAccept({ matchMedia: media(true) }, c) && !freeAccept({}, a);
});
await check("accept: input.click() from code and a tap (capture listener) both free the list", () => {
  const d = fakeDoc(true); let clicked = 0;
  function HTMLInputElement() {} HTMLInputElement.prototype.click = function () { clicked++; };
  const g = { document: d, matchMedia: media(true), HTMLInputElement };
  const done = installa(g);
  const i = Object.assign(Object.create(HTMLInputElement.prototype), el("input", { type: "file", accept: ".wiff" }));
  i.click();
  const j = el("input", { type: "file", accept: ".raw" }), cap = d.listeners.find(([t, , c]) => t === "click" && c === true);
  cap[1]({ target: j });
  return done.includes("accept") && clicked === 1 && !i.hasAttribute("accept") && !j.hasAttribute("accept");
});

// a clipboard as strict as Safari's: write() and writeText() only while the click runs (here: before the first await of the test)
function strictClip() {
  const c = { live: true, got: null, text: null };
  c.cb = {
    write: async items => { if (!c.live) throw new DOMException("not allowed", "NotAllowedError"); const it = items[0], o = {}; for (const t of it.types) o[t] = await (await it.getType(t)).text(); c.got = o; },
    writeText: async t => { if (!c.live) throw new DOMException("not allowed", "NotAllowedError"); c.text = t; },
  };
  class ClipboardItem { constructor(o) { this.o = o; this.types = Object.keys(o); } getType(t) { return Promise.resolve(this.o[t]); } }
  return { c, g: { navigator: { clipboard: c.cb }, ClipboardItem, Blob, DOMException } };
}
await check("copy: handed over inside the click while the text is still being made", async () => {
  const { c, g } = strictClip();
  let ok; const later = new Promise(r => { ok = r; });
  const done = copyNow(g, { "text/plain": later, "text/html": "<b>x</b>" });
  c.live = false; ok("# mzLab 1\nm/z\t301.1");                       // the click is over before the text is ready
  await done;
  return c.got && c.got["text/plain"] === "# mzLab 1\nm/z\t301.1" && c.got["text/html"] === "<b>x</b>";
});
await check("copy: without ClipboardItem or refused, the plain text goes through writeText; an image alone rejects", async () => {
  const a = strictClip(); delete a.g.ClipboardItem; await copyNow(a.g, { "text/plain": Promise.resolve("t1") });
  const b = strictClip(); b.c.cb.write = async () => { throw new DOMException("no", "NotAllowedError"); }; await copyNow(b.g, { "text/plain": "t2" });
  const c = strictClip(); delete c.g.ClipboardItem; let rej = false; await copyNow(c.g, { "image/png": new Blob(["png"]) }).catch(() => { rej = true; });
  return a.c.text === "t1" && b.c.text === "t2" && rej && c.c.text === null;
});

// a fake page for the pinch: a surface (touch-action:none) with a canvas, a plain paragraph; document listeners fired in order
function pinchPage() {
  const node = (ta, parent) => ({ nodeType: 1, parentElement: parent, ta, wheels: [], dispatchEvent(w) { this.wheels.push(w); return true; } });
  const surf = node("none", null), canvas = node("auto", surf), para = node("auto", null), L = [];
  const d = { addEventListener: (t, f, c) => L.push([t, f, c]) };
  const fire = (t, o) => { const e = { defaultPrevented: false, preventDefault() { this.defaultPrevented = true; }, ...o }; for (const [k, f] of L) if (k === t) f(e); return e; };
  class WheelEvent { constructor(type, o) { Object.assign(this, o, { type }); } }
  const g = { document: d, WheelEvent, GestureEvent: function () {}, getComputedStyle: n => ({ touchAction: n.ta }), matchMedia: media(false) };
  return { g, surf, canvas, para, fire };
}
await check("pinch: installed with gesture events and a fine pointer only", () => {
  const a = pinchPage(), b = pinchPage(), c = pinchPage();
  delete b.g.GestureEvent; c.g.matchMedia = media(true);
  return installa(a.g).includes("pinch") && !installa(b.g).includes("pinch") && !installa(c.g).includes("pinch");
});
await check("pinch: gesturechange over a zoomable surface is a Ctrl+wheel there, the page does not zoom", () => {
  const P = pinchPage(); pinchWheel(P.g);
  const s = P.fire("gesturestart", { target: P.canvas, scale: 1 }), c1 = P.fire("gesturechange", { target: P.canvas, scale: 1.5, clientX: 10, clientY: 20 }), c2 = P.fire("gesturechange", { target: P.canvas, scale: 0.75, clientX: 10, clientY: 20 });
  const e = P.fire("gestureend", { target: P.canvas, scale: 0.75 }), [w1, w2] = P.canvas.wheels;
  return s.defaultPrevented && c1.defaultPrevented && e.defaultPrevented && P.canvas.wheels.length === 2 && w1.type === "wheel" && w1.ctrlKey && w1.cancelable && w1.bubbles
    && w1.clientX === 10 && w1.clientY === 20 && Math.abs(w1.deltaY + 100 * Math.log(1.5)) < 1e-9 && Math.abs(w2.deltaY - 100 * Math.log(2)) < 1e-9 && w1.deltaY < 0 && w2.deltaY > 0;
});
await check("pinch: left alone outside a zoomable surface, when a view handles it, and with fingers on a screen", () => {
  const P = pinchPage(); pinchWheel(P.g);
  const s = P.fire("gesturestart", { target: P.para, scale: 1 }), c = P.fire("gesturechange", { target: P.para, scale: 2 });
  P.fire("gesturestart", { target: P.canvas, scale: 1, defaultPrevented: true }); P.fire("gesturechange", { target: P.canvas, scale: 2, defaultPrevented: true });
  P.fire("pointerdown", { pointerType: "touch" }); P.fire("gesturestart", { target: P.canvas, scale: 1 }); P.fire("gesturechange", { target: P.canvas, scale: 2 });
  P.fire("pointerup", { pointerType: "touch" }); const s2 = P.fire("gesturestart", { target: P.canvas, scale: 1 }); P.fire("gesturechange", { target: P.canvas, scale: 2 });
  return !s.defaultPrevented && !c.defaultPrevented && P.para.wheels.length === 0 && s2.defaultPrevented && P.canvas.wheels.length === 1;
});

await check("canvas: a scale that would pass Safari's 16.7 Mpx cap is lowered, a small one kept", () => {
  const big = canvasScale(1600, 1400, 3), w = Math.round(1600 * big), h = Math.round(1400 * big);
  return MAX_CANVAS_PX === 16777216 && big < 3 && w * h <= MAX_CANVAS_PX && w * h > 0.99 * MAX_CANVAS_PX && canvasScale(400, 300, 8) === 8 && canvasScale(0, 10, 2) === 2;
});

process.stdout.write(JSON.stringify(out));
