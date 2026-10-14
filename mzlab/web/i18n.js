// @ts-check
// @ts-strict
"use strict";
// Interface languages (Italian and English). Classic script, loaded FIRST by index.html (before the catalogs lang/it.js and lang/en.js).
//  - Code uses semantic keys ("load.drop.title"); the texts live in the two catalogs, loaded as plain <script> tags (no fetch: CSP, offline).
//  - Italian is the fallback language: a key missing in English shows the Italian text; a key missing everywhere shows the key itself.
//  - Default language: the first of the browser languages that is Italian or English (none -> English). The gear (settings.js) can change it
//    (stored in localStorage "mzlab.lang"); ?lang=it|en in the address forces the language for that tab only.
//  - index.html decides the language in a tiny inline script at the top of the head (same rules as detect below), sets I18N_LANG, <html lang>
//    and the title, and loads ONLY the catalog of that language.
//  - Messages use a reduced ICU format: {name}, {n, number}, {n, plural, one {# file} other {# files}}. One sentence = one key: never glue
//    translated pieces together (genders and articles differ between the languages).
//  - Numbers that are DATA (m/z, RT, intensity, ppm, areas) go through I18N.num: always a decimal point, in both languages.
//  - What the user writes (file names, annotations, labels) is never translated.
(function (root) {
  const LANGS = ["it", "en"], KEY = "mzlab.lang";   // "qqq.lang" (old QqQ-lab site, same origin) is ignored on purpose
  const cat = { it: {}, en: {} }, missing = new Set();
  let lang = "it";

  const store = {
    get() { try { return localStorage.getItem(KEY); } catch (_) { return null; } },
    set(v) { try { localStorage.setItem(KEY, v); } catch (_) { /* storage blocked: the choice lasts until the page is closed */ } },
  };

  // language chosen at start: ?lang= (this tab only) > stored choice > the first browser language that is Italian or English (default English)
  // keep in step with the inline script at the top of the head of index.html
  function detect(search, stored, browser) {
    const q = /[?&]lang=(it|en)(?:&|#|$)/.exec(search || "");
    if (q) return q[1];
    if (LANGS.includes(stored)) return stored;
    for (const b of [].concat(browser || [])) {
      const l = String(b).toLowerCase();
      if (/^it(-|_|$)/.test(l)) return "it";
      if (/^en(-|_|$)/.test(l)) return "en";
    }
    return "en";
  }

  // ---- reduced ICU: {name} {name, number} {name, plural, =0 {..} one {..} other {..}}  (# = the number inside a plural branch)
  function matchBrace(s, i) {                      // index of the "}" closing the "{" at i
    let d = 0;
    for (let j = i; j < s.length; j++) { if (s[j] === "{") d++; else if (s[j] === "}" && --d === 0) return j; }
    return -1;
  }
  function parseBranches(s) {                      // "one {a} other {b}" -> {one:"a", other:"b"}
    const out = {}; let i = 0;
    while (i < s.length) {
      while (i < s.length && /\s/.test(s[i])) i++;
      if (i >= s.length) break;
      let j = i; while (j < s.length && !/[\s{]/.test(s[j])) j++;
      const name = s.slice(i, j); while (j < s.length && /\s/.test(s[j])) j++;
      if (s[j] !== "{") break;
      const e = matchBrace(s, j); if (e < 0) break;
      out[name] = s.slice(j + 1, e); i = e + 1;
    }
    return out;
  }
  // THE number format of the whole program (not the Teoria): decimal point in every language, no thousands separator (a comma would be read as a decimal one)
  const NF = dec => new Intl.NumberFormat("en-US", dec === undefined ? { useGrouping: false } : { useGrouping: false, minimumFractionDigits: dec, maximumFractionDigits: dec });
  function format(msg, params, lg) {
    params = params || {}; let out = "", i = 0;
    while (i < msg.length) {
      const o = msg.indexOf("{", i);
      if (o < 0) { out += msg.slice(i); break; }
      out += msg.slice(i, o);
      const e = matchBrace(msg, o);
      if (e < 0) { out += msg.slice(o); break; }
      const inner = msg.slice(o + 1, e), c1 = inner.indexOf(",");
      const name = (c1 < 0 ? inner : inner.slice(0, c1)).trim(), v = params[name];
      if (c1 < 0) out += v === undefined ? "{" + name + "}" : String(v);
      else {
        const rest = inner.slice(c1 + 1), c2 = rest.indexOf(","), type = (c2 < 0 ? rest : rest.slice(0, c2)).trim();
        if (type === "number") out += v === undefined ? "{" + name + "}" : NF().format(+v);
        else if (type === "plural" && c2 >= 0) {
          const br = parseBranches(rest.slice(c2 + 1)), n = +v;
          const pick = br["=" + n] !== undefined ? br["=" + n] : (br[new Intl.PluralRules(lg === "it" ? "it" : "en").select(n)] ?? br.other ?? "");
          out += format(pick.replace(/#/g, NF().format(n)), params, lg);
        } else out += "{" + inner + "}";
      }
      i = e + 1;
    }
    return out;
  }

  // {app} is always the name of appname.js, even when the caller passes no parameters (an explicit `app` wins)
  function withApp(m, params) {
    const name = /** @type {any} */ (root).APP_NAME;
    return m.indexOf("{app") < 0 || !name || (params && params.app !== undefined) ? params : Object.assign({ app: name }, params);
  }
  function t(key, params) {
    let m = cat[lang][key];
    if (m === undefined) m = cat.it[key];
    if (m === undefined) { if (!missing.has(key)) { missing.add(key); try { console.warn("i18n: missing key " + key); } catch (_) { /* no console */ } } return key; }
    return params === undefined && m.indexOf("{") < 0 ? m : format(m, withApp(m, params), lang);
  }
  function add(lg, dict) { Object.assign(cat[lg] || (cat[lg] = {}), dict); }
  // fixed decimals, always with the point (file sizes inside prose: "1.5 MB"); data values use num()
  const fix = (x, dec) => NF(dec).format(+x);
  const num = (x, dec) => (x === null || x === undefined || Number.isNaN(+x)) ? "" : (dec === undefined ? String(+x) : (+x).toFixed(dec));

  // an answer {"error", "error_key", "params"} of the server -> Error with the message in the language of the page (e.key = the key)
  function err(j) {
    if (!j || !j.error_key) return new Error(j && j.error ? j.error : String(j));
    return Object.assign(new Error(t(j.error_key, j.params || {})), { key: j.error_key });
  }

  // static HTML: the Italian text stays in the page (works before JS); tools/controlla_i18n.py checks that it matches it.js
  const ATTRS = [["data-i18n-title", "title"], ["data-i18n-placeholder", "placeholder"], ["data-i18n-aria-label", "aria-label"]];
  function apply(rootEl) {
    const r = rootEl || (typeof document !== "undefined" ? document : null); if (!r) return;
    const P = { app: (typeof window !== "undefined" && window.APP_NAME) || "" };      // {app} = the visible name of the program (appname.js)
    r.querySelectorAll("[data-i18n]").forEach(el => { el.textContent = t(el.getAttribute("data-i18n"), P); });
    r.querySelectorAll("[data-i18n-html]").forEach(el => { el.innerHTML = t(el.getAttribute("data-i18n-html"), P); });   // catalog text with b/i/u/code/sub/sup/a/br (trusted)
    ATTRS.forEach(([a, target]) => r.querySelectorAll("[" + a + "]").forEach(el => el.setAttribute(target, t(el.getAttribute(a), P))));
  }
  function setLangAttr() { if (typeof document !== "undefined") document.documentElement.setAttribute("lang", lang); }

  // explicit choice (gear): remembered, then the page reloads (open files come back with the session; the ?lang= of the address is dropped
  // so that the choice is not overridden)
  function set(lg) {
    if (!LANGS.includes(lg)) return;
    store.set(lg);
    if (typeof location === "undefined") { lang = lg; return; }
    try { const u = new URL(location.href); u.searchParams.delete("lang"); history.replaceState(null, "", u.toString()); } catch (_) { /* old browser */ }
    location.reload();
  }

  const loc = typeof location !== "undefined" ? location : {}, nav = typeof navigator !== "undefined" ? navigator : {};
  lang = LANGS.includes(root.I18N_LANG) ? root.I18N_LANG : detect(loc.search, store.get(), nav.languages && nav.languages.length ? Array.from(nav.languages) : nav.language);   // I18N_LANG: set before this script by pages that exist in one language only (the Teoria)
  setLangAttr();
  if (typeof document !== "undefined") {
    const go = () => apply(document);
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", go); else go();
  }

  const I18N = { add, t, apply, set, num, fix, err, detect, format, get lang() { return lang; }, LANGS, KEY, missing };
  /** @type {any} */ (root).I18N = I18N;
  if (typeof module !== "undefined" && module.exports) module.exports = I18N;
})(typeof window !== "undefined" ? window : globalThis);

// a saved setting read from localStorage: always a plain object (a missing, old, damaged or foreign value gives an empty one), never an error
if (typeof window !== "undefined") window.lsObj = function (key) {
  try { var o = JSON.parse(localStorage.getItem(key) || "{}"); return o && typeof o === "object" && !Array.isArray(o) ? o : {}; } catch (e) { return {}; }
};
