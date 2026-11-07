// @ts-check
// @ts-strict
// Safe reading of JSON files that students exchange (.mzflow, notebook, lists): size and depth caps before parsing, no key that can
// reach the prototype, and a copy that keeps only known keys of the same type. Content is data and is never evaluated.
// Single-pass scanners over JSON text (outside strings, no recursion), used by FILESICURI.parse for its caps.
const JSONSCAN = (() => {
  /** Deepest nesting of [] and {} outside strings, found without recursion (a deep file must not exhaust the stack).
   * @type {(text: string) => number} */
  function depthOf(text) {
    let d = 0, max = 0, inStr = false;
    for (let i = 0; i < text.length; i++) {
      const c = text.charCodeAt(i);
      if (inStr) { if (c === 92) i++; else if (c === 34) inStr = false; }
      else if (c === 34) inStr = true;
      else if (c === 91 || c === 123) { if (++d > max) max = d; }
      else if (c === 93 || c === 125) d--;
    }
    return max;
  }

  /** Number of JSON values in `text` (the root, every array element and every object member value), counted outside strings without recursion:
   * the root, plus one per comma, plus one per non-empty container. Same count as the calls of a JSON.parse reviver on valid JSON.
   * @type {(text: string) => number} */
  function countNodes(text) {
    let n = 1, inStr = false, prev = 0;
    for (let i = 0; i < text.length; i++) {
      const c = text.charCodeAt(i);
      if (inStr) { if (c === 92) i++; else if (c === 34) inStr = false; continue; }
      if (c === 34) inStr = true;
      else if (c === 44) n++;
      else if ((c === 93 && prev !== 91) || (c === 125 && prev !== 123)) n++;
      if (c !== 32 && c !== 9 && c !== 10 && c !== 13) prev = c;
    }
    return n;
  }
  return { depthOf, countNodes };
})();

const FILESICURI = (() => {
  const MAX_BYTES = 8 * 1024 * 1024, MAX_DEPTH = 32, MAX_NODES = 500000;
  const BAD_KEYS = new Set(["__proto__", "constructor", "prototype"]);

  /** Error whose message is the reason code, `key` the i18n key of its text and `params` the limit it names (the caller shows I18N.t(key, params)).
   * @type {(code: "tooBig" | "tooDeep", key: string, params: Record<string, string>) => Error & { key: string, params: Record<string, string> }} */
  const fail = (code, key, params) => Object.assign(new Error(code), { key, params });
  const MB = 1024 * 1024, fmtN = (/** @type {number} */ n) => (typeof I18N !== "undefined" ? I18N.num(n) : String(n));     // node tests run without I18N
  const big = (/** @type {number} */ bytes) => fail("tooBig", "workflow.err.tooBig", { mb: fmtN(Math.round(bytes / MB * 10) / 10) });
  const many = (/** @type {number} */ n) => fail("tooBig", "workflow.err.tooMany", { n: fmtN(n) });

  /** @typedef {{ maxBytes?: number, maxDepth?: number, maxNodes?: number }} Limits */
  /** JSON.parse with caps; keys named __proto__, constructor or prototype are dropped. Throws Error (message = reason code).
   * @type {(text: string, lim?: Limits) => any} */
  function parse(text, lim = {}) {
    const { maxBytes = MAX_BYTES, maxDepth = MAX_DEPTH, maxNodes = MAX_NODES } = lim;
    if (typeof text !== "string" || text.length > maxBytes) throw big(maxBytes);
    if (JSONSCAN.depthOf(text) > maxDepth) throw fail("tooDeep", "workflow.err.tooDeep", { n: fmtN(maxDepth) });
    // Fast path: without a backslash a forbidden key can only be written literally, so a text that holds none of the names needs no reviver
    // (a JSON escape such as \u005f would hide the name from the search, hence the backslash test); the node cap is then counted by countNodes
    if (!text.includes("\\") && !/__proto__|constructor|prototype/.test(text)) {
      const v = JSON.parse(text);
      if (JSONSCAN.countNodes(text) > maxNodes) throw many(maxNodes);
      return v;
    }
    let n = 0;
    return JSON.parse(text, function (k, v) {
      if (++n > maxNodes) throw many(maxNodes);
      return BAD_KEYS.has(k) ? undefined : v;
    });
  }

  /** Reads a File as JSON, refusing it by size before reading it.
   * @type {(file: File, lim?: Limits) => Promise<any>} */
  async function readJson(file, lim = {}) {
    if (file.size > (lim.maxBytes || MAX_BYTES)) throw big(lim.maxBytes || MAX_BYTES);
    return parse(await file.text(), lim);
  }

  /** Reads a File as text (CSV, TSV, lists), refusing it by size before reading it. Throws the error of `fail`.
   * @type {(file: File, maxBytes?: number) => Promise<string>} */
  async function readText(file, maxBytes = MAX_BYTES) {
    if (file.size > maxBytes) throw big(maxBytes);
    return file.text();
  }

  /** Copies into `target` only the own keys it already has, when the type is the same (finite for numbers); returns `target`.
   * @type {(target: Record<string, any>, src: any) => Record<string, any>} */
  function assignKnown(target, src) {
    if (!src || typeof src !== "object" || Array.isArray(src)) return target;
    for (const k of Object.keys(src)) {
      if (BAD_KEYS.has(k) || !Object.prototype.hasOwnProperty.call(target, k)) continue;
      const v = src[k], t = target[k];
      if (typeof v !== typeof t) continue;
      if (typeof v === "number" && !Number.isFinite(v)) continue;
      if (typeof v === "string" && v.length > 200) continue;
      target[k] = v;
    }
    return target;
  }

  /** @type {(v: any, lo: number, hi: number) => number | null} */
  const num = (v, lo, hi) => typeof v === "number" && Number.isFinite(v) && v >= lo && v <= hi ? v : null;

  return { parse, readJson, readText, assignKnown, num, depthOf: JSONSCAN.depthOf, countNodes: JSONSCAN.countNodes };
})();
