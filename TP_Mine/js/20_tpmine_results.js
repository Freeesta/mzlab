// @ts-strict
// TPMINE-PRIVATE  Tabui results: the summary card, the table, the map, the kinetics, the row panel and the exports of the low-resolution Dig (factory used by 20_tpmine.js).
(() => {
  "use strict";
  const B = /** @type {any} */ (window).TPBASE;
  const { esc, specHtml, fmtA, TYPES, fukHtml, LAB, call, hue, chart } = B;
  /** @param {{ st: any, $: (s: string) => any, msg: (t: string, err?: boolean) => void }} env the state, the finder and the message line of the open tab */
  window.TPRESULTS = env => {
    const { st, $, msg } = env;
    const W = /** @type {any} */ (window), dt = (/** @type {string} */ k) => (W.TPHELP ? ` data-tt="${k}"` : "");
    // ---- results
    const VIEWS = [["tab", "Table"], ["map", "🕸 Reaction map"], ["kin", "📈 Kinetics"]];
    function countUp(el) {
      const to = +el.dataset.count, dec = +(el.dataset.dec || 0), t0 = performance.now(), D = 900;
      const step = t => { const k = Math.min(1, (t - t0) / D), e = 1 - Math.pow(1 - k, 3); el.textContent = (to * e).toFixed(dec); if (k < 1) requestAnimationFrame(step); };
      requestAnimationFrame(step);
    }
    function confetti() {
      const cv = document.createElement("canvas"); cv.className = "confetti"; cv.setAttribute("aria-hidden", "true"); cv.style.cssText = "position:fixed;inset:0;width:100%;height:100%;pointer-events:none;z-index:95"; document.body.appendChild(cv);
      cv.width = innerWidth; cv.height = innerHeight; const g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")), cols = ["#2b5c8a", "#047857", "#d99a1d", "#b42318", "#7c3aed", "#0ea5e9"];
      const P = Array.from({ length: 140 }, () => ({ x: innerWidth / 2 + (Math.random() - .5) * 300, y: innerHeight * 0.35, vx: (Math.random() - .5) * 14, vy: -Math.random() * 13 - 3, r: 3 + Math.random() * 4, c: cols[(Math.random() * cols.length) | 0], rot: Math.random() * 6, vr: (Math.random() - .5) * .4 }));
      const t0 = performance.now();
      const f = t => { const k = (t - t0) / 1800; g.clearRect(0, 0, cv.width, cv.height); for (const p of P) { p.vy += .35; p.x += p.vx; p.y += p.vy; p.vx *= .99; p.rot += p.vr; g.save(); g.translate(p.x, p.y); g.rotate(p.rot); g.globalAlpha = Math.max(0, 1 - k); g.fillStyle = p.c; g.fillRect(-p.r, -p.r / 2, p.r * 2, p.r); g.restore(); } k < 1 ? requestAnimationFrame(f) : cv.remove(); };
      requestAnimationFrame(f);
    }
    const toast = t => { const d = document.createElement("div"); d.className = "tp-toast"; d.textContent = t; document.body.appendChild(d); setTimeout(() => d.classList.add("out"), 2200); setTimeout(() => d.remove(), 2800); };
    const good = r => r.label === "forte" || r.label === "possibile";
        const shortN = (t, n = 26) => t.length > n ? t.slice(0, n - 1) + "…" : t;
    const col = r => r.label === "progenitore" ? "#2b5c8a" : r.kind === "unexpected" ? "#7c3aed" : r.label === "forte" ? "#047857" : "#d99a1d";

    const calmMotion = () => !(matchMedia("(prefers-reduced-motion: reduce)").matches);
    function render() {
      const s = st.summary, R = $("#tp-right");
      const par = s.rows.find(r => r.label === "progenitore");
      const cnt = k => s.rows.filter(r => (k === "unexp" ? r.kind === "unexpected" && good(r) : r.kind === "candidate" && r.label === k)).length;
      const nF = cnt("forte"), nP = cnt("possibile"), nU = cnt("unexp"), hl = s.decay && s.decay.half_life_min != null ? s.decay.half_life_min : null;
      const nW = s.rows.filter(r => r.label === "debole" && !notFound(r)).length, left = s.unexpected_left || {}, nLeft = (left.over_limit || 0) + (left.isotope_or_flat || 0), off = s.offset && s.offset.applied ? s.offset.offset : null;
      // the summary card: three blocks (Parent, Output, Download); every item has its help, the numbers count up only without reduced motion
      const it = (k, label, val, help, dec) => `<span class="hc" data-k="${k}"${dt(help)} tabindex="0"><span class="hl">${label}</span> <b${dec != null ? ` data-count="${val}" data-dec="${dec}"` : ""}>${dec != null ? (+val).toFixed(dec) : val}</b></span>`;
      const pIt = [st.pname ? `<span class="hc hn" data-k="name">${esc(MOLUI.cap(st.pname))}</span>` : "", s.parent_ok === false ? it("pnf", "Status", "Parent not found", "ft.sum.pnf") : "", it("formula", "Neutral formula", esc(s.parent.formula || "—"), "ft.sum.formula"), it("mz", `${esc(s.parent.adduct)} <i>m/z</i>`, s.parent.mz, "ft.sum.mz"),
        par && par.ref_rt ? it("rt", "RT", `${par.ref_rt} min`, "ft.sum.rt") : "", hl != null ? `${it("half", "t½", hl, "ft.sum.half", 1).replace("</b></span>", "</b> min</span>")}` : "",
        off != null ? it("offset", "<i>m/z</i> offset", `${off >= 0 ? "+" : "−"}${Math.abs(off).toFixed(2)} Da`, "ft.sum.offset") : ""].filter(Boolean).join("");
      const cIt = (k, n, label, help) => `<span class="hc" data-k="${k}"${dt(help)} tabindex="0"><b data-count="${n}">${n}</b> ${label}</span>`;
      const words = [cIt("strong", nF, "strong", "ft.chip.strong"), cIt("possible", nP, "possible", "ft.chip.possible"), cIt("unexpected", nU, "unexpected", "ft.chip.unexpected"),
        // the weak band is off by default but never out of sight: its count is always here, with the unexpected ions the Dig left out
        `<span class="hc" data-k="weak"${dt("ft.chip.weak")} tabindex="0"><b>${nW}</b> <span id="tp-weak-w">weak evidence hidden</span></span>${nW ? `<button type="button" id="tp-weak-s" class="hero-l"${dt("ft.sum.weakshow")}>show</button>` : ""}`,
        nLeft ? `<span class="hc" data-k="left"${dt("ft.sum.left")} tabindex="0"><b>${nLeft}</b> unexpected not listed</span>` : ""].join("");
      const oIt = it("ions", "Ions analysed", s.rows.length, "ft.sum.ions") + it("took", "Computed in", `${(st.took / 1000).toFixed(1)} s`, "ft.sum.took");
      const nMs2 = s.ms2_links && s.ms2_links.links ? s.ms2_links.links.length : 0, ms2c = window.TPMS2 ? window.TPMS2.card(s) : "", mrmc = mrmCard(s.mrm);
      R.innerHTML = `<div class="hero" role="region" aria-label="Summary of the results">
        <section class="hero-b" aria-labelledby="tp-h-par"><h2 id="tp-h-par">Parent</h2><div class="hero-i">${pIt}<span id="tp-fuk" role="status">${fukHtml()}</span></div></section>
        <section class="hero-b" aria-labelledby="tp-h-out"><h2 id="tp-h-out">Output</h2><div class="hero-i"><span id="tp-cnt">${words}</span>${oIt}</div></section>
        <section class="hero-b" aria-labelledby="tp-h-dl"><h2 id="tp-h-dl">Download</h2><div class="hero-x" role="group" aria-labelledby="tp-h-dl"><button type="button" id="tp-x-xlsx"${dt("ft.sum.xlsx")}>Excel (.xlsx)</button><button type="button" id="tp-x-csv"${dt("ft.sum.csv")}>CSV</button><button type="button" id="tp-x-json"${dt("ft.sum.json")}>JSON</button><button type="button" id="tp-x-txt"${dt("ft.sum.txt")}>Copy summary</button></div></section></div>
        ${s.parent_ok === false ? `<p id="tp-nopar" role="status">In these files the ion of the parent is not a chromatographic peak that falls over time, so no product is proposed. Check the formula and the adduct of the parent and the files of the series; the criteria are in the parent row.</p>` : ""}
        <div class="vtabs">${(s.gruppi ? [...VIEWS, ["cond", "Compare conditions"]] : VIEWS).map(([k, n]) => `<button class="vt${st.view === k ? " on" : ""}" data-v="${k}">${n}</button>`).join("")}</div>
        <div id="tp-view"></div>
        <div class="tp-mores">${ms2c ? `<details name="tp-more" id="tp-m-ms2"><summary>MS2 matches (${nMs2})</summary>${ms2c}</details>` : ""}${mrmc ? `<details name="tp-more" id="tp-m-mrm"><summary>MRM (${s.mrm.rows.length} files)</summary>${mrmc}</details>` : ""}</div>`;
      if (window.TPMS2) window.TPMS2.mount({ st, call, refresh: ms2Refresh, show });
      if (calmMotion()) R.querySelectorAll(".hero b[data-count]").forEach(countUp);
      if (W.TPHELP) W.TPHELP.attach(R.querySelector(".hero"));
      const ws = $("#tp-weak-s"); if (ws) ws.onclick = () => { if (st.view !== "tab") { st.view = "tab"; R.querySelectorAll(".vt").forEach(x => x.classList.toggle("on", x.getAttribute("data-v") === "tab")); view(); } if (T) T.setChip("weak", !weakOn()); weakSync(); };
      R.querySelectorAll(".vt").forEach(b => b.onclick = () => { st.view = b.dataset.v; R.querySelectorAll(".vt").forEach(x => x.classList.toggle("on", x === b)); view(); });
      $("#tp-x-xlsx").onclick = () => exportAll("xlsx"); $("#tp-x-csv").onclick = () => exportAll("csv"); $("#tp-x-json").onclick = () => exportAll("json");
      $("#tp-x-txt").onclick = async () => { try { await navigator.clipboard.writeText(summaryText()); toast("Summary copied to the clipboard"); } catch (_) { dl("summary_Tabui.txt", summaryText(), "text/plain"); } };
      view(); weakSync();
      const mm = $("#tp-m-mrm");           // the chart of the calibration needs the width of an open section
      if (mm) mm.addEventListener("toggle", () => { const cc = $("#tp-c-cal"); if (mm.open && cc) { const c = JSON.parse(cc.dataset.cal); chart(cc, [{ x: [0, c.hi], y: [c.intercept, c.slope * c.hi + c.intercept], color: "#2b5c8a" }, { x: c.points.map(p => p.conc), y: c.points.map(p => p.area), color: "#b42318", dots: true, w: 0 }], { xlabel: "concentration (mg/L)", x0: 0 }); } });
      if (nF && calmMotion()) confetti();
    }

    function view() {
      const V = $("#tp-view"), s = st.summary;
      const list = s.rows.filter(r => r.label !== "progenitore" && good(r));
      T = null; if (st.gv) { st.gv.destroy(); st.gv = null; }
      if (st.view === "tab") {
        const nd = s.rows.filter(notFound).length;
        V.innerHTML = `${nd ? `<label class="mut" style="display:inline-flex;gap:6px;align-items:center;min-height:24px"><input type="checkbox" id="tp-nd"> Show searched, not detected (${nd})</label>` : ""}${s.gruppi ? condFilterHtml(s.gruppi) : ""}<div id="tp-tbl"></div>`;
        mountTable($("#tp-tbl"));
        const sel = /** @type {HTMLSelectElement | null} */ ($("#tp-gflt")); if (sel) sel.onchange = () => { if (T) T.setRows(tableRows()); };
      } else if (st.view === "cond") {
        V.innerHTML = `<div class="card" id="tp-gv"><span class="mut" role="status">Loading the comparison...</span></div>`;
        const host = $("#tp-gv"), W = /** @type {any} */ (window);
        W.TPLAZY.load("gruppi-vista.js").then(() => { if (st.view === "cond" && host.isConnected) st.gv = W.TPGRUPPI_VIEW.mount(host, s.gruppi, { parentName: s.parent.name || s.parent.formula, onOpenRow: (/** @type {number} */ id) => show(id) }); })
          .catch(() => { host.textContent = "The comparison of conditions could not be loaded."; });
      } else if (st.view === "map") {
        V.innerHTML = `<div class="card"><div class="mut">Each node is an ion that passes the criteria: the radius follows the maximum area, the colour the confidence (green strong, amber possible, purple unexpected). Lines show the transformation (Δ formula). Click a node for details.</div><div id="tp-map"></div></div>`;
        mapView($("#tp-map"));
      } else if (st.view === "kin") {
        V.innerHTML = `<div class="card"><h3>All kinetics, each at its own maximum (100 %)</h3><canvas id="tp-c-all" style="height:320px"></canvas><div class="row" id="tp-leg-all"></div></div>`;
        const top = list.filter(r => r.kind === "candidate").slice(0, 8), par = s.rows.find(r => r.label === "progenitore");
        const ser = [par, ...top].map((r, i) => { const k = r.kinetics, mx = Math.max(...k.map(p => p.area)) || 1; return { x: k.map(p => p.time), y: k.map(p => 100 * p.area / mx), color: i ? hue(i - 1, top.length) : "#555", dash: i ? [] : [6, 4], w: i ? 2 : 2.4, label: r.name, dots: !!i }; });
        chart($("#tp-c-all"), ser, { xlabel: "time (min)" });
        $("#tp-leg-all").innerHTML = ser.map(t => `<span class="mut"><span style="display:inline-block;width:14px;border-top:2px ${t.dash.length ? "dashed" : "solid"} ${t.color};vertical-align:middle"></span> ${esc(shortN(t.label, 34))}</span>`).join("");
      }
    }

    function mapView(host) {
      const s = st.summary, par = s.rows.find(r => r.label === "progenitore");
      const cand = s.rows.filter(r => r.kind === "candidate" && r.label !== "progenitore" && good(r));
      const unx = s.rows.filter(r => r.kind === "unexpected" && good(r) && !(r.insource && !(r.isf && r.isf.doubtful))).slice(0, 10);
      const cols = [[par], cand.filter(r => r.steps === 1), cand.filter(r => r.steps === 2), [...cand.filter(r => r.steps >= 3), ...unx]];
      const W = 1000, colX = [90, 330, 590, 850], rowsN = Math.max(...cols.map(c => c.length)), H = Math.max(300, rowsN * 56 + 50);
      const maxA = Math.max(...s.rows.map(r => r.max_area)) || 1, pos = {};
      cols.forEach((c, ci) => c.forEach((r, i) => { pos[r.id] = [colX[ci], 30 + (i + 0.5) * (H - 40) / c.length]; }));
      const node = r => pos[r.id];
      const byName = Object.fromEntries(cand.filter(r => r.steps === 1).map(r => [r.name, r]));
      let edges = "";
      for (const r of [...cand, ...unx]) {
        const src = r.kind === "unexpected" ? [par] : r.steps === 1 ? [par] : (() => { const parts = r.name.split(" + ").map(n => byName[n]).filter(Boolean); return parts.length ? parts : [par]; })();
        for (const a of src) {
          const [x1, y1] = node(a), [x2, y2] = node(r), mx = (x1 + x2) / 2, my = (y1 + y2) / 2, lab = r.kind === "unexpected" ? `Δ${r.delta_mz >= 0 ? "+" : ""}${r.delta_mz}` : (r.steps === 1 ? r.delta : "");
          edges += `<path d="M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}" fill="none" stroke="${col(r)}" stroke-opacity=".45" stroke-width="1.6"${r.kind === "unexpected" || src[0] === par && r.steps > 1 ? ' stroke-dasharray="5 4"' : ""}/>${lab ? `<text x="${mx}" y="${(y1 + y2) / 2 - 4}" font-size="10" fill="#687080" text-anchor="middle">${esc(lab)}</text>` : ""}`;
        }
      }
      const nodes = [par, ...cand, ...unx].map(r => { const [x, y] = node(r), rad = 9 + 15 * Math.log10(1 + r.max_area) / Math.log10(1 + maxA);
        return `<g class="mnode${r.id === st.sel ? " sel" : ""}" data-id="${r.id}" transform="translate(${x},${y})"><circle r="${rad.toFixed(1)}" fill="${col(r)}" fill-opacity=".85" stroke="#fff" stroke-width="2"><title>${esc(r.name)}</title></circle><text y="${(rad + 13).toFixed(0)}" font-size="11" text-anchor="middle" fill="currentColor">${esc(shortN(r.name, 24))}</text><text y="${(rad + 25).toFixed(0)}" font-size="10" text-anchor="middle" fill="#687080">${r.mz.toFixed(1)}</text></g>`; }).join("");
      host.innerHTML = `<svg viewBox="0 0 ${W} ${H}" width="100%" style="max-height:${H}px">${[["parent", 0], ["1 step", 1], ["2 steps", 2], ["3 steps / unexpected", 3]].map(([t, i]) => `<text x="${colX[i]}" y="14" font-size="11" fill="#687080" text-anchor="middle" font-weight="700">${t}</text>`).join("")}${edges}${nodes}</svg>`;
      host.querySelectorAll(".mnode").forEach(g => g.onclick = () => show(+g.dataset.id));
    }


    function summaryText() {
      const s = st.summary, par = s.rows.find(r => r.label === "progenitore"), L = [];
      L.push(`Search for transformation products of ${s.parent.formula || "parent"} (${s.parent.adduct}, m/z ${s.parent.mz}${par && par.ref_rt ? `, RT ${par.ref_rt} min` : ""}).`);
      if (s.decay) L.push(`The parent decays with first-order kinetics: k = ${s.decay.k_per_min} min^-1, t1/2 = ${s.decay.half_life_min} min (R2 = ${s.decay.r2}).`);
      if (s.offset) L.push(`m/z offset measured on the parent: ${s.offset.offset >= 0 ? "+" : ""}${s.offset.offset} Da${s.offset.applied ? " (corrected)" : ""}.`);
      const top = s.rows.filter(r => r.label !== "progenitore" && good(r) && !(r.insource && !(r.isf && r.isf.doubtful)));
      L.push(`Candidates that grow over time and are absent from the blank: ${top.filter(r => r.kind === "candidate").length}; unexpected ions: ${top.filter(r => r.kind === "unexpected").length}.`);
      for (const r of top.slice(0, 15)) L.push(`- ${r.name}${r.formula ? ` (${r.formula}${r.delta ? ", " + r.delta : ""})` : ""}: m/z ${r.mz.toFixed(1)}, RT ${r.ref_rt} min, maximum at ${r.tmax} min, area ${fmtA(r.max_area)}, verdict ${LAB[r.label] || r.label}${r.pred ? `, prediction Prev. ${r.pred}` : ""}.`);
      L.push("Unit resolution: each row is a candidate, not an identification; no identification level is proposed, the chemist decides it. «Prev. N» is the prediction of the transformation (Löffler et al. 2025), not a level.");
      return L.join("\n");
    }

    function mrmCard(m) {
      if (!m) return "";
      const q = m.quantifier, qual = m.transitions.slice(1).map(t => t.name), c = m.calibration;
      const body = m.rows.map(r => { const i = r.ion[q]; return `<tr><td>${esc(r.label)}</td><td>${TYPES[r.type] || r.type}</td><td class="n">${r.time ?? ""}</td>${c ? `<td class="n">${r.conc ?? ""}</td>` : ""}<td class="n">${i.detected ? fmtA(i.area) : "-"}</td><td class="n">${i.rt != null ? i.rt.toFixed(2) : ""}</td><td class="n">${i.snr ? i.snr.toFixed(0) : ""}</td>${qual.map(n => `<td class="n">${r.ion[n].detected ? fmtA(r.ion[n].area) : "-"}</td><td class="n">${r.ratios[n] != null ? r.ratios[n].toFixed(2) : ""}</td>`).join("")}<td>${r.ratio_ok == null ? "" : r.ratio_ok ? "✓" : "<span class='err'>out</span>"}</td>${c ? `<td class="n">${r.quant != null ? (+r.quant.toPrecision(3)) : ""}</td><td>${r.in_range == null ? "" : r.in_range ? "" : "<span class='err' title='outside the range of the calibration points'>off line</span>"}</td>` : ""}</tr>`; }).join("");
      return `<div class="card"><h3>MRM: automatic integration</h3><div class="cols"><div><div class="mut">Quantifier ${esc(q)}${qual.length ? ` · qualifier ${qual.map(esc).join(", ")}` : ""} · reference RT ${m.ref_rt ? m.ref_rt.toFixed(2) : "?"} min · ion ratio of the standards ${Object.values(m.ratio_ref).map(v => v == null ? "-" : v.toFixed(2)).join(", ")} (±${Math.round(m.ratio_tol * 100)} %)</div>
        <div class="tbl" style="max-height:300px"><table><tr><th>File</th><th>Type</th><th>t</th>${c ? "<th>conc. (mg/L)</th>" : ""}<th>Quant. area</th><th>RT</th><th>S/N</th>${qual.map(n => `<th>Qual. area</th><th>Qual/Quant</th>`).join("")}<th>Ratio</th>${c ? "<th>Calculated (mg/L)</th><th></th>" : ""}</tr>${body}</table></div></div>
        ${c ? `<div><div class="mut">Line: area = ${c.slope.toPrecision(4)} · conc ${c.intercept >= 0 ? "+" : "−"} ${Math.abs(c.intercept).toPrecision(3)} · weights ${c.weighting} · R² (unweighted) ${c.r2.toFixed(4)} · ${c.points.length} points, ${c.lo}–${c.hi} mg/L</div><canvas id="tp-c-cal" data-cal='${esc(JSON.stringify(c))}'></canvas></div>` : ""}</div></div>`;
    }
    const hasMs2 = (/** @type {any} */ s) => !!(s.ms2_links && s.ms2_links.experiments && s.ms2_links.experiments.length);      // a series without MS2 file has no card and no column
    /** @param {number[]} touched */
    function ms2Refresh(touched) {          // after a correction of the MS2 matches: the card, the table of rows and the open detail
      const c = $("#tp-ms2map"), s = st.summary; if (!c || !s) return;
      const h = document.createElement("div"); h.innerHTML = window.TPMS2.card(s); c.replaceWith(h.firstElementChild);
      window.TPMS2.mount({ st, call, refresh: ms2Refresh, show });
      const sm = $("#tp-m-ms2 > summary"); if (sm) sm.textContent = `MS2 matches (${s.ms2_links && s.ms2_links.links ? s.ms2_links.links.length : 0})`;
      if (T) { T.setRows(tableRows()); if (st.sel != null && touched.includes(st.sel) && T.openId === st.sel) { T.close(); T.open(st.sel); } }
    }
    /** @type {any} the TPTAB of the results while the view "Table" is shown */
    let T = null;
    /** the weak band of the table is off by default: the card says how many rows it hides and shows them in one click */
    const weakOn = () => T ? T.chips.includes("weak") : !!st.weakOn;
    function weakSync() {
      const w = $("#tp-weak-w"), b = $("#tp-weak-s"), on = weakOn(); if (!w || !b) return;
      w.textContent = `weak evidence ${on ? "shown" : "hidden"}`; b.textContent = on ? "hide" : "show"; b.setAttribute("aria-pressed", String(on));
    }
    // the Dig proposes no identification level (`level` is null): the rows go by verdict, then score; the level is the one the chemist decides (TPDEC)
    const RANK = { progenitore: 0, forte: 1, possibile: 2 }, lvSort = r => (RANK[r.label] ?? 3) * 1000 - (r.score ?? 0);
    const idLabel = r => r.kind === "unexpected" && !r.ms2_target ? `m/z ${r.mz.toFixed(2)} (unexpected)` : MOLUI.cap(r.name);
    /** a hypothesis of the rules with no peak in any sample: searched, not detected (no TP name, no area) */
    const notFound = r => r.status === "Searched, not detected";
    /** the row an unexpected ion co-elutes with: its m/z, the m/z difference and the RT difference (a measurement, not a verdict) */
    const coelText = c => `m/z ${c.mz.toFixed(2)} (Δm/z ${c.delta_mz.toFixed(2)}, ΔRT ${c.delta_rt >= 0 ? "+" : ""}${c.delta_rt.toFixed(2)} min)`;
    const coelCell = c => esc(coelText(c));
    const cellStates = r => (r.kinetics || []).map(p => p.area > 0 ? "detected" : "absent");
    const rowNum = r => r.id >= 100000 ? "n" + (r.id - 100000) : String(r.id);
    /** the row of gruppi/1 of a row of the table (comparison of conditions), null without it */
    const gIndex = new WeakMap();      // the rows of a gruppi/1 by id (kept apart: the summary is exported as it is)
    const gRow = (/** @type {any} */ r) => { const g = st.summary.gruppi; if (!g) return null; let m = gIndex.get(g); if (!m) gIndex.set(g, m = new Map(g.rows.map((/** @type {any} */ x) => [x.row, x]))); return m.get(r.id) || null; };
    const condIds = () => st.summary.gruppi.conditions.map((/** @type {any} */ c) => c.id);
    /** the filter of the rows by condition: all, in all, only in X, present in X (the sets of gruppi/1; no row is dropped by a criterion of the engine) */
    function condFilterHtml(/** @type {any} */ g) {
      const ids = g.conditions.map((/** @type {any} */ c) => c.id), only = (/** @type {string} */ c) => g.rows.some((/** @type {any} */ r) => r.only_in === c), e = (/** @type {string} */ t) => esc(t);
      return `<label class="mut tp-gf" style="display:inline-flex;gap:6px;align-items:center;min-height:24px;margin:0 8px">Conditions <select id="tp-gflt" aria-label="Filter the rows by condition"><option value="">All rows</option><option value="all">In all conditions</option>`
        + ids.filter(only).map((/** @type {string} */ c) => `<option value="only:${e(c)}">Only in ${e(c)}</option>`).join("") + ids.map((/** @type {string} */ c) => `<option value="in:${e(c)}">Present in ${e(c)}</option>`).join("") + `</select></label>`;
    }
    /** the rows of the table: without «searched, not detected» unless asked, and by the filter of conditions */
    function tableRows() {
      const s = st.summary, nd = /** @type {HTMLInputElement | null} */ ($("#tp-nd")), f = (/** @type {HTMLSelectElement | null} */ ($("#tp-gflt")) || { value: "" }).value, par = (/** @type {any} */ r) => r.label === "progenitore";
      const ok = (/** @type {any} */ r) => { if (!f || par(r)) return true; const g = gRow(r); return !!g && (f === "all" ? g.in_all : f.startsWith("only:") ? g.only_in === f.slice(5) : g.present_in.includes(f.slice(3))); };
      return s.rows.filter((/** @type {any} */ r) => (nd && nd.checked || !notFound(r)) && ok(r));
    }
    function mountTable(host) {
      const W = /** @type {any} */ (window), TPTAB = W.TPTAB, TPRIGA = W.TPRIGA, s = st.summary, ms2 = hasMs2(s);
      const chip = (key, label, help, test, on) => ({ key, label, help, test, on, ...(key === "ms2" ? {} : { group: "class" }) });
      const alt = r => r.alternatives.length ? ` <button type="button" class="tp-alt mut"${W.TPHELP ? ` data-tt="ft.alt"` : ""} data-alt="${r.id}" aria-label="${r.alternatives.length} other candidate ${r.alternatives.length === 1 ? "identity" : "identities"} with the same mass: show them">+${r.alternatives.length} other${r.alternatives.length === 1 ? "" : "s"}</button>` : "";
      const cols = [
        { key: "level", label: "Lvl", help: "ft.col.lvlr", sticky: true, sort: lvSort, cell: () => "", text: () => "" },          // empty until the chemist decides (the badge of `decide`)
        { key: "pred", off: true, label: "Prev.", help: "ft.col.pred", sort: r => r.pred ?? 9, cell: r => r.pred ? TPTAB.pred(r.pred) : "", text: r => r.pred ? `Prev. ${r.pred}` : "" },
        { key: "tpname", label: "TP", help: "ft.col.tpname", sticky: true, sort: r => r.tp_name || "", cell: r => esc(r.tp_name || "") },
        { key: "identity", label: "Identity", help: "ft.col.identity", sticky: true, sort: r => idLabel(r).toLowerCase(), cell: r => esc(idLabel(r)) + alt(r), text: r => idLabel(r) + (r.alternatives.length ? ` (+${r.alternatives.length} other)` : "") },
        { key: "status", label: "Status", help: "ft.col.status", sort: r => notFound(r) || r.status === "Parent not found" ? 1 : 0, cell: r => notFound(r) ? "Searched, not detected" : esc(r.status || "Detected"), text: r => r.status || "Detected" },
        { key: "net", label: "Net change (formula)", help: "ft.col.net", sort: r => r.net_change || "", cell: r => esc(r.net_change || "").replace(/([A-Za-z)])(\d+)/g, "$1<sub>$2</sub>") },          // subscripts in the cell only: the export and the copy keep the plain text (C2H2)
        { key: "modif", label: "Modification", help: "ft.col.modif", sort: r => r.modification || "", cell: r => esc(r.modification || "") },
        { key: "mz", label: "<i>m/z</i>", help: "ft.col.mz", num: true, sort: r => r.mz, cell: r => r.mz.toFixed(2) },
        { key: "rt", label: "RT", help: "ft.col.rt", num: true, sort: r => r.ref_rt, cell: r => r.ref_rt == null ? "" : r.ref_rt.toFixed(2) },
        { key: "trend", label: "Trend", help: "ft.col.trend", cell: r => (r.kinetics || []).length ? TPTAB.spark(r.kinetics.map(p => p.area), "area") : "", text: r => (r.kinetics || []).map(p => Math.round(p.area)).join(" ") },
        ...(s.gruppi ? [{ key: "conds", label: "Conditions", help: "ft.col.conds", sort: (/** @type {any} */ r) => { const g = gRow(r); return g ? g.present_in.length : -1; }, cell: (/** @type {any} */ r) => { const g = gRow(r); return g ? TPTAB.dots(g.cond.map((/** @type {any} */ c) => ({ id: c.id, present: c.present }))) : ""; },
          text: (/** @type {any} */ r) => { const g = gRow(r); return g ? g.present_in.join("; ") : ""; } }] : []),
        { key: "coel", label: "Co-elutes with", help: "ft.col.coel", sort: r => r.coelutes ? r.coelutes.mz : 0, cell: r => r.coelutes ? coelCell(r.coelutes) : "", text: r => r.coelutes ? coelText(r.coelutes) : "" },
        { key: "cells", label: "Cells", help: "ft.col.cells", cell: r => cellStates(r).length ? TPTAB.cells(cellStates(r), 12, r.kinetics.map(p => ({ time: p.time, area: p.area }))) : TPTAB.state("na"), text: r => cellStates(r).map(x => x[0]).join("") },
        { key: "max", label: "Max area", help: "ft.col.maxarea", num: true, sort: r => r.max_area, cell: r => notFound(r) ? "—" : fmtA(r.max_area), text: r => notFound(r) ? "—" : fmtA(r.max_area), value: r => notFound(r) ? null : r.max_area },
        { key: "score", off: true, label: "Score", help: "ft.col.score", num: true, sort: r => r.score, cell: r => r.score == null ? "" : `<span class="badge b-${r.label}">${r.score} ${LAB[r.label]}</span>`, text: r => r.score == null ? "" : String(r.score) },
        { key: "formula", off: true, label: "Neutral formula", help: "ft.col.formula", sort: r => r.formula || "", cell: r => esc(r.formula || "") },
        { key: "tmax", off: true, label: "t max", help: "ft.col.tmax", num: true, sort: r => r.tmax, cell: r => String(r.tmax ?? "") },
        ...(ms2 ? [{ key: "ms2", off: true, label: "MS2", help: "ft.col.ms2match", num: true, sort: r => r.ms2 || 0, cell: r => W.TPMS2.cell(r), text: r => String(r.ms2 || 0) }] : []),
        { key: "num", off: true, label: "#", help: "ft.col.rank", sort: r => r.id, cell: r => rowNum(r) },
      ];
      const chips = [chip("strong", "Strong", "ft.chip.strong", r => r.kind !== "unexpected" && r.label === "forte", true), chip("possible", "Possible", "ft.chip.possible", r => r.kind !== "unexpected" && r.label === "possibile", true),
        chip("weak", "Weak evidence", "ft.chip.weak", r => r.label === "debole", false), chip("unexpected", "Unexpected", "ft.chip.unexpected", r => r.kind === "unexpected" && good(r), true),
        ...(ms2 ? [chip("ms2", "With MS2", "ft.chip.ms2", r => r.ms2 > 0, false)] : [])];
      T = TPTAB.mount(host, { id: "tp-t", label: "Transformation product candidates", cols, rowId: r => r.id, esc, name: r => `${r.tp_name || idLabel(r)}, ${idLabel(r)}, m/z ${r.mz.toFixed(2)}`, columns: true, chips, decide: decideOf() || undefined, rowClass: r => notFound(r) ? "tpt-nd" : "", exportName: (s.parent.formula || "parent") + "_Tabui_view",
        search: r => [r.tp_name, r.name, r.formula, String(r.mz), r.delta, r.net_change, r.modification].join(" "), pin: r => r.label === "progenitore",
        tabs: TPRIGA.tabs, head: (r, box) => TPRIGA.head(box, headModel(r)), onOpen: (r, body, tab) => { void fillTab(r, body, tab); },
        onFilter: keys => { st.weakOn = keys.includes("weak"); weakSync(); } });
      st.weakOn = T.chips.includes("weak"); weakSync();
      host.insertAdjacentHTML("beforeend", TPTAB.legend(["detected", "absent"], true));
      T.setRows(tableRows()); T.sortBy("level", 1);
      const box = /** @type {HTMLInputElement | null} */ ($("#tp-nd"));
      if (box) box.onchange = () => { if (T) T.setRows(tableRows()); };
      // the "+N other" button of the Identity cell: open the row on the tab with the other identities (capture: before the row toggles)
      host.addEventListener("click", e => {
        const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest(".tp-alt")); if (!b || !T) return;
        e.stopPropagation(); e.preventDefault(); const id = +/** @type {string} */ (b.dataset.alt);
        if (T.openId !== id) T.open(id);
        T.selectTab("identity");
      }, true);
    }
    /** the decision of the chemist (45_tabui_decisioni.js): the series is the files and the parent, a row is its m/z and RT; no level is proposed */
    const decideOf = () => { const D = /** @type {any} */ (window).TPDEC, s = st.summary; return D && s && s.files ? { series: () => ({ files: s.files.map((/** @type {any} */ f) => f.name), formula: s.parent.formula }), key: (/** @type {any} */ r) => D.rowKey(r.mz, r.ref_rt ?? 0, false), level: () => null } : null; };
    /** the head of the open row: the decided level, the prediction, copy and the way to the card of the MS2 matches */
    function headModel(r) {
      const dc = decideOf(), acts = [{ key: "copy", label: "Copy", help: "ft.riga.copy", run: () => {
        const text = [r.tp_name || "", idLabel(r), r.net_change || "", "m/z " + r.mz.toFixed(2), "RT " + (r.ref_rt ?? ""), r.pred ? "Prev. " + r.pred : "", r.score ?? ""].join("\t");
        Promise.resolve(navigator.clipboard && navigator.clipboard.writeText(text)).then(() => toast("Copied"), () => toast("Copy is not allowed here"));
      } }];
      if (hasMs2(st.summary)) acts.push({ key: "ms2", label: "MS2 matches", help: "ft.lr.ms2link", run: () => { const d = $("#tp-m-ms2"); if (d) { d.open = true; d.scrollIntoView({ behavior: calmMotion() ? "smooth" : "auto", block: "nearest" }); } } });
      // the head repeats the closed row (its key cells)
      const summary = [{ label: "TP", value: r.tp_name || "", help: "ft.col.tpname" }, { label: "Identity", value: idLabel(r), help: "ft.col.identity" }, { label: "Net change", value: r.net_change || "", help: "ft.col.net" },
        { label: "Modification", value: r.modification || "", help: "ft.col.modif" }, { label: "m/z", value: r.mz.toFixed(2), help: "ft.col.mz" }, { label: "RT", value: r.ref_rt == null ? "" : r.ref_rt.toFixed(2) + " min", help: "ft.col.rt" },
        { label: "Max area", value: notFound(r) ? "" : fmtA(r.max_area), help: "ft.col.maxarea" }];
      return { title: (r.tp_name && r.tp_name !== "Parent" ? r.tp_name + " · " : "") + idLabel(r), pred: r.pred || undefined, summary, actions: acts, ...(dc ? { decide: { series: dc.series(), key: dc.key(r), level: null } } : {}) };
    }
    const GROUP = r => r.type === "blank" ? "Blank" : r.type === "control" ? "Dark" : r.type === "standard" ? "Other" : r.time === 0 ? "t0" : "Treated";
    /** the model of the panel of a row (TPRIGA) from its detail; nothing is computed here */
    function lrModel(r, d) {
      const by = new Map(d.rows.map(x => [x.label, x])), pk = d.traces.map(t => t.peak).filter(Boolean), missing = {};
      /** @type {any} */
      const m = { ...headModel(r), apexRt: d.ref_rt, lo: pk.length ? Math.min(...pk.map(p => p.left)) : null, hi: pk.length ? Math.max(...pk.map(p => p.right)) : null, missing };
      if (d.traces.length) m.xic = d.traces.map(t => { const x = by.get(t.label); return { file: t.label, group: GROUP(x || t), time: x ? x.time : null, rt: t.rt, y: t.y }; }); else missing.xic = "no trace for this row";
      const rp = d.ms2_rep && d.ms2_rep.mz && d.ms2_rep.mz.length ? d.ms2_rep : null;
      // what was the line under the charts is their help now: the window of the XIC, the representative scan and the fragments found
      m.xicTip = `RT ${d.ref_rt ? d.ref_rt.toFixed(2) : "?"} min${m.lo != null ? ` (${m.lo.toFixed(2)}-${m.hi.toFixed(2)})` : ""}; m/z ${d.mz_extracted} extracted${d.mz_observed ? `, ${d.mz_observed.toFixed(2)} observed` : ""}.`;
      if (rp) m.spectrum = { kind: "ms2", mz: rp.mz, y: rp.int, prec: rp.precursor, label: `MS2 at RT ${rp.rt.toFixed(2)} min, precursor m/z ${rp.precursor.toFixed(2)}`, tip: [window.TPMS2 ? window.TPMS2.repLine(d) : "", d.ms2_text || ""].filter(Boolean).join(" ") };
      else missing.spectrum = d.ms2_text || "no MS2 matched to this ion";
      const R = /** @type {any} */ (window).TPRIGA && /** @type {any} */ (window).TPRIGA.toData;
      if (R) m.open = { xic: () => R.xic(r.mz), ...(rp ? { ms2: () => R.ms2(rp.precursor, rp.rt, rp.file) } : {}) };
      m.samples = d.rows.map(x => ({ file: x.label, type: GROUP(x), time: x.time, area: x.detected ? x.area : 0, state: x.detected ? "detected" : "absent", apexRt: x.detected ? x.apex_rt : null, lo: m.lo, hi: m.hi }));
      m.identity = { neutral: null, adduct: st.summary.parent.adduct, candidates: lrCands(r, d) };
      const status = n => { const c = d.criteria.find(x => x.name.toLowerCase().includes(n)); return c ? c.status : null; };
      const best = d.rows.filter(x => x.detected).reduce((a, x) => !a || x.area > a.area ? x : a, null), pres = n => { const v = status(n); return v == null || v === "n/a" ? null : { present: v === "fail" }; };
      m.quality = { snr: best ? best.snr : null, flags: [], blank: pres("blank"), t0: pres("zero"), rejected: [] };
      return m;
    }
    /** the column of a criterion of the engine among the proofs of the candidates (TPRIGA): its name says which; the others stay in «Details» */
    const PF_OF = /** @type {[RegExp, string][]} */ ([[/^persistent/i, "persist"], [/time zero|\bt0\b/i, "t0"], [/grows/i, "grow"], [/isotope/i, "iso"]]);
    /** the candidates of the trace of a row (`candidates` of the detail: the row's own first, each with its neutral and [M+H]+ formula and Δm/z), all with the proofs of the trace */
    function lrCands(r, d) {
      const proofs = d.criteria.filter(c => c.status === "pass" || c.status === "fail").map(c => { const k = PF_OF.find(([re]) => re.test(c.name)); return { key: k ? k[1] : undefined, ok: c.status === "pass", text: `${c.name}: ${c.text}` }; });
      if (d.ms2 && d.ms2.scans) { const sh = d.ms2.fragments.filter(f => f.note).length; proofs.push({ key: "ms2", ok: sh > 0, text: `${d.ms2.scans} MS2 scans of precursor ${d.ms2.precursor}${window.TPMS2 ? window.TPMS2.from(d) : ""}; ${sh ? `${sh} fragments explained by the parent` : "no fragment matches those of the parent"}.${d.sim ? ` Against the MS2 of the parent: modified cosine ${d.sim.cos.total.toFixed(3)}.` : ""}` }); }
      const ad = st.summary.parent.adduct, list = d.candidates && d.candidates.length ? d.candidates : [{ name: idLabel(r), formula: d.formula || "", ion_formula: null, dmz: null, own: true }];
      return list.map(c => ({ name: c.own ? idLabel(r) : c.name, formula: c.formula || "", ion: c.ion_formula, dmz: c.dmz, adduct: ad, source: c.own ? d.delta || "" : "", proofs }));
    }
    /** the tab `tab` of the open row into `body`: TPRIGA draws the four parts, this adds what only the low-resolution Dig has */
    async function fillTab(r, body, tab) {
      st.sel = r.id; body.innerHTML = `<div class="mut">Computing...</div>`;
      let d; try { d = await getDetail(r.id); } catch (e) { body.innerHTML = `<div class="err">${esc(e.message || e)}</div>`; return; }
      if (!T || T.openId !== r.id || T.tab !== tab) return;                  // another row or tab was opened while this one loaded
      const TPRIGA = /** @type {any} */ (window).TPRIGA, m = lrModel(r, d);
      if (tab === "samples") {
        const g = gRow(r);      // several conditions: the kinetics are one per condition (never the samples of all conditions in one curve)
        body.innerHTML = `<div class="lr-sg"><div class="lr-s1"></div><div class="tpr-ch">${g ? `<h4>By condition</h4><div id="tp-gbc"></div>` : `<h4>Kinetics (peak area)</h4><canvas id="tp-c-kin"></canvas>`}</div></div>`;
        TPRIGA.fill(body.querySelector(".lr-s1"), tab, m);
        if (!g) kinChart(d);
        else {
          const W = /** @type {any} */ (window), box = /** @type {HTMLElement} */ (body.querySelector("#tp-gbc"));
          try { await W.TPLAZY.load("gruppi-vista.js"); } catch (_) { box.textContent = "The comparison of conditions could not be loaded."; return; }
          if (T && T.openId === r.id && T.tab === tab && box.isConnected) W.TPGRUPPI_VIEW.rowDetail(box, st.summary.gruppi, r.id);
        }
      } else {
        TPRIGA.fill(body, tab, m);
        if (tab === "identity") identityExtras(r, d, body); else if (tab === "quality") qualityExtras(r, d, body);
      }
    }
    function kinChart(d) {
      const kin = d.rows.filter(x => x.type === "sample" && x.time != null).sort((a, b) => a.time - b.time), cv = $("#tp-c-kin");
      chart(cv, [{ x: kin.map(x => x.time), y: kin.map(x => x.detected ? x.area : 0), color: "#2b5c8a", dots: true }], { xlabel: "time (min)" });
      if (window.TDATI) TDATI.series(cv, { title: "Kinetics of the ion", xl: "treatment time (min)", yl: "peak area", series: [{ name: "area", x: kin.map(x => x.time), y: kin.map(x => x.detected ? x.area : 0) }] });
    }
    /** in «Details» of the tab Identity (closed): the specificity, the MS2, the similarity, the transitions, the position of the change and the fragmentation tree */
    function identityExtras(r, d, body) {
      const par = st.summary.rows.find(x => x.label === "progenitore"), isPar = par && r.id === par.id, box = document.createElement("div");
      box.className = "lr-idx"; box.tabIndex = 0; box.setAttribute("role", "region"); box.setAttribute("aria-label", "More about the candidate");
      box.innerHTML = `${specHtml(d.specificity)}
        <div class="card">${window.TPMS2.detail(d)}${d.sim && window.TPHR ? `<h3 style="margin-top:8px">Similarity to the parent's MS2</h3>${window.TPHR.simCard(d.sim)}` : ""}
        ${d.transitions.length ? `<h3 style="margin-top:8px">Suggested MRM transitions</h3><table><tr><th>Q1</th><th>Q3</th><th>CE</th><th>RT</th></tr>${d.transitions.map(t => `<tr><td class="n">${t.Q1}</td><td class="n">${t.Q3_consigliato}</td><td class="n">${t.CE}</td><td class="n">${t.RT_attesa_min}</td></tr>`).join("")}</table>` : ""}</div>`;
      (/** @type {any} */ (window).TPRIGA.details ? /** @type {any} */ (window).TPRIGA.details(body) : body).appendChild(box);
      window.TPMS2.bindDetail(box, d);
      if (window.TPPOS && d.ms2 && d.ms2.scans) window.TPPOS.mount(box, { smiles: (($("#tp-smi") || {}).value || "").trim(), delta: d.delta || "", peaks: d.ms2.fragments.map(f => [f.mz, f.rel]), positive: !window.TPPOS.negative((($("#tp-add") || {}).value) || ""), tol: 0.5, ppm: false, label: "Change" });
      if (window.TPALBERO && (isPar || (d.ms2 && d.ms2.scans))) { const a = document.createElement("div"); a.className = "card"; a.id = "tp-alb"; box.appendChild(a); window.TPALBERO.mount(a, { esc, call }, isPar ? {} : { compareId: r.id }); }
    }
    /** under the quality figures: the isotope criterion and the verdict on the in-source fragments */
    function qualityExtras(r, d, body) {
      const iso = d.criteria.filter(c => /isotop/i.test(c.name)), isf = r.isf, P = isf ? Math.round(100 * isf.probs.isf) : null;
      const src = r.insource ? `in-source fragment of the parent${P != null ? ` (P ISF ${P} %${isf.doubtful ? ", doubtful" : ""})` : ""}` : isf && isf.doubtful ? `doubtful origin (P ISF ${P} %)` : "not flagged";
      body.insertAdjacentHTML("beforeend", `<dl class="tpr-q"><dt>Isotope</dt><dd>${iso.length ? iso.map(c => `${c.status === "pass" ? "✓" : c.status === "fail" ? "✗" : "–"} ${esc(c.name)}: ${esc(c.text)}`).join("<br>") : "no isotope peak stronger than this ion in the window"}</dd><dt>In-source</dt><dd>${esc(src)}${isf && isf.explanation ? `<br><span class="mut">${esc(isf.explanation)}</span>` : ""}</dd></dl>`);
    }
    async function getDetail(id) { return st.det[id] || (st.det[id] = await call("detail", id)); }
    /** opens the row of a candidate in the table: from the map and the kinetics one click goes back to the view "Table"; a row hidden by the chips gets the chip it needs */
    function show(id) {
      st.sel = id;
      if (st.view !== "tab") { st.view = "tab"; $("#tp-right").querySelectorAll(".vt").forEach(x => x.classList.toggle("on", x.dataset.v === "tab")); view(); }
      if (!T || T.open(id)) return;
      const r = st.summary.rows.find(x => x.id === id); if (!r) return;
      T.setChip(r.kind === "unexpected" ? "unexpected" : r.label === "forte" ? "strong" : r.label === "possibile" ? "possible" : "weak", true);
      if (!T.open(id)) { T.reset(); T.open(id); }
    }

    // ---- export
    async function exportAll(kind) {
      const s = st.summary; msg("Preparing the export...");
      try {
        const top = s.rows.filter(r => r.label === "forte" || r.label === "possibile");
        const found = s.rows.filter(r => !notFound(r));
        const det = []; for (const r of top) det.push(await getDetail(r.id));
        const times = s.times, name = (st.summary.parent.formula || "parent") + "_Tabui";
        const cg = !!s.gruppi;      // several conditions: the columns «Present in» and «Only in» (gruppi/1)
        const head = ["#", "TP", "Candidate", "Neutral formula", "Change", "Net change (formula)", "Modification", "m/z", "RT (min)", "Prediction (Löffler)", "Score", "Verdict", "t max (min)", "Max area", "Max area (number)", ...times.map(t => "Area t" + t), ...(cg ? ["Present in", "Only in"] : [])];
        const line = r => [r.id, r.tp_name, r.name, r.formula, r.delta, r.net_change ?? "", r.modification ?? "", r.mz, r.ref_rt, r.pred ?? "", r.score ?? "", LAB[r.label] || r.label, r.tmax ?? "", fmtA(r.max_area), r.max_area, ...times.map(t => { const k = r.kinetics.find(p => p.time === t); return k ? Math.round(k.area) : ""; }), ...(cg ? (g => [g ? g.present_in.join("; ") : "", g && g.only_in ? g.only_in : ""])(gRow(r)) : [])];
        const trans = det.flatMap(d => d.transitions);
        if (kind === "json") return dl(name + ".json", JSON.stringify({ summary: s, details: det, ms2_overrides: window.TPMS2 ? window.TPMS2.overrides() : [] }, null, 1), "application/json");
        if (kind === "csv") return dl(name + ".csv", "sep=;\n" + [head, ...found.map(line)].map(r => r.map(v => `"${String(v ?? "").replace(/"/g, '""')}"`).join(";")).join("\n"), "text/csv");
        const sheets = [{ name: "Ranking", head, rows: found.map(line), widths: [6, 12, 38, 16, 14, 18, 30, 10, 9, 8, 9, 11, 10, 12, 14, ...times.map(() => 11), ...(cg ? [28, 14] : [])] },
          { name: "Criteria", head: ["#", "Candidate", "Criterion", "Outcome", "Detail"], rows: det.flatMap(d => d.criteria.map(c => [d.id, d.name, c.name, c.status, c.text])), widths: [6, 38, 28, 8, 70] },
          { name: "MS2", head: ["#", "Candidate", "Product m/z", "% rel.", "Loss", "Note", "Similarity to the parent"], rows: det.flatMap(d => d.ms2.fragments.map(f => [d.id, d.name, f.mz, f.rel, f.loss ?? "", f.note || "", d.sim ? d.sim.cos.total : ""])), widths: [6, 38, 12, 9, 9, 50, 14] },
          ...(window.TPMS2 && window.TPMS2.sheet() ? [window.TPMS2.sheet()] : []),
          ...(window.TPMS2 && window.TPMS2.repSheet(det) ? [window.TPMS2.repSheet(det)] : []),
          { name: "MRM transitions", head: ["Candidate", "Neutral formula", "Q1", "Q3 observed", "Q3 suggested", "CE", "Expected RT", "RT window", "% rel.", "MS2 scans", "Note"], rows: trans.map(t => [t.compound, t.formula, t.Q1, t.Q3_osservato, t.Q3_consigliato, t.CE, t.RT_attesa_min, t.finestra_RT_min, t["intensita_rel_%"], t.scansioni_ms2, t.nota]), widths: [34, 16, 8, 12, 14, 7, 10, 11, 8, 12, 40] },
          ...(s.mrm ? [mrmSheet(s.mrm)] : []),
          { name: "Information", head: ["Item", "Value"], rows: [["Parent", s.parent.formula], ["Neutral mass", s.parent.neutral], ["Adduct", s.parent.adduct], ["Parent m/z", s.parent.mz], ["m/z offset (Da)", s.offset ? s.offset.offset : ""], ["k (1/min)", s.decay ? s.decay.k_per_min : ""], ["t1/2 (min)", s.decay ? s.decay.half_life_min : ""], ...Object.entries(s.settings).map(([k, v]) => [k, String(v)]), ...s.files.map(f => ["File", `${f.name} (${f.kind}, ${f.type}, t=${f.time ?? ""})`]), ["Note", "Unit resolution: each row is a candidate, not an identification. No identification level is proposed: the chemist decides it. Prediction = Löffler et al. 2025, not a level."]], widths: [22, 90] }];
        dlx(name + ".xlsx", sheets);
      } catch (e) { msg(String(e.message || e), true); return; }
      msg("");
    }
    function mrmSheet(m) {
      const q = m.quantifier, qual = m.transitions.slice(1).map(t => t.name);
      return { name: "MRM", head: ["File", "Type", "t (min)", ...(m.calibration ? ["Nominal conc. (mg/L)"] : []), "Area " + q, "RT", "S/N", ...qual.flatMap(n => ["Area " + n, "Qual/Quant " + n]), "Ion ratio", ...(m.calibration ? ["Calculated conc. (mg/L)", "Note"] : [])],
        rows: m.rows.map(r => [r.file, r.type, r.time ?? "", ...(m.calibration ? [r.conc ?? ""] : []), Math.round(r.ion[q].area), r.ion[q].rt ?? "", Math.round(r.ion[q].snr), ...qual.flatMap(n => [Math.round(r.ion[n].area), r.ratios[n] ?? ""]), r.ratio_ok == null ? "" : r.ratio_ok ? "ok" : "out", ...(m.calibration ? [r.quant ?? "", r.in_range === false ? "outside the calibration range" : ""] : [])]),
        widths: [34, 10, 8, ...(m.calibration ? [14] : []), 12, 8, 8, ...qual.flatMap(() => [12, 12]), 12, ...(m.calibration ? [16, 34] : [])] };
    }
    const dl = (n, text, type) => window.dl(n, text, type, false);
    return { render, toast, dl };
  };
})();
