// @ts-strict
// «Identification with the libraries» tab of the information bar (banco.js, BODY.idn): the MS2 spectrum of the active cell, the loaded libraries as rows with a box
// (the compatible ones are ticked by themselves), «Load file…» / «Load folder…», the menu of the public libraries (ident-catalogo.js) and the table of candidates.
// The search itself is the engine's (LIB.call("search", {ids})); JS only chooses the libraries and draws. Loaded by banco.js on first use.
import { menu as catalogMenu } from "./ident-catalogo.js";

/** What the tab remembers between two draws: the user's ticks, the last table, whether a new spectrum must be searched by itself. */
const S = /** @type {{ ticks: Map<string, boolean>, res: any, key: string, auto: boolean, msg: string, busy: boolean }} */ ({ ticks: new Map(), res: null, key: "", auto: false, msg: "", busy: false });
const READABLE = /\.(msp|mgf|json|jsonl|ndjson|mzlib)$/i;
const css = `.idn{display:flex;flex-direction:column;gap:6px;font-size:12px}.idn-row{display:flex;gap:6px;align-items:center;flex-wrap:wrap}.idn-sc{overflow-x:auto}.idn table{border-collapse:collapse;width:100%}
.idn th,.idn td{border-bottom:1px solid var(--line);padding:2px 6px;text-align:left}.idn .num{text-align:right}.idn tr.off{color:var(--muted)}.idn .bt,.idn button{min-height:24px;min-width:24px}
.idn input[type=checkbox]{width:18px;height:18px}.idn [aria-busy=true]{opacity:.6}@media (pointer:coarse){.idn button,.idn input[type=checkbox]{min-height:40px;min-width:40px}}`;

/** The line above the button: RT, precursor and polarity of the spectrum. */
function specLine(/** @type {any} */ q) {
  const pol = q.pol > 0 ? "+" : q.pol < 0 ? "−" : "?";
  return I18N.t("bn.idn.spec", { rt: q.rt != null ? q.rt.toFixed(2) + " min" : "–", mz: q.prec != null ? Number(q.prec).toFixed(4) : "–", pol });
}
const levelOf = (/** @type {any} */ l) => (/msn/i.test(`${l.name} ${l.file || ""}`) ? 3 : 2);
const polOf = (/** @type {any} */ l) => (l.pos && !l.neg ? 1 : l.neg && !l.pos ? -1 : 0);
const fmtOf = (/** @type {any} */ l) => String((l.fmt || (String(l.file || "").match(/\.(\w+)$/) || [])[1] || "mzlib")).toLowerCase();
/** Why a library is (not) searched for this spectrum: the key of the short tooltip. */
function reason(/** @type {any} */ l, /** @type {any} */ q, /** @type {number} */ level) {
  if (q.pol && polOf(l) && polOf(l) !== q.pol) return "bn.idn.why.pol";
  if (levelOf(l) > level) return "bn.idn.why.lvl";
  return "bn.idn.why.ok";
}
const isOn = (/** @type {any} */ l, /** @type {any} */ q, /** @type {number} */ level) => (S.ticks.has(l.id) ? !!S.ticks.get(l.id) : reason(l, q, level) === "bn.idn.why.ok");

function tableOf(/** @type {any} */ res, /** @type {string} */ unit) {
  if (!res) return "";
  if (!res.libs) return h`<p class="muted" role="status">${I18N.t("bn.idn.none")}</p>`;
  if (!res.results.length) return h`<p class="muted" role="status">${I18N.t("bn.idn.noRes")}</p>`;
  const dp = (/** @type {number} */ d) => (unit === "ppm" ? d.toFixed(1) : (d >= 0 ? "+" : "") + d.toFixed(4));
  const rows = res.results.slice(0, 15).map((/** @type {any} */ r) => h`<tr><th scope="row">${r.name}</th><td class="num">${r.ent.toFixed(3)}</td><td class="num">${dp(r.dprec)}</td><td class="num">${r.shared}</td><td>${r.lib}</td></tr>`).join("");
  return h`<div class="idn-sc"><table><caption class="sr-only">${I18N.t("bn.idn.res")}</caption><thead><tr><th scope="col">${I18N.t("lib.col.name")}</th><th scope="col" class="num">${I18N.t("lib.col.entropy")}</th><th scope="col" class="num">Δ ${unit}</th><th scope="col" class="num">${I18N.t("lib.col.shared")}</th><th scope="col">${I18N.t("lib.col.library")}</th></tr></thead><tbody>${raw(rows)}</tbody></table></div>`;
}

/** Draws the tab in `b` for the spectrum panel `sp` (null when the cell has none). `stale()` tells if a newer draw has replaced this one. */
export async function render(/** @type {HTMLElement} */ b, /** @type {any} */ sp, /** @type {() => boolean} */ stale) {
  if (!document.getElementById("idn-css")) { const st = document.createElement("style"); st.id = "idn-css"; st.textContent = css; document.head.appendChild(st); }
  const LIBS = /** @type {any} */ (window).LIB, q = sp && sp.level === 2 ? LIBS.queryOf(sp) : null;
  const prof = q ? LIBS.profOf(q.f) : { hr: false, v: 0.5, unit: "Da" };
  let libs = [];
  try { libs = (await LIBS.call("list")).libs || []; } catch (e) { libs = []; }
  if (stale()) return;
  const key = q ? `${q.f && q.f.k}|${q.prec}|${q.rt}|${q.peaks.length}` : "";
  const ids = () => libs.filter((/** @type {any} */ l) => q && isOn(l, q, 2)).map((/** @type {any} */ l) => l.id);
  const draw = () => {
    const rows = libs.map((/** @type {any} */ l) => { const on = q ? isOn(l, q, 2) : false, why = q ? I18N.t(reason(l, q, 2)) : "";
      return h`<tr class="${on ? "" : "off"}" title="${why}"><td><input type="checkbox" data-id="${l.id}" ${raw(on ? "checked" : "")} aria-label="${l.name}"></td><td>${l.name}</td><td class="num">${Number(l.n).toLocaleString()}</td><td>${polOf(l) > 0 ? "+" : polOf(l) < 0 ? "−" : "±"}</td><td>${levelOf(l) === 3 ? "MSn" : "MS2"}</td><td>${fmtOf(l)}</td></tr>`; }).join("");
    const libTable = libs.length ? h`<div class="idn-sc"><table><caption class="sr-only">${I18N.t("bn.idn.libs")}</caption><thead><tr><th><span class="sr-only">${I18N.t("bn.idn.libs")}</span></th><th scope="col">${I18N.t("lib.col.name")}</th><th scope="col" class="num">${I18N.t("lib.col.spectra")}</th><th scope="col">${I18N.t("bn.idn.col.pol")}</th><th scope="col">${I18N.t("bn.idn.col.level")}</th><th scope="col">${I18N.t("bn.idn.col.fmt")}</th></tr></thead><tbody>${raw(rows)}</tbody></table></div>` : h`<p class="muted">${I18N.t("bn.idn.none")}</p>`;
    b.innerHTML = h`<div class="idn"><div class="idn-row" role="status"><span>${q ? specLine(q) : I18N.t("bn.idn.needMs2")}</span></div>
      <div class="idn-row"><button type="button" id="idn-go" ${raw(q ? "" : "disabled")}>${I18N.t("bn.idn.search")}</button><button type="button" id="idn-file">${I18N.t("bn.idn.loadFile")}</button><button type="button" id="idn-dir">${I18N.t("bn.idn.loadDir")}</button>
        <input type="file" id="idn-fin" accept=".msp,.mgf,.json,.jsonl,.ndjson,.mzlib" multiple hidden><input type="file" id="idn-din" webkitdirectory multiple hidden></div>
      <div id="idn-menu"></div><div class="sm muted" id="idn-msg" role="status" aria-live="polite">${S.msg}</div>${raw(libTable)}
      <div id="idn-res" aria-busy="${S.busy}">${raw(tableOf(S.res, prof.unit))}</div></div>`;
    b.querySelectorAll("input[data-id]").forEach(c => { const i = /** @type {HTMLInputElement} */ (c); i.onchange = () => { S.ticks.set(String(i.dataset.id), i.checked); draw(); }; });
    const go = /** @type {HTMLButtonElement} */ (b.querySelector("#idn-go")); go.onclick = () => run(true);
    const fin = /** @type {HTMLInputElement} */ (b.querySelector("#idn-fin")), din = /** @type {HTMLInputElement} */ (b.querySelector("#idn-din"));
    /** @type {HTMLElement} */ (b.querySelector("#idn-file")).onclick = () => fin.click(); /** @type {HTMLElement} */ (b.querySelector("#idn-dir")).onclick = () => din.click();
    fin.onchange = () => { load([...(fin.files || [])]); fin.value = ""; }; din.onchange = () => { load([...(din.files || [])]); din.value = ""; };
    catalogMenu(/** @type {HTMLElement} */ (b.querySelector("#idn-menu")));
  };
  const say = (/** @type {string} */ t) => { S.msg = t; const m = b.querySelector("#idn-msg"); if (m) m.textContent = t; };
  /** Reads what the engine knows (msp, mgf, json, mzlib); the rest of a folder is counted, not read. */
  async function load(/** @type {File[]} */ files) {
    const ok = files.filter(f => READABLE.test(f.name)); let done = 0, bad = files.length - ok.length;
    for (const f of ok) {
      say(`${f.name}: ${I18N.t("lib.reading")}`);
      try { await LIBS.call("add", { file: f }, (/** @type {any} */ m) => say(`${f.name}: ${I18N.t("lib.readN", { n: Number(m.n).toLocaleString(), pct: Math.round(m.read / m.total * 100) })}`)); done++; }
      catch (e) { bad++; }
    }
    S.msg = I18N.t("bn.idn.loaded", { n: done, m: bad });
    if (!stale()) render(b, sp, stale);
  }
  async function run(/** @type {boolean} */ user) {
    if (!q) return;
    const use = ids(); if (!use.length) { say(I18N.t("bn.idn.noSel")); return; }
    if (user) S.auto = true;
    S.busy = true; const rb = b.querySelector("#idn-res"); if (rb) rb.setAttribute("aria-busy", "true");
    try { S.res = await LIBS.call("search", { peaks: q.peaks, prec: q.prec, pol: q.pol, tol: { v: prof.v, unit: prof.unit }, frag: prof.hr ? 0.01 : 0.5, ids: use }); S.msg = ""; }
    catch (e) { S.msg = e instanceof Error ? e.message : String(e); }
    S.busy = false; S.key = key;
    if (!stale()) draw();
  }
  draw();
  // the table follows the cell: a new spectrum is searched again by itself once the user has asked for a search (the old table stays until the new one arrives)
  if (q && S.auto && key !== S.key && !S.busy) run(false);
}
/** @type {any} */ (window).IDP = { render };
