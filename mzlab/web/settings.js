// @ts-strict
"use strict";
// Settings gear: ONLY three controls (text size, theme, chart colours). Classic script, loaded after explore.js/tabs.js
// (uses UIP, PALS, setPal, paintFiles, Q, redrawAll from there). Everything is stored in localStorage inside try/catch: it may be blocked
// (private window, strict settings) and the page must work without it.
const UIP_KEY = "qqq.prefs"; // kept from the old name: renaming it would lose the users' data
const uipRead = k => { try { return localStorage.getItem(k); } catch (e) { return null; } };
const uipWrite = (k, v) => { try { localStorage.setItem(k, v); } catch (e) { /* storage not available: the setting lasts until the page is closed */ } };
function a11yRead() { return lsObj("qqq.a11y"); }
function a11yApply(o) {
  const r = document.documentElement;
  if (o.font) r.setAttribute("data-a11y-font", o.font); else r.removeAttribute("data-a11y-font");
  if (o.bg === "hc") r.setAttribute("data-a11y-bg", "hc"); else r.removeAttribute("data-a11y-bg");
  const rm = /** @type {any} */ (window).matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (o.motion === "always" || (rm && o.motion !== "never")) r.setAttribute("data-a11y-reduce", ""); else r.removeAttribute("data-a11y-reduce");
}
function a11yWrite(o) {
  try { localStorage.setItem("qqq.a11y", JSON.stringify(o)); } catch (e) { /* storage blocked: the setting applies now and is not remembered */ }
  a11yApply(o);
}
window.QA11Y = window.QA11Y || { read: a11yRead, apply: a11yApply, write: a11yWrite };
// calibration standards (MRM): one orange hue by concentration (used by paintFiles in explore.js)
// one orange hue, light (low concentration) to dark (high) on white
const STD_SEQ_LIGHT = ["#fdae6b", "#e6550d", "#a63603", "#5a1d02"];
// luminous peach to bright orange/coral on dark
const STD_SEQ_DARK  = ["#ffedd5", "#fed7aa", "#fba94b", "#f97316", "#ff5722"];
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
  // high resolution: only what differs from the defaults
  if (UIP.hrPpm !== 5) o.hrPpm = UIP.hrPpm; if (UIP.hrDec !== 4) o.hrDec = UIP.hrDec;
  uipWrite(UIP_KEY, JSON.stringify(o));
}
// the settings panel of the Teoria (inside the iframe) changes the theme of the whole program through here
window.uipSetTheme = t => { if (["auto", "light", "dark"].includes(t)) { UIP.theme = t; uipSave(); uipApply(); } };
function uipApply() {
  const r = document.documentElement;
  if (UIP.theme === "auto") r.removeAttribute("data-theme"); else r.setAttribute("data-theme", UIP.theme);
  // the Teoria (iframe) follows the theme too (it reads it itself when it loads; this is for a change while it is open)
  const tf = /** @type {HTMLIFrameElement | null} */ (document.getElementById("tframe"));
  if (tf && tf.contentWindow) { try { tf.contentWindow.postMessage({ type: "qqq-theme", theme: UIP.theme }, location.origin); } catch (_) { /* not loaded yet */ } }
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
function uipAdminOn() { try { return localStorage.getItem("qqq.admin") === "1" || localStorage.getItem("qqq.teoria.esteso") === "1"; } catch (_) { return false; } }

// known contaminants on HR spectra: on unless the student turned them off (read from the same key as liste.js, which may not be loaded yet)
function uipContOn() { try { return localStorage.getItem("qqq.contaminanti.on") !== "false"; } catch (_) { return true; } }

// --- popup of the gear
function uipOpen(btn) {
  const old = Q("#uipset"); if (old) { old.remove(); return; }
  const d = document.createElement("div"); d.id = "uipset"; d.setAttribute("role", "dialog"); d.setAttribute("aria-label", I18N.t("settings.title"));
  d.innerHTML = `<div class="sm" style="font-weight:600;margin-bottom:6px">${I18N.t("settings.title")}</div>
    <div class="row"><span>${I18N.t("settings.language.label")}</span> <select id="uip-lang" title="${I18N.t("settings.language.title")}" aria-label="${I18N.t("settings.language.label")}"><option value="it">Italiano</option><option value="en">English</option></select></div>
    <div class="row">${I18N.t("settings.theme")} <select id="uip-th" aria-label="${I18N.t("settings.theme")}"><option value="auto">${I18N.t("settings.theme.auto")}</option><option value="light">${I18N.t("settings.theme.light")}</option><option value="dark">${I18N.t("settings.theme.dark")}</option></select></div>
    <details id="uip-a11y" class="uip-sub"><summary>${I18N.t("settings.a11y.title")}</summary>
    <div class="row"><span>${I18N.t("settings.font.label")}</span> <select id="uip-font" aria-label="${I18N.t("settings.font.label")}"><option value="">${I18N.t("settings.font.default")}</option><option value="atkinson">${I18N.t("settings.font.atkinson")}</option><option value="dyslexic">${I18N.t("settings.font.dyslexic")}</option></select></div>
    <label class="row" style="align-items:flex-start;gap:6px"><input type="checkbox" id="uip-hc"> <span>${I18N.t("settings.hc.label")}</span></label>
    <div class="row">${I18N.t("settings.pal.label")} <select id="uip-pal" title="${I18N.t("settings.pal.title")}">${["file", ...Object.keys(PALS).filter(k => k !== "file")].map(k => `<option value="${k}">${I18N.t(`settings.pal.${k}`)}</option>`).join("")}</select></div>
    </details>
    <label class="row" style="align-items:flex-start;gap:6px"><input type="checkbox" id="uip-merge" ${UIP.merge ? "checked" : ""}> <span>${I18N.t("settings.merge")}</span></label>
    <label class="row" style="align-items:flex-start;gap:6px" title="${I18N.t("settings.beside.title")}"><input type="checkbox" id="uip-beside" ${UIP.beside ? "checked" : ""}> <span>${I18N.t("settings.beside")}</span></label>
    ${(typeof E !== "undefined" && E.files && E.files.some(f => window.HR && HR.isHr(f))) ? `
    <div class="row">${I18N.t("settings.libs.label")} <button id="uip-lib" title="${I18N.t("settings.libs.title")}">${I18N.t("settings.libs.button")}</button></div>` : ""}
    ${(typeof E !== "undefined" && E.files && E.files.some(f => window.HR && HR.isHr(f))) ? `
    <div class="row hr-only" title="${I18N.t("settings.hrPpm.title")}">${I18N.t("settings.hrPpm", { input: `<input type="number" id="uip-ppm" min="1" max="50" step="1" value="${UIP.hrPpm}" style="width:56px">` })}</div>
    <label class="row hr-only" style="align-items:flex-start;gap:6px" title="${I18N.t("settings.cont.title")}"><input type="checkbox" id="uip-cont" ${uipContOn() ? "checked" : ""}> <span>${I18N.t("settings.cont")}</span></label>
    <div class="row hr-only" title="${I18N.t("settings.hrDec.title")}">${I18N.t("settings.hrDec")} <input type="number" id="uip-hdec" min="3" max="5" step="1" value="${UIP.hrDec}" style="width:44px"></div>
    ` : ""}
    <div class="uip-arch" id="uip-space-row" style="margin:8px 0;padding-top:8px;border-top:1px solid var(--line)">${window.MZLAB_BROWSER && typeof centroidTargets === "function" && centroidTargets().length ? `<div style="display:flex;flex-wrap:wrap;gap:6px;align-items:center;margin-bottom:6px"><button id="uip-cen" type="button" title="${I18N.t("settings.cen.title")}">${I18N.t("settings.cen.btn")}</button><button id="uip-cen-x" type="button" hidden>${I18N.t("settings.cen.stop")}</button><span class="sm" id="uip-cen-st" role="status" aria-live="polite"></span></div>` : ""}</div>
    ${window.DESKUPD ? DESKUPD.row() : ""}
    ${window.MZLAB_DESKTOP ? `<label class="row" style="align-items:flex-start;gap:6px" title="${I18N.t("settings.ext.title")}"><input type="checkbox" id="uip-ext"> <span>${I18N.t("settings.ext")}</span></label>` : ""}
    ${!(window.BANCO && BANCO.on()) ? `
    <div class="uip-tour-sec" style="display:flex;gap:8px;align-items:center;justify-content:space-between;margin:6px 0;margin-top:8px;border-top:1px solid var(--line);padding-top:8px"><span>${I18N.t("tour.impost.titolo")}</span> <button id="uip-tour" type="button">${I18N.t(typeof E !== "undefined" && E.files && E.files.length ? "tour.impost.rifai" : "tour.impost.esempio")}</button></div>
    ` : ""}
    `;
  document.body.appendChild(d);
  if (window.DESKUPD) DESKUPD.bind(d);
  // the label is always bilingual: whoever cannot read the current language finds it
  const uipLang = d.querySelector("#uip-lang"); uipLang.value = I18N.lang;
  // remembered in qqq.lang, then the page reloads (open files come back with the session)
  uipLang.onchange = e => I18N.set(e.target.value);
  // closed at first, then as the user left it
  const sub = /** @type {HTMLDetailsElement} */ (d.querySelector("#uip-a11y"));
  try { sub.open = localStorage.getItem("qqq.set.a11y") === "1"; } catch (_) { /* closed */ }
  sub.addEventListener("toggle", () => { try { localStorage.setItem("qqq.set.a11y", sub.open ? "1" : "0"); } catch (_) { /* not remembered */ } });
  const a11yCur = QA11Y.read();
  const uipFont = d.querySelector("#uip-font");
  if (uipFont) {
    uipFont.value = a11yCur.font || "";
    uipFont.onchange = e => { const c = QA11Y.read(); c.font = e.target.value; QA11Y.write(c); };
  }
  const uipHc = d.querySelector("#uip-hc");
  if (uipHc) {
    uipHc.checked = a11yCur.bg === "hc";
    uipHc.onchange = e => {
      const c = QA11Y.read(); c.bg = e.target.checked ? "hc" : ""; QA11Y.write(c);
      if (typeof redrawAll === "function") redrawAll();
    };
  }
  d.querySelector("#uip-th").value = UIP.theme; d.querySelector("#uip-pal").value = UIP.pal;
  const r = btn.getBoundingClientRect(); d.style.top = r.bottom + 6 + "px"; d.style.left = Math.max(8, Math.min(r.left, innerWidth - d.offsetWidth - 8)) + "px";
  // spectra are asked again to the server with / without the merge
  d.querySelector("#uip-merge").onchange = e => {
    UIP.merge = e.target.checked; uipSave(); CACHE.clear(); SC.m.clear(); SC.n.clear();
    if (typeof redrawAll === "function") redrawAll();
  };
  // read when a spectrum is opened (newSpec): nothing to redraw
  d.querySelector("#uip-beside").onchange = e => { UIP.beside = e.target.checked; uipSave(); };
  const uipTour = d.querySelector("#uip-tour");
  // the tour of Dati lives in Dati; in Disegno the button starts the tour of Disegno
  if (uipTour) uipTour.onclick = () => {
    d.remove();
    // in Disegno the button starts the tour of Disegno
    if (window.TOUR && typeof S !== "undefined" && S.view === "draw") { TOUR.disegno(); return; }
    const go = () => { if (!window.TOUR) return; if (typeof E !== "undefined" && E.files && E.files.length) TOUR.inizia(1, 0); else TOUR.caricaEsempio(); };
    if (typeof S !== "undefined" && S.view !== "data" && typeof setView === "function") { setView("data"); setTimeout(go, 150); } else go();
  };
  const uipCont = d.querySelector("#uip-cont"); if (uipCont) uipCont.onchange = e => { if (window.LISTE) LISTE.setOn(e.target.checked); };
  uipReset(d);
  // desktop app: external downloads of public libraries (off until the user turns it on)
  const uipExt = d.querySelector("#uip-ext");
  if (uipExt && window.__TAURI__ && __TAURI__.core) {
    __TAURI__.core.invoke("lib_get_config").then(c => { uipExt.checked = !!(c && c.external); }, () => {});
    uipExt.onchange = () => __TAURI__.core.invoke("lib_set_external", { on: uipExt.checked }).catch(() => { uipExt.checked = !uipExt.checked; });
  }
  const uipLib = d.querySelector("#uip-lib"); if (uipLib) uipLib.onclick = () => { d.remove(); if (window.LIB) LIB.open(); };
  const hrChanged = () => {
    uipSave(); CACHE.clear(); SC.m.clear(); SC.n.clear();
    if (typeof renderFileList === "function" && E.files.length) renderFileList();
    if (typeof redrawAll === "function") redrawAll();
  };
  const uipPpm = d.querySelector("#uip-ppm"); if (uipPpm) uipPpm.onchange = e => { UIP.hrPpm = Math.min(50, Math.max(1, +e.target.value || 5)); e.target.value = UIP.hrPpm; hrChanged(); };
  const uipHdec = d.querySelector("#uip-hdec"); if (uipHdec) uipHdec.onchange = e => { UIP.hrDec = Math.min(5, Math.max(3, Math.round(+e.target.value || 4))); e.target.value = UIP.hrDec; hrChanged(); };
  d.querySelector("#uip-th").onchange = e => { UIP.theme = e.target.value; uipSave(); uipApply(); };
  // applied at once: file colours, lists, legends, every graph
  d.querySelector("#uip-pal").onchange = e => {
    setPal(e.target.value); uipSave();
    if (typeof paintFiles === "function" && typeof E !== "undefined") { paintFiles(); if (E.files.length) { renderFileList(); } }
    uipApply();
  };
  // Esc closes the settings and gives the focus back to the gear
  document.addEventListener("keydown", function esc(e) { if (!d.isConnected) return document.removeEventListener("keydown", esc); if (e.key === "Escape") { d.remove(); btn.focus(); document.removeEventListener("keydown", esc); } });
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.contains(/** @type {Node} */ (e.target)) && e.target !== btn && !btn.contains(e.target)) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
}

uipLoad();
(() => {
  const nav = Q("#nav"), b = document.createElement("button"); b.id = "np-set"; b.className = "ib gear"; b.title = I18N.t("settings.gear.title"); b.setAttribute("aria-label", I18N.t("settings.title"));
  b.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>';
  // in the Teoria the gear opens the settings of the Teoria, elsewhere the ones of the program
  nav.insertAdjacentElement("afterend", b); b.onclick = e => {
    e.stopPropagation();
    const tw = /** @type {any} */ (document.documentElement.classList.contains("in-theory") && /** @type {HTMLIFrameElement} */ (document.getElementById("tframe")).contentWindow);
    if (tw && tw.TEORIA && tw.TEORIA.settings) tw.TEORIA.settings(); else uipOpen(b);
  };
  uipApply();
})();
