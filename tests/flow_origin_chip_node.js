// The Table view of the Flow (flow/lazy/flow-results.js) on a list with an `origin` column: one chip per combination of series with its count, a chip filters the rows of the
// whole file, a second chip adds its rows, the same chip again removes them; a list without the column has no chips. Prints one JSON line; a tiny DOM stands in for the page.
const fs = require("fs"), vm = require("vm"), path = require("path");
const root = process.argv[2];
class Node { constructor(tag) { this.tag = tag; this.children = []; this.attrs = {}; this.textContent = ""; }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  replaceChildren(...k) { this.children = k; }
  append(...k) { this.children.push(...k); } }
const el = (tag, a, ...kids) => { const n = new Node(tag); for (const [k, v] of Object.entries(a || {})) { if (k === "text") n.textContent = v; else if (k === "onclick") n.onclick = v; else n.attrs[k] = String(v); } n.children.push(...kids); return n; };
const walk = (n, f) => { f(n); for (const c of n.children) walk(c, f); };
const ctx = vm.createContext({ FLOW: {} });
vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-results.js"), "utf8"), ctx);
const F = ctx.FLOW; F.el = el;
const view = F.VIEWS.find(v => v.id === "table");
const open = async csv => { const host = new Node("div"); await view.open(host, { label: "u.csv", text: async () => csv }); return host; };
const body = host => { let rows = 0; walk(host, n => { if (n.tag === "tbody" && n.children.length && n.children[0].tag === "tr" && n.children[0].children.length > 3) rows = n.children.length; }); return rows; };
const chips = host => { const c = []; walk(host, n => { if (n.attrs["data-origin"] !== undefined) c.push(n); }); return c; };
(async () => {
  const csv = "id,mz,rt_min,rt_max,polarity,role,origin\nU1,100,1,2,,target,direct+suspect\nU2,110,1,2,,target,direct\nU3,120,1,2,,target,suspect\nU4,130,1,2,,target,direct\n";
  const host = await open(csv), out = { chips: chips(host).map(c => c.textContent), all: body(host) };
  const [both, direct, suspect] = chips(host);
  direct.onclick({ currentTarget: direct }); out.direct = body(host); out.pressed = direct.attrs["aria-pressed"];
  suspect.onclick({ currentTarget: suspect }); out.two = body(host);
  direct.onclick({ currentTarget: direct }); out.back = body(host); out.off = direct.attrs["aria-pressed"];
  out.none = chips(await open("id,mz\nU1,100\n")).length;
  console.log(JSON.stringify(out));
})().catch(e => { console.error(e); process.exit(1); });
