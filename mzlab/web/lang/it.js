// @ts-strict
// Interface catalog (it): the texts live in lang/it/<area>.js (key prefix -> file, same file names in it/ and en/); this script loads
// them in order, in one write so the browser fetches them together. Tests and tools read the folder (tests/test_lang_parti.py checks the list).
(function () {
  const base = /** @type {HTMLScriptElement} */ (document.currentScript).src.replace(/[^/]*$/, "") + "it/";
  document.write(["altre-a11y-fair", "altre-feat-lst", "altre-lv-predef", "altre-press-xic", "altre-xicbtn-zoom", "bn", "calcqui", "canali", "cap", "comandi", "comp", "conv", "ctl", "ctx", "disegno", "el", "errori", "grafo-msn", "librerie", "load", "loss", "m3d", "mappe", "masscal", "method", "normative", "og", "ref", "settings", "tour", "valida", "xiclens"].map(n => '<script src="' + base + n + '.js"><\/script>').join(""));
})();
