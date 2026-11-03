// @ts-check
// The one switch of the public site: what is shown to visitors. The value is off for the live site; the desktop app and its releases
// stay in the code and come back by turning it on. Classic script, loaded in the head. Tests turn it on with ?sito=app (or ?sito=tutto).
(function () {
  const w = /** @type {any} */ (window);
  if (!w.SITO_PUBBLICO) {
    const SITO = { scaricaApp: false };
    const q = /[?&]sito=([\w,]+)/.exec(location.search);
    if (q) SITO.scaricaApp = /tutto|app/.test(q[1]);
    w.SITO_PUBBLICO = Object.freeze(SITO);
  }
  // the retired Teoria addresses (#/theory, #/practice) go back to the main page before the shell reads the hash
  if (/^#\/(theory|practice)(\/|$)/.test(location.hash)) history.replaceState(null, "", location.pathname + location.search);
})();
