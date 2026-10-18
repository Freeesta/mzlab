// @ts-check
"use strict";
// The toolbar of the header is part of the first paint: its scripts are read here, in order and before the page is drawn, and barra.js
// draws it as soon as it is loaded (BARRA.primo); carica.js then skips them. Not on a smartphone, which has no toolbar.
const fp = /** @type {any} */ (window);
if (!fp.QQQ_PHONE) {
  fp.QQ_FIRST_PAINT = true;
  ["features", "icons-modi", "barra"].forEach(n => document.write('<script src="static/' + n + '.js"><\/script>'));
}
