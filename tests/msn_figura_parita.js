// Parity of the labels: the texts of the ions and of the losses that msngrafo-vista.js draws (page, Flow, stand-alone HTML) against the golden SVG of the Rust figure
// (crates/mzlab-cli/src/msn_figura.rs, `testi`) on the same msngrafo/1. The drawer runs here on a minimal DOM. Exit code 1 on the first difference.
const fs = require("fs"), path = require("path"), vm = require("vm"), assert = require("assert");
const dir = path.resolve(__dirname, "msn_figura");
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

for (const [name, mode, file] of [["g", "tree", "testi-albero.txt"], ["g", "radial", "testi-radiale.txt"], ["g2", "tree", "g2-testi-albero.txt"], ["g2", "radial", "g2-testi-radiale.txt"]]) {
  const j = JSON.parse(fs.readFileSync(path.join(dir, name + ".json"), "utf8"));
  const host = new El("div");
  const { svg } = mk.drawVista(host, j, { onSelect() {}, onOpen() {} }, { t, mode, proveText: p => PROVE[p], ruleLabel: a => ruleLabel(j, a) });
  const lines = [];
  for (const g of svg.all().filter(e => e.cls.includes("ig-node"))) {
    const texts = g.children.filter(c => c.tag === "text" && ["ig-mz", "ig-lvl", "ig-cand"].some(k => c.cls.includes(k))).map(c => c.textContent);
    lines.push(["node " + g.attrs["data-id"], ...texts].join(" | "));
  }
  // the arcs in the order the drawer draws them (the thin ones first), the radial view labels only the best path
  const best = new Set(j.vista.archi.filter(a => a.best).map(a => a.da + ">" + a.a));
  const arcs = j.archi.slice().sort((p, q) => Number(best.has(p.da + ">" + p.a)) - Number(best.has(q.da + ">" + q.a))).filter(a => mode === "tree" || best.has(a.da + ">" + a.a));
  const groups = svg.all().filter(e => e.cls.includes("ig-lossg"));
  assert.strictEqual(groups.length, arcs.length, "one label per arc");
  const arcLines = arcs.map((a, i) => {
    const tx = groups[i].children.filter(c => c.tag === "text");
    const first = tx.filter(c => c.cls.includes("ig-loss"))[0].textContent, second = (tx.filter(c => c.cls.includes("ig-loss"))[1] || { textContent: "" }).textContent;
    const rule = (tx.find(c => c.cls.includes("ig-rule")) || { textContent: "" }).textContent;
    return `arc ${a.da}>${a.a} | ${first} | ${second} | ${rule}`;
  });
  // the Rust side lists the arcs in the order of the graph: compare the sets of lines, ions in drawing order
  const rust = fs.readFileSync(path.join(dir, file), "utf8").replace(/\n$/, "").split("\n");
  assert.deepStrictEqual(lines, rust.filter(l => l.startsWith("node ")), mode + ": ions");
  assert.deepStrictEqual(arcLines.slice().sort(), rust.filter(l => l.startsWith("arc ")).sort(), mode + ": losses");
  assert(lines.length >= 2 && arcLines.length >= 1);
}
console.log("ok");
