// @ts-check
// Action «Validate mzML» of the menu of a file in Dati: the report of `mzlab-cli validate --json` for an opened mzML, shown in a dialog with a table (severity, message, location).
// Browser: the `valida` module (wasm-lettori/valida/, loaded only now) reads the file from OPFS; desktop app: the native engine (`lr_valida`, the same core function). Same JSON bytes either way.
// `validaMzml` is the route api/mzml_valida (lr.js); `VALIDA_MZML.show(name)` opens the dialog (lista-file-menu.js). Module loaded by carica.js.
import { get } from "./archivio.js";

const W = /** @type {any} */ (globalThis);
/** rows shown at most: a broken file can have one finding per spectrum */
const MAX_ROWS = 500;

/** the JSON text of `mzlab-cli validate --json` for the mzML `name` of the session, or null when the file is not there; `rpc` is the one of lr.js */
export async function validaMzml(/** @type {string} */ name, /** @type {(m: any) => Promise<any>} */ rpc) {
  if (W.MZLAB_DESKTOP) return (await rpc({ op: "lr_valida", name })).info;
  const file = await get("files", name);
  return file ? await W.LETTORI_MOD.valida(file, name) : null;
}

/** @param {any} j the report of one file, as `json_file` writes it */
function body(j, /** @type {string} */ name) {
  const t = (/** @type {string} */ k, /** @type {any} */ p = {}) => I18N.t(k, p), esc = EH;
  const errors = j.problems.filter((/** @type {any} */ p) => p.severity === "error").length, warnings = j.problems.length - errors;
  const head = `<p role="status"><b style="color:var(${j.valid ? "--ok" : "--bad"})">${esc(t(j.valid ? "valida.valid" : "valida.invalid"))}</b> · ${esc(t(j.indexed ? "valida.indexed" : "valida.plain"))} · ${esc(t("valida.counts", { spectra: j.spectra, chroms: j.chromatograms, errors, warnings }))}</p>`;
  if (!j.problems.length) return head + `<p>${esc(t("valida.none"))}</p>`;
  const rows = j.problems.slice(0, MAX_ROWS).map((/** @type {any} */ p) => {
    const bad = p.severity === "error";
    return `<tr><td style="white-space:nowrap;color:var(${bad ? "--bad" : "--warn"})"><b>${esc(t(bad ? "valida.error" : "valida.warning"))}</b></td><td>${esc(p.code)}: ${esc(p.message)}</td><td><code style="overflow-wrap:anywhere">${esc(p.path || "-")}</code></td></tr>`;
  }).join("");
  const more = j.problems.length > MAX_ROWS ? `<p class="muted sm">${esc(t("valida.more", { n: MAX_ROWS, total: j.problems.length }))}</p>` : "";
  return head + `<div style="overflow:auto;max-height:60vh" tabindex="0" role="region" aria-label="${esc(t("valida.caption", { name }))}"><table class="qq-tab"><caption class="sm muted" style="text-align:left">${esc(t("valida.caption", { name }))}</caption><thead><tr><th scope="col">${esc(t("valida.col.severity"))}</th><th scope="col">${esc(t("valida.col.message"))}</th><th scope="col">${esc(t("valida.col.location"))}</th></tr></thead><tbody>${rows}</tbody></table></div>` + more;
}

const VALIDA_MZML = {
  /** validates the mzML `name` and shows the report */
  async show(/** @type {string} */ name) {
    const t = (/** @type {string} */ k, /** @type {any} */ p = {}) => I18N.t(k, p);
    big(t("valida.title", { name }), `<p role="status">${EH(t("valida.working"))}</p>`);
    const put = (/** @type {string} */ html) => { const b = document.getElementById("bigbody"); if (b) b.innerHTML = html; };
    try {
      const r = await fetch("api/mzml_valida?name=" + encodeURIComponent(name)), text = await r.text();
      const j = JSON.parse(text);
      if (!r.ok || j.error_key) throw I18N.err(j);
      put(body(Array.isArray(j) ? j[0] : j, name));
    } catch (e) {
      put(`<p role="alert">${EH(t("valida.fail", { detail: String((e && /** @type {any} */ (e).message) || e) }))}</p>`);
    }
  },
};
W.VALIDA_MZML = VALIDA_MZML;
export default VALIDA_MZML;
