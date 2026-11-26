// @ts-strict
// Interface catalog (en): the texts live in lang/en/<area>.js (key prefix -> file, same file names in it/ and en/); this script loads
// them in order, in one write so the browser fetches them together. Tests and tools read the folder (tests/test_lang_parti.py checks the list).
(function () {
  const base = /** @type {HTMLScriptElement} */ (document.currentScript).src.replace(/[^/]*$/, "") + "en/";
  document.write(["bn", "calcqui", "cap", "channels", "commands", "comp", "conv", "ctl", "ctx", "draw-check", "draw-cip", "draw-evidence", "draw-substructure", "draw", "el", "errors-a-m", "errors-file", "errors-flow", "errors-llm", "errors-n-w", "graph-msn", "groups", "libraries", "load", "loss", "m3d", "maps", "masscal", "method", "more-a11y-fair", "more-feat-lst", "more-lv-defaults", "more-press-xic", "more-xicbtn-zoom", "normative", "og", "peaktable", "pick", "ref", "settings", "spectrum-compare", "tour", "user-list", "validate", "xiclens"].map(n => '<script src="' + base + n + '.js"><\/script>').join(""));
})();
