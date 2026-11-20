// @ts-check
// Structure files on their way into Ketcher's own reader (Indigo), by the three routes: a drop on the canvas, Ketcher's Open dialog and pasted text.
// No format is parsed here. Two changes to the text are made first, each named in a notice, never silently: UTF-16 and 8-bit text is decoded as its
// byte-order mark or XML declaration says, and the empty attributes some writers leave on CDXML tags (RDKit writes BondLength="" on the root) are
// dropped, because Indigo draws nothing from a file that has one. Binary CDX (starts with "VjCD") changes nothing: it goes in as base64, the form the Open dialog gives it.
const CDX_MAGIC = [0x56, 0x6a, 0x43, 0x44];

/** Drops the attributes with an empty value from every start tag of a CDXML text (a quoted value may hold `>`, `<` or the other quote). Other text comes back unchanged.
 * @param {string} text @returns {{ text: string, removed: string[] }} `removed`: the attribute names, each once */
export function tidyCdxml(text) {
  if (!/<CDXML[\s>]/.test(text)) return { text, removed: [] };
  const removed = new Set();
  const attr = /(\s+)([A-Za-z_][\w.:-]*)(\s*=\s*)("[^"]*"|'[^']*')/g;
  const out = text.replace(/<[A-Za-z](?:"[^"]*"|'[^']*'|[^<>"'])*>/g, tag => tag.replace(attr, (all, _ws, name, _eq, value) => {
    if (value.length > 2) return all;
    removed.add(name); return "";
  }));
  return { text: out, removed: [...removed] };
}

/** @param {Uint8Array} bytes @returns {{ text: string, charset: string }} `charset`: "" for plain UTF-8 */
function decode(bytes) {
  if (bytes[0] === 0xff && bytes[1] === 0xfe) return { text: new TextDecoder("utf-16le").decode(bytes), charset: "UTF-16" };
  if (bytes[0] === 0xfe && bytes[1] === 0xff) return { text: new TextDecoder("utf-16be").decode(bytes), charset: "UTF-16" };
  try { return { text: new TextDecoder("utf-8", { fatal: true }).decode(bytes), charset: "" }; } catch (_) { /* 8-bit text: the XML declaration names it */ }
  const declared = /^\s*<\?xml[^>]*encoding=["']([\w.:-]+)["']/.exec(new TextDecoder("latin1").decode(bytes.subarray(0, 200)));
  const charset = declared ? declared[1] : "windows-1252";
  try { return { text: new TextDecoder(charset).decode(bytes), charset }; } catch (_) { return { text: new TextDecoder("windows-1252").decode(bytes), charset: "windows-1252" }; }
}

/** The text Ketcher reads for a file, and what was changed on the way: `{ key, params }` of a `draw.import.fix.*` notice part.
 * @param {File} file @returns {Promise<{ text: string, changes: { key: string, params?: Record<string, string> }[] }>} */
export async function readStructure(file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (CDX_MAGIC.every((b, i) => bytes[i] === b)) {
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    return { text: btoa(bin), changes: [] };
  }
  const { text: raw, charset } = decode(bytes), { text, removed } = tidyCdxml(raw.replace(/^\uFEFF/, ""));
  /** @type {{ key: string, params?: Record<string, string> }[]} */
  const changes = [];
  if (charset) changes.push({ key: "draw.import.fix.charset", params: { charset } });
  if (removed.length) changes.push({ key: "draw.import.fix.empty", params: { names: removed.join(", ") } });
  return { text: charset.toUpperCase().startsWith("UTF-16") ? text.replace(/(<\?xml[^>]*encoding=["'])[^"']*/, "$1UTF-8") : text, changes };
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
  doc.body.addEventListener("dragover", e => { e.preventDefault(); e.stopPropagation(); });
  doc.body.addEventListener("drop", async e => {
    e.preventDefault(); e.stopPropagation();
    const dt = /** @type {DataTransfer} */ (e.dataTransfer), file = dt.files[0];
    if (file) {
      let r;
      try { r = await readStructure(file); } catch (err_) { note(t("draw.import.failed", { message: /** @type {Error} */ (err_).message })); return; }   // an unreadable or too large file is told, not dropped
      if (r.text.trim()) await add(r.text, file.name, changesNote(file.name, r.changes, t)); else note(t("draw.import.unreadable", { name: file.name }));
      return;
    }
    const text = dt.getData("text/plain"), r = tidyCdxml(text);
    if (text) await add(r.text, "", r.removed.length ? changesNote(t("draw.import.pasted"), [{ key: "draw.import.fix.empty", params: { names: r.removed.join(", ") } }], t) : "");
  });
  // pasted CDXML with an empty attribute: Ketcher's own paste would draw nothing, so the cleaned text is added instead; any other paste is Ketcher's
  doc.addEventListener("paste", e => {
    const text = (e.clipboardData && e.clipboardData.getData("text/plain")) || "", r = tidyCdxml(text);
    if (!r.removed.length) return;
    e.preventDefault(); e.stopImmediatePropagation();
    add(r.text, "", changesNote(t("draw.import.pasted"), [{ key: "draw.import.fix.empty", params: { names: r.removed.join(", ") } }], t));
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
    } catch (_) { /* Ketcher reads the file as it is */ }
    seen.add(input); input.dispatchEvent(new w.Event("change", { bubbles: true })); seen.delete(input);
  }, true);
}
