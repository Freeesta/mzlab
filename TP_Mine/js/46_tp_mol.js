// @ts-check
// TPMINE-PRIVATE  TPMOL: the drawing of a transformation product on its parent, in the notation a chemist would use (`drawing` of `interp/1`, chosen by the engine from the confidence of the position):
// a full structure when the site is certain, one hydroxyl bond from the centre of the ring (variable attachment, as in ChemDraw) for a substitution on an aromatic ring, square brackets with the change («+O»,
// «−2H») for any other zone. Never one drawing per possible position. The zone the fragments point to is shaded and hatched; a zone of the uncertain case gets its class number and its own dash.
// 2D coordinates come from OpenChemLib (the SMILES of the parent, atoms in SMILES order), so the drawing in the Draw tab is never touched. Brackets and the dashed attachment are `parentesi.js`.
// Nothing here is chemistry: atoms, ends and labels arrive from the engine. «Open in Draw» builds the V3000 that the Draw tab reads (S-group DAT «MZLAB» / «MZLAB_ENDPTS», docs/agenti/disegno.md).
// Colours only from CSS variables (Okabe-Ito `--oi1..6`); every cue also has a shape (hatch, dash, number). No animation. Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const SRC = /** @type {HTMLScriptElement | null} */ (document.currentScript);
  const PAR_URL = new URL("../parentesi.js", SRC ? SRC.src : location.href).href;        // static/parentesi.js, next to static/mzfinder/
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  /**
   * @typedef {{ el: string, x: number, y: number }} MAtom
   * @typedef {{ a: number, b: number, o: number }} MBond
   * @typedef {{ atoms: MAtom[], bonds: MBond[] }} MGraph
   * @typedef {{ mode: "structure" | "multicenter" | "bracket", delta: string, label?: string, uncertainty?: "none" | "zone" | "whole", atoms?: number[], ends?: number[], sub?: number | null,
   *   highlight?: number[], zones?: { atoms: number[], support?: number, class?: number }[], kept?: number[], region?: string }} Drawing
   */

  // Okabe-Ito as variables (the same six as the panel of positions), the natural size of the drawing and its limit to the width of the box
  if (!document.getElementById("tpm-css")) {
    const st = document.createElement("style"); st.id = "tpm-css";
    st.textContent = ".tpm-svg{--oi1:#0072b2;--oi2:#e69f00;--oi3:#009e73;--oi4:#cc79a7;--oi5:#56b4e9;--oi6:#d55e00;max-width:100%;height:auto;display:block}";
    document.head.appendChild(st);
  }

  /** @type {Promise<any> | null} */ let oclP = null;
  const ocl = () => (oclP = oclP || import(new URL("vendor/openchemlib.js", G.QTOOLS.ctx.base).href));
  /** @type {Promise<any> | null} */ let parP = null;
  const par = () => (parP = parP || import(PAR_URL));
  /** @type {Map<string, Promise<MGraph | null>>} */ const cache = new Map();

  /** the heavy atoms and bonds of a SMILES with 2D coordinates (screen axes: y down); null when it cannot be read @param {string} smiles @returns {Promise<MGraph | null>} */
  function graph(smiles) {
    let p = cache.get(smiles);
    if (!p) {
      p = ocl().then(O => {
        const m = O.Molecule.fromSmiles(smiles); m.inventCoordinates();
        /** @type {MGraph} */ const g = { atoms: [], bonds: [] };
        for (let i = 0; i < m.getAllAtoms(); i++) g.atoms.push({ el: m.getAtomLabel(i), x: m.getAtomX(i), y: m.getAtomY(i) });
        for (let b = 0; b < m.getAllBonds(); b++) g.bonds.push({ a: m.getBondAtom(0, b), b: m.getBondAtom(1, b), o: Math.min(3, Math.max(1, m.getBondOrder(b))) });
        return g;
      }).catch(() => null);
      cache.set(smiles, p);
    }
    return p;
  }

  const mean = (/** @type {number[]} */ v) => v.reduce((s, x) => s + x, 0) / (v.length || 1);
  const unit = (/** @type {number} */ x, /** @type {number} */ y) => { const l = Math.hypot(x, y) || 1; return { x: x / l, y: y / l }; };
  const neighbours = (/** @type {MGraph} */ g, /** @type {number} */ i) => g.bonds.filter(b => b.a === i || b.b === i).map(b => (b.a === i ? b.b : b.a));
  /** the atoms of `drawing` that exist in the graph (the engine and the page agree on the order of the SMILES; a stray index is dropped, never drawn elsewhere) */
  const valid = (/** @type {MGraph} */ g, /** @type {number[] | undefined} */ v) => (v || []).filter(i => Number.isInteger(i) && i >= 0 && i < g.atoms.length);

  /** the atom a change adds, as drawn: «+O» is a hydroxyl, «+Cl» and «+Br» the halogen; null for a change that adds no atom (−2H, −CH2, +O−2H) @param {string} delta */
  function added(delta) {
    const m = /^\+(\d*)([A-Z][a-z]?)$/.exec(String(delta || "").replace(/\s+/g, ""));
    if (!m || m[1] && m[1] !== "1") return null;
    return m[2] === "O" ? "OH" : m[2];
  }

  /** the whole parent in brackets: every atom of the drawing @param {MGraph} g */
  const all = (g) => g.atoms.map((_, i) => i);

  /** true when the atoms in `ids` form one closed ring of their own bonds (each has exactly two neighbours inside the set) @param {MGraph} g @param {number[]} ids */
  function wholeRing(g, ids) {
    const s = new Set(ids);
    return ids.length >= 3 && ids.every(i => neighbours(g, i).filter(j => s.has(j)).length === 2);
  }

  /**
   * where the substituent of a multicentre attachment goes: out of the ring through the middle of one of its bonds (never through an atom, which would read as an attachment to that atom), on the side
   * with most room. Returns the centre of the candidates, the point where the bond ends and the label sits.
   * @param {MGraph} g @param {number[]} ends
   */
  function outward(g, ends) {
    const c = { x: mean(ends.map(i => g.atoms[i].x)), y: mean(ends.map(i => g.atoms[i].y)) }, R = mean(ends.map(i => Math.hypot(g.atoms[i].x - c.x, g.atoms[i].y - c.y))) || 1;
    const set = new Set(ends), others = g.atoms.map((_, i) => i).filter(i => !set.has(i));
    const dirs = g.bonds.filter(b => set.has(b.a) && set.has(b.b)).map(b => unit((g.atoms[b.a].x + g.atoms[b.b].x) / 2 - c.x, (g.atoms[b.a].y + g.atoms[b.b].y) / 2 - c.y));
    if (!dirs.length) { const m = { x: mean(g.atoms.map(a => a.x)), y: mean(g.atoms.map(a => a.y)) }; dirs.push(unit(c.x - m.x, c.y - m.y)); }
    let best = dirs[0], room = -1;
    for (const d of dirs) {
      const p = { x: c.x + d.x * R * 2.2, y: c.y + d.y * R * 2.2 }, r = Math.min(...others.map(i => Math.hypot(g.atoms[i].x - p.x, g.atoms[i].y - p.y)), 9);
      if (r > room) { room = r; best = d; }
    }
    return { c, d: best, R };
  }

  /** the sentence read by assistive technology and shown under the drawing @param {Drawing} d @param {MGraph} g */
  function describe(d, g) {
    const label = d.label || d.delta || "", ids = (d.mode === "multicenter" ? valid(g, d.ends) : valid(g, d.atoms)).map(i => `${g.atoms[i].el}${i + 1}`);
    const where = d.mode === "structure" ? `site ${ids.join(", ")}` : d.uncertainty === "whole" ? "position not determined" : d.mode === "multicenter" ? `position on one of ${ids.join(", ")}` : `position within ${ids.join(", ")}`;
    return `Parent with ${label}: ${where}`;
  }
  /** the short line under the drawing: how sure the position is @param {Drawing} d */
  function caption(d) {
    if (d.uncertainty === "whole") return "Position not determined";
    if (d.uncertainty === "zone") return d.region ? `Position uncertain within ${d.region}` : "Position uncertain";
    return "";
  }

  /**
   * The drawing as an SVG string (viewBox, role «img», aria-label, `<title>`); null when the SMILES cannot be drawn.
   * @param {string} smiles parent SMILES @param {Drawing} d @returns {Promise<string | null>}
   */
  async function svg(smiles, d) {
    const g = await graph(smiles); if (!g || !g.atoms.length) return null;
    const P = await par(), L = 30;                                                       // bond length in px
    const xs = g.atoms.map(a => a.x), ys = g.atoms.map(a => a.y);
    const bl = mean(g.bonds.map(b => Math.hypot(g.atoms[b.a].x - g.atoms[b.b].x, g.atoms[b.a].y - g.atoms[b.b].y))) || 1;
    const k = L / bl, x0 = Math.min(...xs), y0 = Math.min(...ys);
    /** @type {{ x: number, y: number }[]} */ const pt = g.atoms.map(a => ({ x: (a.x - x0) * k, y: (a.y - y0) * k }));
    const doc = document.implementation.createDocument("http://www.w3.org/2000/svg", "svg", null), root = doc.documentElement, grp = (/** @type {Element} */ p) => p.appendChild(doc.createElementNS("http://www.w3.org/2000/svg", "g"));
    const shade = grp(root), bondsG = grp(root), labelsG = grp(root), over = grp(root);
    const el = (/** @type {Element} */ p, /** @type {string} */ tag, /** @type {Record<string, string | number>} */ at, /** @type {string} */ text = "") => {
      const e = doc.createElementNS("http://www.w3.org/2000/svg", tag); for (const [n, v] of Object.entries(at)) e.setAttribute(n, String(v)); if (text) e.textContent = text; p.appendChild(e); return e;
    };
    const f = (/** @type {number} */ v) => v.toFixed(1), keep = d.mode === "structure" && d.kept && d.kept.length ? new Set(valid(g, d.kept)) : null;
    const hetero = new Set(g.atoms.map((a, i) => (a.el !== "C" ? i : -1)).filter(i => i >= 0));
    let minX = 0, minY = 0, maxX = Math.max(...pt.map(p => p.x)), maxY = Math.max(...pt.map(p => p.y));
    const grow = (/** @type {number} */ x, /** @type {number} */ y) => { minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); };

    // the parent: bonds shortened at hetero atoms; the lost part of a loss product is grey and dashed
    for (const b of g.bonds) {
      const A = pt[b.a], B = pt[b.b], u = unit(B.x - A.x, B.y - A.y), ga = hetero.has(b.a) ? 9 : 0, gb = hetero.has(b.b) ? 9 : 0, lost = keep && !(keep.has(b.a) && keep.has(b.b));
      const nrm = { x: -u.y, y: u.x }, off = b.o === 1 ? [0] : b.o === 2 ? [-2.2, 2.2] : [-3.4, 0, 3.4];
      for (const o of off) el(bondsG, "line", { x1: f(A.x + u.x * ga + nrm.x * o), y1: f(A.y + u.y * ga + nrm.y * o), x2: f(B.x - u.x * gb + nrm.x * o), y2: f(B.y - u.y * gb + nrm.y * o),
        stroke: lost ? "var(--muted)" : "var(--ink)", "stroke-width": 1.6, ...(lost ? { "stroke-dasharray": "3 3" } : {}) });
    }
    g.atoms.forEach((a, i) => {
      if (a.el === "C") return;
      el(labelsG, "text", { x: f(pt[i].x), y: f(pt[i].y + 4.5), "text-anchor": "middle", "font-size": 13, "font-weight": 600, "font-family": "Arial, sans-serif",
        fill: keep && !keep.has(i) ? "var(--muted)" : "var(--ink)", stroke: "var(--panel)", "stroke-width": 3, "paint-order": "stroke" }, a.el);
    });

    // the zone the fragments point to: translucent colour plus a hatch, so the colour is never the only cue
    const defs = el(root, "defs", {}), pid = "tpm-h" + Math.random().toString(36).slice(2, 8);
    const hatch = el(defs, "pattern", { id: pid, width: 5, height: 5, patternUnits: "userSpaceOnUse", patternTransform: "rotate(45)" });
    el(hatch, "line", { x1: 0, y1: 0, x2: 0, y2: 5, stroke: "var(--oi6)", "stroke-width": 1.6 });
    const halo = (/** @type {number[]} */ ids, /** @type {string} */ colour, /** @type {number} */ op, /** @type {string} */ dash, /** @type {boolean} */ hatched) => {
      for (const i of ids) {
        if (hatched) el(shade, "circle", { cx: f(pt[i].x), cy: f(pt[i].y), r: 0.5 * L, fill: `url(#${pid})`, "fill-opacity": 0.9 });
        el(shade, "circle", { cx: f(pt[i].x), cy: f(pt[i].y), r: 0.5 * L, fill: colour, "fill-opacity": op, stroke: colour, "stroke-width": 1.4, ...(dash ? { "stroke-dasharray": dash } : {}) });
      }
    };
    const hi = valid(g, d.highlight);
    if (hi.length) halo(hi, "var(--oi6)", 0.2, "", true);
    const DASH = ["", "6 3", "2 2", "8 2 2 2", "4 4", "1 3"], zones = d.mode === "bracket" ? d.zones || [] : [];
    zones.forEach((z, zi) => {
      const ids = valid(g, z.atoms), colour = `var(--oi${zi % 6 + 1})`;
      halo(ids, colour, 0.12 + 0.3 * Math.max(0, Math.min(1, z.support ?? 0.5)), DASH[zi % DASH.length], false);
      if (ids.length) el(over, "text", { x: f(mean(ids.map(i => pt[i].x))), y: f(Math.min(...ids.map(i => pt[i].y)) - 0.6 * L), "text-anchor": "middle", "font-size": 11, "font-weight": 700, fill: "var(--ink)", stroke: "var(--panel)", "stroke-width": 3, "paint-order": "stroke" }, String(z.class ?? zi + 1));
    });

    const label = d.label || d.delta || "", add = added(d.delta), ink = "var(--ink)";
    const text = (/** @type {number} */ x, /** @type {number} */ y, /** @type {string} */ s) => el(over, "text", { x: f(x), y: f(y + 4.5), "text-anchor": "middle", "font-size": 13, "font-weight": 600, "font-family": "Arial, sans-serif", fill: ink, stroke: "var(--panel)", "stroke-width": 3, "paint-order": "stroke" }, s);
    let drawn = false;
    if (d.mode === "structure" && add && valid(g, d.atoms).length === 1) {
      const i = /** @type {number} */ (valid(g, d.atoms)[0]), nb = neighbours(g, i), v = unit(-mean(nb.map(j => pt[j].x - pt[i].x)), -mean(nb.map(j => pt[j].y - pt[i].y))), s = { x: pt[i].x + v.x * L, y: pt[i].y + v.y * L };
      el(bondsG, "line", { x1: f(pt[i].x + v.x * (hetero.has(i) ? 9 : 0)), y1: f(pt[i].y + v.y * (hetero.has(i) ? 9 : 0)), x2: f(s.x - v.x * 9), y2: f(s.y - v.y * 9), stroke: ink, "stroke-width": 1.6 });
      text(s.x, s.y, add); grow(s.x - 12, s.y - 12); grow(s.x + 12, s.y + 12); drawn = true;
    } else if (d.mode === "multicenter" && add && valid(g, d.ends).length >= 2) {
      const ends = valid(g, d.ends), o = outward(g, ends), end = { x: o.c.x + o.d.x * (o.R + 0.9 * L), y: o.c.y + o.d.y * (o.R + 0.9 * L) }, lab = { x: end.x + o.d.x * 9, y: end.y + o.d.y * 9 };
      // a whole ring needs no circles on its atoms: the bond from its centre says «any of these»
      const mc = P.drawMulticenter(over, end, ends.map(i => pt[i]), { r: 0.14 * L });
      if (mc && wholeRing(g, ends)) mc.querySelectorAll("circle").forEach((/** @type {Element} */ c) => c.remove());
      text(lab.x, lab.y, add); grow(lab.x - 14, lab.y - 14); grow(lab.x + 14, lab.y + 14); drawn = true;
    }
    if (!drawn) {                                                        // brackets: the zone, or the whole parent, with the change written at the upper right
      const ids = d.mode === "bracket" ? valid(g, d.atoms) : d.mode === "multicenter" ? valid(g, d.ends) : valid(g, d.atoms), use = ids.length ? ids : all(g), bx = P.bracketBox(use.map(i => pt[i]), 0.6 * L);
      const bg = P.drawBrackets(over, use.map(i => pt[i]), { text: label, pad: 0.6 * L, size: 14 });
      if (bg) bg.querySelectorAll("text").forEach((/** @type {Element} */ t) => { t.setAttribute("stroke", "var(--panel)"); t.setAttribute("stroke-width", "3"); t.setAttribute("paint-order", "stroke"); });   // the change stays readable over a bond
      grow(bx.x0 - 4, bx.y0 - 4); grow(bx.x1 + 8 + 9 * label.length, bx.y1 + 4);
    }
    const cap = caption(d), W = maxX - minX + 2 * 16, w = Math.max(W, 8 * cap.length + 24), shift = (w - W) / 2;
    if (cap) el(root, "text", { x: f(w / 2), y: f(maxY - minY + 2 * 16 + 14), "text-anchor": "middle", "font-size": 12, fill: "var(--muted)" }, cap);
    const h = maxY - minY + 2 * 16 + (cap ? 22 : 0), tr = `translate(${f(16 - minX + shift)},${f(16 - minY)})`;
    for (const q of [shade, bondsG, labelsG, over]) q.setAttribute("transform", tr);
    root.setAttribute("viewBox", `0 0 ${f(w)} ${f(h)}`); root.setAttribute("width", f(w)); root.setAttribute("height", f(h)); root.setAttribute("role", "img"); root.setAttribute("class", "tpm-svg");
    const say = describe(d, g) + (cap ? `. ${cap}` : "");
    root.setAttribute("aria-label", say);
    const t = doc.createElementNS("http://www.w3.org/2000/svg", "title"); t.textContent = say; root.insertBefore(t, root.firstChild);
    return new XMLSerializer().serializeToString(root);
  }

  /**
   * The V3000 molfile the Draw tab reads («Open in Draw»): the parent with the change, written as the Draw tab does (docs/agenti/disegno.md): `structure` = the product with its new atom;
   * `bracket` = the zone as a data S-group «MZLAB» whose data is the change; `multicenter` = the new atom bonded to the first candidate plus a data S-group «MZLAB_ENDPTS» («ANY») on the candidates and it.
   * @param {string} smiles @param {Drawing} d @returns {Promise<string | null>}
   */
  async function molfile(smiles, d) {
    const g = await graph(smiles); if (!g || !g.atoms.length) return null;
    const add = added(d.delta), f = (/** @type {number} */ v) => v.toFixed(4), atoms = g.atoms.map(a => ({ ...a })), bonds = g.bonds.map(b => ({ ...b }));
    const L = mean(g.bonds.map(b => Math.hypot(g.atoms[b.a].x - g.atoms[b.b].x, g.atoms[b.a].y - g.atoms[b.b].y))) || 1;
    /** @type {{ key: string, data: string, ids: number[] } | null} */ let sg = null;
    const place = (/** @type {number} */ host, /** @type {{ x: number, y: number }} */ v) => {
      atoms.push({ el: /** @type {string} */ (add).replace(/H$/, ""), x: g.atoms[host].x + v.x * L, y: g.atoms[host].y + v.y * L }); bonds.push({ a: host, b: atoms.length - 1, o: 1 }); return atoms.length - 1;
    };
    if (d.mode === "structure" && add && valid(g, d.atoms).length === 1) {
      const i = /** @type {number} */ (valid(g, d.atoms)[0]), nb = neighbours(g, i); place(i, unit(-mean(nb.map(j => g.atoms[j].x - g.atoms[i].x)), -mean(nb.map(j => g.atoms[j].y - g.atoms[i].y))));
    } else if (d.mode === "multicenter" && add && valid(g, d.ends).length >= 2) {
      const ends = valid(g, d.ends), o = outward(g, ends), host = ends.reduce((p, q) => (Math.hypot(g.atoms[q].x - (o.c.x + o.d.x * o.R), g.atoms[q].y - (o.c.y + o.d.y * o.R)) < Math.hypot(g.atoms[p].x - (o.c.x + o.d.x * o.R), g.atoms[p].y - (o.c.y + o.d.y * o.R)) ? q : p));
      const n = place(host, o.d); sg = { key: "MZLAB_ENDPTS", data: "ANY", ids: [...ends, n] };
    } else {
      const ids = valid(g, d.atoms); sg = { key: "MZLAB", data: d.delta, ids: ids.length ? ids : all(g) };
    }
    const lines = ["", "  mzLab", "", "  0  0  0     0  0            999 V3000", "M  V30 BEGIN CTAB", `M  V30 COUNTS ${atoms.length} ${bonds.length} ${sg ? 1 : 0} 0 0`, "M  V30 BEGIN ATOM",
      ...atoms.map((a, i) => `M  V30 ${i + 1} ${a.el} ${f(a.x)} ${f(-a.y)} 0 0`), "M  V30 END ATOM", "M  V30 BEGIN BOND", ...bonds.map((b, i) => `M  V30 ${i + 1} ${b.o} ${b.a + 1} ${b.b + 1}`), "M  V30 END BOND"];
    if (sg) lines.push("M  V30 BEGIN SGROUP", `M  V30 1 DAT 0 ATOMS=(${sg.ids.length} ${sg.ids.map(i => i + 1).join(" ")}) FIELDNAME=${sg.key} FIELDDATA="${sg.data}" FIELDTYPE=T`, "M  V30 END SGROUP");
    lines.push("M  V30 END CTAB", "M  END");
    return lines.join("\n");
  }

  /** shows the drawing in the Draw tab: the molfile is loaded in the editor and the view changes @param {string} smiles @param {Drawing} d */
  async function openInDraw(smiles, d) {
    const mf = await molfile(smiles, d);
    if (typeof G.setView === "function") G.setView("draw");                      // the editor starts only when its view is open
    await G.TPDraw.load(mf || smiles);
  }

  G.TPMOL = { graph, svg, molfile, openInDraw, caption, describe, added };
})();
