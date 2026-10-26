// @ts-check
// The one switch of the public site: what is shown to visitors. Both values are off for the live site; the Teoria, the desktop app
// and its releases stay in the code and come back by turning a value on. Classic script, loaded in the head (before the address
// check of index.html) and again before the i18n pass, when the markup exists: the second run adds the entrances of the Teoria.
// Tests turn the values on with ?sito=teoria,app (or ?sito=tutto) in the address.
(function () {
  const w = /** @type {any} */ (window);
  if (!w.SITO_PUBBLICO) {
    const SITO = { scaricaApp: false, teoria: false };
    const q = /[?&]sito=([\w,]+)/.exec(location.search);
    if (q) { SITO.teoria = /tutto|teoria/.test(q[1]); SITO.scaricaApp = /tutto|app/.test(q[1]); }
    w.SITO_PUBBLICO = Object.freeze(SITO);
  }
  // a typed #/theory or #/practice must not reach the Teoria: the address goes back to the main page before index.html reads it
  if (!w.SITO_PUBBLICO.teoria && /^#\/(theory|practice)(\/|$)/.test(location.hash)) history.replaceState(null, "", location.pathname + location.search);
  if (!w.SITO_PUBBLICO.teoria || document.getElementById("v-theory") === null || document.querySelector('#nav [data-v="theory"]')) return;
  // with the Teoria on, its entrances are added here: the served page carries no link to it while it is off
  const nav = document.getElementById("nav"), sec = document.getElementById("v-theory"), ph = document.querySelector("#phone .phw");
  if (nav) nav.insertAdjacentHTML("beforeend", '<button data-v="theory" data-i18n="nav.theory" data-i18n-title="nav.theory.title" title="Teoria: TP, fotocatalisi, LC, ESI, quadrupolo, triplo quadrupolo">Teoria</button>');
  if (sec) sec.innerHTML = '<iframe id="tframe" data-i18n-aria-label="nav.theory" aria-label="Teoria"></iframe>';
  if (ph) ph.insertAdjacentHTML("afterend", '<a class="phb" href="static/teoria/index.html" data-i18n="phone.go">Vai alla Teoria</a>');
})();
