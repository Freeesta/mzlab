// @ts-check
// TPMINE-PRIVATE  TPSTRUCT: the «Structure» tab of an open row of a Tabui table. It reads one `interp/1` row (site + drawing, made by Rust) and shows the head sentence, the product drawn on its
// parent (TPMOL), «Open in Draw», the table of the candidate classes, the limits and the suggestions. It decides nothing: the form (structure, ring bond, brackets) comes from `drawing.mode`.
// Used by TPRIGA (`fill` of the tab «structure») and by the Identity column (`identText`). Texts in English; colours only from CSS variables; no animation under prefers-reduced-motion.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /** @typedef {{ id: number, kind: string, status: string, atoms: number[], group: string | null, n_support: number, n_against: number, support: number }} SiteClass */
  /** @typedef {{ transform: string, confidence: string, reasons: string[], classes: SiteClass[], region: { name: string, atoms: number[] } | null }} Site */
  /** @typedef {{ parent: { smiles: string | null }, change: { delta: string } | null, site: Site | null, drawing: any, tree?: { n_same: number, n_shifted: number } | null }} InterpRow */

  const CSS = `
.tp .tps{display:grid;gap:8px}.tp .tps-head{margin:0;font-size:13px;font-weight:600}
.tp .tps-fig{min-width:0;max-width:420px}.tp .tps-bar{display:flex;flex-wrap:wrap;gap:6px;align-items:center}
.tp .tps-act{display:inline-flex;align-items:center;gap:6px}.tp .tps-act svg{width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:1.6}
.tp .tps-line{margin:0;font-size:12px;color:var(--muted)}.tp .tps ul{margin:0;padding:0 0 0 18px;font-size:12px}
.tp .tps-mark{display:inline-block;min-width:1.2em;text-align:center}
@media(prefers-reduced-motion:no-preference){.tp .tps{animation:tps-in .18s ease-out}@keyframes tps-in{from{opacity:0}to{opacity:1}}}`;

  const REASON = /** @type {Record<string, string>} */ ({
    no_ms2: "no MS2 in any file", chimeric: "the MS2 spectra are chimeric", isomers_not_separated: "isomers are not separated in time", no_reference: "no MS2 of the parent to compare with",
    few_fragments: "too few informative fragments", multiple_changes: "more than one change", change_lost: "the changed part is not in the fragments", unsupported_change: "this change has no position model",
  });
  const STATUS = /** @type {Record<string, [string, string]>} */ ({ supported: ["✓", "supported"], excluded: ["✗", "excluded"], undistinguished: ["≈", "not distinguished"] });
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const delta = (/** @type {InterpRow} */ r) => (r.drawing && r.drawing.label) || (r.change && r.change.delta) || (r.site && r.site.transform) || "";

  /** the Identity column text for a row: «+O», «+O (ring A?)» for a region, «+O (?)» when the position is open; empty without interpretation @param {InterpRow | null | undefined} r */
  function identText(r) {
    if (!r || !r.site || !delta(r)) return "";
    const s = r.site;
    return s.confidence === "site" ? delta(r) : s.confidence === "region" && s.region ? `${delta(r)} (${s.region.name}?)` : `${delta(r)} (?)`;
  }

  /** the atoms as «C7, O9» (element and number in the SMILES order of the parent) @param {string | null} smiles @param {number[]} ids */
  async function atomNames(smiles, ids) {
    const g = smiles && G.TPMOL ? await G.TPMOL.graph(smiles) : null;
    return ids.map(i => (g && g.atoms[i] ? `${g.atoms[i].el}${i + 1}` : `#${i + 1}`)).join(", ");
  }

  /** the head sentence, in words @param {InterpRow} r */
  async function headText(r) {
    const s = r.site, d = delta(r);
    if (!s) return d ? `${d} · No position model for this change` : "No change from the parent was found for this row";
    if (s.confidence === "cannot_say") return `Cannot say: ${(s.reasons.length ? s.reasons : ["unsupported_change"]).map(k => REASON[k] || k).join("; ")}`;
    if (s.confidence === "site") {
      const c = s.classes.find(x => x.status === "supported") || s.classes[0];
      return `${d} · Site: ${c ? await atomNames(r.parent.smiles, c.atoms) : "one atom"}`;
    }
    if (s.confidence === "region") return `${d} · Region: ${s.region ? s.region.name : "one zone"}`;
    const live = s.classes.filter(x => x.status !== "excluded");
    return `${d} · Position uncertain · ${live.length} ${live.length === 1 ? "class" : "classes"} · relative support ${live.map(x => x.support.toFixed(1)).join(" / ")}`;
  }

  /** the rows of the class table (also the data table of the drawing) @param {InterpRow} r */
  async function classRows(r) {
    const cs = (r.site && r.site.classes) || [];
    return Promise.all(cs.map(async c => ({ c, atoms: await atomNames(r.parent.smiles, c.atoms) })));
  }

  /** @param {{ c: SiteClass, atoms: string }[]} rows */
  function classTable(rows) {
    if (!rows.length) return "";
    const th = (/** @type {string} */ t, /** @type {string} */ cls = "") => `<th scope="col"${cls ? ` class="${cls}"` : ""}>${t}</th>`;
    const body = rows.map(({ c, atoms }) => {
      const [mark, word] = STATUS[c.status] || ["", c.status];
      return `<tr><td>${c.id}</td><td>${esc(atoms)}</td><td>${esc(c.group || c.kind)}</td><td><span class="tps-mark" aria-hidden="true">${mark}</span> ${word}</td><td class="n">${c.support.toFixed(2)}</td><td class="n">${c.n_support}</td><td class="n">${c.n_against}</td></tr>`;
    }).join("");
    return `<div class="tpr-tw"><table class="tpr" aria-label="Candidate classes of the position"><thead><tr>${th("Class")}${th("Atoms")}${th("Zone")}${th("Status")}${th("Support", "n")}${th("For", "n")}${th("Against", "n")}</tr></thead><tbody>${body}</tbody></table></div>`;
  }

  const PENCIL = `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2 14l1-4 8-8 3 3-8 8z M10 3l3 3"/></svg>`;

  /** the description of the figure, also its data table (TDATI) @param {InterpRow} r @param {{ c: SiteClass, atoms: string }[]} rows */
  function figureTable(r, rows) {
    const d = delta(r);
    return { title: `${d} on the parent`, head: ["Class", "Atoms", "Status", "Support"], rows: rows.length ? rows.map(({ c, atoms }) => [String(c.id), atoms, (STATUS[c.status] || ["", c.status])[1], c.support.toFixed(2)]) : [[d, r.drawing ? r.drawing.atoms.map((/** @type {number} */ i) => i + 1).join(", ") : "", "site", "1.00"]] };
  }

  /** draws the tab into `panel` (which TPRIGA has emptied); `r` = the `interp/1` row, null when there is none and `why` says why @param {HTMLElement} panel @param {InterpRow | null | undefined} r @param {string} [why] */
  async function fill(panel, r, why) {
    const tok = String(((+(panel.dataset.tps || 0)) + 1)); panel.dataset.tps = tok;
    if (!r) { const p = document.createElement("p"); p.className = "tpr-none"; p.textContent = `Structure: ${why || "no interpretation for this row"}.`; panel.appendChild(p); return; }
    const smiles = r.parent && r.parent.smiles, d = r.drawing && r.site ? { ...r.drawing, region: r.site.region ? r.site.region.name : undefined } : null;
    const [head, rows, svg] = await Promise.all([headText(r), classRows(r), smiles && d && G.TPMOL ? G.TPMOL.svg(smiles, d) : Promise.resolve(null)]);
    if (panel.dataset.tps !== tok) return;                                                   // the row or the tab changed meanwhile
    const tree = r.tree ? `<p class="tps-line">Fragments of the parent: ${r.tree.n_same} found at the same m/z, ${r.tree.n_shifted} shifted by the change.</p>` : "";
    const open = svg ? `<button type="button" class="tpr-act tps-act" data-tps="draw" aria-label="Open this drawing in the Draw tab">${PENCIL}Open in Draw</button>` : "";
    const none = svg ? "" : `<p class="tpr-none">${smiles ? "The drawing needs the parent structure to be readable" : "No structure of the parent was given: set its SMILES to see the drawing"}.</p>`;
    panel.innerHTML = `<div class="tps"><p class="tps-head">${esc(head)}</p><div class="tps-bar">${open}</div><div class="tps-fig">${svg || ""}</div>${none}${classTable(rows)}${tree}
<p class="tps-line">The fragments rank the positions; they do not prove one. Support is an order, not a probability. Literature products are hypotheses.</p>
<p class="tps-line">To separate positions: MS3 of the product, H/D exchange, or an authentic standard of the isomer.</p></div>`;
    const el = panel.querySelector(".tps-fig svg");
    if (el && G.TDATI) { G.TDATI.describe(el, head); G.TDATI.addButton(el, () => figureTable(r, rows), true); }
    const b = panel.querySelector('[data-tps="draw"]');
    if (b && d && smiles) b.addEventListener("click", () => { G.TPMOL.openInDraw(smiles, d); });
  }

  if (!document.getElementById("tps-css")) { const s = document.createElement("style"); s.id = "tps-css"; s.textContent = CSS; document.head.appendChild(s); }
  G.TPSTRUCT = { fill, identText, headText };
})();
