// @ts-check
// Copy, cut, paste and "select molecule" of the Draw editor for a finger (Ketcher in the iframe `d`; page only: no Rust, no CLI route).
// Reuses Ketcher: its own copy / cut buttons fill the system clipboard AND an internal clipboard here (the same KET + molfile, so atoms, bonds, charges, stereo,
// isotopes, labels and annotations survive even when the browser refuses the system clipboard); paste feeds Ketcher's paste handler (a synthetic `paste`
// event, because its toolbar button refuses to read the clipboard) and its paste tool shows the structure on the cursor until the drop. On a touch screen a
// copy "arms" the paste: the next tap drops the structure centred on the finger, a drag adjusts it before the release.
const CANVAS = '[class*="StructEditor-module_canvas"]';
const wait = (/** @type {number} */ ms) => new Promise(r => setTimeout(r, ms));
const CHIP = "tp-paste-chip";

/** @param {Document} d  the Ketcher document
 * @param {any} K  the ketcher object
 * @param {HTMLIFrameElement} fr
 * @param {{ molfile?: (ids: number[]) => Promise<string> | string }} deps */
export function initAppunti(d, K, fr, deps = {}) {
  const w = /** @type {any} */ (d.defaultView);
  if (w.__tpAppunti) return w.__tpAppunti;
  /** @type {Record<string, string> | null} */ let clip = null; let stamp = 0, lastWrite = 0, feeding = false;
  const remember = (/** @type {Record<string, string>} */ t) => { if (t && Object.keys(t).length) { clip = t; stamp++; } };

  // Ketcher copies with the async clipboard API (or a copy event for Ctrl+C): both are seen here before the browser may refuse them
  const cb = w.navigator && w.navigator.clipboard;
  if (cb) {
    const ow = cb.write && cb.write.bind(cb), ot = cb.writeText && cb.writeText.bind(cb);
    if (ow) cb.write = async (/** @type {any[]} */ items) => {
      try { const t = /** @type {Record<string, string>} */ ({}); for (const it of items) for (const ty of it.types) t[ty] = await (await it.getType(ty)).text(); lastWrite = Date.now(); remember(t); } catch (_) { /* the system clipboard still gets it */ }
      return ow(items);
    };
    // Ketcher's paste reads the clipboard through the async API (a `paste` event alone is not enough): while a paste of ours runs, it is given the internal clipboard
    const or = cb.read && cb.read.bind(cb), ort = cb.readText && cb.readText.bind(cb);
    if (or) cb.read = (/** @type {any[]} */ ...a) => (feeding && clip ? Promise.resolve([new w.ClipboardItem(Object.fromEntries(Object.entries(clip).map(([k, v]) => [k, new Blob([v], { type: k.replace(/^web /, "") })])))]) : or(...a));
    if (ort) cb.readText = (/** @type {any[]} */ ...a) => (feeding && clip ? Promise.resolve(clip["text/plain"] || "") : ort(...a));
    if (ot) cb.writeText = async (/** @type {string} */ s) => { if (Date.now() - lastWrite > 500) remember({ "text/plain": s }); return ot(s); };   // a plain copy right after a typed one never downgrades it
  }
  for (const t of ["copy", "cut"]) w.addEventListener(t, (/** @type {ClipboardEvent} */ e) => {
    const cd = e.clipboardData; if (!cd || !cd.types || !cd.types.length) return;
    /** @type {Record<string, string>} */ const o = {}; for (const ty of cd.types) { try { o[ty] = cd.getData(ty); } catch (_) { /* not readable */ } } lastWrite = Date.now(); remember(o);
  });

  const selection = () => (K.editor.selection() || {});
  const hasSel = () => Object.values(selection()).some(v => Array.isArray(v) && v.length > 0);
  const mark = () => { const st = K.editor.struct(); return `${K.editor.historyPtr}:${st.atoms.size}:${st.bonds.size}`; };
  const fire = (/** @type {string} */ type, /** @type {number} */ x, /** @type {number} */ y) => {
    const el = d.elementFromPoint(x, y) || d.body;
    el.dispatchEvent(new w.MouseEvent(type, { bubbles: true, cancelable: true, view: w, clientX: x, clientY: y, button: 0, buttons: type === "mousedown" ? 1 : 0 }));
  };
  // a toolbar button of Ketcher is enabled a moment after the selection changes
  async function press(/** @type {string} */ id) {
    for (let i = 0; i < 24; i++) { const b = /** @type {HTMLButtonElement | null} */ (d.querySelector(`[data-testid=${id}]`)); if (b && !b.disabled) { b.click(); return true; } await wait(25); }
    return false;
  }
  async function viaButton(/** @type {string} */ id) {
    if (!hasSel()) return false;
    const s0 = stamp; if (!(await press(id))) return false;
    for (let i = 0; i < 20 && stamp === s0; i++) await wait(25);
    if (stamp === s0 && deps.molfile) {                     // Ketcher did not hand the data over: the molfile of the selection (no annotations) is the closest copy
      const ids = [...(selection().atoms || [])]; if (ids.length) { try { remember({ "text/plain": String(await deps.molfile(ids)) }); } catch (_) { /* nothing to keep */ } }
    }
    return stamp !== s0;
  }

  // ---- paste: Ketcher's own paste handler, fed from the internal clipboard
  async function startPaste() {
    if (!clip) return false;
    const before = mark(); feeding = true;
    try {
      (d.querySelector(".cliparea,[data-cliparea]") || d.body).dispatchEvent(new w.ClipboardEvent("paste", { clipboardData: new w.DataTransfer(), bubbles: true, cancelable: true }));
      for (let i = 0; i < 80; i++) { if (mark() !== before) return true; await wait(25); }
      return false;
    } finally { feeding = false; }
  }
  // the structure follows the pointer from the first move; the drop is a click
  async function place(/** @type {number} */ x, /** @type {number} */ y, /** @type {boolean} */ drop) {
    if (!(await startPaste())) return false;
    fire("mousemove", x, y); await wait(30); fire("mousemove", x, y);
    if (drop) { await wait(30); fire("mousedown", x, y); fire("mouseup", x, y); holdClick = Date.now() + 700; }
    return true;
  }

  // ---- touch: armed paste (a tap drops, a drag adjusts)
  let armed = false, holdClick = 0;
  /** @type {{ id: number, x: number, y: number, ready: Promise<boolean> } | null} */
  let g = null;
  let chip = /** @type {HTMLElement | null} */ (null);
  const W = /** @type {any} */ (window), tr = (/** @type {string} */ k) => (W.I18N ? W.I18N.t(k) : k);
  function showChip() {
    if (!chip) {
      chip = document.createElement("div"); chip.id = CHIP; chip.setAttribute("role", "status");
      const sp = document.createElement("span"), b = document.createElement("button"); b.type = "button";
      b.addEventListener("click", () => disarm()); chip.append(sp, b); document.body.appendChild(chip);
    }
    const sp = /** @type {HTMLElement} */ (chip.firstElementChild), b = /** @type {HTMLElement} */ (chip.lastElementChild);
    sp.textContent = tr("draw.paste.armed"); b.textContent = tr("draw.paste.cancel"); b.title = tr("draw.paste.cancel.title");
    const r = fr.getBoundingClientRect(); chip.style.top = Math.max(8, r.top + 12) + "px"; chip.style.left = r.left + r.width / 2 + "px"; chip.hidden = false;
  }
  function disarm() { armed = false; if (chip) chip.hidden = true; }
  function arm() { if (!clip) return false; armed = true; showChip(); return true; }

  w.addEventListener("pointerdown", (/** @type {PointerEvent} */ e) => {
    if (!armed || e.pointerType === "mouse" || !e.isTrusted || g) return;
    const el = /** @type {Element | null} */ (e.target); if (!el || !el.closest || !el.closest(CANVAS)) return;
    e.preventDefault(); e.stopImmediatePropagation(); disarm(); holdClick = Date.now() + 60000;
    g = { id: e.pointerId, x: e.clientX, y: e.clientY, ready: startPaste() };
    const mine = g; mine.ready.then(ok => { if (ok && g === mine) { fire("mousemove", mine.x, mine.y); fire("mousemove", mine.x, mine.y); } });
  }, true);
  w.addEventListener("pointermove", (/** @type {PointerEvent} */ e) => {
    if (!g || e.pointerId !== g.id) return;
    const mine = g; mine.x = e.clientX; mine.y = e.clientY; e.preventDefault(); mine.ready.then(ok => { if (ok && g === mine) fire("mousemove", mine.x, mine.y); });
  }, true);
  const end = async (/** @type {PointerEvent} */ e) => {
    if (!g || e.pointerId !== g.id) return;
    const mine = g; g = null; mine.x = e.clientX || mine.x; mine.y = e.clientY || mine.y;
    if (await mine.ready) { fire("mousemove", mine.x, mine.y); await wait(20); fire("mousedown", mine.x, mine.y); fire("mouseup", mine.x, mine.y); }
    else if (W.toast) W.toast(tr("draw.paste.fail"));
    holdClick = Date.now() + 700;
  };
  w.addEventListener("pointerup", end, true); w.addEventListener("pointercancel", end, true);
  // the browser's own click after the lift would select or clear what the drop has just placed
  w.addEventListener("click", (/** @type {Event} */ e) => { if (Date.now() < holdClick) { e.stopImmediatePropagation(); e.preventDefault(); } }, true);
  d.addEventListener("keydown", (/** @type {KeyboardEvent} */ e) => { if (e.key === "Escape") disarm(); }, true);
  document.addEventListener("keydown", e => { if (e.key === "Escape" && armed) disarm(); }, true);

  // ---- the whole molecule under a tap
  function moleculeAt(/** @type {any} */ ev) {
    const st = K.editor.struct(); /** @type {any} */ let hit = null;
    try { hit = K.editor.findItem(ev, ["atoms", "bonds"]); } catch (_) { /* no hit-test */ }
    if (!hit) return null;
    let aid = hit.id; if (hit.map === "bonds") { const b = st.bonds.get(hit.id); if (!b) return null; aid = b.begin; }
    const a = st.atoms.get(aid); if (!a) return null;
    /** @type {number[]} */ const atoms = [];
    /** @type {number[]} */ const bonds = [];
    st.atoms.forEach((/** @type {any} */ x, /** @type {number} */ id) => { if (x.fragment === a.fragment) atoms.push(id); });
    st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ id) => { const A = st.atoms.get(b.begin); if (A && A.fragment === a.fragment) bonds.push(id); });
    return { atoms, bonds };
  }
  const covers = (/** @type {{ atoms: number[] }} */ m) => { const s = new Set(selection().atoms || []); return m.atoms.every(a => s.has(a)); };

  const api = {
    has: () => !!clip,
    types: () => (clip ? Object.fromEntries(Object.entries(clip).map(([k, v]) => [k, v.length])) : {}),
    armed: () => armed,
    arm, disarm,
    copy: () => viaButton("copy-button"),
    cut: () => viaButton("cut-button"),
    // Ketcher erases the selection the moment the eraser is chosen (a keydown we dispatch is ignored), then the selection tool comes back
    del: () => { if (!hasSel()) return false; K.editor.tool("eraser"); const b = /** @type {HTMLElement | null} */ (d.querySelector("[data-testid=select-rectangle]")); if (b) b.click(); else K.editor.tool("select"); return true; },
    /** drop at the point (iframe coordinates) at once */
    pasteHere: (/** @type {number} */ x, /** @type {number} */ y) => place(x, y, true),
    /** a mouse: the structure follows the cursor and a click drops it */
    pasteAttached: (/** @type {number} */ x, /** @type {number} */ y) => place(x, y, false),
    moleculeAt,
    selectMolecule: (/** @type {{ atoms: number[], bonds: number[] }} */ m) => { K.editor.selection({ atoms: m.atoms, bonds: m.bonds }); },
    covers,
  };
  w.__tpAppunti = api;
  return api;
}
