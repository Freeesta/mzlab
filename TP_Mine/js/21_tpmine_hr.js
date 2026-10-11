// TPMINE-PRIVATE  TP Mine, high-resolution mode: ranking of the possible TPs, detail of a candidate (kinetics by session, mirror MS2 with S/U evidence,
// localisation, criteria), MSn tree, exports. Called by 20_tpmine.js (window.TPHR.render) with its helpers; it does not talk to the worker itself.
(() => {
  "use strict";
  const CSS = `
.tp .hrn{border-left:3px solid #d99a1d;background:rgba(217,154,29,.10);padding:6px 10px;border-radius:0 6px 6px 0;font-size:12px;margin:0 0 8px}
.tp .comp{display:grid;grid-template-columns:90px 1fr 42px;gap:6px;align-items:center;font-size:12px}.tp .comp i{display:block;height:8px;background:var(--accent);border-radius:4px}
.tp .flag{color:#9a5b00;font-size:11px}
.tp .fl-s{color:#b42318;font-weight:600}.tp .fl-u{color:#2b5c8a;font-weight:600}
.tp{--sim-eq:#0072B2;--sim-sh:#D55E00;--sim-nl:#009E73}
.tp{--rta-0:#000000;--rta-1:#E69F00;--rta-2:#56B4E9;--rta-3:#009E73;--rta-4:#F0E442;--rta-5:#0072B2;--rta-6:#D55E00;--rta-7:#CC79A7}
.tp .rta-svg{display:block;width:100%;max-width:640px;height:auto}.tp .rta-svg text{fill:var(--muted);font-size:10px}.tp .rta-svg .ax{stroke:var(--line);stroke-width:1}
.tp .rta-svg .zero{stroke:var(--muted);stroke-dasharray:2 3}.tp .rta-leg{display:flex;flex-wrap:wrap;gap:2px 14px;font-size:11px;margin-top:4px}
.tp .rta-open{display:inline-block;width:8px;height:8px;border:2px solid var(--muted);border-radius:50%;vertical-align:middle;margin-right:4px}
.tp .pd-1{--lvc:#009E73}.tp .pd-2{--lvc:#0072B2}.tp .pd-3{--lvc:#E69F00}.tp .pd-4{--lvc:#999999}
.tp .sim-bar{display:inline-flex;width:64px;height:8px;border-radius:4px;background:var(--soft);overflow:hidden;vertical-align:middle}.tp .sim-bar i{display:block;height:100%}
.tp .sim3{display:grid;grid-template-columns:150px 1fr 1fr 44px;gap:4px 10px;font-size:12px;align-items:center;margin:6px 0}
.tp .sim3 .bar3{display:block;height:8px;border-radius:4px;background:var(--soft);overflow:hidden}.tp .sim3 .bar3 i{display:block;height:100%}
.tp .sw{display:inline-block;width:14px;height:0;border-top:3px solid;vertical-align:middle;margin-right:5px}
.tp .mnet-wrap{overflow:auto;max-height:70vh;border:1px solid var(--line);border-radius:6px;background:var(--panel)}
.tp .mn{cursor:pointer}.tp .mn:focus-visible circle,.tp .mn.sel circle{stroke-width:3.5}.tp .mn text{fill:var(--ink);font-size:10px;text-anchor:middle;pointer-events:none}
.tp .me text{fill:var(--muted);font-size:9px;text-anchor:middle}`;
  const LV_TEXT = { 3: "region localised", 4: "formula and derivation", 5: "appears with the treatment" };       // the badges and the colours are those of livelli.js (.lvb, .lv-N)
  const SIM_LABEL = ["equal fragments", "shifted fragments", "common neutral losses"], SIM_VAR = ["--sim-eq", "--sim-sh", "--sim-nl"], SIM_DASH = [[], [6, 3], [2, 3]];
  const PD_TEXT = { 1: "confirmed by the data", 2: "observed in the literature", 3: "specific to the class", 4: "generic rule" };
  const predBadge = r => r.pred ? `<span class="lvb pd-${r.pred}" title="prediction level ${r.pred}: ${esc(PD_TEXT[r.pred])}">P${r.pred}</span>` : "";
  const simBar = sim => sim ? `<span class="sim-bar" role="img" aria-label="cosine: equal ${sim.cos[0]}, shifted ${sim.cos[1]}, neutral losses ${sim.cos[2]}" title="cosine: equal ${sim.cos[0]} · shifted ${sim.cos[1]} · common neutral losses ${sim.cos[2]}">${sim.cos.map((v, i) => `<i style="width:${Math.round(100 * v)}%;background:var(${SIM_VAR[i]})"></i>`).join("")}</span>` : "";
  const lvBadge = (r, title) => `<span class="lvb lv-${r.level}" title="${esc(title)}">${r.level}${r.diagnostic ? " ◆" : ""}</span>`;

  // Shift of the apex of the anchor features in every file against the median over the files (piecewise linear between the nodes of the curve).
  const RTA_DASH = [[], [6, 3], [2, 3], [8, 2, 2, 2]];
  function rtAlign(a, esc) {
    if (!a || !a.rt || !a.rt.length) return "";
    const W = 560, H = 200, L = 46, T = 10, B = 30, Rt = 12, lo = Math.min(...a.rt), hi = Math.max(...a.rt, lo + 1);
    const ym = Math.max(0.05, ...a.shift.flat().map(Math.abs)), X = v => L + (v - lo) / (hi - lo) * (W - L - Rt), Y = v => T + (1 - (v + ym) / (2 * ym)) * (H - T - B);
    const line = (k, sh) => `<polyline fill="none" stroke="var(--rta-${k % 8})" stroke-width="1.6" stroke-dasharray="${RTA_DASH[Math.floor(k / 8) % 4].join(" ")}" points="${a.rt.map((r, j) => X(r).toFixed(1) + "," + Y(sh[j]).toFixed(1)).join(" ")}"/>`;
    const xt = [0, 1, 2, 3, 4].map(i => lo + (hi - lo) * i / 4), yt = [-ym, 0, ym];
    const svg = `<svg class="rta-svg" viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(`RT shift of ${a.files.length} files against the median, ${a.anchors} anchor features, largest shift ${ym.toFixed(2)} min`)}">
      <line class="ax" x1="${L}" y1="${T}" x2="${L}" y2="${H - B}"/><line class="ax" x1="${L}" y1="${H - B}" x2="${W - Rt}" y2="${H - B}"/><line class="zero" x1="${L}" y1="${Y(0)}" x2="${W - Rt}" y2="${Y(0)}"/>
      ${xt.map(v => `<text x="${X(v).toFixed(1)}" y="${H - B + 14}" text-anchor="middle">${v.toFixed(1)}</text>`).join("")}${yt.map(v => `<text x="${L - 4}" y="${(Y(v) + 3).toFixed(1)}" text-anchor="end">${v > 0 ? "+" : ""}${v.toFixed(2)}</text>`).join("")}
      <text x="${(L + W - Rt) / 2}" y="${H - 4}" text-anchor="middle">RT (min)</text><text transform="rotate(-90)" x="${-(T + H - B) / 2}" y="11" text-anchor="middle">shift (min)</text>
      ${a.shift.map((sh, k) => line(k, sh)).join("")}</svg>`;
    const leg = a.files.map((f, k) => `<span><span class="sw" style="border-top-color:var(--rta-${k % 8});border-top-style:${RTA_DASH[Math.floor(k / 8) % 4].length ? "dashed" : "solid"}"></span>${esc(f)}</span>`).join("");
    return `<details class="card" id="hr-rta"><summary>Retention time alignment (${a.anchors} anchor features)</summary>
      <div class="mut">Shift of the apex of the anchors (strong, alone in their <i>m/z</i> and RT neighbourhood, present in the files) in each file against the median of the files. No signal is moved: the curve tells where a feature is expected in a file when its area is filled.</div>${svg}<div class="rta-leg">${leg}</div></details>`;
  }

  function render(R, s, h) {
    if (!document.getElementById("tp-hr-css")) { const st = document.createElement("style"); st.id = "tp-hr-css"; st.textContent = CSS; document.head.appendChild(st); }
    const { esc, fmtA, chart, hue, call, toast, dl } = h;
    const $ = q => R.querySelector(q);
    const rows = s.rows, st = { sel: null, minLevel: 5, text: "", onlyLoc: false, det: {}, order: "score" };
    const lvRank = r => r.diagnostic ? 4 : +r.level;
    const n = l => rows.filter(r => String(r.level) === l && !r.diagnostic).length, nDiag = rows.filter(r => r.diagnostic).length;
    const nMs2 = rows.filter(r => r.ms2_scans > 0).length;
    R.innerHTML = `<div class="hero"><div class="hero-t"><span class="hero-i">⛏</span>
      <div><h2>Ranking of possible TPs</h2><div class="hero-s">${esc(s.parent.name || "")} ${esc(s.parent.formula)} · ${esc(s.parent.ion)} <i>m/z</i> ${s.parent.mz}${s.parent.rt ? ` · RT ${s.parent.rt} min` : ""} · high resolution · ${s.timing.total} s</div></div></div>
      <div class="stats"><div class="stat g"><b>${nDiag}</b><span>level 3, diagnostic evidence ◆</span></div><div class="stat a"><b>${n("3")}</b><span>level 3</span></div><div class="stat v"><b>${n("4") + n("5")}</b><span>level 4-5</span></div>
      <div class="stat"><b>${nMs2}</b><span>with MS<sup>2</sup></span></div><div class="stat"><b>${s.n_candidates}</b><span>candidates after filters</span></div><div class="stat"><b>${s.files.length}</b><span>files</span></div></div>
      <div class="row" style="margin-top:10px"><button id="hr-x-xlsx">Excel (.xlsx)</button><button id="hr-x-csv">CSV</button><button id="hr-x-json">JSON</button><button id="hr-x-incl" title="Inclusion list for a second targeted injection: the best candidates without MS2">Inclusion list (CSV)</button></div></div>
      <div class="card hrn">Each row is a hypothesis, not an identification: the level says how much evidence supports it (5 appears with the treatment, 4 unique and derivable formula, 3 MS² linked to the parent and region localised; ◆ diagnostic evidence: site determined with margin, consistent RT and kinetics, which only the chemist can turn into level 2b). Levels 2a and 1 require a standard.</div>
      ${s.warnings.length ? `<div class="card">${s.warnings.map(w => `<div class="flag">⚠ ${esc(w)}</div>`).join("")}</div>` : ""}
      <div class="card"><div class="row"><label class="mut">Minimum level</label><select id="hr-lv"><option value="5">all</option><option value="4">4 or higher</option><option value="3">3 or higher</option><option value="d">3 with diagnostic evidence only</option></select>
        <input type="text" id="hr-q" placeholder="filter: formula, m/z, region" class="grow"><label class="mut">Sort by</label><select id="hr-ord"><option value="score">score</option><option value="level">level, then score</option></select><label><input type="checkbox" id="hr-loc"> only with region</label><span class="mut" id="hr-cnt"></span></div>
        <div class="tbl"><table id="hr-t"></table></div></div>
      <div id="tp-det"></div>
      <div class="card" id="hr-mnet"></div>
      <details class="card"><summary>Applied filters (funnel) and sessions</summary>${s.funnel.map(f => `<div class="mut">${f.n.toLocaleString("en")} · ${esc(f.text)}</div>`).join("")}
        <div class="mut" style="margin-top:6px">Sessions: ${esc([...new Set(s.sessions.labels)].map(l => l + " (" + s.sessions.labels.filter(x => x === l).length + " files)").join(", "))}${s.sessions.factors.applied ? "; areas rescaled to the reference session" : "; no correction needed"}.</div>
        <div class="mut">Timings (s): ${esc(Object.entries(s.timing).map(([k, v]) => k + " " + v).join(" · "))}</div></details>
      ${rtAlign(s.rt_align, esc)}
      <div class="card" id="hr-tree"></div>`;

    function spark(series) {
      const mx = Math.max(...series) || 1, w = 70, ht = 18, pts = series.map((v, i) => `${(i / Math.max(1, series.length - 1) * (w - 4) + 2).toFixed(1)},${(ht - 2 - v / mx * (ht - 4)).toFixed(1)}`).join(" ");
      return `<svg width="${w}" height="${ht}"><polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;
    }
    const ordered = () => st.order === "level" ? [...rows].sort((x, y) => lvRank(y) - lvRank(x) || y.score - x.score) : rows;
    const keep = () => ordered().filter(r => (st.minLevel === 5 || lvRank(r) >= (st.minLevel === "d" ? 4 : st.minLevel)) && (!st.onlyLoc || r.region) &&
      (!st.text || [r.formula, String(r.mz), r.region, r.delta, r.derivation].join(" ").toLowerCase().includes(st.text)));
    function table() {
      const o = ordered(), k = keep();
      $("#hr-cnt").textContent = `${k.length} of ${rows.length}`;
      $("#hr-t").innerHTML = `<tr><th>#</th><th>Lvl</th><th title="prediction level (Löffler et al. 2025)">Pred.</th><th title="modified cosine split in equal / shifted / common neutral loss fragments">Sim.</th><th>Score</th><th><i>m/z</i></th><th>Formula (ppm)</th><th>Change</th><th>RT</th><th>t max</th><th>Class</th><th>Region</th><th>Kinetics</th></tr>` +
        k.map((r, i) => `<tr class="clk${r.id === st.sel ? " sel" : ""}" data-id="${r.id}"><td>${o.indexOf(r) + 1}</td><td>${lvBadge(r, r.diagnostic ? "level 3, diagnostic evidence" : LV_TEXT[r.level] || "")}</td>
          <td>${predBadge(r)}</td><td>${simBar(r.sim)}</td><td class="n">${r.score}</td><td class="n">${r.mz.toFixed(4)}</td><td>${esc(r.formula || "-")}${r.ppm != null ? ` <span class="mut">${r.ppm >= 0 ? "+" : ""}${r.ppm}</span>` : ""}${r.n_formulas > 1 ? ` <span class="mut" title="possible formulas within 3 ppm">(${r.n_formulas})</span>` : ""}</td>
          <td>${esc(r.derivation || r.delta || "")}${r.flags.length ? `<div class="flag">${r.flags.map(esc).join("; ")}</div>` : ""}</td><td class="n">${r.rt}</td><td class="n">${r.tmax ?? ""}</td><td>${esc(r.class || "")}</td>
          <td>${esc(r.region || "")}</td><td>${spark(r.series)}${r.filled.length ? ` <span class="mut" title="${r.filled.length} area(s) re-integrated where no peak was found (filled)">○${r.filled.length}</span>` : ""}</td></tr>`).join("");
      $("#hr-t").querySelectorAll("tr.clk").forEach(tr => tr.onclick = () => show(+tr.dataset.id));
    }
    $("#hr-lv").onchange = e => { st.minLevel = e.target.value === "d" ? "d" : +e.target.value; table(); };
    $("#hr-q").oninput = e => { st.text = e.target.value.trim().toLowerCase(); table(); };
    $("#hr-ord").onchange = e => { st.order = e.target.value; table(); };
    $("#hr-loc").onchange = e => { st.onlyLoc = e.target.checked; table(); };
    if (window.TPALBERO) window.TPALBERO.mount($("#hr-tree"), h, { compareId: () => st.sel });
    table();

    const fmtDm = v => (v >= 0 ? "+" : "") + v.toFixed(4);
    function massNet() {
      const host = $("#hr-mnet"), net = s.mass_network, ns = st.net || (st.net = { only: "all", every: false });
      if (!net || !net.edges.length) { host.innerHTML = `<h3>Mass differences</h3><div class="mut">No candidate differs from the parent or from another ion of the series by a transformation of the list.</div>`; return; }
      const lab = n => n.id === "progenitore" ? "parent" : (n.formula || "");
      const best = net.nodes.filter(n => n.id !== "progenitore").sort((a, b) => (b.score || 0) - (a.score || 0));
      const keepIds = new Set(["progenitore", ...(ns.every ? best : best.slice(0, 60)).map(n => n.id)]);
      const edges = net.edges.filter(e => keepIds.has(e.from) && keepIds.has(e.to) && (ns.only === "all" || (ns.only === "network" ? e.kind !== "mass" : (e.from === st.sel || e.to === st.sel))));
      const nodes = net.nodes.filter(n => keepIds.has(n.id) && (n.id === "progenitore" || edges.some(e => e.from === n.id || e.to === n.id)));
      const depth = new Map([["progenitore", 0]]);
      for (let ch = true; ch;) { ch = false; for (const e of edges) if (depth.has(e.from) && !depth.has(e.to)) { depth.set(e.to, depth.get(e.from) + 1); ch = true; } }
      const last = Math.max(0, ...depth.values()) + 1;
      nodes.forEach(n => { if (!depth.has(n.id)) depth.set(n.id, last); });
      const cols = [...new Set(nodes.map(n => depth.get(n.id)))].sort((a, b) => a - b), col = new Map(cols.map((d, i) => [d, i])), pos = new Map();
      cols.forEach(d => nodes.filter(n => depth.get(n.id) === d).sort((a, b) => (a.mz || 0) - (b.mz || 0)).forEach((n, i) => pos.set(n.id, { x: 60 + col.get(d) * 200, y: 34 + i * 50 })));
      const W = 120 + (cols.length - 1) * 200, H = 40 + Math.max(1, ...cols.map(d => nodes.filter(n => depth.get(n.id) === d).length)) * 50;
      const showLab = edges.length <= 40;
      const line = e => {
        const a = pos.get(e.from), b = pos.get(e.to); if (!a || !b) return "";
        const dx = b.x - a.x, dy = b.y - a.y, L = Math.hypot(dx, dy) || 1, ux = dx / L, uy = dy / L, x1 = a.x + ux * 16, y1 = a.y + uy * 16, x2 = b.x - ux * 18, y2 = b.y - uy * 18;
        const stroke = e.kinetic === "against" ? "var(--bad)" : "var(--muted)", dash = e.kind === "both" ? "" : e.kind === "mass" ? "6 3" : "2 3", on = e.from === st.sel || e.to === st.sel;
        const tip = `${e.rule} (${fmtDm(e.dm)})${e.kind === "network" ? ", product network" : ""}${e.kinetic === "against" ? ", the maximum of the product is before its source" : ""}${e.shifted ? `, ${e.shifted.n} shifted fragments` : ""}`;
        return `<g class="me"><line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" stroke="${stroke}" stroke-width="${e.kind === "both" || on ? 2.4 : 1.3}" ${dash ? `stroke-dasharray="${dash}"` : ""} marker-end="url(#mnarr)"><title>${esc(tip)}</title></line>
          ${showLab || on ? `<text x="${((x1 + x2) / 2).toFixed(1)}" y="${((y1 + y2) / 2 - 3).toFixed(1)}">${esc(e.rule)} ${fmtDm(e.dm)}${e.shifted && e.shifted.n ? " ✓" + e.shifted.n : ""}</text>` : ""}</g>`;
      };
      const node = n => { const p = pos.get(n.id), lv = n.level, c = lv ? `var(--lv${lv})` : "var(--accent)";
        return `<g class="mn${n.id === st.sel ? " sel" : ""}" data-id="${n.id}" transform="translate(${p.x},${p.y})" tabindex="0" role="button" aria-label="${esc(lab(n))}${n.mz ? `, m/z ${n.mz}` : ""}${lv ? `, level ${lv}` : ""}">
          <circle r="14" style="fill:color-mix(in srgb,${c} 25%,var(--panel));stroke:${c}" stroke-width="2"></circle><text y="4">${n.id === "progenitore" ? "M" : lv || ""}</text><text y="29">${esc(lab(n))}</text></g>`; };
      const row = e => { const f = id => id === "progenitore" ? "parent" : ((net.nodes.find(n => n.id === id) || {}).formula || id);
        return `<tr><td>${esc(f(e.from))}</td><td>${esc(f(e.to))}</td><td>${esc(e.rule)}</td><td class="n">${fmtDm(e.dm)}</td><td>${e.scope ? esc(e.scope) : ""}</td><td>${esc(e.kinetic)}</td><td class="n">${e.shifted ? e.shifted.n : ""}</td><td>${e.kind === "both" ? "mass difference and product network" : e.kind === "mass" ? "mass difference" : "product network"}</td></tr>`; };
      host.innerHTML = `<h3>Mass differences between the ions of the series <span class="mut">${edges.length} of ${net.n_edges} edges</span></h3>
        <div class="mut">An arrow joins two ions whose formulas differ by a transformation of the list, from the one that peaks first. Solid: it is also the precursor chosen in the product network; dashed: only the mass difference; dotted: precursor of the network that is not a single rule (two steps or related by MS²); red: the maximum of the product is before its source. ✓n: fragments shifted by the mass difference in the MS² of both ions. The circle shows the confidence level.</div>
        <div class="row"><label class="mut" for="hr-mn-f">Show</label><select id="hr-mn-f"><option value="all">all mass differences</option><option value="network">product network only</option><option value="sel">only those of the selected candidate</option></select>
          <label><input type="checkbox" id="hr-mn-all"> all ions (not only the 60 best)</label></div>
        <div class="mnet-wrap"><svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" role="group" aria-label="Network of mass differences: ${nodes.length} ions, ${edges.length} edges"><defs><marker id="mnarr" markerWidth="8" markerHeight="8" refX="6" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="var(--muted)"></path></marker></defs>
          ${edges.map(line).join("")}${nodes.map(node).join("")}</svg></div>
        <details><summary>Edges as a table (${edges.length})</summary><div class="tbl"><table><tr><th>From</th><th>To</th><th>Rule</th><th>Δ<i>m/z</i></th><th>Rule scope</th><th>Kinetics</th><th>Shifted fragments</th><th>Kind</th></tr>${edges.slice(0, 300).map(row).join("")}</table></div></details>`;
      $("#hr-mn-f").value = ns.only; $("#hr-mn-all").checked = ns.every;
      $("#hr-mn-f").onchange = e => { ns.only = e.target.value; massNet(); }; $("#hr-mn-all").onchange = e => { ns.every = e.target.checked; massNet(); };
      host.querySelectorAll("g.mn").forEach(g => { const go = () => { if (g.dataset.id !== "progenitore") show(+g.dataset.id); }; g.onclick = go; g.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(); } }; });
    }

    function simCard(sim) {
      const row = i => { const k = ["equal", "shifted", "loss"][i]; return `<span><span class="sw" style="border-color:var(${SIM_VAR[i]});border-top-style:${i === 0 ? "solid" : i === 1 ? "dashed" : "dotted"}"></span>${SIM_LABEL[i]}</span>
        <span class="bar3" title="cosine ${sim.cos[k].toFixed(3)}"><i style="width:${Math.round(100 * sim.cos[k])}%;background:var(${SIM_VAR[i]})"></i></span>
        <span class="bar3" title="entropy ${sim.ent[k].toFixed(3)}"><i style="width:${Math.round(100 * sim.ent[k])}%;background:var(${SIM_VAR[i]})"></i></span><span class="n">${sim.n[k]}</span>`; };
      return `<div class="sim3"><b></b><b>cosine</b><b>entropy</b><b title="matched peaks">n</b>${[0, 1, 2].map(row).join("")}
        <b>total</b><span class="n">${sim.cos.total.toFixed(3)} <span class="mut">(modified cosine)</span></span><span class="n">${sim.ent.total.toFixed(3)}</span><span></span></div>
        <div class="mut">Mass difference of the precursors ${sim.delta >= 0 ? "+" : ""}${sim.delta.toFixed(4)}. Equal: the fragment does not contain the modification. Shifted: it moved with the precursor, so it contains it. Common neutral losses: shifted fragments close to the precursor (the same small neutral is lost by both).</div>`;
    }
    async function detail(id) { return st.det[id] || (st.det[id] = await call("detail", id)); }
    async function show(id) {
      st.sel = id; table(); if (st.net && st.net.only === "sel") massNet();
      const D = $("#tp-det"); D.innerHTML = `<div class="card mut">Computing...</div>`;
      let d; try { d = await detail(id); } catch (e) { D.innerHTML = `<div class="card err">${esc(e.message || e)}</div>`; return; }
      const r = d.row, loc = d.localization, w = s.weights;
      const comp = Object.keys(w).filter(k => typeof d.components[k] === "number").map(k => [k, d.components[k]]).map(([k, v]) => `<div class="comp" title="${esc((d.components.text || {})[k] || "")}"><span>${esc(k)} <span class="mut">×${w[k]}</span></span><span><i style="width:${Math.round(100 * v)}%"></i></span><span class="mut">${v.toFixed(2)}</span></div>`).join("");
      const kin = d.kinetics || {};
      D.innerHTML = `<div class="card"><h3>${esc(r.formula || "formula not assigned")} ${lvBadge(r, d.level_text || "")} <span class="mut">${esc(d.level_text || "")}</span></h3>
        <div class="kv"><b><i>m/z</i></b><span>${r.mz}${r.ppm != null ? ` · ${r.ppm >= 0 ? "+" : ""}${r.ppm} ppm` : ""}${r.n_formulas > 1 ? ` · ${r.n_formulas} possible formulas` : ""}</span><b>Change</b><span>${esc(r.derivation || "-")} <span class="mut">(${esc(r.delta || "")})</span></span>
        <b>RT</b><span>${r.rt} min${s.parent.rt ? ` (${(r.rt - s.parent.rt) >= 0 ? "+" : ""}${(r.rt - s.parent.rt).toFixed(2)} relative to the parent)` : ""}</span>
        <b>Kinetics</b><span>${esc(kin.class_text || "-")}${kin.tmax != null ? ` · maximum at ${kin.tmax} min` : ""}${kin.onset != null ? ` · appears at ${kin.onset} min` : ""}${kin.unimodal === false ? " · not unimodal" : ""}</span>
        <b>Region</b><span>${r.region ? esc(r.region) : "<span class='mut'>not localised</span>"}${loc && loc.ok ? ` <span class="mut">· margin ${loc.margin.toFixed(2)} · ${Math.round(100 * loc.fraction_explained)}% of intensity explained</span>` : (loc && loc.note ? ` <span class="mut">${esc(loc.note)}</span>` : "")}</span>
        <b>Origin</b><span>${d.predecessor && d.predecessor.predecessor != null ? `from ${d.predecessor.predecessor === "progenitore" ? "parent" : "#" + esc(d.predecessor.predecessor)}${d.predecessor.transformation ? ", " + esc(d.predecessor.transformation) : ""}${d.predecessor.cos != null ? `, cosine ${(+d.predecessor.cos).toFixed(2)}` : ""}` : "<span class='mut'>no precursor found</span>"}</span>
        ${d.prediction ? `<b>Prediction</b><span>${predBadge(r)} ${esc(d.prediction.text)}<ul class="crit" style="margin:2px 0 0;padding-left:18px">${d.prediction.evidence.map(e => `<li class="mut">${esc(e)}</li>`).join("")}</ul></span>` : ""}
        ${d.isotopes ? `<b>Isotopes</b><span>${esc(d.isotopes.text || d.isotopes.status)}</span>` : ""}${r.flags.length ? `<b>Flags</b><span class="flag">${r.flags.map(esc).join("; ")}</span>` : ""}</div></div>
        <div class="cols"><div class="card"><h3>Kinetics by session</h3><canvas id="hr-kin"></canvas><div class="row" id="hr-kleg"></div></div><div class="card"><h3>Score ${r.score}</h3>${comp}
          <h3 style="margin-top:10px">Criteria</h3><ul class="crit" style="margin:0;padding-left:18px">${d.criteria.map(c => `<li class="${c.status}"><b>${c.status === "pass" ? "✓" : c.status === "fail" ? "✗" : "–"} ${esc(c.name)}</b> <span class="mut">${esc(c.text)}</span></li>`).join("")}</ul></div></div>
        ${d.ms2 ? `<div class="card"><h3>MS<sup>2</sup> of the candidate (below) and of the parent (above)</h3><canvas id="hr-ms2" style="height:300px"></canvas>
          ${d.similarity ? simCard(d.similarity) : `<div class="mut"><span class="fl-s">■ shifted (S)</span>: contains the modification · <span class="fl-u">■ unshifted (U)</span>: the modified part is outside the fragment · grey: no relation to the parent</div>`}</div>` :
          `<div class="card mut">No DDA MS² for this ion: no localisation. A second injection with an inclusion list is needed.</div>`}
        ${loc && loc.ok ? `<div class="card"><h3>Localisation</h3><div class="row"><button id="hr-cmp-tree">Compare with parent tree</button></div><div class="mut">${loc.n_sites} candidate sites · type ${esc(loc.type)} · ${loc.n_shifted} S fragments, ${loc.n_unshifted} U</div>
          <table><tr><th>Site</th><th>log-likelihood</th></tr>${loc.top_sites.map(t => `<tr><td>${esc(t.site)}</td><td class="n">${t.ll.toFixed(2)}</td></tr>`).join("")}</table>
          <table style="margin-top:6px"><tr><th><i>m/z</i></th><th>%</th><th>formula</th><th>evidence</th></tr>${loc.evidence.map(e => `<tr><td class="n">${e.mz.toFixed(4)}</td><td class="n">${e.rel.toFixed(0)}</td><td>${esc(e.formula)}</td><td class="${e.kind === "S" ? "fl-s" : "fl-u"}">${e.kind === "S" ? "S (shifted)" : "U (unshifted)"}</td></tr>`).join("")}</table></div>` : ""}
        ${d.iimn ? `<div class="card"><h3>Ion family (same peak)</h3><div class="mut">${esc(d.iimn.explained_text || d.iimn.role || "")}</div></div>` : ""}
        ${d.isomers && d.isomers.k > 1 ? `<div class="card"><h3>Co-eluting isomers: ${d.isomers.k} components</h3>${d.isomers.components.map((c, i) => `<div class="mut">component ${i + 1}: apex at RT ${c.rt_apex.toFixed(2)} min, ${c.mz.length} MS² peaks</div>`).join("")}</div>` : ""}`;
      D.scrollIntoView({ behavior: "smooth", block: "nearest" });
      const labs = [...new Set(d.sessions)], t = d.times, series = [];
      labs.forEach((lab, j) => {
        const ix = t.map((_, i) => i).filter(i => d.sessions[i] === lab && t[i] >= 0).sort((a, b) => t[a] - t[b]);
        const fl = new Set(d.row.filled || []);
        if (ix.length) series.push({ x: ix.map(i => t[i]), y: ix.map(i => d.areas[i]), color: hue(j, Math.max(2, labs.length)), dots: true, hollow: ix.map((i, p) => fl.has(i) ? p : -1).filter(p => p >= 0), label: lab });
      });
      chart($("#hr-kin"), series, { xlabel: "treatment time (min)" });
      $("#hr-kleg").innerHTML = series.map(x => `<span class="mut"><span style="display:inline-block;width:14px;border-top:2px solid ${x.color};vertical-align:middle"></span> ${x.label === "?" ? "area" : "session " + esc(x.label)}</span>`).join("") +
        ((d.row.filled || []).length ? `<span class="mut"><span class="rta-open"></span> filled: area re-integrated where no peak was found (${d.row.filled.map(k => t[k]).join(", ")} min)</span>` : "");
      if (d.ms2) mirror($("#hr-ms2"), d);
      const cb = $("#hr-cmp-tree");
      if (cb) cb.onclick = () => { window.TPALBERO && window.TPALBERO.compare(id); const t = $("#hr-tree"); if (t) t.scrollIntoView({ behavior: "smooth", block: "nearest" }); };
    }

    function mirror(cv, d) {
      const dpr = devicePixelRatio || 1, W = cv.clientWidth || 600, H = cv.clientHeight || 300;
      cv.width = W * dpr; cv.height = H * dpr;
      const g = cv.getContext("2d"); g.scale(dpr, dpr); g.clearRect(0, 0, W, H);
      const P = d.ms2.parent, C = d.ms2.candidate, ev = (d.localization && d.localization.evidence) || [], pairs = (d.similarity && d.similarity.pairs) || [];
      const cs = getComputedStyle(cv), typeIx = t => ["equal", "shifted", "loss"].indexOf(t), typeCol = t => cs.getPropertyValue(SIM_VAR[typeIx(t)]).trim() || "#888";
      const near = (list, key, m) => list.find(p => Math.abs(p[key] - m) < 1e-4);
      const M = { l: 40, r: 10, t: 10, b: 24 }, pw = W - M.l - M.r, mid = M.t + (H - M.t - M.b) / 2, half = (H - M.t - M.b) / 2 - 12;
      const lo = Math.max(50, Math.min(...P.mz, ...C.mz) - 10), hi = Math.max(...P.mz, ...C.mz) + 15, X = v => M.l + (v - lo) / (hi - lo) * pw;
      const css = getComputedStyle(document.documentElement), ink = css.getPropertyValue("--muted") || "#666", line = css.getPropertyValue("--line") || "#ddd";
      g.strokeStyle = line; g.beginPath(); g.moveTo(M.l, mid); g.lineTo(W - M.r, mid); g.stroke();
      g.fillStyle = ink; g.font = "11px system-ui"; g.textAlign = "center";
      const step = Math.max(10, Math.round((hi - lo) / 8 / 10) * 10);
      for (let v = Math.ceil(lo / step) * step; v <= hi; v += step) g.fillText(v, X(v), H - 8);
      const peaks = (spec, sign, colorOf) => spec.mz.forEach((m, i) => {
        const y = mid - sign * spec.rel[i] / 100 * half, c = colorOf(m); g.strokeStyle = c; g.lineWidth = 1.6; g.beginPath(); g.moveTo(X(m), mid); g.lineTo(X(m), y); g.stroke();
        if (spec.rel[i] >= 8) { g.fillStyle = c; g.textAlign = "center"; const f = sign < 0 ? (ev.find(e => Math.abs(e.mz - m) < 0.003) || {}).formula : null; g.fillText(f || m.toFixed(3), X(m), sign > 0 ? y - 3 : y + 11); }
      });
      for (const p of pairs) {        // the matched pairs: a line between the two peaks, colour and dash by type (equal solid, shifted dashed, neutral loss dotted)
        const ya = mid - (P.rel[P.mz.findIndex(v => Math.abs(v - p.a) < 1e-4)] || 0) / 100 * half, yb = mid + (C.rel[C.mz.findIndex(v => Math.abs(v - p.b) < 1e-4)] || 0) / 100 * half;
        g.strokeStyle = typeCol(p.type); g.lineWidth = 1; g.setLineDash(SIM_DASH[typeIx(p.type)]); g.beginPath(); g.moveTo(X(p.a), ya); g.lineTo(X(p.b), yb); g.stroke(); g.setLineDash([]);
      }
      const grey = "#7a8190";
      peaks(P, 1, m => { const p = near(pairs, "a", m); return p ? typeCol(p.type) : grey; });
      peaks(C, -1, m => { const p = near(pairs, "b", m); if (p) return typeCol(p.type); const e = pairs.length ? null : ev.find(x => Math.abs(x.mz - m) < 0.003); return e ? (e.kind === "S" ? "#b42318" : "#2b5c8a") : grey; });
      g.fillStyle = ink; g.textAlign = "left"; g.fillText("parent", M.l + 4, M.t + 10); g.fillText("candidate", M.l + 4, H - M.b - 4);
    }

    async function exportAll(kind) {
      const name = (s.parent.formula || "parent") + "_TPMine_HR";
      const keys = Object.keys(s.weights), times = s.times;
      const head = ["#", "Level", "Diagnostic evidence", "Score", "m/z", "Formula", "ppm", "Possible formulas", "Change", "RT (min)", "t max (min)", "Class", "Appears at (min)", "Region", "Margin", "Precursor", "Transformation", "MS2 scans", "Modified cosine", "Matched peaks", "Prediction level", "Cosine: equal fragments", "Cosine: shifted fragments", "Cosine: common neutral losses", "Flags", "Filled areas (t, min)", ...keys.map(k => "score " + k), ...times.map(t => "Area t" + t)];
      const line = (r, i) => [i + 1, r.level, r.diagnostic ? "yes" : "", r.score, r.mz, r.formula ?? "", r.ppm ?? "", r.n_formulas ?? "", r.derivation ?? r.delta ?? "", r.rt, r.tmax ?? "", r.class ?? "", r.onset ?? "", r.region ?? "", r.margin ?? "", r.predecessor ?? "", r.transformation ?? "", r.ms2_scans, r.modcos ?? "", r.n_matched, r.pred ?? "", ...(r.sim ? r.sim.cos : ["", "", ""]), r.flags.join("; "), r.filled.map(k => times[k]).join("; "), ...keys.map(k => r.components[k]), ...r.series];
      if (kind === "json") return dl(name + ".json", JSON.stringify(s, null, 1), "application/json");
      if (kind === "csv") return dl(name + ".csv", "sep=;\n" + [head, ...rows.map(line)].map(x => x.map(v => `"${String(v ?? "").replace(/"/g, '""')}"`).join(";")).join("\n"), "text/csv");
      toast("Preparing the Excel file...");
      const top = rows.slice(0, 50), det = []; for (const r of top) det.push(await detail(r.id));
      const sheets = [{ name: "Ranking", head, rows: rows.map(line), widths: [6, 8, 9, 10, 18, 7, 9, 28, 8, 9, 18, 9, 30, 8, 12, 24, 9, 9, 9, 9, 9, 9, 9, 40, 16, ...keys.map(() => 10), ...times.map(() => 11)] },
        { name: "Criteria", head: ["#", "Formula", "Criterion", "Outcome", "Detail"], rows: det.flatMap((d, i) => d.criteria.map(c => [i + 1, d.row.formula ?? "", c.name, c.status, c.text])), widths: [6, 18, 32, 8, 80] },
        { name: "Localisation", head: ["#", "Formula", "Region", "Fragment m/z", "% rel.", "Fragment formula", "Evidence"], rows: det.flatMap((d, i) => ((d.localization && d.localization.evidence) || []).map(e => [i + 1, d.row.formula ?? "", d.row.region ?? "", e.mz, e.rel, e.formula, e.kind])), widths: [6, 18, 30, 12, 8, 18, 8] },
        { name: "Mass differences", head: ["From", "To", "Rule", "Δm/z", "Rule scope", "Kinetics", "Shifted fragments", "Share of the cosine", "Kind"], rows: ((s.mass_network || {}).edges || []).map(e => [e.from, e.to, e.rule, e.dm, e.scope ?? "", e.kinetic, e.shifted ? e.shifted.n : "", e.shifted ? e.shifted.share : "", e.kind]), widths: [12, 12, 36, 10, 12, 12, 9, 9, 10] },
        { name: "MSn tree", head: ["Node", "Level", "Parent", "Formula", "Precursor m/z", "ppm", "Scans", "Ghost"], rows: (s.tree || []).map(t => [t.id, t.level, t.parent ?? "", t.formula ?? "", t.prec_mz ?? "", t.ppm ?? "", t.n_scans, t.ghost ? "yes" : ""]), widths: [8, 8, 8, 18, 12, 8, 10, 9] },
        { name: "Inclusion list", head: ["m/z", "RT from", "RT to", "Formula", "Score"], rows: s.inclusion.map(x => [x.mz, x.rt_from, x.rt_to, x.formula ?? "", x.score]), widths: [12, 9, 9, 18, 10] },
        { name: "Information", head: ["Item", "Value"], rows: [["Parent", s.parent.formula], ["Ion", s.parent.ion], ["m/z", s.parent.mz], ["Score weights", keys.map(k => k + " " + s.weights[k]).join(", ")], ...s.funnel.map(f => ["Filter: " + f.text, f.n]), ...s.warnings.map(x => ["Warning", x])], widths: [44, 60] }];
      dlx(name + ".xlsx", sheets);
    }
    $("#hr-x-xlsx").onclick = () => exportAll("xlsx").catch(e => toast(String(e.message || e)));
    $("#hr-x-csv").onclick = () => exportAll("csv"); $("#hr-x-json").onclick = () => exportAll("json");
    $("#hr-x-incl").onclick = async () => dl((s.parent.formula || "parent") + "_inclusion.csv", await h.callRaw("inclusion_csv", "50"), "text/csv");
    massNet();
    if (rows.length) show(rows[0].id);
  }

  window.TPHR = { render };
})();
