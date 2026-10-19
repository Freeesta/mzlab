// @ts-strict
"use strict";
// Settings gear: ONLY three controls (text size, theme, chart colours). Classic script, loaded after explore.js/tabs.js
// (uses UIP, PALS, setPal, paintFiles, Q, redrawAll from there). Everything is stored in localStorage inside try/catch: it may be blocked
// (private window, strict settings) and the page must work without it.
const UIP_KEY = NOMI.KEY("prefs"); // kept from the old name: renaming it would lose the users' data
const uipRead = k => { try { return localStorage.getItem(k); } catch (e) { return null; } };
const uipWrite = (k, v) => { try { localStorage.setItem(k, v); } catch (e) { /* storage not available: the setting lasts until the page is closed */ } };
function a11yRead() { return lsObj(NOMI.KEY("a11y")); }
function a11yApply(o) {
  const r = document.documentElement;
  if (o.font) r.setAttribute("data-a11y-font", o.font); else r.removeAttribute("data-a11y-font");
  if (o.bg === "hc") r.setAttribute("data-a11y-bg", "hc"); else r.removeAttribute("data-a11y-bg");
  const rm = /** @type {any} */ (window).matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (o.motion === "always" || (rm && o.motion !== "never")) r.setAttribute("data-a11y-reduce", ""); else r.removeAttribute("data-a11y-reduce");
}
function a11yWrite(o) {
  try { localStorage.setItem(NOMI.KEY("a11y"), JSON.stringify(o)); } catch (e) { /* storage blocked: the setting applies now and is not remembered */ }
  a11yApply(o);
}
window.QA11Y = window.QA11Y || { read: a11yRead, apply: a11yApply, write: a11yWrite };
// calibration standards (MRM): one orange hue by concentration (used by paintFiles in explore.js)
const STD_SEQ_LIGHT = ["#fdae6b", "#e6550d", "#a63603", "#5a1d02"];     // one orange hue, light (low concentration) to dark (high) on white
const STD_SEQ_DARK  = ["#ffedd5", "#fed7aa", "#fba94b", "#f97316", "#ff5722"]; // luminous peach to bright orange/coral on dark
function stdColor(u) {                                       // u in 0..1
  const seq = isDark() ? STD_SEQ_DARK : STD_SEQ_LIGHT;
  const P = PALS[UIP.pal], x = (P.hc ? 0.45 + 0.55 * u : u) * (seq.length - 1), i = Math.min(Math.floor(x), seq.length - 2), f = x - i;
  const c = h => [1, 3, 5].map(k => parseInt(h.slice(k, k + 2), 16)), A = c(seq[i]), B = c(seq[i + 1]);
  return "#" + A.map((v, k) => Math.round(v + (B[k] - v) * f).toString(16).padStart(2, "0")).join("");
}
function uipLoad() {
  QA11Y.apply(QA11Y.read());
  try { const o = JSON.parse(uipRead(UIP_KEY) || "{}"); if (["auto", "light", "dark"].includes(o.theme)) UIP.theme = o.theme; if (PALS[o.pal]) UIP.pal = o.v >= 2 || o.pal !== "time" ? o.pal : "file";   /* before v2 "time" was the default, not a choice */ if (o.merge === false) UIP.merge = false; if (o.beside === true) UIP.beside = true;  if (o.hrPpm >= 1 && o.hrPpm <= 50) UIP.hrPpm = +o.hrPpm; if ([3, 4, 5].includes(o.hrDec)) UIP.hrDec = o.hrDec; } catch (e) { /* corrupt value: defaults */ }
  setPal(UIP.pal);
}
function uipSave() {
  const o = { v: 2, theme: UIP.theme, pal: UIP.pal, merge: UIP.merge };
  if (UIP.beside) o.beside = true;
  if (UIP.hrPpm !== 5) o.hrPpm = UIP.hrPpm; if (UIP.hrDec !== 4) o.hrDec = UIP.hrDec;         // high resolution: only what differs from the defaults
  uipWrite(UIP_KEY, JSON.stringify(o));
}
// the settings panel of the Teoria (inside the iframe) changes the theme of the whole program through here
window.uipSetTheme = t => { if (["auto", "light", "dark"].includes(t)) { UIP.theme = t; uipSave(); TEMASOFT.run(uipApply); } };
function uipApply() {
  const r = document.documentElement;
  if (UIP.theme === "auto") r.removeAttribute("data-theme"); else r.setAttribute("data-theme", UIP.theme);
  const tf = /** @type {HTMLIFrameElement | null} */ (document.getElementById("tframe"));                // the Teoria (iframe) follows the theme too (it reads it itself when it loads; this is for a change while it is open)
  if (tf && tf.contentWindow) { try { tf.contentWindow.postMessage({ type: NOMI.KEY("theme"), theme: UIP.theme }, location.origin); } catch (_) { /* not loaded yet */ } }
  if (typeof setPal === "function") setPal(UIP.pal);
  if (typeof paintFiles === "function" && typeof E !== "undefined") {
    paintFiles();
    if (E.files && E.files.length && typeof renderFileList === "function") renderFileList();
  }
  if (typeof redrawAll === "function") redrawAll();           // canvases read the colours from the CSS variables
}
if (typeof window !== "undefined" && window.matchMedia) {
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (UIP.theme === "auto") uipApply();
  });
}

// «Riduci ogni picco a un solo punto»: every profile file at low resolution gets a copy `<name>.centroidi.mzML` next to it (the original is not changed); the copy is loaded like any file
const CEN_EXT = /\.centroidi\.mzML$/i;
function centroidTargets() {
  const all = typeof ST !== "undefined" && ST.files || [], have = new Set(all.map(f => f.name));
  return all.filter(f => !f.gone && f.spectrum_mode === "profile" && !f.hr_profile && /\.mzML$/i.test(f.name) && !CEN_EXT.test(f.name) && !have.has(f.name.replace(/\.mzML$/i, ".centroidi.mzML")));
}
async function centroidAll(btn, stop, st, refresh) {
  const rb = window.MZLAB_RUST_READY ? await window.MZLAB_RUST_READY : null;
  const list = centroidTargets();
  if (!rb || !rb.convertCentroids || !list.length) return;
  const sizeOf = f => f.size || 0;
  const ask = I18N.t("settings.cen.ask", { n: list.length, size: (list.reduce((a, f) => a + sizeOf(f), 0) / 1e6).toFixed(0) + " MB" });
  if (typeof yesno === "function" && !(await yesno(ask))) return;
  const ctl = new AbortController(); let made = 0, failed = 0;
  btn.disabled = true; stop.hidden = false; stop.onclick = () => ctl.abort();
  for (let i = 0; i < list.length && !ctl.signal.aborted; i++) {
    const f = list[i], name = f.name.replace(/\.mzML$/i, ".centroidi.mzML");
    const line = frac => { st.textContent = I18N.t("settings.cen.run", { n: i + 1, m: list.length, name: f.name }) + (frac >= 0 ? " " + Math.floor(100 * frac) + " %" : ""); };
    line(-1);
    try {
      const blob = await rb.convertCentroids(f.name, sizeOf(f), p => { if (p.scansTotal) line(p.scans / p.scansTotal); }, ctl.signal);
      if (blob) { await upload([new File([blob], name, { type: "application/xml" })], "data"); made++; }
    } catch (e) {
      if (e && e.cancelled) break;
      failed++; st.textContent = I18N.t("settings.cen.fail", { name: f.name });
    }
  }
  stop.hidden = true; btn.disabled = false;
  if (!failed) st.textContent = ctl.signal.aborted ? I18N.t("settings.cen.stopped", { n: made }) : I18N.t("settings.cen.done", { n: made });
  refresh();
}

// Bottom of the settings: the centroid command (browser only) and the full reset (with its own confirmation)
function uipReset(d) {
  const cen = d.querySelector("#uip-cen");
  if (cen) cen.onclick = () => centroidAll(cen, d.querySelector("#uip-cen-x"), d.querySelector("#uip-cen-st"), () => {});
}

// Administrator mode (5 clicks on the logo): the exit row shows only while it is on; the flag is shared with the Teoria
function uipAdminOn() { try { return localStorage.getItem(NOMI.KEY("admin")) === "1" || localStorage.getItem(NOMI.KEY("teoria.esteso")) === "1"; } catch (_) { return false; } }

// known contaminants on HR spectra: off by default; on only if explicitly turned on (read from the same key as liste.js, which may not be loaded yet)
function uipContOn() { try { return localStorage.getItem(NOMI.KEY("contaminanti.on")) === "true"; } catch (_) { return false; } }

// --- settings panel (same look as the one of the Teoria: shared pieces in teoria/impostazioni-comp.js)
const UIP_GROUP_KEY = id => NOMI.KEY(id === "uip-a11y" ? "set.a11y" : "set." + id);      // the group «Accessibility» keeps the key it always had
function uipGroupOpen(id, dflt) { try { const v = localStorage.getItem(UIP_GROUP_KEY(id)); return v === null ? dflt : v === "1"; } catch (_) { return dflt; } }
function uipRestore() {            // theme, colours, reading and high-resolution options go back to the factory values; files and sessions are never touched
  Object.assign(UIP, { theme: "auto", pal: "file", merge: true, beside: false, hrPpm: 5, hrDec: 4 });
  QA11Y.write({}); uipSave(); uipApply();
}
function uipOpen(btn) {
  const old = Q("#uipset"); if (old) { old.remove(); return; }
  const C = window.IMPCOMP, T = (k, p) => I18N.t(k, p), esc = C.esc;
  const mem = window.MZLAB_RUST && window.MZLAB_RUST.budgetRow && uipAdminOn() ? window.MZLAB_RUST.budgetRow() : null;      // memory budget of the engine: only a read-only record, in the administrator's diagnostics
  const files = typeof E !== "undefined" && E.files ? E.files : [], hr = files.some(f => window.HR && HR.isHr(f)), a11y = QA11Y.read();
  const d = document.createElement("div"); d.id = "uipset"; d.className = "impset"; d.setAttribute("role", "dialog"); d.setAttribute("aria-label", T("settings.title"));
  const pals = ["file", ...Object.keys(PALS).filter(k => k !== "file")].map(k => `<option value="${k}">${I18N.t(`settings.pal.${k}`)}</option>`).join("");
  const fld = (label, tip, inner) => `<div class="imp-fld" title="${esc(tip)}"><span>${label}</span>${inner}</div>`;
  const look = C.section(T("settings.theme"), C.opts("seg3", C.radios("uip-th", [["auto", T("settings.theme.auto")], ["light", T("settings.theme.light")], ["dark", T("settings.theme.dark")]], UIP.theme, "seg")))
    + C.section(T("settings.pal.label"), `<select id="uip-pal" title="${esc(T("settings.pal.title"))}" aria-label="${esc(T("settings.pal.label"))}">${pals}</select>`)
    + C.sw(T("settings.beside"), T("settings.beside.title"), 'id="uip-beside"', UIP.beside, "side");
  const read = C.section(T("settings.font.label"), C.opts("f3", C.radios("uip-font", [["", T("settings.font.default")], ["atkinson", T("settings.font.atkinson")], ["dyslexic", T("settings.font.dyslexic")]], a11y.font || "", "font")))
    + C.sw(T("settings.hc.label"), "", 'id="uip-hc"', a11y.bg === "hc", "hc") + C.sw(T("settings.motion"), T("settings.motion.title"), 'id="uip-motion"', a11y.motion === "always", "motion");
  const cen = window.MZLAB_BROWSER && typeof centroidTargets === "function" && centroidTargets().length
    ? `<div class="imp-fld"><button id="uip-cen" type="button" title="${esc(T("settings.cen.title"))}">${T("settings.cen.btn")}</button><button id="uip-cen-x" type="button" hidden>${T("settings.cen.stop")}</button></div><div class="hint" id="uip-cen-st" role="status" aria-live="polite"></div>` : "";
  const data = ((!files.length || files.some(f => f.spectrum_mode !== "profile" || (window.HR && HR.isHr(f)))) ? C.sw(T("settings.merge"), "", 'id="uip-merge"', UIP.merge, "cont") : "")
    + (hr ? fld(`<span class="ic">${C.GICON.lib}</span>${T("settings.libs.label")}`, T("settings.libs.title"), `<button id="uip-lib" type="button">${T("settings.libs.button")}</button>`)
      + fld(T("settings.hrPpm"), T("settings.hrPpm.title"), `<input type="number" id="uip-ppm" min="1" max="50" step="1" value="${UIP.hrPpm}" style="width:64px" aria-label="${esc(T("settings.hrPpm"))}">`)
      + fld(T("settings.hrDec"), T("settings.hrDec.title"), `<input type="number" id="uip-hdec" min="3" max="5" step="1" value="${UIP.hrDec}" style="width:56px" aria-label="${esc(T("settings.hrDec"))}">`) : "") + cen;
  const tourOn = !(window.BANCO && BANCO.on());
  const app = (window.DESKUPD ? DESKUPD.row() : "") + (window.MZLAB_DESKTOP ? C.sw(T("settings.ext"), T("settings.ext.title"), 'id="uip-ext"', false, "ext") : "")
    + (tourOn ? `<div class="imp-fld"><button id="uip-tour" type="button"><span class="ic">${C.GICON.play}</span>${T(files.length ? "tour.impost.rifai" : "tour.impost.esempio")}</button></div>` : "");
  const grp = (id, icon, title, body, dflt) => body ? C.group(id, icon, T(title), body, uipGroupOpen(id, dflt)) : "";
  d.innerHTML = `<div class="a-hd"><h2>${T("settings.title")}</h2><button type="button" class="x" id="uip-x" aria-label="${esc(T("settings.close"))}">&times;</button></div>`
    + C.section(T("settings.language.label"), C.opts("seg2", C.radios("uip-lang", [["it", "Italiano", T("settings.language.title"), C.GICON.lang], ["en", "English", T("settings.language.title"), C.GICON.lang]], I18N.lang, "seg")))
    + grp("uip-g-look", "look", "settings.g.look", look, true) + grp("uip-a11y", "read", "settings.g.read", read, false) + grp("uip-g-data", "data", "settings.g.data", data, false)
    + grp("uip-g-app", "app", "settings.g.app", app, false) + grp("uip-g-diag", "cont", "settings.g.diag", mem ? mem.html : "", false)
    + `<div class="row"><button type="button" id="uip-reset" title="${esc(T("settings.reset.title"))}">${T("settings.reset")}</button></div>`;
  document.body.appendChild(d);
  d.querySelectorAll("details.imp-g").forEach(g => g.addEventListener("toggle", () => { try { localStorage.setItem(UIP_GROUP_KEY(g.id), /** @type {HTMLDetailsElement} */ (g).open ? "1" : "0"); } catch (_) { /* not remembered */ } }));
  if (window.DESKUPD) DESKUPD.bind(d);
  if (mem) mem.bind(d);
  const on = (/** @type {string} */ sel, /** @type {string} */ ev, /** @type {(e: any) => void} */ fn) => { const el = d.querySelector(sel); if (el) el.addEventListener(ev, fn); };
  const radio = (/** @type {string} */ name, /** @type {(v: string) => void} */ fn) => d.querySelectorAll(`input[name="${name}"]`).forEach(r => r.addEventListener("change", e => { if (/** @type {HTMLInputElement} */ (e.target).checked) fn(/** @type {HTMLInputElement} */ (e.target).value); }));
  radio("uip-lang", v => I18N.set(v));                                  // remembered in qqq.lang, then the page reloads (open files come back with the session)
  radio("uip-th", v => { UIP.theme = v; uipSave(); TEMASOFT.run(uipApply); });
  radio("uip-font", v => { const c = QA11Y.read(); c.font = v; QA11Y.write(c); });
  on("#uip-hc", "change", e => { const c = QA11Y.read(); c.bg = e.target.checked ? "hc" : ""; QA11Y.write(c); if (typeof redrawAll === "function") redrawAll(); });
  on("#uip-motion", "change", e => { const c = QA11Y.read(); c.motion = e.target.checked ? "always" : "auto"; QA11Y.write(c); });
  const pal = /** @type {HTMLSelectElement} */ (d.querySelector("#uip-pal")); pal.value = UIP.pal;
  pal.onchange = () => { setPal(pal.value); uipSave(); uipApply(); };              // applied at once: file colours, lists, legends, every graph
  on("#uip-merge", "change", e => {                                     // spectra are asked again to the server with / without the merge
    UIP.merge = e.target.checked; uipSave(); CACHE.clear(); SC.m.clear(); SC.n.clear();
    if (typeof redrawAll === "function") redrawAll();
  });
  on("#uip-beside", "change", e => { UIP.beside = e.target.checked; uipSave(); });      // read when a spectrum is opened (newSpec): nothing to redraw
  on("#uip-tour", "click", () => {      // the tour of Dati lives in Dati; in Disegno the button starts the tour of Disegno
    d.remove();
    if (window.TOUR && typeof S !== "undefined" && S.view === "draw") { TOUR.disegno(); return; }
    const go = () => { if (!window.TOUR) return; if (files.length) TOUR.inizia(1, 0); else TOUR.caricaEsempio(); };
    if (typeof S !== "undefined" && S.view !== "data" && typeof setView === "function") { setView("data"); setTimeout(go, 150); } else go();
  });
  uipReset(d);
  const uipExt = d.querySelector("#uip-ext");           // desktop app: external downloads of public libraries (off until the user turns it on)
  if (uipExt && window.__TAURI__ && __TAURI__.core) {
    __TAURI__.core.invoke("lib_get_config").then(c => { uipExt.checked = !!(c && c.external); }, () => {});
    uipExt.onchange = () => __TAURI__.core.invoke("lib_set_external", { on: uipExt.checked }).catch(() => { uipExt.checked = !uipExt.checked; });
  }
  on("#uip-lib", "click", () => { d.remove(); if (window.LIB) LIB.open(); });
  const hrChanged = () => {
    uipSave(); CACHE.clear(); SC.m.clear(); SC.n.clear();
    if (typeof renderFileList === "function" && E.files.length) renderFileList();
    if (typeof redrawAll === "function") redrawAll();
  };
  on("#uip-ppm", "change", e => { UIP.hrPpm = Math.min(50, Math.max(1, +e.target.value || 5)); e.target.value = UIP.hrPpm; hrChanged(); });
  on("#uip-hdec", "change", e => { UIP.hrDec = Math.min(5, Math.max(3, Math.round(+e.target.value || 4))); e.target.value = UIP.hrDec; hrChanged(); });
  on("#uip-reset", "click", () => { uipRestore(); d.remove(); uipOpen(btn); });
  const close = () => { d.remove(); if (btn.isConnected) btn.focus(); };
  on("#uip-x", "click", close);
  document.addEventListener("keydown", function esc(e) { if (!d.isConnected) return document.removeEventListener("keydown", esc); if (e.key === "Escape") { close(); document.removeEventListener("keydown", esc); } });       // Esc closes the settings and gives the focus back to the gear
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.isConnected) return document.removeEventListener("mousedown", h); if (!d.contains(/** @type {Node} */ (e.target)) && e.target !== btn && !btn.contains(e.target)) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
  (d.querySelector("input:checked") || d.querySelector("input,button")).focus();
}

uipLoad();
(() => {
  const nav = Q("#nav"), b = document.getElementById("np-set") || document.createElement("button"); b.id = "np-set"; b.className = "ib gear"; b.title = I18N.t("settings.gear.title"); b.setAttribute("aria-label", I18N.t("settings.title"));
  b.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>';
  if (!b.isConnected) nav.insertAdjacentElement("afterend", b); b.onclick = e => {      // in the Teoria the gear opens the settings of the Teoria, elsewhere the ones of the program
    e.stopPropagation();
    const tw = /** @type {any} */ (document.documentElement.classList.contains("in-theory") && /** @type {HTMLIFrameElement} */ (document.getElementById("tframe")).contentWindow);
    if (tw && tw.TEORIA && tw.TEORIA.settings) tw.TEORIA.settings(); else uipOpen(b);
  };
  uipApply();
})();
