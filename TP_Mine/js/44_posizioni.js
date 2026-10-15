// @ts-check
// TPMINE-PRIVATE  TPPOS: the button «Substitution position» in the open row of a TP (administrator mode only) and its panel: the classes of positions the MS2 of the TP supports, excludes or
// cannot tell apart (`posiz/1`, docs/agenti/posizioni.md), the evidence, the type of oxidation and the checks that would settle it. A proposal 3a/3b is written as a proposal: no button saves a level.
// The engine is the `positions` of the `.wasm` «grafo» (verified against its SHA-256 in wasm-feat/manifest.json) or, in the app, the native route `feat/grafo`.
// Colours only from CSS variables (Okabe-Ito); every class also has its own marker shape and dash; the spectrum has its table (TDATI), zoom by buttons and keys, no animation. Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const admin = () => typeof G.uipAdminOn === "function" && G.uipAdminOn();

  /** the changes the panel understands, by their spelling in the rows of Tabui */
  const TRANSFORMS = /** @type {Record<string, string>} */ ({ "+O": "+O", "-CH2": "-CH2", "+O-2H": "+O-2H", "+O-H2": "+O-2H", "+Cl-H": "+Cl-H", "+Br-H": "+Br-H" });
  /** @param {any} delta @returns {string} the change as the engine spells it, «» when the panel does not cover it */
  const transformOf = delta => TRANSFORMS[String(delta ?? "").replace(/\s+/g, "").replace(/[−–]/g, "-")] || "";

  const DOI = { ox: "10.1021/ac9911692", hdx: "10.3390/analytica7030057", ms3: "10.1007/s11419-018-0410-4" };
  const doi = (/** @type {string} */ d, /** @type {string} */ t) => `<a href="https://doi.org/${d}" target="_blank" rel="noopener noreferrer">${esc(t)}</a>`;
  const ERR = /** @type {Record<string, string>} */ ({
    "error.posiz.noSites": "The parent has no atom where this change can sit.", "error.posiz.transform": "This change is not one of those the panel covers.",
    "error.posiz.formula": "The sites of this change do not give the same formula.", "error.posiz.peaks": "The MS2 peaks could not be read.", "error.posiz.tol": "The tolerance is not valid.",
  });

  // ------------------------------------------------------------ engine
  /** @type {any} */ let grafo = null;
  /** @type {any} */ let pool = null;
  /** @param {string} smiles @param {string} transform @param {number[][]} peaks @param {number} tol @param {boolean} ppm @param {boolean} positive @returns {Promise<any>} the `posiz/1` */
  async function compute(smiles, transform, peaks, tol, ppm, positive) {
    const pj = JSON.stringify(peaks);
    if (G.MZLAB_DESKTOP) {                                  // the app: the native route feat/grafo through the pool of the Pipeline
      if (!pool) { const url = URL.createObjectURL(new Blob([G.QTOOLS.ctx.files["feat-desktop-pool.js"]], { type: "text/javascript" })); pool = (await import(url)).createDesktopPool({ notify() {} }); }
      return JSON.parse(await pool.positions(smiles, transform, pj, tol, ppm, positive));
    }
    if (!grafo) {
      const u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, G.QTOOLS.ctx.base).href, g = (await (await fetch(u("manifest.json"))).json()).grafo;
      if (!g) throw Object.assign(new Error("error.featFailed"), { key: "error.featFailed" });
      const bytes = await (await fetch(u(g.wasm))).arrayBuffer(), hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
      if (g.sha256 && hex(await crypto.subtle.digest("SHA-256", bytes)) !== g.sha256) throw Object.assign(new Error("error.featWasmHash"), { key: "error.featWasmHash" });
      const m = await import(u(g.js)); await m.default({ module_or_path: bytes }); grafo = m;
    }
    try { return JSON.parse(grafo.positions(smiles, transform, pj, tol, ppm, positive)); }
    catch (e) { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } throw j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key }) : e; }
  }

  // ------------------------------------------------------------ atoms of the SMILES (symbols for the table)
  /** the element symbols of a SMILES in the order of its atoms (the order of the indices of `posiz/1`) @param {string} smiles @returns {string[]} */
  function symbols(smiles) {
    const out = /** @type {string[]} */ ([]), re = /\[(?:\d+)?([A-Z][a-z]?|[a-z]{1,2})[^\]]*\]|Cl|Br|[BCNOPSFI]|[bcnops]/g;
    for (const m of smiles.matchAll(re)) { const s = m[1] || m[0]; out.push(s.charAt(0).toUpperCase() + s.slice(1)); }
    return out;
  }

  // ------------------------------------------------------------ look
  const CSS = `
.tp .tpp{--oi1:#0072b2;--oi2:#e69f00;--oi3:#009e73;--oi4:#cc79a7;--oi5:#56b4e9;--oi6:#d55e00}
.tp .tpp h4{margin:10px 0 3px;font-size:12px;color:var(--muted);font-weight:600}
.tp .tpp-btn,.tp .tpp-zb{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;padding:0 10px;font:inherit;font-size:12px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
.tp .tpp .tdbtn{min-height:24px;min-width:24px}
.tp .tpp-btn:hover,.tp .tpp-zb:hover{background:var(--sel)}.tp .tpp-btn:focus-visible,.tp .tpp-zb:focus-visible,.tp .tpp svg:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.tp .tpp-prop{margin:6px 0;font-size:13px;font-weight:600}.tp .tpp-note{margin:3px 0;font-size:12px;color:var(--muted)}.tp .tpp .err{color:var(--bad)}
.tp .tpp-tw{max-width:100%;overflow-x:auto}.tp .tpp table{border-collapse:collapse;font-size:12px;width:100%}.tp .tpp th{text-align:left;font-weight:600;color:var(--muted);border-bottom:1px solid var(--line);padding:2px 8px 2px 0;white-space:nowrap}
.tp .tpp td{padding:2px 8px 2px 0;border-bottom:1px solid var(--line);vertical-align:top}.tp .tpp td.n{text-align:right;font-variant-numeric:tabular-nums}
.tp .tpp .sw{display:inline-block;width:12px;height:12px;vertical-align:-2px;margin-right:4px}.tp .tpp .st{font-weight:600}
.tp .tpp .st-supported{color:var(--ok)}.tp .tpp .st-excluded{color:var(--muted);text-decoration:line-through}
.tp .tpp-ch{margin:4px 0}.tp .tpp-ch svg{display:block;width:100%;height:auto;max-height:230px}.tp .tpp-ch svg text{fill:var(--muted);font-size:9px}
.tp .tpp-ch svg .ax{stroke:var(--muted);fill:none}.tp .tpp-ch svg .pk{stroke-width:2}.tp .tpp-ch svg .o{stroke:var(--muted);stroke-width:1}
.tp .tpp-ch svg .u{stroke:var(--muted);stroke-width:1;stroke-dasharray:1 2}.tp .tpp-ch svg .sh{stroke:var(--ink)}
.tp .tpp-zr{display:flex;flex-wrap:wrap;gap:4px;margin:3px 0}.tp .tpp-leg{display:flex;flex-wrap:wrap;gap:2px 12px;font-size:11px;color:var(--muted);margin:2px 0}
.tp .tpp-box{margin:6px 0;padding:6px 8px;border:1px solid var(--line);border-radius:5px;font-size:12px}.tp .tpp-box p{margin:2px 0}
@media (pointer:coarse){.tp .tpp-btn,.tp .tpp-zb,.tp .tpp .tdbtn{min-height:40px;min-width:40px}}
@media (prefers-reduced-motion:reduce){.tp .tpp *{transition:none!important;animation:none!important}}`;
  if (!document.getElementById("tpp-css")) { const s = document.createElement("style"); s.id = "tpp-css"; s.textContent = CSS; document.head.appendChild(s); }

  /** the six colours of Okabe-Ito, each with its own marker and dash so that colour is never the only cue */
  const SHAPES = ["circle", "square", "triangle", "diamond", "cross", "plus"], DASH = ["", "6 3", "2 2", "8 2 2 2", "4 4", "1 3"];
  /** @param {number} i @param {number} x @param {number} y @param {string} fill @param {boolean} [hollow] */
  function marker(i, x, y, fill, hollow) {
    const f = hollow ? `fill="var(--panel)" stroke="${fill}" stroke-width="1.4"` : `fill="${fill}" stroke="none"`, s = SHAPES[i % SHAPES.length], X = x.toFixed(1), Y = y.toFixed(1), r = 4;
    if (s === "circle") return `<circle cx="${X}" cy="${Y}" r="${r}" ${f}/>`;
    if (s === "square") return `<rect x="${(x - r).toFixed(1)}" y="${(y - r).toFixed(1)}" width="${2 * r}" height="${2 * r}" ${f}/>`;
    if (s === "triangle") return `<path d="M${X},${(y - r - 1).toFixed(1)}L${(x + r + 1).toFixed(1)},${(y + r).toFixed(1)}L${(x - r - 1).toFixed(1)},${(y + r).toFixed(1)}Z" ${f}/>`;
    if (s === "diamond") return `<path d="M${X},${(y - r - 1).toFixed(1)}L${(x + r + 1).toFixed(1)},${Y}L${X},${(y + r + 1).toFixed(1)}L${(x - r - 1).toFixed(1)},${Y}Z" ${f}/>`;
    return `<path d="M${(x - r).toFixed(1)},${Y}H${(x + r).toFixed(1)}M${X},${(y - r).toFixed(1)}V${(y + r).toFixed(1)}${s === "cross" ? `M${(x - r).toFixed(1)},${(y - r).toFixed(1)}L${(x + r).toFixed(1)},${(y + r).toFixed(1)}M${(x + r).toFixed(1)},${(y - r).toFixed(1)}L${(x - r).toFixed(1)},${(y + r).toFixed(1)}` : ""}" fill="none" stroke="${fill}" stroke-width="1.8"/>`;
  }
  const colourOf = (/** @type {number} */ i) => `var(--oi${i % 6 + 1})`;

  // ------------------------------------------------------------ the pieces of the panel
  /** @param {any} P the `posiz/1` @param {string[]} sym @returns {string} */
  function classTable(P, sym) {
    const atoms = (/** @type {number[]} */ a) => a.map(i => `${sym[i] || "?"}${i + 1}`).join(", "), frags = (/** @type {any[]} */ l) => l.length ? l.map(f => `${(+f.mz).toFixed(4)} ${esc(f.frag_formula)}`).join("<br>") : "–";
    const MARK = /** @type {Record<string, string>} */ ({ supported: "✓", excluded: "✗", undistinguished: "≈" });
    const rows = P.classes.map((/** @type {any} */ c, /** @type {number} */ i) => {
      const live = c.status !== "excluded", sw = live ? `<svg class="sw" viewBox="0 0 12 12" aria-hidden="true" focusable="false">${marker(i, 6, 6, colourOf(i))}</svg>` : "";
      return `<tr><th scope="row">${sw}${c.id}</th><td>${esc(atoms(c.atoms))}</td><td>${esc(String(c.kind).replace(/_/g, " "))}</td><td><span class="st st-${esc(c.status)}"><span aria-hidden="true">${MARK[c.status] || ""}</span> ${esc(c.status)}</span></td>`
        + `<td>${frags(c.support)}</td><td>${frags(c.against)}</td></tr>`;
    }).join("");
    const un = P.unexplained.length ? `<tr><th scope="row">unexplained</th><td colspan="5">m/z ${P.unexplained.map((/** @type {number} */ v) => (+v).toFixed(4)).join(", ")}: no candidate explains these peaks</td></tr>` : "";
    return `<div class="tpp-tw" tabindex="0" role="region" aria-label="Position classes"><table><caption class="sr-only">Classes of positions of the change ${esc(P.transform)} on ${esc(P.parent)}</caption>`
      + `<thead><tr><th scope="col">Class</th><th scope="col">Atoms (SMILES index)</th><th scope="col">Type</th><th scope="col">Status</th><th scope="col">Fragments in favour (with the change)</th><th scope="col">Fragments against (without it)</th></tr></thead><tbody>${rows}${un}</tbody></table></div>`;
  }

  /** how each peak of the spectrum is explained @param {any} P @param {number[][]} peaks */
  function peakRoles(P, peaks) {
    const near = (/** @type {number} */ a, /** @type {number} */ b) => Math.abs(a - b) < 1e-6;
    return peaks.map(([mz]) => {
      const cls = /** @type {number[]} */ ([]); let without = false;
      P.classes.forEach((/** @type {any} */ c, /** @type {number} */ i) => {
        if (c.status !== "excluded" && c.support.some((/** @type {any} */ f) => near(f.mz, mz))) cls.push(i);
        if (c.against.some((/** @type {any} */ f) => near(f.mz, mz))) without = true;
      });
      const un = P.unexplained.some((/** @type {number} */ v) => near(v, mz));
      return cls.length === 1 ? { kind: "class", i: cls[0] } : cls.length > 1 ? { kind: "shared", cls } : without ? { kind: "without" } : un ? { kind: "unexplained" } : { kind: "other" };
    });
  }

  const SW = 520, SH = 170, M = { l: 34, r: 8, t: 14, b: 22 };
  /** the bars of the MS2 coloured by class, in the window [lo, hi] of m/z @param {any} P @param {number[][]} peaks @param {number} lo @param {number} hi @returns {string} */
  function spectrumSvg(P, peaks, lo, hi) {
    const roles = peakRoles(P, peaks), X = (/** @type {number} */ v) => M.l + (v - lo) / (hi - lo) * (SW - M.l - M.r), top = SH - M.b, up = top - M.t;
    let s = `<svg viewBox="0 0 ${SW} ${SH}" preserveAspectRatio="xMidYMid meet" tabindex="0"><path class="ax" d="M${M.l},${M.t}V${top}H${SW - M.r}"/>`;
    peaks.forEach(([mz, rel], i) => {
      if (mz < lo || mz > hi) return;
      const x = X(mz), y = top - rel / 100 * up, r = roles[i];
      const cls = r.kind === "class" ? `pk" stroke="${colourOf(r.i || 0)}" stroke-dasharray="${DASH[(r.i || 0) % 6]}` : r.kind === "shared" ? "pk sh" : r.kind === "without" ? "pk o" : r.kind === "unexplained" ? "pk u" : "pk o";
      s += `<line class="${cls}" x1="${x.toFixed(1)}" x2="${x.toFixed(1)}" y1="${top}" y2="${y.toFixed(1)}"/>`;
      if (r.kind === "class") s += marker(r.i || 0, x, y - 5, colourOf(r.i || 0));
      else if (r.kind === "shared") s += marker(3, x, y - 5, "var(--ink)");
      else if (r.kind === "without") s += marker(0, x, y - 5, "var(--muted)", true);
      if (r.kind !== "other" && rel >= 8) s += `<text x="${x.toFixed(1)}" y="${Math.max(9, y - 12).toFixed(1)}" text-anchor="middle">${mz.toFixed(mz % 1 ? (P.tol_unit === "ppm" ? 4 : 1) : 0)}</text>`;
    });
    return s + `<text x="${M.l}" y="${SH - 8}">${lo.toFixed(1)}</text><text x="${SW - M.r}" y="${SH - 8}" text-anchor="end">${hi.toFixed(1)} m/z</text><text x="${M.l - 3}" y="${M.t + 6}" text-anchor="end">100</text><text x="${M.l - 3}" y="${top}" text-anchor="end">0</text></svg>`;
  }
  /** the table of the data of the spectrum, for TDATI @param {any} P @param {number[][]} peaks */
  function spectrumRows(P, peaks) {
    return peakRoles(P, peaks).map((r, i) => [peaks[i][0].toFixed(4), String(+peaks[i][1].toFixed(1)),
      r.kind === "class" ? `with the change, class ${P.classes[r.i || 0].id}` : r.kind === "shared" ? `with the change, classes ${(r.cls || []).map((/** @type {number} */ k) => P.classes[k].id).join(" and ")}` : r.kind === "without" ? "without the change" : r.kind === "unexplained" ? "unexplained" : "not used"]);
  }

  /** the spectrum with its zoom buttons (no dragging): + and − centre on the middle of the window, the arrows move it @param {HTMLElement} box @param {any} P @param {number[][]} peaks */
  function spectrum(box, P, peaks) {
    const all = peaks.map(p => p[0]).concat([P.tp_mz]), full = { lo: Math.max(0, Math.min(...all) - 10), hi: Math.max(...all) + 10 }, w = { ...full };
    box.className = "tpp-ch"; box.innerHTML = `<h4>MS2 of the TP, peaks marked by class</h4><div class="tpp-fig"></div>`
      + `<div class="tpp-zr" role="group" aria-label="Zoom of the spectrum">${[["in", "Zoom in", "+"], ["out", "Zoom out", "−"], ["left", "Move left", "◀"], ["right", "Move right", "▶"], ["reset", "Whole spectrum", "Reset"]]
        .map(([k, l, t]) => `<button type="button" class="tpp-zb" data-z="${k}" aria-label="${l}">${t}</button>`).join("")}</div>`
      + `<p class="tpp-leg" aria-hidden="true">${P.classes.map((/** @type {any} */ c, /** @type {number} */ i) => c.status === "excluded" ? "" : `<span><svg width="12" height="12" viewBox="0 0 12 12">${marker(i, 6, 6, colourOf(i))}</svg> class ${c.id}</span>`).join("")}`
      + `<span><svg width="12" height="12" viewBox="0 0 12 12">${marker(3, 6, 6, "var(--ink)")}</svg> shared by classes</span><span><svg width="12" height="12" viewBox="0 0 12 12">${marker(0, 6, 6, "var(--muted)", true)}</svg> without the change</span><span>dotted: unexplained</span></p>`;
    const fig = /** @type {HTMLElement} */ (box.querySelector(".tpp-fig"));
    const draw = () => {
      fig.innerHTML = spectrumSvg(P, peaks, w.lo, w.hi);
      const el = /** @type {SVGElement} */ (fig.querySelector("svg")), title = "MS2 of the TP: peaks coloured by position class";
      el.setAttribute("role", "img"); el.setAttribute("aria-label", title);
      if (G.TDATI) { G.TDATI.describe(el, `${title}: ${peaks.length} peaks, window m/z ${w.lo.toFixed(1)} to ${w.hi.toFixed(1)}.`); G.TDATI.addButton(el, () => ({ title, head: ["m/z", "Relative intensity (%)", "Explained as"], rows: spectrumRows(P, peaks) })); }
      el.addEventListener("keydown", e => { const k = ({ "+": "in", "=": "in", "-": "out", ArrowLeft: "left", ArrowRight: "right", Home: "reset" })[/** @type {"+"} */ (e.key)]; if (k) { e.preventDefault(); zoom(k); } });
    };
    /** @param {string} k */
    const zoom = k => {
      const c = (w.lo + w.hi) / 2, span = w.hi - w.lo;
      if (k === "reset") { w.lo = full.lo; w.hi = full.hi; }
      else if (k === "in" || k === "out") { const n = Math.min(full.hi - full.lo, Math.max(5, span * (k === "in" ? 0.5 : 2))); w.lo = Math.max(full.lo, c - n / 2); w.hi = Math.min(full.hi, w.lo + n); w.lo = Math.max(full.lo, w.hi - n); }
      else { const d = span * 0.4 * (k === "left" ? -1 : 1); w.lo = Math.min(Math.max(full.lo, w.lo + d), full.hi - span); w.hi = w.lo + span; }
      draw();
    };
    box.querySelectorAll("button[data-z]").forEach(b => b.addEventListener("click", () => { zoom(/** @type {HTMLElement} */ (b).dataset.z || "reset"); }));
    draw();
  }

  /** @param {any} P */
  function oxidation(P) {
    const o = P.ox_type; if (!o) return "";
    const yn = (/** @type {boolean} */ v) => v ? "seen" : "not seen";
    return `<div class="tpp-box"><h4 style="margin-top:0">Oxidation type</h4><p>Loss of H<sub>2</sub>O from the precursor: <b>${yn(o.h2o_loss)}</b>. [M+H−O]<sup>+</sup>: <b>${yn(o.minus_o)}</b>.</p>`
      + `<p class="tpp-note">In ESI an aliphatic OH loses water more readily than a phenol, and [M+H−O]<sup>+</sup> is typical of N-oxides (${doi(DOI.ox, "Ramanathan et al. 2000")}). Shown as evidence only: it does not remove any class.</p></div>`;
  }
  /** @param {any} P */
  function checks(P) {
    const L = /** @type {Record<string, string>} */ ({
      hdx: `<b>H/D exchange.</b> Carbon hydroxyl and N- or S-oxide candidates both remain: an exchangeable OH shifts in D<sub>2</sub>O, an oxide does not (${doi(DOI.hdx, "Cajka et al. 2026")}).`,
      ms3: `<b>MS3 or CE ratios.</b> A fragment that carries the change is shared by several classes: MS3 of that fragment, or its intensity ratios at several collision energies, may tell them apart (${doi(DOI.ms3, "Murakami et al. 2018")}).`,
      standard: `<b>Reference standard.</b> Only a standard of the candidate isomer, compared on retention time and MS2, fixes the position.` });
    return `<div class="tpp-box"><h4 style="margin-top:0">Suggested checks</h4>${(P.suggest || []).map((/** @type {string} */ k) => L[k] ? `<p>${L[k]}</p>` : "").join("")}</div>`;
  }
  /** the line of the proposal: a proposal for the chemist, no level is saved @param {any} P */
  function proposal(P) {
    const n = P.classes.filter((/** @type {any} */ c) => c.status !== "excluded").length;
    return `<p class="tpp-prop" role="status">Proposal: Schymanski ${esc(P.proposal.sublevel)} — ${n} position class${n === 1 ? "" : "es"} remain${n === 1 ? "s" : ""}; the chemist decides</p>`;
  }

  // ------------------------------------------------------------ the button and the panel
  /**
   * @typedef {{ smiles: string, delta: string, peaks: number[][], positive?: boolean, tol: number, ppm: boolean, label?: string }} PosModel
   * @param {PosModel | null | undefined} m @returns {boolean} true when the button belongs in the row: administrator mode, SMILES, a covered change and MS2 peaks
   */
  const eligible = m => !!(admin() && m && m.smiles && transformOf(m.delta) && m.peaks && m.peaks.length);

  /** draws the button (and, on click, the panel) at the end of `host`; nothing when the row is not eligible @param {HTMLElement} host @param {PosModel | null | undefined} m */
  function mount(host, m) {
    const old = host.querySelector(".tpp"); if (old) old.remove();
    if (!m || !eligible(m)) return;
    const card = document.createElement("div"); card.className = "card tpp"; host.appendChild(card);
    card.innerHTML = `<button type="button" class="tpp-btn" aria-expanded="false">Substitution position</button><div class="tpp-out" aria-live="polite"></div>`;
    const btn = /** @type {HTMLButtonElement} */ (card.querySelector("button")), out = /** @type {HTMLElement} */ (card.querySelector(".tpp-out"));
    let done = false;
    btn.addEventListener("click", async () => {
      const open = btn.getAttribute("aria-expanded") !== "true"; btn.setAttribute("aria-expanded", String(open)); out.hidden = !open;
      if (!open || done) return;
      out.innerHTML = `<p class="tpp-note">Computing…</p>`;
      try {
        const peaks = m.peaks.map(p => [+p[0], +p[1]]).filter(p => Number.isFinite(p[0]) && Number.isFinite(p[1])), P = await compute(m.smiles, transformOf(m.delta), peaks, m.tol, m.ppm, m.positive !== false);
        out.innerHTML = `<h4>${esc(m.label || "")} ${esc(P.transform)} on the parent, ${P.tol} ${P.tol_unit === "ppm" ? "ppm" : "Da"}</h4>${proposal(P)}`;
        out.insertAdjacentHTML("beforeend", classTable(P, symbols(m.smiles)));
        const chart = document.createElement("div"); out.appendChild(chart); spectrum(chart, P, peaks);
        out.insertAdjacentHTML("beforeend", oxidation(P) + checks(P)); done = true;
      } catch (e) { const k = /** @type {any} */ (e).key; out.innerHTML = `<p class="err" role="alert">${esc(ERR[k] || String(/** @type {any} */ (e).message || e))}</p>`; }
    });
    out.hidden = true;
  }

  /** true for a negative-mode adduct («[M-H]-», «M-H») @param {any} adduct */
  const negative = adduct => /[-−]\s*$/.test(String(adduct ?? "").trim()) || /M\s*[-−]\s*H/.test(String(adduct ?? ""));

  /** @type {any} */ (window).TPPOS = { eligible, mount, transformOf, symbols, negative };
})();
