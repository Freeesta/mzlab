// @ts-strict
// @ts-noany
// «Is this in the catalogue?» of the Draw page: every drawn structure (or the selected one) is looked up in the molecule catalogue by the Rust engine
// (`identity` of the molid .wasm, `molid/1`; the app asks `feat/grafo` kind `identity`, the CLI is `mzlab-cli mol-id`). Exact = the same structure; near =
// the same skeleton with other stereo or isotopes, said as such. It names the drawing, never a signal; offline, nothing leaves the computer. Loaded on the first click.
import { featManifest } from "./feat-manifesto.js";

/** @typedef {{ id: string, nome: string, formula: string, carica: number, massa: number, smiles: string, differs?: string[] }} Voce */
/** @typedef {{ format: string, key: string, skeleton: string, compared: number, exact: Voce[], near: Voce[] }} Risposta */

/** @type {Promise<string> | null} */
let testo = null;
/** the catalogue the molecules window loads too (the browser keeps it cached) */
const catalogo = () => testo || (testo = fetch("static/molecole/catalogo.csv").then(r => { if (!r.ok) throw new Error("catalogo.csv: " + r.status); return r.text(); }).catch(e => { testo = null; throw e; }));

/** @type {Promise<{ identity: (request: string) => string | Promise<string> }> | null} */
let modulo = null;
/** the molid module, checked against the SHA-256 of wasm-feat/manifest.json; in the app the native engine answers (`feat/grafo`), its error text as the .wasm throws it */
function carica() {
  if (modulo) return modulo;
  if (/** @type {any} */ (window).MZLAB_DESKTOP) {
    const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
    return (modulo = Promise.resolve({ identity: async (/** @type {string} */ request) => {
      const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", kind: "identity", request }) }), t = await r.text();
      if (r.status !== 200) throw t;
      return t;
    } }));
  }
  modulo = (async () => {
    const base = new URL("./wasm-feat/", import.meta.url), e = (await featManifest(base)).molid;
    if (!e) throw new Error("molid");
    const bytes = await (await fetch(new URL(e.wasm, base))).arrayBuffer();
    const hex = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(x => x.toString(16).padStart(2, "0")).join("");
    if (hex !== e.sha256) throw new Error("sha");
    const mod = await import(new URL(e.js, base).href);
    await mod.default({ module_or_path: bytes });
    return mod;
  })();
  modulo.catch(() => { modulo = null; });
  return modulo;
}

/** The answer for one molfile; an engine error comes back as its `{error_key}` text. @returns {Promise<Risposta>} */
export async function identify(/** @type {string} */ molfile) {
  const [m, cat] = await Promise.all([carica(), catalogo()]);
  return JSON.parse(await m.identity(JSON.stringify({ molfile, catalogo: cat })));
}

const esc = (/** @type {string} */ s) => s.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] || c);

/** One entry as a list item: name, formula, exact mass, and for a near one what differs. @param {Voce} v @param {(k: string, p?: Record<string, any>) => string} t */
function item(v, t) {
  const what = (/** @type {string} */ x) => x === "stereo" ? t("draw.id.differs.stereo") : x === "isotopes" ? t("draw.id.differs.isotopes") : x;
  const d = v.differs ? " — " + v.differs.map(what).join(", ") : "";
  return `<li><b>${esc(v.nome)}</b> · ${esc(v.formula)} · <span title="${esc(t("draw.id.mass"))}">${v.massa.toFixed(4)}</span>${esc(d)}</li>`;
}

/** The text of the answers, one block per structure (numbered when there are several); `null` = a piece with no formula (R group, abbreviation...).
 * @param {(Risposta | null | { error: string })[]} rs @param {(k: string, p?: Record<string, any>) => string} t */
export function html(rs, t) {
  return rs.map((r, n) => {
    const head = rs.length > 1 ? `<div style="font-weight:600">${esc(t("draw.id.piece", { n: n + 1 }))}</div>` : "";
    if (r === null) return head + `<div>${esc(t("draw.id.noFormula"))}</div>`;
    if ("error" in r) return head + `<div>${esc(t("draw.id.error", { message: r.error }))}</div>`;
    const ex = r.exact.length ? `<div>${esc(t("draw.id.exact"))}</div><ul style="margin:2px 0 4px 18px;padding:0">${r.exact.map(v => item(v, t)).join("")}</ul>` : "";
    const ne = r.near.length ? `<div>${esc(t("draw.id.near"))}</div><ul style="margin:2px 0 4px 18px;padding:0">${r.near.map(v => item(v, t)).join("")}</ul>` : "";
    return head + (ex + ne || `<div>${esc(t("draw.id.none", { n: r.compared }))}</div>`);
  }).join("") + `<div class="muted">${esc(t("draw.id.note"))}</div>`;
}

/** Looks up every molfile and writes the answers in `out`. @param {(string | null)[]} molfiles @param {HTMLElement} out @param {(k: string, p?: Record<string, any>) => string} t */
export async function show(molfiles, out, t) {
  if (!molfiles.length) { out.textContent = t("draw.id.empty"); return; }
  out.textContent = t("draw.id.busy");
  const rs = await Promise.all(molfiles.map(mf => mf === null ? null : identify(mf).catch(e => {
    let key = String(e && e.message || e);
    try { key = JSON.parse(String(e)).error_key || key; } catch (_) { /* not the engine's JSON */ }
    return { error: t(key) };
  })));
  out.innerHTML = html(rs, t);
}
