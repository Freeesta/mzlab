// @ts-check
// Size of the atom symbols in the editor (Disegno): three sizes, kept in this browser. Ketcher takes it as its "fontsz" option;
// only editor.setOptions applies it at once (ketcher.setSettings does not), and Ketcher keeps it in its own localStorage as well.
const KEY = "qqq.disegno.font";
export const SIZES = /** @type {const} */ ([13, 16, 20]);        // small, medium (default), large
export const DEFAULT = 16;

/** @returns {number} the saved size, or the default */
export function fontSize() {
  try { const n = Number(localStorage.getItem(KEY)); if (SIZES.includes(/** @type {any} */ (n))) return n; } catch (_) { /* default */ }
  return DEFAULT;
}

/** Applies the saved size to a running Ketcher.
 * @param {any} k the ketcher object
 * @param {number} [size] */
export function applyFont(k, size = fontSize()) {
  try { k.editor.setOptions(JSON.stringify({ fontsz: size })); } catch (_) { /* editor not ready: applied at its start */ }
}

/** Draws the size choice (three radio buttons) in `box`.
 * @param {HTMLElement} box
 * @param {() => any} getK the running ketcher, or null
 * @param {(key: string) => string} t translator */
export function mountFont(box, getK, t) {
  const cur = fontSize(), names = [t("draw.font.small"), t("draw.font.medium"), t("draw.font.large")];
  box.innerHTML = `<div role="radiogroup" aria-label="${t("draw.font.label")}" title="${t("draw.font.title")}"><span>${t("draw.font.label")}</span> ` +
    SIZES.map((n, i) => `<label style="margin-left:8px;white-space:nowrap"><input type="radio" name="dfont" value="${n}" ${n === cur ? "checked" : ""}> ${names[i]}</label>`).join("") + "</div>";
  box.querySelectorAll("input[name=dfont]").forEach(i => i.addEventListener("change", () => {
    const n = Number(/** @type {HTMLInputElement} */ (i).value);
    try { localStorage.setItem(KEY, String(n)); } catch (_) { /* this tab only */ }
    const k = getK(); if (k) applyFont(k, n);
  }));
}
