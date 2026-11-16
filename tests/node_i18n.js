// Loaded into every `node` that the tests start (NODE_OPTIONS in conftest.py): the pure functions of the page use I18N.t(), here with the English catalog.
globalThis.I18N_LANG = "en";
const fs = require("fs"), path = require("path");
const web = path.join(__dirname, "..", "mzlab", "web");
require(path.join(web, "i18n.js"));
// lang/en.js is the browser loader; the texts are the area files of lang/en/
for (const f of fs.readdirSync(path.join(web, "lang", "en")).sort()) new Function("I18N", fs.readFileSync(path.join(web, "lang", "en", f), "utf8"))(globalThis.I18N);
