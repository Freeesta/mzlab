// @ts-check
// The one switch of the public site: what is shown to visitors. The Teoria is always open; the desktop app and its releases
// stay in the code and come back by turning scaricaApp on. Classic script, loaded in the head (before the address
// check of index.html), right after <nav> (the tab of the Teoria goes in before the gear is drawn) and again before the i18n pass, when the rest of the markup exists.
// Tests turn the app on with ?sito=app (or ?sito=tutto) and the Teoria off with ?sito=senza-teoria.
(function () {
  const w = /** @type {any} */ (window);
  if (!w.SITO_PUBBLICO) {
    const SITO = { scaricaApp: false, teoria: true };
    const q = /[?&]sito=([\w,-]+)/.exec(location.search);
    if (q) { SITO.teoria = !/senza-teoria/.test(q[1]); SITO.scaricaApp = /tutto|app/.test(q[1]); }
    w.SITO_PUBBLICO = Object.freeze(SITO);
  }
  // a typed #/theory or #/practice must not reach the Teoria: the address goes back to the main page before index.html reads it
  if (!w.SITO_PUBBLICO.teoria && /^#\/(theory|practice)(\/|$)/.test(location.hash)) history.replaceState(null, "", location.pathname + location.search);
  if (!w.SITO_PUBBLICO.teoria) return;
  // its entrances are added here: the served page carries no link to it while it is off. The tab goes in right after <nav> (index.html
  // loads this file there too) and is translated at once, so the gear that follows it is where it stays at first paint
  const nav = () => {
    const n = document.getElementById("nav");
    if (!n || n.querySelector('[data-v="theory"]')) return;
    n.insertAdjacentHTML("beforeend", '<button data-v="theory" data-i18n="nav.theory" data-i18n-title="nav.theory.title" title="Teoria: TP, fotocatalisi, LC, ESI, quadrupolo, triplo quadrupolo">Teoria</button>');
    if (w.I18N && w.I18N.apply) w.I18N.apply(n);
  };
  nav();
  const sec = document.getElementById("v-theory"), ph = document.querySelector("#phone .phw");
  if (sec && !sec.firstChild) sec.innerHTML = '<iframe id="tframe" data-i18n-aria-label="nav.theory" aria-label="Teoria"></iframe>';
  if (ph && !document.querySelector("a.phb[href$='teoria/index.html']")) ph.insertAdjacentHTML("afterend", '<a class="phb" href="static/teoria/index.html" data-i18n="phone.go">Vai alla Teoria</a>');
})();
