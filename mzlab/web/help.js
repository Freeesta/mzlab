// @ts-strict
"use strict";
// Context help: every <button class="hq" data-help="key"> opens a short explanation (HELP[key]) next to it.
// Texts for students, in the catalogs (keys help.<key>.title / .body). To add one: a key here and a "?" button where it is needed. Classic script.
const HELP = {
  "start": [I18N.t("help.start.title"), I18N.t("help.start.body")],
  "filegrandi": [I18N.t("help.filegrandi.title"), I18N.t("help.filegrandi.body")],
  "files": [I18N.t("help.files.title"), I18N.t("help.files.body")],
  "toolbar": [I18N.t("help.toolbar.title"), I18N.t("help.toolbar.body")],
  "nav": [I18N.t("help.nav.title"), I18N.t("help.nav.body")],
  "tools": [I18N.t("help.tools.title"), I18N.t("help.tools.body")],
  "prop": [I18N.t("help.prop.title"), I18N.t("help.prop.body")],
  "scorrimento": [I18N.t("help.scorrimento.title"), I18N.t("help.scorrimento.body")],
  "pnl-chrom": [I18N.t("help.pnlChrom.title"), I18N.t("help.pnlChrom.body")],
  "pnl-spec": [I18N.t("help.pnlSpec.title"), I18N.t("help.pnlSpec.body")],
  "pnl-xic": [I18N.t("help.pnlXic.title"), I18N.t("help.pnlXic.body")],
  "pnl-mrm": [I18N.t("help.pnlMrm.title"), I18N.t("help.pnlMrm.body")],
  "pnl-map": [I18N.t("help.pnlMap.title"), I18N.t("help.pnlMap.body")],
  "mappa": [I18N.t("help.mappa.title"), I18N.t("help.mappa.body")],
  "origine": [I18N.t("help.origine.title"), I18N.t("help.origine.body")],
  "liste": [I18N.t("help.liste.title"), I18N.t("help.liste.body")],
  "losses": [I18N.t("help.losses.title"), I18N.t("help.losses.body")],
  "adducts": [I18N.t("help.adducts.title"), I18N.t("help.adducts.body")],
  "language": [I18N.t("help.language.title"), I18N.t("help.language.body")],
  "header": ["", I18N.t("help.header.body") + (window.__TAURI__ ? "" : I18N.t("help.header.app"))],      // the download link is pointless inside the desktop app
};

(() => {
  const pop = document.createElement("div"); pop.id = "helppop"; pop.hidden = true; document.body.appendChild(pop);
  let cur = null;
  const close = () => { pop.hidden = true; cur = null; };
  document.addEventListener("click", e => {
    const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest(".hq[data-help]"));
    if (!b) { if (!pop.contains(/** @type {Node} */ (e.target))) close(); return; }
    e.preventDefault(); e.stopPropagation();
    if (cur === b) return close();
    const key = /** @type {string} */ (b.dataset.help), mk = /^modo-(full|ms2|mrm)$/.exec(key);
    const h = mk && typeof QMODI !== "undefined" ? [QMODI.M[ESP.didattica(mk[1])].name, QMODI.html(ESP.didattica(mk[1]))] : HELP[key]; if (!h) return;
    pop.classList.toggle("wide", !!mk); pop.classList.remove("guide");
    pop.innerHTML = `<div class="hp-t"><b>${h[0]}</b><button class="x" title="${I18N.t("help.close")}">&times;</button></div><div>${h[1]}</div>`;      // no title for the "Informazioni" box (key header)
    pop.querySelector(".x").onclick = close;
    pop.hidden = false; cur = b;
    const r = b.getBoundingClientRect(), w = pop.offsetWidth, hgt = pop.offsetHeight;
    let left = Math.min(innerWidth - w - 8, Math.max(8, r.left - 12)), top = r.bottom + 6;
    if (top + hgt > innerHeight - 8) top = Math.max(8, r.top - hgt - 6);
    pop.style.left = left + "px"; pop.style.top = top + "px";
  }, true);
  document.addEventListener("keydown", e => { if (e.key === "Escape") close(); });
  addEventListener("scroll", e => { if (!pop.contains(/** @type {Node} */ (e.target))) close(); }, true);
})();
const shortHelp = key => { const h = HELP[key]; if (!h) return ""; const t = h[1].replace(/<br>.*/s, "").replace(/<[^>]+>/g, "").replace(/&nbsp;/g, " ").trim(); return t.length > 190 ? t.slice(0, t.lastIndexOf(" ", 187)) + "…" : t; };
// the general guide, now the first chapter of the Teoria ("Come si usa QqQ lab", teoria/00-uso.html): all the explanations in one place, by topic
function guideHtml() {
  const sec = (title, keys) => `<details open><summary><b>${title}</b></summary>${keys.filter(k => HELP[k]).map(k => `<div class="hp-s"><b>${HELP[k][0]}</b><div>${HELP[k][1]}</div></div>`).join("")}</details>`;
  let modes = "";
  try { modes = `<details><summary><b>${I18N.t("help.guide.modes")}</b></summary>${ESP.ids.map(t => QMODI.html(ESP.didattica(t))).join("")}</details>`; } catch (e) { /* the modes text is optional */ }
  const credits = `<details><summary><b>${I18N.t("help.guide.credits.title")}</b></summary><div class="hp-s">${I18N.t("help.guide.credits.body")}</div></details>`;
  return sec(I18N.t("help.guide.open"), ["start", "filegrandi", "files"]) + sec(I18N.t("help.guide.charts"), ["toolbar", "nav", "scorrimento", "pnl-chrom", "pnl-spec", "pnl-xic", "pnl-mrm", "pnl-map", "mappa"]) + sec(I18N.t("help.guide.method"), ["tools", "prop", "language"]) + modes + credits;
}
