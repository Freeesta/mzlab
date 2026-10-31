// Draws a msngrafo/1 file with the page drawer (mzlab/web/msngrafo-vista.js on a minimal DOM) and prints the texts of its ions and losses as a JSON list;
// see tests/test_msnfigura_wasm.py. usage: node msn_figura_wasm_node.js <graph.json> <tree|radial>
const fs = require("fs"), path = require("path"), vm = require("vm");
const src = fs.readFileSync(path.resolve(__dirname, "../mzlab/web/msngrafo-vista.js"), "utf8").split("\n").map(l => l.replace(/^export /, "")).join("\n");

class El {
  constructor(tag) { this.tag = tag; this.attrs = {}; this.children = []; this.text = null; }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  getAttribute(k) { return this.attrs[k] === undefined ? null : this.attrs[k]; }
  appendChild(c) { this.children.push(c); return c; }
  addEventListener() {}
  set textContent(v) { this.children = []; this.text = v === "" ? null : String(v); }
  get textContent() { return this.text !== null ? this.text : this.children.filter(c => c.tag !== "title").map(c => c.textContent).join(""); }
  get cls() { return (this.attrs.class || "").split(/\s+/); }
  all(out = []) { for (const c of this.children) if (c instanceof El) { out.push(c); c.all(out); } return out; }
  querySelectorAll(sel) { const k = sel.replace(/^\./, ""); return this.all().filter(e => e.cls.includes(k)); }
}
const ctx = vm.createContext({
  document: { createElementNS: (_, tag) => new El(tag), createTextNode: t => ({ tag: "#text", textContent: t, children: [] }), },
  DOMParser: class {},
});
// the text nodes count as children of the text element
const mk = vm.runInContext(`(() => { ${src}\nreturn { drawVista, visible }; })()`, ctx);

const EN = {
  "msngrafo.more": "+{n} more", "msngrafo.amb": "Several candidates", "msngrafo.graph.aria": "x", "msngrafo.v.graph.radial": "x",
  "msngrafo.v.aria.levels": "levels {levels}", "msngrafo.v.aria.rel": "{rel} % of the base peak", "msngrafo.v.aria.formula": "formula {f}", "msngrafo.v.aria.cands": "{n} candidate formulas",
  "msngrafo.v.aria.common": "reached from {n} precursors", "msngrafo.v.aria.collapsed": "{n} ions closed below", "msngrafo.v.common": "Reached from {n} different precursors",
  "msngrafo.v.compat": "compatible with {f}", "msngrafo.v.hidden": "{n} closed", "msngrafo.v.hyp": "hypothesis",
};
const PROVE = { perdita: "known loss", isotopi: "isotopes", msn: "MSn" };
const t = (k, p) => (EN[k] || k).replace(/\{(\w+)\}/g, (_, a) => String(p && p[a] !== undefined ? p[a] : ""));
// the rule label of the tab (infusione-grafo-regole.js `arcLabel`), which the drawer takes as an option
const ruleLabel = (j, a) => {
  const r = j.regole && j.regole.archi.find(x => x.da === a.da && x.a === a.a);
  if (!r || !(r.regole.length || r.composta.length)) return "";
  const ids = r.regole.length ? r.regole.map(x => x.id + (x.priorita === 3 ? " (weak)" : "")) : r.composta.map(c => c.join(" + "));
  return ids.slice(0, 2).join(", ") + (ids.length > 2 ? ` +${ids.length - 2}` : "");
};

const j = JSON.parse(fs.readFileSync(process.argv[2], "utf8")), mode = process.argv[3] || "tree";
const host = new El("div");
const { svg } = mk.drawVista(host, j, { onSelect() {}, onOpen() {} }, { t, mode, proveText: p => PROVE[p], ruleLabel: a => ruleLabel(j, a) });
const texts = svg.all().filter(e => e.tag === "text" && ["ig-mz", "ig-lvl", "ig-loss"].some(k => e.cls.includes(k))).map(e => e.textContent).filter(x => x !== "");
process.stdout.write(JSON.stringify(texts));
