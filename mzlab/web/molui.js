// @ts-strict
// Shared chemistry presentation helpers: formula composition (order-free comparison, Hill order), theme-aware molecule SVG, compound-name display.
// Classic script: window.MOLUI in the page, module.exports for the node tests.
(function (root) {
  // ---- formulas: composition = {element: count}; the order of the elements never matters
  const SYM = /^[A-Z][a-z]?/;
  function parse(text) {
    const s = String(text || "").replace(/\s+/g, "").replace(/\]?[+-]\d*$/, "");   // a trailing charge does not change the composition
    if (!s) return null;
    let total = {};
    for (const part of s.split(/[·.*]/)) {       // hydrates / adducts: CuSO4·5H2O -> CuSO4 + 5 H2O
      if (!part) return null;
      const lead = /^(\d+)/.exec(part), k = lead ? +lead[1] : 1, body = lead ? part.slice(lead[1].length) : part;
      const c = parseBody(body);
      if (!c) return null;
      for (const el in c) total[el] = (total[el] || 0) + c[el] * k;
    }
    return Object.keys(total).length ? total : null;
  }
  function parseBody(s) {
    const stack = [{}];
    let i = 0;
    while (i < s.length) {
      const ch = s[i];
      if (ch === "(" || ch === "[") { stack.push({}); i++; continue; }
      if (ch === ")" || ch === "]") {
        i++;
        const m = /^\d+/.exec(s.slice(i)), k = m ? +m[0] : 1;
        i += m ? m[0].length : 0;
        if (stack.length < 2) return null;
        const inner = stack.pop(), top = stack[stack.length - 1];
        for (const el in inner) top[el] = (top[el] || 0) + inner[el] * k;
        continue;
      }
      const m = SYM.exec(s.slice(i));
      if (!m) return null;
      i += m[0].length;
      const n = /^\d+/.exec(s.slice(i));
      i += n ? n[0].length : 0;
      const top = stack[stack.length - 1];
      top[m[0]] = (top[m[0]] || 0) + (n ? +n[0] : 1);
    }
    return stack.length === 1 ? stack[0] : null;
  }
  // Hill order: C, H, then alphabetical; without C everything is alphabetical (H included)
  function hill(c) {
    const els = Object.keys(c).filter(e => c[e] > 0);
    const order = "C" in c ? ["C", ...(els.includes("H") ? ["H"] : []), ...els.filter(e => e !== "C" && e !== "H").sort()] : els.sort();
    return order.map(e => e + (c[e] !== 1 ? c[e] : "")).join("");
  }
  const hillOf = text => { const c = parse(text); return c ? hill(c) : String(text || ""); };
  // compare two formulas: {ok, a, b (both in Hill order), diffs: [{el, a, b}]}; ok is null when one of them cannot be read
  function compare(fa, fb) {
    const ca = parse(fa), cb = parse(fb);
    if (!ca || !cb) return { ok: null, a: ca ? hill(ca) : String(fa || ""), b: cb ? hill(cb) : String(fb || ""), diffs: [] };
    const diffs = [...new Set([...Object.keys(ca), ...Object.keys(cb)])].sort().filter(e => (ca[e] || 0) !== (cb[e] || 0)).map(el => ({ el, a: ca[el] || 0, b: cb[el] || 0 }));
    return { ok: !diffs.length, a: hill(ca), b: hill(cb), diffs };
  }
  function mismatchText(fa, fb, la = "formula", lb = "SMILES") {
    const r = compare(fa, fb);
    if (r.ok !== false) return "";
    return `The ${la} (${r.a}) does not match the ${lb} (${r.b}): ` + r.diffs.map(d => `${d.el}: ${la} ${d.a}, ${lb} ${d.b}`).join("; ") + ".";
  }

  // ---- compound names: only the first letter is capitalised (acronyms, digits, Greek letters, inner capitals stay as they are); presentation only
  const cap = s => { s = String(s == null ? "" : s); return /^[a-z]/.test(s) ? s[0].toUpperCase() + s.slice(1) : s; };

  // ---- molecule SVG (OpenChemLib): bonds and carbon labels follow the text colour, heteroatoms use theme-aware CSS variables.
  // The letter of the element is always drawn, so colour is never the only cue.
  const ATOM_RGB = { "255,13,13": "o", "48,80,248": "n", "144,224,80": "f", "205,205,38": "s", "31,240,31": "cl", "166,41,41": "br", "148,0,148": "i", "255,128,0": "p" };
  const LIGHT = { o: "#c8261b", n: "#2b49c7", f: "#1b7f35", s: "#8a6d00", cl: "#1b7f35", br: "#a04a1c", i: "#8a1f9e", p: "#b85300", def: "#444444" };
  const DARK = { o: "#ff7b6e", n: "#8fa6ff", f: "#6bdc86", s: "#e6cb3a", cl: "#6bdc86", br: "#f0996f", i: "#dc9bf2", p: "#ffa94d", def: "#c8cdd4" };
  const KEYS = Object.keys(LIGHT);
  function css() {
    const vars = (m, o) => KEYS.map(k => `${o}${k}:${m[k]}`).join(";");
    const L = vars(LIGHT, "--mol-"), D = vars(DARK, "--mol-");
    return `:root{${L}}@media (prefers-color-scheme:dark){:root:not([data-theme=light]){${D}}}:root[data-theme=dark]{${D}}` +
      KEYS.map(k => `svg.molsvg .mol-${k}{fill:var(--mol-${k})}`).join("") + `svg.molsvg{color:var(--ink,#1c1f23)}`;
  }
  function ensureStyle() {
    if (typeof document === "undefined" || document.getElementById("molui-css")) return;
    const st = document.createElement("style"); st.id = "molui-css"; st.textContent = css(); document.head.appendChild(st);
  }
  // mode "themed" (default): classes + currentColor, follows the page theme at run time; "light" / "dark": a standalone file with literal colours and a background
  function svg(raw, mode = "themed") {
    const pal = mode === "dark" ? DARK : LIGHT, standalone = mode === "light" || mode === "dark", ink = mode === "dark" ? "#e4e7eb" : "#000000";
    let out = String(raw).replace(/(fill|stroke)="rgb\((\d+),(\d+),(\d+)\)"/g, (all, attr, r, g, b) => {
      const key = `${r},${g},${b}`;
      if (key === "0,0,0") return standalone ? `${attr}="${ink}"` : `${attr}="currentColor"`;
      const el = ATOM_RGB[key] || "def";
      if (key === "255,255,255") return all;
      return standalone ? `${attr}="${pal[el]}"` : attr === "fill" ? `fill="${pal[el]}" class="mol-${el}"` : `${attr}="${pal[el]}"`;
    });
    if (standalone) {
      out = out.replace(/<svg\b([^>]*)>/, (m, a) => { const vb = /viewBox="0 0 ([\d.]+) ([\d.]+)"/.exec(a); return m + (vb ? `<rect width="${vb[1]}" height="${vb[2]}" fill="${mode === "dark" ? "#14171b" : "#ffffff"}"/>` : ""); });
    } else {
      ensureStyle();
      out = out.replace(/<svg\b/, '<svg class="molsvg"');
    }
    return out;
  }
  const api = { parse, hill, hillOf, compare, mismatchText, cap, svg, css, LIGHT, DARK };
  if (typeof module !== "undefined" && module.exports) module.exports = api; else root.MOLUI = api;
})(typeof window !== "undefined" ? window : globalThis);
