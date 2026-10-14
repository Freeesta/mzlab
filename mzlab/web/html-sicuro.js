// @ts-check
// Tagged template for HTML built from strings: every `${}` is escaped (quotes included) unless wrapped in raw(). Guard: tools/html_sicuro.py.
/** Trusted HTML marker for `h`: the only way to let markup through an interpolation. */
class RawHtml { /** @param {string} s */ constructor(s) { this.s = s; } }
/** @type {(s: any) => RawHtml} */
const raw = s => new RawHtml(String(s));
/** @type {(v: any) => string} */
const hv = v => v instanceof RawHtml ? v.s : Array.isArray(v) ? v.map(hv).join("") : v == null ? "" : String(v).replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c] || c));
/** Arrays are escaped item by item and joined.
 * @type {(parts: TemplateStringsArray, ...vals: any[]) => string} */
const h = (parts, ...vals) => parts.reduce((o, p, i) => o + p + (i < vals.length ? hv(vals[i]) : ""), "");
