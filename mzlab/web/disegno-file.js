// @ts-check
// Structure files on their way into Ketcher's own reader (Indigo), by the three routes: a drop on the canvas, Ketcher's Open dialog and pasted text.
// No format is parsed here and nothing is tidied here: the engine (`open/1`, the molkit `.wasm` on the site, `feat/grafo` in the app, `mzlab-cli mol open`)
// gives the text Ketcher reads and the list of what it changed, and each change is named in a notice, never silently: UTF-16 and 8-bit text decoded as its
// byte-order mark or XML declaration says, and the empty attributes some writers leave on CDXML tags (RDKit writes BondLength="" on the root) dropped,
// because Indigo draws nothing from a file that has one. Binary CDX (starts with "VjCD") changes nothing: it goes in as base64, the form the Open dialog gives it.

/** The notice part of one change of `open/1`: `{ key, params }` of a `draw.import.fix.*` text.
 * @param {{ fix: string, charset?: string, count?: number, names?: string[] }} c */
function changeText(c) {
  if (c.fix === "charset") return { key: "draw.import.fix.charset", params: { charset: String(c.charset) } };
  if (c.fix === "replaced") return { key: "draw.import.fix.replaced", params: { count: String(c.count) } };
  return { key: "draw.import.fix.empty", params: { names: (c.names || []).join(", ") } };
}

/** An engine that could not be reached (the `.wasm` did not load, the app did not answer), as against one that answered with a refusal. */
const unavailable = (/** @type {any} */ e) => Object.assign(new Error(e && e.message ? e.message : String(e)), { unavailable: true });

/** The text Ketcher reads for the bytes of a file, and what was changed on the way: `{ key, params }` of a `draw.import.fix.*` notice part.
 * An engine refusal (a file over the size limit) is thrown as the error with its text; an engine that is not there is thrown with `unavailable` set.
 * @param {Uint8Array} bytes @returns {Promise<{ text: string, changes: { key: string, params?: Record<string, string> }[] }>} */
async function engineText(bytes) {
  let v;
  try { v = await import("./disegno-verifica.js"); } catch (e) { throw unavailable(e); }
  let r;
  try { r = await v.openFile(bytes); } catch (e) {
    let refused = false;
    try { refused = !!JSON.parse(String(e)).error_key; } catch (_) { /* not an engine error */ }
    if (refused) throw new Error(v.message(e));
    throw unavailable(e);
  }
  return { text: r.text, changes: r.changes.map(changeText) };
}

/** The same, or, when the engine is not there, the bytes as UTF-8 text with no change: Ketcher then reads the file as it did before the engine route, a drop is never lost to it.
 * @param {Uint8Array} bytes */
async function engineOrRaw(bytes) {
  try { return await engineText(bytes); } catch (e) {
    if (!(/** @type {any} */ (e)).unavailable) throw e;
    console.warn("open/1 engine unavailable, the file goes to Ketcher as it is:", e);
    return { text: new TextDecoder().decode(bytes), changes: [] };
  }
}

/** True for a text that is CDXML: an optional XML declaration and DOCTYPE, then the root. */
export const isCdxml = (/** @type {string} */ t) => /^\s*\uFEFF?\s*(?:<\?xml[^>]*\?>\s*)?(?:<!DOCTYPE[^>]*>\s*)?<CDXML[\s>]/.test(t);

/** @param {File} file */
export async function readStructure(file) {
  return engineText(new Uint8Array(await file.arrayBuffer()));
}

/** The notice for what was changed, or "" when nothing was.
 * @param {string} name @param {{ key: string, params?: Record<string, string> }[]} changes @param {(key: string, p?: Record<string, any>) => string} t translator */
export function changesNote(name, changes, t) {
  return changes.length ? t("draw.import.changed", { name, what: changes.map(c => t(c.key, c.params)).join("; ") }) : "";
}

/** Wires the three routes on the editor document.
 * @param {Document} doc @param {any} k Ketcher
 * @param {{ t: (key: string, p?: Record<string, any>) => string, note: (text: string) => void, done: () => void }} ui `done` runs after a structure was drawn */
export function mountImport(doc, k, ui) {
  const w = /** @type {any} */ (doc.defaultView), { t, note } = ui;
  /** Adds the text; a canvas that did not change means Indigo could not read it. */
  const add = async (/** @type {string} */ text, /** @type {string} */ name, /** @type {string} */ extra) => {
    try {
      const before = await k.getKet();
      await k.addFragment(text);
      if (await k.getKet() === before) note(name ? t("draw.import.unreadable", { name }) : t("draw.import.unsupported"));
      else { ui.done(); note(extra); }
    } catch (err_) { note(t("draw.import.failed", { message: /** @type {Error} */ (err_).message })); }
  };
  /** Pasted or dropped text: the engine reads it as the bytes of a UTF-8 file. */
  const addText = async (/** @type {string} */ text) => {
    let r;
    try { r = await engineOrRaw(new TextEncoder().encode(text)); } catch (err_) { note(t("draw.import.failed", { message: /** @type {Error} */ (err_).message })); return; }
    await add(r.text, "", changesNote(t("draw.import.pasted"), r.changes, t));
  };
  doc.body.addEventListener("dragover", e => { e.preventDefault(); e.stopPropagation(); });
  doc.body.addEventListener("drop", async e => {
    e.preventDefault(); e.stopPropagation();
    const dt = /** @type {DataTransfer} */ (e.dataTransfer), file = dt.files[0];
    if (file) {
      let r;
      try { r = await engineOrRaw(new Uint8Array(await file.arrayBuffer())); } catch (err_) { note(t("draw.import.failed", { message: /** @type {Error} */ (err_).message })); return; }   // a file over the size limit is told, not dropped; an engine that is not there is not in the way
      if (r.text.trim()) await add(r.text, file.name, changesNote(file.name, r.changes, t)); else note(t("draw.import.unreadable", { name: file.name }));
      return;
    }
    const text = dt.getData("text/plain");
    if (text) await addText(text);
  });
  // pasted CDXML (text that opens with the XML declaration, a DOCTYPE or the CDXML root): the engine's text is added, so an empty attribute is dropped and named, where Ketcher's own paste would draw nothing; any other paste is Ketcher's
  doc.addEventListener("paste", e => {
    // a paste into Ketcher's own text fields (the text tool, the Open dialog's box, a search field) is theirs, whatever it holds
    const tg = /** @type {HTMLElement | null} */ (e.target);
    if (tg && ((tg.closest && tg.closest("input, textarea, select")) || tg.isContentEditable)) return;
    const text = (e.clipboardData && e.clipboardData.getData("text/plain")) || "";
    if (!isCdxml(text)) return;
    e.preventDefault(); e.stopImmediatePropagation();
    addText(text);
  }, true);
  // Ketcher's Open dialog: a .cdxml that needs a change is swapped for the changed copy before Ketcher's own handler reads the input, which then gets the event again
  const seen = new WeakSet();
  doc.addEventListener("change", async e => {
    const input = /** @type {HTMLInputElement} */ (e.target), file = input && input.type === "file" && input.files && input.files[0];
    if (!file || seen.has(input) || !/\.cdxml$/i.test(file.name)) return;
    e.stopImmediatePropagation();
    try {
      const r = await readStructure(file);
      if (r.changes.length) { const dt = new w.DataTransfer(); dt.items.add(new w.File([r.text], file.name, { type: file.type })); input.files = dt.files; note(changesNote(file.name, r.changes, t)); }
    } catch (err_) { if (!(/** @type {any} */ (err_)).unavailable) note(t("draw.import.failed", { message: /** @type {Error} */ (err_).message })); }   // Ketcher then reads the file as it is
    seen.add(input); input.dispatchEvent(new w.Event("change", { bubbles: true })); seen.delete(input);
  }, true);
}
