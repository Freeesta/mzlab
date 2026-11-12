// @ts-check
// «Expected ions» card of Draw: the ESI+ and ESI− ions of the drawn structure (or of the selection) from the Rust engine (`ions` of the grafo
// .wasm, `ions/1`; the app asks `feat/grafo`), and per row «Show XIC» on the loaded high-resolution file with the user's ppm (the Data XIC route,
// `xicDirect` + `HR.ionTrace`). Without a high-resolution file the table is titled as theory and the XIC buttons say why they are off.
import { carica as grafo } from "./disegno-frammenti.js";

/** @typedef {{ name: string, pol: number, z: number, common: boolean, formula: string, text: string, mz: number, mzText: string, lo: number, hi: number, odd: boolean }} Riga */

const W = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {any} */ p = undefined) => W.I18N.t(k, p);
const fmtF = (/** @type {string} */ f) => (W.fmtFormula ? W.fmtFormula(f) : h`${f}`).replace(/\[(\d+)([A-Z][a-z]?)\]/g, "<sup>$1</sup>$2");

/** the first high-resolution file of the current Data tab, with its decimals and ppm (the user's settings of the profile), or null */
function hrFile() {
  try {
    const f = W.HR && W.HR.xicFiles().find((/** @type {any} */ x) => W.HR.isHr(x, x.lv));
    if (!f) return null;
    const q = W.HR.prof(f, f.lv);
    return { f, dec: q.dec, ppm: q.tol, name: f.label || f.file || "" };
  } catch (_) { return null; }
}

/** opens Data with the XIC of one ion of the high-resolution file (one more trace in a new panel) */
function showXic(/** @type {number} */ mz, /** @type {string} */ label) {
  W.setView("data");
  const tr = W.HR.ionTrace(mz, { prefix: label });
  if (tr) W.xicDirect(mz, null, null, tr);
}

/** the charge of an ion as superscript: +, 2+, −, 2− */
const sup = (/** @type {number} */ z) => h`<sup>${Math.abs(z) > 1 ? Math.abs(z) : ""}${z > 0 ? "+" : "\u2212"}</sup>`;

/** one row: ion (formula below), m/z (XIC window below with a high-resolution file), XIC button */
function row(/** @type {Riga} */ r, /** @type {number} */ n, /** @type {number} */ i, /** @type {ReturnType<typeof hrFile>} */ hf) {
  const odd = r.odd ? ` (${t("draw.ions.odd")})` : "", win = hf ? h`<div class="muted">${r.lo.toFixed(hf.dec)}\u2013${r.hi.toFixed(hf.dec)}</div>` : "";
  const btn = hf ? h`<button type="button" class="sm ions-xic" data-s="${n}" data-i="${i}" aria-label="${t("draw.ions.xic.aria", { ion: r.name, mz: r.mzText })}">${t("draw.ions.xic")}</button>`
    : h`<button type="button" class="sm ions-xic" data-s="${n}" data-i="${i}" disabled title="${t("draw.ions.xic.noFile")}" aria-label="${t("draw.ions.xic.aria", { ion: r.name, mz: r.mzText })}">${t("draw.ions.xic")}</button>`;
  return h`<tr><th scope="row">${r.name}<div class="muted">${raw(fmtF(r.formula))}${raw(sup(r.z))}${odd}</div></th><td class="num">${r.mzText}${raw(win)}</td><td>${raw(btn)}</td></tr>`;
}

/** three columns, to fit the side column of Draw */
function table(/** @type {any} */ s, /** @type {number} */ n, /** @type {ReturnType<typeof hrFile>} */ hf) {
  const rows = /** @type {Riga[]} */ (s.rows).map((r, i) => raw(row(r, n, i, hf)));
  const mz = t("draw.ions.col.mz") + (hf ? " (" + t("draw.ions.col.window") + ")" : "");
  return h`<table class="ions-t" data-testid="ions-table"><caption>${t("draw.ions.caption", { n: n + 1, formula: s.text, mass: s.mass.toFixed(hf ? hf.dec : 4) })}</caption>`
    + h`<thead><tr><th scope="col">${t("draw.ions.col.ion")}</th><th scope="col">${mz}</th><th scope="col"><span class="sr-only">${t("draw.ions.col.xic")}</span></th></tr></thead><tbody>${rows}</tbody></table>`
    + (s.charge ? h`<div class="muted">${t("draw.ions.charged")}</div>` : "");
}

/** the answer of the last run, for the XIC buttons */
let last = /** @type {any} */ (null);

/** computes and shows the table; called on the click of «Show expected ions» (draw.js loads the module then) */
export async function carica() {
  const body = /** @type {HTMLElement} */ (document.getElementById("ions-body")), head = document.getElementById("ions-h");
  if (!body) return;
  const hf = hrFile();
  body.innerHTML = "";                                                     // never a table of the previous drawing while this one is computed
  if (head) head.textContent = t(hf ? "draw.ions.heading" : "draw.ions.theory");
  // a lone charged atom (Cl−, Na+) is a counter-ion, not a molecule of its own
  const all = /** @type {any[][]} */ (await W.TPDraw.structures()).filter(a => !(a.length === 1 && a[0].q));
  if (!all.length) { body.innerHTML = h`<div class="muted">${t("draw.ions.empty")}</div>`; return; }
  const req = { structures: all, decimals: hf ? hf.dec : 4, ppm: hf ? hf.ppm : 5 };
  try { last = JSON.parse(await (await grafo()).ions(JSON.stringify(req))); }
  catch (_) { body.innerHTML = h`<div class="muted">${t("draw.ions.unavailable")}</div>`; return; }
  const note = hf ? t("draw.ions.hr", { file: hf.name, ppm: +hf.ppm.toFixed(1) }) : t("draw.ions.lowRes");
  const tables = last.structures.map((/** @type {any} */ s, /** @type {number} */ n) => raw(s.error ? h`<div class="muted">${t(s.error, { label: s.label })}</div>` : table(s, n, hf)));
  body.innerHTML = h`<div class="muted" style="margin-bottom:4px">${note}</div>${tables}`;
  if (!body.dataset.wired) {
    body.dataset.wired = "1";
    body.addEventListener("click", ev => {
      const b = /** @type {HTMLElement} */ (ev.target).closest(".ions-xic");
      if (!(b instanceof HTMLButtonElement) || b.disabled || !last) return;
      const s = last.structures[+(b.dataset.s || 0)], r = s && s.rows[+(b.dataset.i || 0)];
      if (r && hrFile()) showXic(r.mz, `${r.name} ${s.formula}`);
    });
  }
}
