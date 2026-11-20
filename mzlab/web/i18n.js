// @ts-check
// @ts-strict
"use strict";
// Interface texts (English only). Classic script, loaded FIRST by index.html (before the catalog lang/en.js).
//  - Code uses semantic keys ("load.drop.title"); the texts live in the catalog lang/en/<area>.js, loaded as plain <script> tags (no fetch: CSP, offline).
//    The catalog keeps the key structure of a multi-language catalog so that a translation could be added again; today there is exactly one language.
//  - A key missing from the catalog shows the key itself (and a console warning).
//  - A language stored by an older version ("mzlab.lang") is ignored on purpose: the program always starts in English.
//  - index.html sets I18N_LANG = "en", <html lang> and the title in a tiny inline script at the top of the head.
//  - Messages use a reduced ICU format: {name}, {n, number}, {n, plural, one {# file} other {# files}}. One sentence = one key: never glue
//    translated pieces together (genders and articles differ between languages).
//  - Numbers that are DATA (m/z, RT, intensity, ppm, areas) go through I18N.num: always a decimal point.
//  - What the user writes (file names, annotations, labels) is never translated.
//  - On the site the catalog comes in two parts (tools/build_site.py bundle_lang): the keys of the first view before the first paint, the rest
//    right after (carica.js loads it before any other script). Until then a key of the rest gives "" (never the bare key) and is noted in
//    `late` (the e2e tests require it empty); static HTML keeps its text until the rest re-applies it.
(function (root) {
  const LANGS = ["en"], lang = "en";
  const cat = { en: {} }, missing = new Set(), late = new Set(), held = new Set();
  let rest = "";      // rest: path of the second part of the catalog while it is still to come

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
  // THE number format of the whole program: decimal point in every language, no thousands separator (a comma would be read as a decimal one).
  // One formatter per number of decimals: building an Intl.NumberFormat costs ~25 µs, formatting with one ~0.5 µs, and the progress rows of a loading format thousands of numbers
  const NFS = new Map();
  const NF = dec => { let f = NFS.get(dec); if (!f) { f = new Intl.NumberFormat("en-US", dec === undefined ? { useGrouping: false } : { useGrouping: false, minimumFractionDigits: dec, maximumFractionDigits: dec }); NFS.set(dec, f); } return f; };
  function format(msg, params) {
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
          const pick = br["=" + n] !== undefined ? br["=" + n] : (br[new Intl.PluralRules("en").select(n)] ?? br.other ?? "");
          out += format(pick.replace(/#/g, NF().format(n)), params);
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
    const m = cat.en[key];
    if (m === undefined && rest) { if (!late.has(key)) { late.add(key); try { console.error("i18n: key asked before the rest of the catalog: " + key); } catch (_) { /* no console */ } } return ""; }
    if (m === undefined) { if (!missing.has(key)) { missing.add(key); try { console.warn("i18n: missing key " + key); } catch (_) { /* no console */ } } return key; }
    return params === undefined && m.indexOf("{") < 0 ? m : format(m, withApp(m, params));
  }
  function add(lg, dict) { if (lg === "en") Object.assign(cat.en, dict); }   // a catalog of another language is not loaded
  // first part of the catalog: the rest (a path under static/) is still to come; the rest calls done() at its end, then apply runs again
  // on the elements it had to leave as they were
  function awaitRest(path) { rest = path; }
  function done() {
    rest = "";
    const els = [...held]; held.clear();
    els.forEach(el => { if (el.isConnected) fill(el); });
  }
  // fixed decimals, always with the point (file sizes inside prose: "1.5 MB"); data values use num()
  const fix = (x, dec) => NF(dec).format(+x);
  // table cells: "–" for a missing or non-finite value, else fixed decimals; sci() switches to 2-digit scientific notation for >= 1e4 or < 0.01
  const dash = (v, d) => (v == null || !isFinite(v) ? "–" : Number(v).toFixed(d));
  const sci = (v, d = 2) => (v == null || !isFinite(v) ? "–" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? Number(v).toExponential(2) : Number(v).toFixed(d));
  // areas and heights, one form everywhere (numfmt.rs): 7.841E+04, point, no thousands separator, signed two-digit exponent; "" when not a number
  const sciE = (x, d = 3) => { if (x === null || x === undefined || x === "" || !isFinite(+x)) return ""; const [m, e] = (+x === 0 ? 0 : +x).toExponential(d).split("e"); return `${m}E${+e < 0 ? "-" : "+"}${String(Math.abs(+e)).padStart(2, "0")}`; };
  const num = (x, dec) => (x === null || x === undefined || Number.isNaN(+x)) ? "" : (dec === undefined ? String(+x) : (+x).toFixed(dec));

  // the texts written for the desktop app, where the limit is the memory of the computer and not of the browser
  const NATIVE = new Set(["error.memoryBudget.native", "error.memoryBudgetFase.native", "error.memoryBudgetFase.indexPeaks.native", "error.memoryBudgetFase.detect.native", "error.memoryBudgetFase.read.native", "error.memoryBudgetFase.msnGraph.native", "error.memoryBudgetFase.openFiles.native", "error.memoryBudgetFase.whole.native", "error.badIndex.native", "error.generic.native", "err.engine.indexCrash.native", "err.file.nativeStopped.native"]);
  // an answer {"error", "error_key", "params"} of the server -> Error with the message in the language of the page (e.key = the key)
  function err(j) {
    if (!j || !j.error_key) return new Error(j && j.error ? j.error : String(j));
    // a refusal of the memory contract names the phase, the estimate and the budget: its own text says by how much the file overshoots
    // (a phase with a text of its own, `error.memoryBudgetFase.<id>`, names its real limit)
    // in the desktop app the limit is the memory of the computer, not of the browser: a key with a `.native` text uses that one (error_text.rs, same choice)
    const nat = key => (typeof window !== "undefined" && window.MZLAB_DESKTOP && NATIVE.has(key + ".native") ? key + ".native" : key), FP = "error.memoryBudgetFase.";
    const fase = j.error_key === "error.memoryBudget" && j.params && j.params.fase, own = fase && cat.en[FP + fase] && nat(FP + fase).slice(FP.length);
    const k = nat(fase ? "error.memoryBudgetFase" : j.error_key);
    // the phase has a name for people when the catalog has one (`memory.phase.<id>`)
    const ph = j.params && j.params.fase && cat.en["memory.phase." + j.params.fase] ? t(`memory.phase.${j.params.fase}`) : "";
    const params = ph ? Object.assign({}, j.params, { fase: ph }) : j.params;
    let msg = own ? t(`error.memoryBudgetFase.${own}`, params) : t(k, params || {});
    // a key with no text in the catalogs never reaches the student: the generic sentence carries the engine's own detail
    if (msg === k || msg === "") msg = t("error.generic", { detail: (j.params && j.params.detail) || j.error || "–" });
    return Object.assign(new Error(msg), { key: j.error_key, params: j.params });
  }

  // static HTML: the English text stays in the page (works before JS); tools/controlla_i18n.py checks that it matches lang/en
  const ATTRS = [["data-i18n-title", "title"], ["data-i18n-placeholder", "placeholder"], ["data-i18n-aria-label", "aria-label"]];
  function apply(rootEl) {
    const r = rootEl || (typeof document !== "undefined" ? document : null); if (!r) return;
    if (rest) { applyKnown(r); return; }
    // {app} = the visible name of the program (appname.js)
    const P = { app: (typeof window !== "undefined" && window.APP_NAME) || "" };
    r.querySelectorAll("[data-i18n]").forEach(el => putText(el, t(el.getAttribute("data-i18n"), P)));
    // catalog text with b/i/u/code/sub/sup/a/br (trusted)
    r.querySelectorAll("[data-i18n-html]").forEach(el => putHtml(el, t(el.getAttribute("data-i18n-html"), P)));
    ATTRS.forEach(([a, target]) => r.querySelectorAll("[" + a + "]").forEach(el => el.setAttribute(target, t(el.getAttribute(a), P))));
    marked(r);
  }
  // while the rest is to come: an element whose key is not here yet keeps its text (the English of the page) and waits for done()
  const ALL = "[data-i18n],[data-i18n-html]," + ATTRS.map(([a]) => "[" + a + "]").join(",");
  function applyKnown(r) { r.querySelectorAll(ALL).forEach(el => { if (!fill(el)) held.add(el); }); marked(r); }
  // the keys of one element; false when one of them is not in the catalog yet (that one is left as it is)
  function fill(el) {
    const P = { app: (typeof window !== "undefined" && window.APP_NAME) || "" }, has = k => !rest || cat.en[k] !== undefined;
    let all = true;
    const put = (a, f) => { const k = el.getAttribute(a); if (k === null) return; if (has(k)) f(t(k, P)); else all = false; };
    put("data-i18n", v => putText(el, v));
    put("data-i18n-html", v => putHtml(el, v));
    ATTRS.forEach(([a, target]) => put(a, v => el.setAttribute(target, v)));
    return all;
  }

  // Marked-up texts are written ONCE, in their final form: "m/z" (a quantity symbol) always in italics and MS2 with a real superscript
  // (never the Unicode characters). The writer marks what it writes before the first paint, and a node whose text did not change is not
  // rewritten (a bare textContent write would drop the <i> and draw the label upright for a frame). Text written by other code is marked by
  // the observer started with the first apply(document), in the same task as the write. Not in inputs, options, titles, scripts or an existing <i>/<sup>.
  const SKIP = /^(SCRIPT|STYLE|OPTION|TEXTAREA|INPUT|CANVAS|TITLE|SELECT)$/, MARK = /(m\/z|MS2)/;
  function mark(node) {
    if (typeof document === "undefined" || !node || node.nodeType !== 1) return;
    const w = document.createTreeWalker(node, NodeFilter.SHOW_TEXT, { acceptNode: n => { const p = n.parentElement;
      return MARK.test(n.nodeValue || "") && p && !SKIP.test(p.tagName) && !(p.tagName === "I" && n.nodeValue === "m/z") && p.tagName !== "SUP" ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT; } });
    /** @type {Text[]} */ const L = []; while (w.nextNode()) L.push(/** @type {Text} */ (w.currentNode));
    for (const n of L) {
      const f = document.createDocumentFragment();
      n.data.split(MARK).forEach(s => {
        if (s === "m/z") { const e = document.createElement("i"); e.textContent = "m/z"; f.appendChild(e); }
        else if (s === "MS2") { f.appendChild(document.createTextNode("MS")); const e = document.createElement("sup"); e.textContent = "2"; f.appendChild(e); }
        else if (s) f.appendChild(document.createTextNode(s));
      });
      n.replaceWith(f);
    }
  }
  function putText(el, v) { if (el.textContent !== v) el.textContent = v; mark(el); }
  // the last catalog html written in an element and what it became once marked: the same text over an untouched node is not written again
  const wrote = new WeakMap();
  function putHtml(el, v) { const w = wrote.get(el); if (w && w[0] === v && w[1] === el.innerHTML) return; el.innerHTML = v; mark(el); wrote.set(el, [v, el.innerHTML]); }
  let watching = false;
  // the whole page: its static texts (with or without a key) are marked, and the observer starts
  function marked(r) {
    if (typeof document === "undefined" || r !== document || !document.body) return;
    mark(document.body);
    if (watching) return;
    watching = true;
    let busy = false;
    new MutationObserver(ms => { if (busy) return; busy = true; try { for (const m of ms) { if (m.type === "characterData") { if (m.target.parentElement) mark(m.target.parentElement); } else m.addedNodes.forEach(a => { if (a.nodeType === 1) mark(a); else if (a.nodeType === 3 && a.parentElement) mark(a.parentElement); }); } } finally { busy = false; } })
      .observe(document.body, { childList: true, subtree: true, characterData: true });
  }
  if (typeof document !== "undefined") document.documentElement.setAttribute("lang", lang);

  if (typeof document !== "undefined") {
    const go = () => apply(document);
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", go); else go();
  }

  const I18N = { add, t, apply, num, fix, dash, sci, sciE, err, format, lang, LANGS, missing, late, awaitRest, done, get rest() { return rest; } };
  /** @type {any} */ (root).I18N = I18N;
  if (typeof module !== "undefined" && module.exports) module.exports = I18N;
})(typeof window !== "undefined" ? window : globalThis);

// a saved setting read from localStorage: always a plain object (a missing, old, damaged or foreign value gives an empty one), never an error
if (typeof window !== "undefined") window.lsObj = function (key) {
  try { var o = JSON.parse(localStorage.getItem(key) || "{}"); return o && typeof o === "object" && !Array.isArray(o) ? o : {}; } catch (e) { return {}; }
};
