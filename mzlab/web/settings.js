"use strict";
// Settings gear: ONLY three controls (text size, theme, chart colours). Classic script, loaded after explore.js/tabs.js
// (uses UIP, PALS, setPal, paintFiles, Q, redrawAll from there). Everything is stored in localStorage inside try/catch: it may be blocked
// (private window, strict settings) and the page must work without it.
const UIP_KEY = "qqq.prefs"; // kept from the old name: renaming it would lose the users' data
const uipRead = k => { try { return localStorage.getItem(k); } catch (e) { return null; } };
const uipWrite = (k, v) => { try { localStorage.setItem(k, v); } catch (e) { /* storage not available: the setting lasts until the page is closed */ } };
function a11yRead() { var o = {}; try { o = JSON.parse(localStorage.getItem("qqq.a11y") || "{}") || {}; } catch (e) {} return o; }
function a11yApply(o) {
  const r = document.documentElement;
  if (o.font) r.setAttribute("data-a11y-font", o.font); else r.removeAttribute("data-a11y-font");
  if (o.bg === "hc") r.setAttribute("data-a11y-bg", "hc"); else r.removeAttribute("data-a11y-bg");
  const rm = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (o.motion === "always" || (rm && o.motion !== "never")) r.setAttribute("data-a11y-reduce", ""); else r.removeAttribute("data-a11y-reduce");
}
function a11yWrite(o) {
  try { localStorage.setItem("qqq.a11y", JSON.stringify(o)); } catch (e) {}
  a11yApply(o);
}
window.QA11Y = window.QA11Y || { read: a11yRead, apply: a11yApply, write: a11yWrite };
function uipLoad() {
  QA11Y.apply(QA11Y.read());
  try { const o = JSON.parse(uipRead(UIP_KEY) || "{}"); if (["auto", "light", "dark"].includes(o.theme)) UIP.theme = o.theme; if (PALS[o.pal]) UIP.pal = o.pal; if (o.merge === false) UIP.merge = false; if (o.tog === true) UIP.tog = true;  if (o.hrPpm >= 1 && o.hrPpm <= 50) UIP.hrPpm = +o.hrPpm; if ([3, 4, 5].includes(o.hrDec)) UIP.hrDec = o.hrDec; } catch (e) { /* corrupt value: defaults */ }
  setPal(UIP.pal);
}
function uipSave() {
  const o = { theme: UIP.theme, pal: UIP.pal, merge: UIP.merge };
  if (UIP.tog) o.tog = true;
  if (UIP.hrPpm !== 5) o.hrPpm = UIP.hrPpm; if (UIP.hrDec !== 4) o.hrDec = UIP.hrDec;         // high resolution: only what differs from the defaults
  uipWrite(UIP_KEY, JSON.stringify(o));
}
function uipApply() {
  const r = document.documentElement;
  if (UIP.theme === "auto") r.removeAttribute("data-theme"); else r.setAttribute("data-theme", UIP.theme);
  const tf = document.getElementById("tframe");                // the Teoria (iframe) follows the theme too (it reads it itself when it loads; this is for a change while it is open)
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

// --- popup of the gear
function uipOpen(btn) {
  const old = Q("#uipset"); if (old) { old.remove(); return; }
  const d = document.createElement("div"); d.id = "uipset";
  d.innerHTML = `<div class="sm" style="font-weight:600;margin-bottom:6px">${I18N.t("settings.title")}</div>
    <div class="row"><span>${I18N.t("settings.language.label")}</span> <select id="uip-lang" title="${I18N.t("settings.language.title")}" aria-label="${I18N.t("settings.language.label")}"><option value="it">Italiano</option><option value="en">English</option></select></div>
    <div class="row">${I18N.t("settings.theme")} <select id="uip-th"><option value="auto">${I18N.t("settings.theme.auto")}</option><option value="light">${I18N.t("settings.theme.light")}</option><option value="dark">${I18N.t("settings.theme.dark")}</option></select></div>
    <div class="row"><span>${I18N.t("settings.font.label")}</span> <select id="uip-font"><option value="">${I18N.t("settings.font.default")}</option><option value="atkinson">${I18N.t("settings.font.atkinson")}</option><option value="dyslexic">${I18N.t("settings.font.dyslexic")}</option></select></div>
    <label class="row" style="align-items:flex-start;gap:6px"><input type="checkbox" id="uip-hc"> <span>${I18N.t("settings.hc.label")}</span></label>
    <div class="row">${I18N.t("settings.pal.label")} <select id="uip-pal" title="${I18N.t("settings.pal.title")}">${Object.keys(PALS).map(k => `<option value="${k}">${I18N.t(`settings.pal.${k}`)}</option>`).join("")}</select></div>
    <label class="row" style="align-items:flex-start;gap:6px"><input type="checkbox" id="uip-merge" ${UIP.merge ? "checked" : ""}> <span>${I18N.t("settings.merge")}</span></label>
    ${typeof E !== "undefined" && E.files && E.files.length && !(window.BANCO && BANCO.on()) ? `<label class="row" style="align-items:flex-start;gap:6px" title="${I18N.t("settings.together.title")}"><input type="checkbox" id="uip-tog" ${UIP.tog ? "checked" : ""}> <span>${I18N.t("settings.together")}</span></label>` : ""}
    ${(typeof E !== "undefined" && E.files && E.files.some(f => (window.HR && HR.isHr(f)) || f.kind === "ms2" || (f.ms2_exps && f.ms2_exps.length))) ? `
    <div class="row">${I18N.t("settings.libs.label")} <button id="uip-lib" title="${I18N.t("settings.libs.title")}">${I18N.t("settings.libs.button")}</button></div>` : ""}
    ${(typeof E !== "undefined" && E.files && E.files.some(f => window.HR && HR.isHr(f))) ? `
    <div class="row hr-only" title="${I18N.t("settings.hrPpm.title")}">${I18N.t("settings.hrPpm", { input: `<input type="number" id="uip-ppm" min="1" max="50" step="1" value="${UIP.hrPpm}" style="width:56px">` })}</div>
    <label class="row hr-only" style="align-items:flex-start;gap:6px" title="${I18N.t("settings.cont.title")}"><input type="checkbox" id="uip-cont" ${window.LISTE && LISTE.isOn() ? "checked" : ""}> <span>${I18N.t("settings.cont")}</span></label>
    <div class="row hr-only">${I18N.t("settings.lab.label")} <button id="uip-lab" type="button">${I18N.t("settings.lab.button")}</button></div>
    <div class="row hr-only" title="${I18N.t("settings.hrDec.title")}">${I18N.t("settings.hrDec")} <input type="number" id="uip-hdec" min="3" max="5" step="1" value="${UIP.hrDec}" style="width:44px"></div>
    ` : ""}
    ${window.DESKUPD ? DESKUPD.row() : ""}
    ${!(window.BANCO && BANCO.on()) ? `
    <div class="uip-tour-sec" style="display:flex;gap:8px;align-items:center;justify-content:space-between;margin:6px 0;margin-top:8px;border-top:1px solid var(--line);padding-top:8px"><span>${I18N.t("tour.impost.titolo")}</span> <button id="uip-tour" type="button">${I18N.t(typeof E !== "undefined" && E.files && E.files.length ? "tour.impost.rifai" : "tour.impost.esempio")}</button></div>
    ` : ""}
    `;
  document.body.appendChild(d);
  if (window.DESKUPD) DESKUPD.bind(d);
  const uipLang = d.querySelector("#uip-lang"); uipLang.value = I18N.lang;   // the label is always bilingual: whoever cannot read the current language finds it
  uipLang.onchange = e => I18N.set(e.target.value);                         // remembered in qqq.lang, then the page reloads (open files come back with the session)
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
  d.querySelector("#uip-merge").onchange = e => {            // spectra are asked again to the server with / without the merge
    UIP.merge = e.target.checked; uipSave(); CACHE.clear(); SC.m.clear(); SC.n.clear();
    if (typeof redrawAll === "function") redrawAll();
  };
  const uipTour = d.querySelector("#uip-tour");
  if (uipTour) uipTour.onclick = () => {
    d.remove();
    if (window.TOUR) {
      if (typeof E !== "undefined" && E.files && E.files.length) TOUR.inizia(1, 0);
      else TOUR.caricaEsempio();
    }
  };
  const uipCont = d.querySelector("#uip-cont"); if (uipCont) uipCont.onchange = e => { if (window.LISTE) LISTE.setOn(e.target.checked); };
  const uipLab = d.querySelector("#uip-lab"); if (uipLab) uipLab.onclick = () => { d.remove(); if (window.LISTE) LISTE.openLab(); };
  const uipTog = d.querySelector("#uip-tog"); if (uipTog) uipTog.onchange = e => { if (typeof setTogether === "function") setTogether(e.target.checked); };
  const uipLib = d.querySelector("#uip-lib"); if (uipLib) uipLib.onclick = () => { d.remove(); if (window.LIB) LIB.open(); };
  const hrChanged = () => {
    uipSave(); CACHE.clear(); SC.m.clear(); SC.n.clear();
    if (typeof renderFileList === "function" && E.files.length) renderFileList();
    if (typeof redrawAll === "function") redrawAll();
  };
  const uipPpm = d.querySelector("#uip-ppm"); if (uipPpm) uipPpm.onchange = e => { UIP.hrPpm = Math.min(50, Math.max(1, +e.target.value || 5)); e.target.value = UIP.hrPpm; hrChanged(); };
  const uipHdec = d.querySelector("#uip-hdec"); if (uipHdec) uipHdec.onchange = e => { UIP.hrDec = Math.min(5, Math.max(3, Math.round(+e.target.value || 4))); e.target.value = UIP.hrDec; hrChanged(); };
  d.querySelector("#uip-th").onchange = e => { UIP.theme = e.target.value; uipSave(); uipApply(); };
  d.querySelector("#uip-pal").onchange = e => {              // applied at once: file colours, lists, legends, every graph
    setPal(e.target.value); uipSave();
    if (typeof paintFiles === "function" && typeof E !== "undefined") { paintFiles(); if (E.files.length) { renderFileList(); } }
    uipApply();
  };
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.contains(e.target) && e.target !== btn && !btn.contains(e.target)) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
}

uipLoad();
(() => {
  const nav = Q("#nav"), b = document.createElement("button"); b.id = "np-set"; b.className = "ib gear"; b.title = I18N.t("settings.gear.title"); b.setAttribute("aria-label", I18N.t("settings.title"));
  b.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>';
  nav.insertAdjacentElement("afterend", b); b.onclick = e => { e.stopPropagation(); uipOpen(b); };
  uipApply();
})();
