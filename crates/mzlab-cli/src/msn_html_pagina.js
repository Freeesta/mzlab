// The page of the stand-alone HTML: reads the embedded graph, draws it with the drawer above (no network, no other file) and adds the view switch, the zoom
// buttons, the candidates of the selected ion, the downloads of the static figure and the data table with «Copy data».
const EN = {
  "msngrafo.more": "+{n} more", "msngrafo.amb": "Several candidates", "msngrafo.graph.aria": "Fragmentation graph: columns by MS level, arrows for the losses",
  "msngrafo.v.graph.radial": "Fragmentation graph, radial view: the precursor in the centre, one ring per step of the best path, arrows for the losses",
  "msngrafo.v.aria.levels": "levels {levels}", "msngrafo.v.aria.rel": "{rel} % of the base peak", "msngrafo.v.aria.formula": "formula {f}", "msngrafo.v.aria.cands": "{n} candidate formulas",
  "msngrafo.v.aria.common": "reached from {n} precursors", "msngrafo.v.aria.collapsed": "{n} ions closed below", "msngrafo.v.common": "Reached from {n} different precursors",
  "msngrafo.v.compat": "compatible with {f}", "msngrafo.v.hidden": "{n} closed", "msngrafo.v.hyp": "hypothesis",
};
const PROVE = { perdita: "known loss", isotopi: "isotopes", msn: "MSn" };
const t = (key, p) => (EN[key] || key).replace(/\{(\w+)\}/g, (_, k) => String(p && p[k] !== undefined ? p[k] : ""));
const $ = id => document.getElementById(id);
const j = JSON.parse($("mzlab-graph").textContent);
const rootSvg = $("mzlab-root").textContent.trim() || null;
const v = vistaOf(j), byId = new Map(v.nodi.map(x => [x.id, x]));
const dflt = j.polarita === -1 ? "-" : "+";
let mode = "tree", sel = null, zoom = 1;

const ruleLabel = a => {
  const r = j.regole && j.regole.archi.find(x => x.da === a.da && x.a === a.a);
  if (!r) return "";
  const ids = r.regole.length ? r.regole.map(x => x.id + (x.priorita === 3 ? " (weak)" : "")) : r.composta.map(c => c.join(" + "));
  return ids.slice(0, 2).join(", ") + (ids.length > 2 ? ` +${ids.length - 2}` : "");
};
const plain = f => { const [b, c] = splitCharge(f, dflt); return plainFormula(b, c); };
const chosen = n => { const f = n.formula; return f ? f.scelta || (f.candidati.length === 1 ? f.candidati[0].formula : null) : null; };
const say = text => { $("status").textContent = text; };

function applyZoom() {
  const svg = $("graph").querySelector("svg");
  if (svg) { svg.setAttribute("width", String(Math.round(Number(svg.viewBox.baseVal.width) * zoom))); svg.setAttribute("height", String(Math.round(Number(svg.viewBox.baseVal.height) * zoom))); }
  $("z-reset").textContent = Math.round(zoom * 100) + " %";
}

function draw() {
  const r = drawVista($("graph"), j, {
    onSelect: id => { sel = id; draw(); const on = id && $("graph").querySelector(`.ig-node[data-id="${CSS.escape(id)}"]`); if (on) on.focus({ preventScroll: true }); detail(id); },
    onOpen: () => {},
  }, { t, sel, mode, proveText: p => PROVE[p] || p, ruleLabel, root: rootSvg, chosen });
  applyZoom();
  return r;
}

function cell(tr, text, cls, tag = "td") { const c = document.createElement(tag); c.textContent = text; if (cls) c.className = cls; if (tag === "th") c.scope = "col"; tr.appendChild(c); }

function detail(id) {
  const box = $("side");
  box.textContent = "";
  const n = id && j.nodi.find(x => x.id === id);
  if (!n) return;
  const x = byId.get(id), h = document.createElement("h2");
  h.textContent = `${x.livelli.map(l => "MS" + l).join(", ")} · m/z ${n.mz.toFixed(4)}`;
  box.appendChild(h);
  const cs = (n.formula && n.formula.candidati) || [];
  if (!cs.length) { const p = document.createElement("p"); p.className = "note"; p.textContent = "No candidate formula for this ion."; box.appendChild(p); return; }
  const tb = document.createElement("table"), cap = document.createElement("caption"), hd = document.createElement("tr");
  cap.textContent = "Candidate formulas of the ion (proposals, not identifications)";
  tb.appendChild(cap);
  for (const c of ["Candidate", "Error (ppm)", "Chosen", "Proofs"]) cell(hd, c, "", "th");
  tb.appendChild(hd);
  for (const c of cs) {
    const tr = document.createElement("tr");
    cell(tr, plain(c.formula)); cell(tr, c.ppm.toFixed(2), "num"); cell(tr, chosen(n) === c.formula ? "yes" : ""); cell(tr, (c.prove || []).map(p => PROVE[p] || p).join(", "));
    tb.appendChild(tr);
  }
  box.appendChild(tb);
}

/** The rows of the table of the data: one per ion, ASCII formulas (the same as «Copy data»). */
function rows() {
  const out = [["id", "levels", "mz", "rel_pct", "chosen", "candidates", "loss_from_best_parent", "common"]];
  for (const n of j.nodi) {
    const x = byId.get(n.id), ch = chosen(n), cs = (n.formula && n.formula.candidati) || [];
    const a = j.archi.find(k => k.a === n.id && k.da === x.best.da);
    const loss = a ? (a.perdita.formula || `${a.perdita.da_massa.toFixed(3)} Da`) : "";
    out.push([n.id, x.livelli.map(l => "MS" + l).join(","), n.mz.toFixed(5), x.best.da && x.rel != null ? x.rel.toFixed(1) : "", ch ? plain(ch) : "", cs.map(c => `${plain(c.formula)} (${c.ppm.toFixed(2)} ppm)`).join("; "), loss, x.comune ? String(x.entranti) : ""]);
  }
  return out;
}

function table() {
  const rs = rows(), tb = document.createElement("table"), cap = document.createElement("caption");
  cap.textContent = "Ions of the fragmentation graph";
  tb.appendChild(cap);
  rs.forEach((r, i) => { const tr = document.createElement("tr"); r.forEach((c, k) => cell(tr, c, !i ? "" : k === 2 || k === 3 ? "num" : "", i ? "td" : "th")); tb.appendChild(tr); });
  $("table").replaceChildren(tb);
}

function save(name, type, body) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([body], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
const figure = () => $(mode === "radial" ? "mzlab-svg-radial" : "mzlab-svg-tree").textContent;
const stem = ((j.sorgente && j.sorgente.nome) || "msn-graph").replace(/\.[^.]*$/, "");

function setMode(m) {
  mode = m === "radial" && !v.fallback ? "radial" : "tree";
  $("v-tree").setAttribute("aria-pressed", String(mode === "tree"));
  $("v-radial").setAttribute("aria-pressed", String(mode === "radial"));
  draw();
}

$("v-tree").addEventListener("click", () => setMode("tree"));
$("v-radial").addEventListener("click", () => setMode("radial"));
$("z-in").addEventListener("click", () => { zoom = Math.min(3, Math.round((zoom + 0.25) * 100) / 100); applyZoom(); });
$("z-out").addEventListener("click", () => { zoom = Math.max(0.25, Math.round((zoom - 0.25) * 100) / 100); applyZoom(); });
$("z-reset").addEventListener("click", () => { zoom = 1; applyZoom(); });
$("dl-svg").addEventListener("click", () => save(`${stem}-${mode}.svg`, "image/svg+xml", figure()));
$("dl-png").addEventListener("click", () => {
  const url = URL.createObjectURL(new Blob([figure()], { type: "image/svg+xml" })), img = new Image();
  img.onload = () => {
    const c = document.createElement("canvas"), s = 2;
    c.width = img.naturalWidth * s; c.height = img.naturalHeight * s;
    const g = c.getContext("2d"); g.scale(s, s); g.drawImage(img, 0, 0);
    URL.revokeObjectURL(url);
    c.toBlob(b => { if (b) { const a = document.createElement("a"); a.href = URL.createObjectURL(b); a.download = `${stem}-${mode}.png`; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); } });
  };
  img.onerror = () => say("The image could not be made here: download the SVG instead.");
  img.src = url;
});
$("copy").addEventListener("click", async () => {
  const text = rows().map(r => r.join("\t")).join("\n");
  try { await navigator.clipboard.writeText(text); say("Data copied."); }
  catch (_) {
    const ta = document.createElement("textarea"); ta.value = text; document.body.appendChild(ta); ta.select();
    say(document.execCommand("copy") ? "Data copied." : "The data could not be copied."); ta.remove();
  }
});
if (v.fallback) $("v-radial").disabled = true;
table();
