// @ts-strict
// @ts-noany
// Export file names of Draw: disegno_AAAA-MM-GG_HHMM.<ext>, one base name for the PNG, JPEG, SVG and .ket of the same minute, "_trasparente"
// for a transparent PNG/SVG, "-2", "-3" for a repeated name; ASCII lower case only. The student can change the base name in #ex-name.
const Q = (/** @type {string} */ s) => /** @type {any} */ (document.querySelector(s));
const AUTO = /^(disegno|drawing)_\d{4}-\d\d-\d\d_\d{4}$/, usedNames = new Map();
export const stamp = () => { const d = new Date(), z = (/** @type {number} */ n) => String(n).padStart(2, "0"); return `${I18N.t("draw.export.filePrefix")}_${d.getFullYear()}-${z(d.getMonth() + 1)}-${z(d.getDate())}_${z(d.getHours())}${z(d.getMinutes())}`; };
const cleanName = (/** @type {unknown} */ t) => String(t || "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[^a-z0-9_-]+/g, "_").replace(/_{2,}/g, "_").replace(/^[_-]+|[_-]+$/g, "").slice(0, 60);
export function freshName() { const f = Q("#ex-name"); if (!f.value.trim() || AUTO.test(f.value.trim())) f.value = stamp(); }
/** @param {string} ext @param {boolean} [clear] */ export function exportName(ext, clear) {
  freshName();
  const base = (cleanName(Q("#ex-name").value) || stamp()) + (clear ? "_" + I18N.t("draw.export.transparentSuffix") : ""), key = base + "." + ext, n = (usedNames.get(key) || 0) + 1;
  usedNames.set(key, n);
  return base + (n > 1 ? "-" + n : "") + "." + ext;
}
Q("#ex-name").addEventListener("change", () => { const c = cleanName(Q("#ex-name").value); Q("#ex-name").value = c || stamp(); NB.drawName = AUTO.test(Q("#ex-name").value) ? "" : Q("#ex-name").value; nbSave(); });
