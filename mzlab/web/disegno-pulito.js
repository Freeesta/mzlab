// @ts-strict
// Draw, «unsaved changes»: the canvas is dirty when it is not empty and its canonical .ket differs from the last one saved, exported or loaded (undo back to it = clean).
// One question, `confirmDiscard()`, guards every way the canvas is lost on the page («New», a load over it, the desktop window close); the browser's own
// `beforeunload` prompt is armed only while dirty (a page cannot word that prompt nor start a download from it). Page only: no CLI route, nothing in the engine.

/** @param {unknown} v @returns {unknown} the same value with every object's keys in sorted order and numbers rounded to 1e-4 */
function sorted(v) {
  if (Array.isArray(v)) return v.map(sorted);
  if (typeof v === "number") return Math.round(v * 1e4) / 1e4;        // Ketcher moves coordinates by ~1e-7 after a load or a zoom: not an edit
  if (v && typeof v === "object") { const o = /** @type {Record<string, unknown>} */ (v), r = /** @type {Record<string, unknown>} */ ({}); for (const k of Object.keys(o).sort()) r[k] = sorted(o[k]); return r; }
  return v;
}

/** @param {string} ket @returns {string} a stable text of the canvas: key order and white space do not matter */
export function canon(ket) { try { return JSON.stringify(sorted(JSON.parse(ket))); } catch (_) { return ket; } }

/** @param {string} ket the canvas now @param {string | null} saved `canon` of the last saved state, null = nothing saved yet @param {boolean} blank nothing drawn @returns {boolean} true = there is something to lose */
export function isDirty(ket, saved, blank) { return !blank && (saved === null || canon(ket) !== saved); }

/** @typedef {{ K: () => any, save: (kind: string) => Promise<boolean> }} Hooks */
/** the texts by reason and format, written out so that the catalog check sees every key */
const T = () => ({
  text: { close: t("draw.unsaved.text.close"), new: t("draw.unsaved.text.new"), load: t("draw.unsaved.text.load") },
  save: { close: t("draw.unsaved.save.close"), new: t("draw.unsaved.save.new"), load: t("draw.unsaved.save.load") },
  drop: { close: t("draw.unsaved.drop.close"), new: t("draw.unsaved.drop.new"), load: t("draw.unsaved.drop.load") },
  fmt: { ket: t("draw.unsaved.fmt.ket"), mol: t("draw.unsaved.fmt.mol"), sdf: t("draw.unsaved.fmt.sdf"), png: t("draw.unsaved.fmt.png"), svg: t("draw.unsaved.fmt.svg") },
});
const t = (/** @type {string} */ k) => /** @type {any} */ (window).I18N.t(k);
/** @type {Hooks | null} */ let hooks = null;
/** @type {string | null} */ let saved = null;
let dirty = false, gen = 0;       // `gen` changes when the saved state does: a check that started before it is stale and dropped
/** @type {Promise<boolean> | null} */ let pending = null;

const warn = (/** @type {Event} */ e) => { e.preventDefault(); /** @type {any} */ (e).returnValue = ""; };       // the browser shows its own generic prompt

/** @param {boolean} on arms the browser prompt only while there is something to lose (never on a clean state) */
function arm(on) {
  if (on === dirty) return;
  dirty = on;
  if (on) window.addEventListener("beforeunload", warn); else window.removeEventListener("beforeunload", warn);
}

/** @returns {Promise<boolean>} recomputes the state from the canvas and arms or disarms the prompt */
export async function check() {
  const K = hooks && hooks.K(), g = gen;
  if (!K) return false;
  let blank = true, ket = "";
  try { blank = K.editor.struct().isBlank(); if (!blank) ket = await K.getKet(); } catch (_) { return dirty; }
  if (g === gen) arm(isDirty(ket, saved, blank)); else return check();
  return dirty;
}

/** the canvas as it is now is the saved state (after an export, a load, or the notebook put it back) */
export async function markSaved() {
  const K = hooks && hooks.K();
  if (!K) return;
  gen++;
  try { saved = K.editor.struct().isBlank() ? null : canon(await K.getKet()); } catch (_) { return; }
  arm(false); gen++;
}

/** @param {Hooks} h `K` gives Ketcher (or null), `save(kind)` is the panel's own export (true = the download was triggered) */
export function mount(h) {
  const again = !!hooks; hooks = h;
  if (again) return;                                  // a start tried again: the close question is registered once
  const tau = /** @type {any} */ (window).__TAURI__, w = tau && tau.window && typeof tau.window.getCurrentWindow === "function" ? tau.window.getCurrentWindow() : null;
  if (w && typeof w.onCloseRequested === "function") {          // desktop app: the same question on the window's close button (the permission to destroy the window is in capabilities/default.json)
    w.onCloseRequested(async (/** @type {{ preventDefault: () => void }} */ ev) => { if (await check()) { ev.preventDefault(); if (await confirmDiscard("close")) await w.destroy(); } });
  }
}

/**
 * The one question before the canvas is lost. Clean or empty canvas: true at once, no dialog. Otherwise «Your drawing is not saved» with
 * Save (the chosen format, through the panel's export), Don't save and Cancel (default; Esc too). A non-empty canvas that was saved asks the plain «Clear the drawing?» for «new».
 * @param {"close" | "new" | "load"} why @returns {Promise<boolean>} true = go on (saved first, or the user chose to lose it)
 */
export async function confirmDiscard(why) {
  const K = hooks && hooks.K();
  if (!K || !hooks) return true;
  if (pending) return pending.then(() => false);
  const d = await check();
  let blank = true; try { blank = K.editor.struct().isBlank(); } catch (_) { /* no isBlank: treat as empty */ }
  if (blank || (!d && why !== "new")) return true;
  const fr = /** @type {HTMLIFrameElement | null} */ (document.getElementById("kframe")), doc = fr && fr.contentDocument;
  if (doc && doc.fullscreenElement) doc.exitFullscreen().catch(() => {});            // the question lives on the page, outside Ketcher's full screen
  const { choiceBox, confirmBox } = await import("./memoria-avviso.js");
  const h = hooks, x = T();
  if (!d) pending = confirmBox({ id: "draw-new", title: t("draw.new.ask"), yes: t("draw.new.yes"), no: t("draw.new.no") });      // saved already: the plain question of «New»
  else {
    const wrap = document.createElement("label"), sel = document.createElement("select");
    wrap.className = "mm-fmt"; wrap.title = t("draw.unsaved.format.tip"); sel.id = "mm-fmt";
    for (const [f, label] of Object.entries(x.fmt)) { const o = document.createElement("option"); o.value = f; o.textContent = label; sel.appendChild(o); }
    wrap.append(t("draw.unsaved.format") + " ", sel);
    const save = async () => (await h.save(sel.value)) || (/** @type {HTMLElement | null} */ (document.getElementById("ex-warn"))?.textContent || t("draw.unsaved.failed"));
    pending = choiceBox({
      id: "draw-unsaved", title: t("draw.unsaved.title"), text: x.text[why], extra: wrap, cancel: "cancel",
      buttons: [{ key: "save", id: "mm-save", label: x.save[why], tip: t("draw.unsaved.save.tip"), run: save },
        { key: "drop", id: "mm-drop", label: x.drop[why], tip: t("draw.unsaved.drop.tip") },
        { key: "cancel", id: "mm-no", label: t("draw.unsaved.cancel"), tip: t("draw.unsaved.cancel.tip") }],
    }).then(k => k !== "cancel");
  }
  try { return await pending; } finally { pending = null; }
}
