// @ts-check
// Safe reading of JSON files that students exchange (.mzflow, notebook, lists): size and depth caps before parsing, no key that can
// reach the prototype, and a copy that keeps only known keys of the same type. Content is data and is never evaluated.
const FILESICURI = (() => {
  const MAX_BYTES = 8 * 1024 * 1024, MAX_DEPTH = 32, MAX_NODES = 500000;
  const BAD_KEYS = new Set(["__proto__", "constructor", "prototype"]);

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

  /** @typedef {{ maxBytes?: number, maxDepth?: number, maxNodes?: number }} Limits */
  /** JSON.parse with caps; keys named __proto__, constructor or prototype are dropped. Throws Error (message = reason code).
   * @type {(text: string, lim?: Limits) => any} */
  function parse(text, lim = {}) {
    const { maxBytes = MAX_BYTES, maxDepth = MAX_DEPTH, maxNodes = MAX_NODES } = lim;
    if (typeof text !== "string" || text.length > maxBytes) throw new Error("tooBig");
    if (depthOf(text) > maxDepth) throw new Error("tooDeep");
    let n = 0;
    return JSON.parse(text, function (k, v) {
      if (++n > maxNodes) throw new Error("tooBig");
      return BAD_KEYS.has(k) ? undefined : v;
    });
  }

  /** Reads a File as JSON, refusing it by size before reading it.
   * @type {(file: File, lim?: Limits) => Promise<any>} */
  async function readJson(file, lim = {}) {
    if (file.size > (lim.maxBytes || MAX_BYTES)) throw new Error("tooBig");
    return parse(await file.text(), lim);
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

  return { parse, readJson, assignKnown, num, depthOf };
})();
