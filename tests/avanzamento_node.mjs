// Drives avanza().set() of mzlab/web/avanzamento.js on a minimal fake DOM; prints the bar after each call as JSON. See tests/test_avanzamento_nan.py.
const mk = () => {
  const e = { attrs: {}, style: {}, children: {}, textContent: "", hidden: false, classList: { add() {} }, setAttribute(k, v) { this.attrs[k] = String(v); }, removeAttribute(k) { delete this.attrs[k]; }, appendChild() {}, remove() {}, querySelector(s) { const k = s === "i" ? "i" : s; return this.children[k] || (this.children[k] = mk()); } };
  return e;
};
let root;
globalThis.document = { createElement: () => (root = mk()) };
globalThis.matchMedia = () => ({ matches: true });
globalThis.I18N = { t: k => k };
const { avanza } = await import("../mzlab/web/avanzamento.js");
const p = avanza(mk(), { title: "t", n: 1 });
const out = [];
for (const f of [0.2, NaN, 0.5, Infinity, null, undefined, 0.4]) {
  p.set(f);
  const b = root.children["[data-r=bar]"];
  out.push([String(f), b.attrs["aria-valuenow"] ?? null, b.children.i.style.width]);
}
console.log(JSON.stringify(out));
process.exit(0);
