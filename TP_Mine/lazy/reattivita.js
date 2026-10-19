// @ts-check
// TPMINE-PRIVATE  TPREATT: the panel of reactivity of the parent compound in Tabui (administrator mode only). It shows a `reatt/1` (docs/agenti/reattivita.md): the 2D drawing with a halo under the heavy atoms,
// the 3D model (TPV3D, viewer3d.js), the cards f(−) (default), f(0), f(+), Info, Notes and JSON, the threshold, the table per atom (TDATI) and the exports (XYZ, SMILES, JSON, CSV) with «How to cite».
// Every value is a «Prev.» (prediction for the isolated molecule in implicit water); no level is assigned and no class of positions is reordered or removed. Texts in English, without keys of lang/.
// Colours only from CSS variables (Okabe-Ito); the atoms above the threshold also have a thick outline and the written index; every figure has its table.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const admin = () => typeof G.uipAdminOn === "function" && G.uipAdminOn();
  /** @typedef {{ i: number, el: string, heavy_index: number|null, x: number, y: number, z: number, q: number|null, f_plus: number, f_zero: number|null, f_minus: number, dual: number, homo?: number, lumo?: number }} RAtom */
  /** @typedef {{ format: string, version: number, engine?: string, method?: string, charges?: string, solvent?: string, smiles: string, atoms: RAtom[], bonds?: {a:number,b:number,order?:number}[], hbonds?: {donor_h:number,acceptor:number}[], global?: Record<string, number|null>, conformers?: {n?:number, chosen?:number, f_minus_spread?:number|null}, scc?: {converged?: boolean|boolean[], iterations?: number|number[]}, protomer?: {ph: number, smiles: string, charge: number, changes: {group: string, heavy_index: number, action: string, pka: number}[], note?: string}, notes?: string[], cite?: any[], ch_sites?: {class:string, heavy_index:number, n_h:number, f_minus:number}[] }} Reatt */

  const CARDS = /** @type {{ id: string, label: string, key: keyof RAtom }[]} */ ([
    { id: "fminus", label: "f(−)", key: "f_minus" }, { id: "fzero", label: "f(0)", key: "f_zero" }, { id: "fplus", label: "f(+)", key: "f_plus" }]);
  const TABS = ["f(−)", "f(0)", "f(+)", "Info", "Notes", "JSON"];
  const ERR = /** @type {Record<string, string>} */ ({
    "error.chem.element": "An element of the parent is not covered by the engine.", "error.chem.smiles": "The SMILES of the parent could not be read.",
    "error.chem.scc": "The self-consistent field did not converge.", "error.chem.geometry": "No 3D geometry could be built.", "error.chem.embed": "No 3D geometry could be built.",
    "error.chem.charge": "The charge of the molecule is not possible for the engine.", "error.chem.opts": "The options of the calculation are not valid.", "error.featFailed": "The engine could not be started.",
  });
  const NOTES = [
    "Prev.: prediction for the isolated molecule in implicit water; not evidence.",
    "f(−) orders the sites of an electrophilic attack (such as •OH); it does not see the surface of TiO2, the steric hindrance or the transition state.",
    "f(0) radical susceptibility may fail on deactivated rings with •OH; f(−) is recommended for electrophilic attack.",
    "TiO2: the molecule is calculated alone, not adsorbed on the catalyst.",
    "The program proposes, the chemist decides: no level of identification is given and no position class is removed or reordered by these values.",
  ];

  const CSS = `
.tpr{margin:10px 0}.tpr h4{margin:0 0 6px}.tpr .tpr-st{margin:6px 0;color:var(--muted)}
.tpr .tpr-tabs{display:flex;flex-wrap:wrap;gap:4px;margin:6px 0}
.tpr .tpr-tabs button{min-height:28px;min-width:28px;padding:2px 10px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);font:inherit}
.tpr .tpr-tabs button[aria-selected=true]{border-color:var(--accent);box-shadow:inset 0 -2px 0 var(--accent);font-weight:600}
.tpr button:focus-visible,.tpr input:focus-visible,.tpr summary:focus-visible,.tpr pre:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.tpr .tpr-figs{display:flex;flex-wrap:wrap;gap:12px;align-items:flex-start}.tpr .tpr-fig2{flex:0 1 320px;max-width:100%}.tpr .tpr-fig3{flex:1 1 320px;min-width:0}
.tpr .tpr-thr{display:flex;flex-wrap:wrap;gap:8px;align-items:center;margin:8px 0;font-size:12px}.tpr .tpr-thr input[type=number]{width:6em;min-height:24px}
.tpr table.tpr-t{border-collapse:collapse;font-size:12px}.tpr table.tpr-t th,.tpr table.tpr-t td{padding:2px 8px;border-bottom:1px solid var(--line);text-align:right}
.tpr table.tpr-t th:first-child,.tpr table.tpr-t td:first-child{text-align:left}.tpr table.tpr-t td{font-variant-numeric:tabular-nums}
.tpr table.tpr-t th button{all:unset;cursor:pointer;min-height:24px;min-width:24px;display:inline-flex;align-items:center;font-weight:600;color:var(--muted)}
.tpr table.tpr-t th button:focus-visible{outline:2px solid var(--accent)}
.tpr table.tpr-t tr.over td{background:var(--sel);font-weight:600}
.tpr .tpr-ex{display:flex;flex-wrap:wrap;gap:6px;margin:8px 0}.tpr .tpr-ex button{min-height:28px;padding:2px 10px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);font:inherit}
.tpr pre{max-height:320px;overflow:auto;background:var(--code,var(--soft));padding:8px;border-radius:5px;font-size:11px}
.tpr svg.tpr-2d{width:100%;height:auto;border:1px solid var(--line);border-radius:6px;background:var(--panel)}
@media(pointer:coarse){.tpr .tpr-tabs button,.tpr .tpr-ex button{min-height:40px;min-width:40px}.tpr table.tpr-t th button{min-height:40px}}`;
  const injectCss = () => { if (document.getElementById("tpr-css")) return; const s = document.createElement("style"); s.id = "tpr-css"; s.textContent = CSS; document.head.appendChild(s); };

  /** the index written for an atom: 1-based among the heavy atoms (as in «Substitution position»), H<n> for the hydrogens @param {RAtom} a */
  const indexOf = a => a.heavy_index == null ? "H" + a.i : String(a.heavy_index + 1);
  const num = (/** @type {any} */ v, d = 4) => v !== null && v !== "" && Number.isFinite(+v) ? (+v).toFixed(d) : "—";

  // ------------------------------------------------------------ exports
  /** @param {Reatt} r @returns {string} XYZ: the number of atoms, a comment, one line per atom (N + 2 lines) */
  function xyz(r) {
    const head = `${r.method || "GFN2-xTB"}${r.solvent && r.solvent !== "none" ? " ALPB " + r.solvent : ""} geometry, ${r.engine || "mzlab-chem"}, SMILES ${r.smiles}`.replace(/[\r\n]+/g, " ");
    return [String(r.atoms.length), head, ...r.atoms.map(a => `${a.el.padEnd(2)} ${a.x.toFixed(6).padStart(12)} ${a.y.toFixed(6).padStart(12)} ${a.z.toFixed(6).padStart(12)}`)].join("\n") + "\n";
  }
  /** @param {Reatt} r @returns {string} CSV of the table per atom */
  function csv(r) {
    const mo = r.atoms.some(a => a.homo != null), h = ["index", "atom", "element", "heavy_index", "q", "f_plus", "f_zero", "f_minus", "dual", ...(mo ? ["homo_share", "lumo_share"] : [])];
    return [h.join(","), ...r.atoms.map(a => [indexOf(a), a.i, a.el, a.heavy_index ?? "", a.q, a.f_plus, a.f_zero, a.f_minus, a.dual, ...(mo ? [a.homo, a.lumo] : [])].join(","))].join("\n") + "\n";
  }
  /** @param {string} name @param {string} text @param {string} type */
  const download = (name, text, type) => { if (typeof G.dl === "function") G.dl(name, text, type, false); };

  // ------------------------------------------------------------ 2D drawing with halos
  /** @type {Promise<any>|null} */ let oclP = null;
  const ocl = () => (oclP = oclP || import(new URL("vendor/openchemlib.js", G.QTOOLS.ctx.base).href));
  /**
   * maps the heavy atoms of OpenChemLib (order of the SMILES) to the `heavy_index` of the `reatt/1`: the same order when element and neighbours agree, else a search by element and neighbours
   * @param {any} mol @param {Reatt} r @returns {number[]|null} for each atom of OpenChemLib, its heavy_index, or null when the two graphs differ
   */
  function mapHeavy(mol, r) {
    const hv = r.atoms.filter(a => a.heavy_index != null).sort((a, b) => /** @type {number} */ (a.heavy_index) - /** @type {number} */ (b.heavy_index)), n = mol.getAllAtoms();
    if (hv.length !== n) return null;
    const idx = new Map(hv.map((a, k) => [a.i, k])), adjR = hv.map(() => /** @type {number[]} */ ([]));
    for (const b of r.bonds || []) { const p = idx.get(b.a), q = idx.get(b.b); if (p !== undefined && q !== undefined) { adjR[p].push(q); adjR[q].push(p); } }
    const adjO = Array.from({ length: n }, () => /** @type {number[]} */ ([]));
    for (let b = 0; b < mol.getAllBonds(); b++) { const p = mol.getBondAtom(0, b), q = mol.getBondAtom(1, b); adjO[p].push(q); adjO[q].push(p); }
    const el = (/** @type {number} */ j) => mol.getAtomLabel(j);
    const same = hv.every((a, k) => el(k) === a.el && adjO[k].length === adjR[k].length && adjO[k].every(x => adjR[k].includes(x)));
    if (same) return hv.map((_, k) => k);
    const map = /** @type {number[]} */ (Array(n).fill(-1)), used = /** @type {boolean[]} */ (Array(n).fill(false));
    const place = (/** @type {number} */ j) => {                  // backtracking over the atoms of OpenChemLib: same element, same degree, bonded atoms already placed bonded
      if (j === n) return true;
      for (let k = 0; k < n; k++) {
        if (used[k] || hv[k].el !== el(j) || adjR[k].length !== adjO[j].length) continue;
        if (!adjO[j].filter(x => x < j).every(x => adjR[k].includes(map[x]))) continue;
        map[j] = k; used[k] = true; if (place(j + 1)) return true; used[k] = false; map[j] = -1;
      }
      return false;
    };
    return place(0) ? map : null;
  }
  /** the SVG of the drawing: bonds from the 2D coordinates of OpenChemLib, a translucent circle under each heavy atom, outline and index above the threshold
   *  @param {any} mol @param {number[]} map @param {Reatt} r @param {RAtom[]} byHeavy @param {number[]} vals @param {number} thr @returns {string} */
  function svg2d(mol, map, r, byHeavy, vals, thr) {
    const n = mol.getAllAtoms(), xs = [], ys = [];
    for (let j = 0; j < n; j++) { xs.push(mol.getAtomX(j)); ys.push(mol.getAtomY(j)); }
    const bl = (() => { let s = 0, c = 0; for (let b = 0; b < mol.getAllBonds(); b++) { const p = mol.getBondAtom(0, b), q = mol.getBondAtom(1, b); s += Math.hypot(xs[p] - xs[q], ys[p] - ys[q]); c++; } return c ? s / c : 1; })();
    const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys), pad = 1.2 * bl, W = 300, k = Math.min((W - 40) / Math.max(x1 - x0, bl), 200 / Math.max(y1 - y0, bl), 46 / bl);
    const w = (x1 - x0) * k + 2 * pad * k, h = (y1 - y0) * k + 2 * pad * k, X = (/** @type {number} */ j) => (xs[j] - x0 + pad) * k, Y = (/** @type {number} */ j) => (ys[j] - y0 + pad) * k;
    const vmax = Math.max(1e-9, ...vals.filter(Number.isFinite)); let o = "";
    for (let j = 0; j < n; j++) { const v = Math.max(0, vals[map[j]]) / vmax; if (v > 0.001) o += `<circle cx="${X(j).toFixed(1)}" cy="${Y(j).toFixed(1)}" r="${(0.28 * bl * k * (1.2 + 1.8 * v)).toFixed(1)}" fill="var(--v3-halo)" fill-opacity="${(0.15 + 0.5 * v).toFixed(2)}"/>`; }
    for (let b = 0; b < mol.getAllBonds(); b++) {
      const p = mol.getBondAtom(0, b), q = mol.getBondAtom(1, b), ord = mol.getBondOrder(b), dx = (ys[q] - ys[p]), dy = (xs[p] - xs[q]), L = Math.hypot(dx, dy) || 1, off = ord > 1 ? 0.09 * bl * k : 0;
      for (let t = 0; t < Math.min(ord, 3); t++) { const s = (t - (Math.min(ord, 3) - 1) / 2) * 2 * off / L * (ord > 1 ? 1 : 0); o += `<line x1="${(X(p) + dx * s).toFixed(1)}" y1="${(Y(p) + dy * s).toFixed(1)}" x2="${(X(q) + dx * s).toFixed(1)}" y2="${(Y(q) + dy * s).toFixed(1)}" stroke="var(--ink)" stroke-width="1.6"/>`; }
    }
    for (let j = 0; j < n; j++) {
      const a = byHeavy[map[j]], over = vals[map[j]] >= thr && vals[map[j]] > 0, lab = a.el !== "C" ? a.el : "";
      if (over) o += `<circle cx="${X(j).toFixed(1)}" cy="${Y(j).toFixed(1)}" r="${(0.26 * bl * k).toFixed(1)}" fill="none" stroke="var(--ink)" stroke-width="3.2"/>`;
      if (lab) o += `<text x="${X(j).toFixed(1)}" y="${(Y(j) + 4).toFixed(1)}" text-anchor="middle" font-size="12" font-weight="600" fill="var(--ink)" stroke="var(--panel)" stroke-width="3" paint-order="stroke">${esc(lab)}</text>`;
      if (over) o += `<text x="${X(j).toFixed(1)}" y="${(Y(j) - 0.34 * bl * k - 4).toFixed(1)}" text-anchor="middle" font-size="12" font-weight="700" fill="var(--ink)" stroke="var(--panel)" stroke-width="3" paint-order="stroke">${esc(indexOf(a))}</text>`;
    }
    return `<svg class="tpr-2d" viewBox="0 0 ${w.toFixed(0)} ${h.toFixed(0)}" role="img" aria-label="2D structure of ${esc(r.smiles)} with halos on the atoms">${o}</svg>`;
  }

  // ------------------------------------------------------------ the panel
  /** @param {HTMLElement} host */
  function clear(host) { const old = /** @type {any} */ (host.querySelector(".tpr")); if (old) { old._v3?.destroy(); old.remove(); } }

  /** the card Info @param {Reatt} r @returns {string} */
  function infoHtml(r) {
    const g = r.global || {}, c = r.conformers || {}, s = r.scc || {};
    const row = (/** @type {string} */ k, /** @type {any} */ v) => v === undefined || v === null || v === "" ? "" : `<tr><th scope="row" style="text-align:left">${esc(k)}</th><td style="text-align:left">${esc(v)}</td></tr>`;
    const conv = Array.isArray(s.converged) ? s.converged.map(x => (x ? "yes" : "no")).join(" / ") + " (N, N+1, N−1)" : s.converged === undefined ? "" : s.converged ? "yes" : "no";
    const p = r.protomer, sgn = (/** @type {number} */ n) => (n > 0 ? "+" : n < 0 ? "−" : "") + Math.abs(n);
    const form = p ? `pH ${p.ph}, ${p.smiles}` : "As drawn", charge = p ? sgn(p.charge) : "";
    const groups = p ? (p.changes.length ? p.changes.map(x => `${x.group} (typical pKa ${x.pka}, atom ${x.heavy_index + 1}) ${x.action}`).join("; ") : "none changed") : "";
    const ev = (/** @type {string} */ k, /** @type {string} */ l) => g[k] == null ? "" : row(l, num(g[k], 3) + " eV");
    return `<table class="tpr-t"><tbody>${row("Method", r.method)}${row("Solvent", r.solvent && r.solvent !== "none" ? r.solvent.charAt(0).toUpperCase() + r.solvent.slice(1) : "None")}${row("Engine", r.engine)}${row("Charges", r.charges ? r.charges.charAt(0).toUpperCase() + r.charges.slice(1) : "")}${row("SMILES", r.smiles)}${row("Form", form)}${row("Total charge", charge)}${row("Protonation changes", groups)}${row("Form note", p && p.note)}${row("Atoms", r.atoms.length)}`
      + `${row("Conformers", c.n !== undefined ? `${c.n}${c.chosen !== undefined ? ", chosen " + c.chosen : ""}${c.f_minus_spread != null ? ", spread of f(−) " + num(c.f_minus_spread, 3) : ""}` : "")}`
      + `${ev("homo_ev", "HOMO")}${ev("lumo_ev", "LUMO")}${ev("gap_ev", "HOMO–LUMO gap")}${ev("ip_ev", "Ionisation potential")}${ev("ea_ev", "Electron affinity")}${ev("mu_ev", "Chemical potential μ")}${ev("eta_ev", "Hardness η")}${ev("omega_ev", "Electrophilicity ω")}`
      + `${row("SCC converged", conv)}${row("SCC iterations", Array.isArray(s.iterations) ? s.iterations.join(" / ") : s.iterations)}</tbody></table>`;
  }
  /** «How to cite», from `reatt.cite`: texts, DOIs as links @param {Reatt} r @returns {string} */
  function citeHtml(r) {
    const items = (r.cite || []).map(c => {
      const t = typeof c === "string" ? c : String(c.text || c.title || ""), d = typeof c === "string" ? (c.match(/10\.\d{4,9}\/[^\s"<>]+/) || [""])[0] : String(c.doi || "");
      return `<li>${esc(t)}${d && !t.includes(d) ? ` <a href="https://doi.org/${esc(d)}" target="_blank" rel="noopener noreferrer">doi:${esc(d)}</a>` : ""}</li>`;
    }).join("");
    return `<details class="tpr-cite"><summary>How to cite</summary>${r.engine ? `<p>Engine: ${esc(r.engine)}.</p>` : ""}<ul>${items || "<li>No references given by the engine.</li>"}</ul></details>`;
  }

  /** builds the table per atom for the card in `view.card`, sortable @param {Reatt} r @param {{ card: number, thr: number, sort: string, dir: number }} view @returns {string} */
  function tableHtml(r, view) {
    const key = CARDS[view.card].key, cols = /** @type {[string, string][]} */ ([["idx", "Index"], ["el", "Element"], ["q", "q"], ["f_plus", "f(+)"], ["f_zero", "f(0)"], ["f_minus", "f(−)"], ["dual", "Dual"], ...(r.atoms.some(a => a.homo != null) ? [["homo", "HOMO share"], ["lumo", "LUMO share"]] : [])]);
    const val = (/** @type {RAtom} */ a, /** @type {string} */ c) => c === "idx" ? (a.heavy_index == null ? 1e6 + a.i : a.heavy_index) : c === "el" ? a.el : /** @type {any} */ (a)[c];
    const rows = r.atoms.slice().sort((a, b) => { const x = val(a, view.sort), y = val(b, view.sort); return (typeof x === "string" ? x.localeCompare(y) : x - y) * view.dir; });
    const head = cols.map(([c, l]) => `<th scope="col" aria-sort="${view.sort === c ? (view.dir > 0 ? "ascending" : "descending") : "none"}"><button type="button" data-sort="${c}" aria-label="Sort by ${esc(l)}">${esc(l)}</button></th>`).join("") + `<th scope="col">Above</th>`;
    const body = rows.map(a => { const over = /** @type {number} */ (a[key]) >= view.thr && /** @type {number} */ (a[key]) > 0;
      return `<tr${over ? ' class="over"' : ""} data-over="${over ? 1 : 0}"><td>${esc(indexOf(a))}</td><td>${esc(a.el)}</td><td>${num(a.q)}</td><td>${num(a.f_plus)}</td><td>${num(a.f_zero)}</td><td>${num(a.f_minus)}</td><td>${num(a.dual)}</td>${a.homo != null ? `<td>${num(a.homo)}</td><td>${num(a.lumo)}</td>` : ""}<td>${over ? "▲ yes" : ""}</td></tr>`; }).join("");
    return `<table class="tpr-t" aria-label="Values per atom, card ${esc(CARDS[view.card].label)}, threshold ${view.thr}"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
  }
  /** @param {Reatt} r @returns {string} the sites C–H by class of symmetry, when the `reatt/1` carries them */
  function sitesHtml(r) {
    if (!r.ch_sites || !r.ch_sites.length) return "";
    return `<h4>C–H sites by class of symmetry</h4><table class="tpr-t" aria-label="C–H sites by class of symmetry"><thead><tr><th scope="col">Class</th><th scope="col">Heavy atom</th><th scope="col">H</th><th scope="col">Prev. f(−)</th></tr></thead><tbody>`
      + r.ch_sites.map(s => `<tr><td>${esc(s.class)}</td><td>${s.heavy_index + 1}</td><td>${s.n_h}</td><td>${num(s.f_minus)}</td></tr>`).join("") + `</tbody></table>`;
  }

  /** @param {HTMLElement} host @param {Reatt} r the `reatt/1` @returns {boolean} true when the panel was drawn */
  function mount(host, r) {
    clear(host); if (!admin() || !r || !r.atoms) return false;
    injectCss(); tell(host, { reatt: r });
    const root = document.createElement("div"); root.className = "card tpr tpv3"; host.appendChild(root);
    const view = { card: 0, tab: 0, thr: 0.05, sort: "idx", dir: 1 };
    root.innerHTML = `<h4>Reactivity of the parent (Prev.)</h4><div class="tpr-tabs" role="tablist" aria-label="Reactivity cards">${TABS.map((t, k) => `<button type="button" role="tab" id="tpr-t${k}" aria-selected="${k === 0}" tabindex="${k === 0 ? 0 : -1}" aria-controls="tpr-p">${esc(t)}</button>`).join("")}</div>`
      + `<div id="tpr-p" role="tabpanel" aria-live="polite"></div><div class="tpr-ex" role="group" aria-label="Export"></div>${citeHtml(r)}`;
    const panel = /** @type {HTMLElement} */ (root.querySelector("#tpr-p")), ex = /** @type {HTMLElement} */ (root.querySelector(".tpr-ex"));
    const base = (r.smiles || "parent").replace(/[^A-Za-z0-9]+/g, "_").slice(0, 40) || "parent";
    [["XYZ", () => download(`reactivity_${base}.xyz`, xyz(r), "chemical/x-xyz")], ["SMILES", () => download(`reactivity_${base}.smi`, r.smiles + "\n", "text/plain")],
      ["JSON", () => download(`reactivity_${base}.json`, JSON.stringify(r, null, 2), "application/json")], ["CSV", () => download(`reactivity_${base}.csv`, csv(r), "text/csv")]]
      .forEach(([l, f]) => { const b = document.createElement("button"); b.type = "button"; b.textContent = "Export " + l; b.dataset.ex = String(l); b.addEventListener("click", /** @type {any} */ (f)); ex.appendChild(b); });
    /** @type {any} */ let viewer = null, mol = null, map = null;
    const byHeavy = /** @type {RAtom[]} */ ([]); r.atoms.forEach(a => { if (a.heavy_index != null) byHeavy[a.heavy_index] = a; });

    const refresh2d = () => {
      const f = panel.querySelector(".tpr-fig2"); if (!f || !mol || !map) return;
      const key = CARDS[view.card].key; f.innerHTML = svg2d(mol, map, r, byHeavy, byHeavy.map(a => /** @type {number} */ (a[key])), view.thr);
      const el = f.querySelector("svg");
      if (el && G.TDATI) { G.TDATI.describe(el, `2D structure with halos, card ${CARDS[view.card].label}, ${byHeavy.length} heavy atoms.`); G.TDATI.addButton(el, tdati); }
    };
    const tdati = () => ({ title: `Reactivity per atom, ${CARDS[view.card].label}`, head: ["Index", "Element", "q", "f(+)", "f(0)", "f(−)", "Dual", ...(r.atoms.some(a => a.homo != null) ? ["HOMO share", "LUMO share"] : [])], rows: r.atoms.map(a => [indexOf(a), a.el, num(a.q), num(a.f_plus), num(a.f_zero), num(a.f_minus), num(a.dual), ...(a.homo != null ? [num(a.homo), num(a.lumo)] : [])]) });
    const refreshTable = () => { const t = panel.querySelector(".tpr-tab"); if (!t) return; t.innerHTML = tableHtml(r, view) + sitesHtml(r); bindSort(); };
    const refreshViews = () => {
      const key = CARDS[view.card].key;
      viewer?.setValues(r.atoms.map(a => /** @type {number} */ (a[key])), CARDS[view.card].label, view.thr, r.atoms.map(indexOf));
      refresh2d(); refreshTable();
      const n = r.atoms.filter(a => /** @type {number} */ (a[key]) >= view.thr && /** @type {number} */ (a[key]) > 0).length, c = panel.querySelector(".tpr-count"); if (c) c.textContent = `${n} atom${n === 1 ? "" : "s"} at or above ${view.thr}`;
    };
    const bindSort = () => panel.querySelectorAll("button[data-sort]").forEach(b => b.addEventListener("click", () => {
      const c = /** @type {HTMLElement} */ (b).dataset.sort || "idx"; view.dir = view.sort === c ? -view.dir : 1; view.sort = c; refreshTable();
      /** @type {HTMLElement|null} */ (panel.querySelector(`button[data-sort="${c}"]`))?.focus();
    }));
    function showCard() {
      panel.innerHTML = `<div class="tpr-figs"><div class="tpr-fig2"></div><div class="tpr-fig3"></div></div>`
        + `<div class="tpr-thr"><label for="tpr-rng">Threshold</label><input id="tpr-rng" type="range" min="0" max="0.15" step="0.005" value="${view.thr}" aria-label="Threshold, slider">`
        + `<input id="tpr-num" type="number" min="0" max="0.15" step="0.005" value="${view.thr}" aria-label="Threshold, value"><span class="tpr-count" role="status"></span></div><div class="tpr-tab" style="overflow-x:auto"></div>`;
      viewer = null; const host3 = /** @type {HTMLElement} */ (panel.querySelector(".tpr-fig3"));
      viewer = G.TPV3D.create(host3, { atoms: r.atoms, bonds: r.bonds, hbonds: r.hbonds, label: r.smiles });
      if (G.TDATI) { G.TDATI.describe(viewer.canvas, `3D model of ${r.smiles}, ${r.atoms.length} atoms.`); G.TDATI.addButton(viewer.canvas, tdati); }
      const rng = /** @type {HTMLInputElement} */ (panel.querySelector("#tpr-rng")), nm = /** @type {HTMLInputElement} */ (panel.querySelector("#tpr-num"));
      const set = (/** @type {string} */ s, /** @type {boolean} */ typing) => { const v = Math.min(0.15, Math.max(0, parseFloat(s))); if (!Number.isFinite(v)) return; view.thr = v; rng.value = String(v); if (!typing) nm.value = String(v); refreshViews(); };
      rng.addEventListener("input", () => set(rng.value, false)); nm.addEventListener("input", () => set(nm.value, true));
      refreshViews();
    }
    const show = (/** @type {number} */ k) => {
      view.tab = k; root.querySelectorAll('[role="tab"]').forEach((b, i) => { b.setAttribute("aria-selected", String(i === k)); /** @type {HTMLElement} */ (b).tabIndex = i === k ? 0 : -1; });
      panel.setAttribute("aria-labelledby", "tpr-t" + k); viewer?.destroy(); viewer = null;
      if (k < 3) { view.card = k; showCard(); }
      else if (k === 3) panel.innerHTML = infoHtml(r);
      else if (k === 4) panel.innerHTML = `<ul>${NOTES.concat(r.notes || []).map(n => `<li>${esc(n)}</li>`).join("")}</ul>`;
      else panel.innerHTML = `<pre tabindex="0" aria-label="reatt/1 as JSON, read only"></pre>`, /** @type {HTMLElement} */ (panel.querySelector("pre")).textContent = JSON.stringify(r, null, 2);
    };
    const tabs = [...root.querySelectorAll('[role="tab"]')];
    tabs.forEach((b, k) => { b.addEventListener("click", () => show(k)); b.addEventListener("keydown", e => {
      const ev = /** @type {KeyboardEvent} */ (e), d = ev.key === "ArrowRight" ? 1 : ev.key === "ArrowLeft" ? -1 : 0, j = ev.key === "Home" ? 0 : ev.key === "End" ? tabs.length - 1 : (k + d + tabs.length) % tabs.length;
      if (d || ev.key === "Home" || ev.key === "End") { ev.preventDefault(); show(j); /** @type {HTMLElement} */ (tabs[j]).focus(); } }); });
    /** @type {any} */ (root)._v3 = { destroy: () => viewer?.destroy() };
    /** @type {any} */ (root)._api = { view, viewer: () => viewer };
    show(0);
    ocl().then(OCL => {                                        // the drawing needs OpenChemLib: without it the panel works without the 2D structure
      try { mol = OCL.Molecule.fromSmiles(r.smiles); map = mapHeavy(mol, r); if (!map) { mol = null; } } catch (_) { mol = null; map = null; }
      if (view.tab < 3) refresh2d();
    }).catch(() => {});
    return true;
  }

  /**
   * the state of the calculation, in the place of the panel: waiting for TPs, computing, failed with the message of its error_key (done is `mount`)
   * @param {HTMLElement} host @param {{ state: "waiting"|"computing"|"failed", error_key?: string, message?: string }|string} s @returns {boolean}
   */
  function state(host, s) {
    clear(host); if (!admin()) return false;
    injectCss();
    const o = typeof s === "string" ? { state: s } : s, st = /** @type {any} */ (o);
    tell(host, { state: st.state });
    const text = st.state === "waiting" ? "waiting for TPs" : st.state === "computing" ? "computing…" : `failed: ${ERR[st.error_key || ""] || st.message || st.error_key || "unknown error"}`;
    const root = document.createElement("div"); root.className = "card tpr"; root.dataset.state = st.state;
    root.innerHTML = `<h4>Reactivity of the parent (Prev.)</h4><p class="tpr-st" role="status">${esc(text)}</p>`; host.appendChild(root);
    return true;
  }

  /** tells the other panels of the parent (the column «Prev. f(−)» of «Substitution position», TPPOS) what this panel shows @param {HTMLElement} host @param {any} d */
  function tell(host, d) { const smiles = host.dataset.smiles || (d.reatt && d.reatt.smiles) || ""; if (smiles) G.dispatchEvent(new CustomEvent("tp-reatt", { detail: { smiles, ...d } })); }

  G.TPREATT = { mount, state, xyz, csv, mapHeavy, svg2d, eligible: () => admin() };
})();
