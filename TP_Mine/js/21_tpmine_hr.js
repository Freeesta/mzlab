// TPMINE-PRIVATE  TP Mine, high-resolution mode: ranking of the possible TPs, detail of a candidate (kinetics by session, mirror MS2 with S/U evidence,
// localisation, criteria), MSn tree, exports. Called by 20_tpmine.js (window.TPHR.render) with its helpers; it does not talk to the worker itself.
(() => {
  "use strict";
  const CSS = `
.tp .lvl{display:inline-block;min-width:22px;text-align:center;border-radius:4px;color:#fff;font-weight:700;font-size:11px;padding:0 5px}
.tp .lvl-2b{background:#047857}.tp .lvl-3{background:#2b5c8a}.tp .lvl-4{background:#8a6d2b}.tp .lvl-5{background:#7a8190}
.tp .hrn{border-left:3px solid #d99a1d;background:rgba(217,154,29,.10);padding:6px 10px;border-radius:0 6px 6px 0;font-size:12px;margin:0 0 8px}
.tp .comp{display:grid;grid-template-columns:90px 1fr 42px;gap:6px;align-items:center;font-size:12px}.tp .comp i{display:block;height:8px;background:var(--accent);border-radius:4px}
.tp .flag{color:#9a5b00;font-size:11px}
.tp .fl-s{color:#b42318;font-weight:600}.tp .fl-u{color:#2b5c8a;font-weight:600}`;
  const LV = { "2b": "2b", 3: "3", 4: "4", 5: "5" };
  const LV_TEXT = { "2b": "site determined", 3: "region localised", 4: "formula and derivation", 5: "appears with the treatment" };

  function render(R, s, h) {
    if (!document.getElementById("tp-hr-css")) { const st = document.createElement("style"); st.id = "tp-hr-css"; st.textContent = CSS; document.head.appendChild(st); }
    const { esc, fmtA, chart, hue, call, toast, dl } = h;
    const $ = q => R.querySelector(q);
    const rows = s.rows, st = { sel: null, minLevel: 5, text: "", onlyLoc: false, det: {}, order: "score" };
    const lvRank = l => l === "2b" ? 4 : +l;
    const n = l => rows.filter(r => String(r.level) === l).length;
    const nMs2 = rows.filter(r => r.ms2_scans > 0).length;
    R.innerHTML = `<div class="hero"><div class="hero-t"><span class="hero-i">⛏</span>
      <div><h2>Ranking of possible TPs</h2><div class="hero-s">${esc(s.parent.name || "")} ${esc(s.parent.formula)} · ${esc(s.parent.ion)} <i>m/z</i> ${s.parent.mz}${s.parent.rt ? ` · RT ${s.parent.rt} min` : ""} · high resolution · ${s.timing.total} s</div></div></div>
      <div class="stats"><div class="stat g"><b>${n("2b")}</b><span>level 2b</span></div><div class="stat a"><b>${n("3")}</b><span>level 3</span></div><div class="stat v"><b>${n("4") + n("5")}</b><span>level 4-5</span></div>
      <div class="stat"><b>${nMs2}</b><span>with MS<sup>2</sup></span></div><div class="stat"><b>${s.n_candidates}</b><span>candidates after filters</span></div><div class="stat"><b>${s.files.length}</b><span>files</span></div></div>
      <div class="row" style="margin-top:10px"><button id="hr-x-xlsx">Excel (.xlsx)</button><button id="hr-x-csv">CSV</button><button id="hr-x-json">JSON</button><button id="hr-x-incl" title="Inclusion list for a second targeted injection: the best candidates without MS2">Inclusion list (CSV)</button></div></div>
      <div class="card hrn">Each row is a hypothesis, not an identification: the level says how much evidence supports it (5 appears with the treatment, 4 unique and derivable formula, 3 MS² linked to the parent and region localised, 2b site determined with margin, consistent RT and kinetics). Levels 2a and 1 require a standard.</div>
      ${s.warnings.length ? `<div class="card">${s.warnings.map(w => `<div class="flag">⚠ ${esc(w)}</div>`).join("")}</div>` : ""}
      <div class="card"><div class="row"><label class="mut">Minimum level</label><select id="hr-lv"><option value="5">all</option><option value="4">4 or higher</option><option value="3">3 or higher</option><option value="2b">2b only</option></select>
        <input type="text" id="hr-q" placeholder="filter: formula, m/z, region" class="grow"><label class="mut">Sort by</label><select id="hr-ord"><option value="score">score</option><option value="level">level, then score</option></select><label><input type="checkbox" id="hr-loc"> only with region</label><span class="mut" id="hr-cnt"></span></div>
        <div class="tbl"><table id="hr-t"></table></div></div>
      <div id="tp-det"></div>
      <details class="card"><summary>Applied filters (funnel) and sessions</summary>${s.funnel.map(f => `<div class="mut">${f.n.toLocaleString("en")} · ${esc(f.text)}</div>`).join("")}
        <div class="mut" style="margin-top:6px">Sessions: ${esc([...new Set(s.sessions.labels)].map(l => l + " (" + s.sessions.labels.filter(x => x === l).length + " files)").join(", "))}${s.sessions.factors.applied ? "; areas rescaled to the reference session" : "; no correction needed"}.</div>
        <div class="mut">Timings (s): ${esc(Object.entries(s.timing).map(([k, v]) => k + " " + v).join(" · "))}</div></details>
      <div class="card" id="hr-tree"></div>`;

    function spark(series) {
      const mx = Math.max(...series) || 1, w = 70, ht = 18, pts = series.map((v, i) => `${(i / Math.max(1, series.length - 1) * (w - 4) + 2).toFixed(1)},${(ht - 2 - v / mx * (ht - 4)).toFixed(1)}`).join(" ");
      return `<svg width="${w}" height="${ht}"><polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;
    }
    const ordered = () => st.order === "level" ? [...rows].sort((x, y) => lvRank(y.level) - lvRank(x.level) || y.score - x.score) : rows;
    const keep = () => ordered().filter(r => (st.minLevel === 5 || lvRank(r.level) >= lvRank(st.minLevel)) && (!st.onlyLoc || r.region) &&
      (!st.text || [r.formula, String(r.mz), r.region, r.delta, r.derivation].join(" ").toLowerCase().includes(st.text)));
    function table() {
      const o = ordered(), k = keep();
      $("#hr-cnt").textContent = `${k.length} of ${rows.length}`;
      $("#hr-t").innerHTML = `<tr><th>#</th><th>Lvl</th><th>Score</th><th><i>m/z</i></th><th>Formula (ppm)</th><th>Change</th><th>RT</th><th>t max</th><th>Class</th><th>Region</th><th>Kinetics</th></tr>` +
        k.map((r, i) => `<tr class="clk${r.id === st.sel ? " sel" : ""}" data-id="${r.id}"><td>${o.indexOf(r) + 1}</td><td><span class="lvl lvl-${r.level}" title="${esc(LV_TEXT[r.level] || "")}">${r.level}</span></td>
          <td class="n">${r.score}</td><td class="n">${r.mz.toFixed(4)}</td><td>${esc(r.formula || "-")}${r.ppm != null ? ` <span class="mut">${r.ppm >= 0 ? "+" : ""}${r.ppm}</span>` : ""}${r.n_formulas > 1 ? ` <span class="mut" title="possible formulas within 3 ppm">(${r.n_formulas})</span>` : ""}</td>
          <td>${esc(r.derivation || r.delta || "")}${r.flags.length ? `<div class="flag">${r.flags.map(esc).join("; ")}</div>` : ""}</td><td class="n">${r.rt}</td><td class="n">${r.tmax ?? ""}</td><td>${esc(r.class || "")}</td>
          <td>${esc(r.region || "")}</td><td>${spark(r.series)}</td></tr>`).join("");
      $("#hr-t").querySelectorAll("tr.clk").forEach(tr => tr.onclick = () => show(+tr.dataset.id));
    }
    $("#hr-lv").onchange = e => { st.minLevel = e.target.value === "2b" ? "2b" : +e.target.value; table(); };
    $("#hr-q").oninput = e => { st.text = e.target.value.trim().toLowerCase(); table(); };
    $("#hr-ord").onchange = e => { st.order = e.target.value; table(); };
    $("#hr-loc").onchange = e => { st.onlyLoc = e.target.checked; table(); };
    if (window.TPALBERO) window.TPALBERO.mount($("#hr-tree"), h, { compareId: () => st.sel });
    table();

    async function detail(id) { return st.det[id] || (st.det[id] = await call("detail", id)); }
    async function show(id) {
      st.sel = id; table();
      const D = $("#tp-det"); D.innerHTML = `<div class="card mut">Computing...</div>`;
      let d; try { d = await detail(id); } catch (e) { D.innerHTML = `<div class="card err">${esc(e.message || e)}</div>`; return; }
      const r = d.row, loc = d.localization, w = s.weights;
      const comp = Object.keys(w).filter(k => typeof d.components[k] === "number").map(k => [k, d.components[k]]).map(([k, v]) => `<div class="comp" title="${esc((d.components.text || {})[k] || "")}"><span>${esc(k)} <span class="mut">×${w[k]}</span></span><span><i style="width:${Math.round(100 * v)}%"></i></span><span class="mut">${v.toFixed(2)}</span></div>`).join("");
      const kin = d.kinetics || {};
      D.innerHTML = `<div class="card"><h3>${esc(r.formula || "formula not assigned")} <span class="lvl lvl-${r.level}">${r.level}</span> <span class="mut">${esc(d.level_text || "")}</span></h3>
        <div class="kv"><b><i>m/z</i></b><span>${r.mz}${r.ppm != null ? ` · ${r.ppm >= 0 ? "+" : ""}${r.ppm} ppm` : ""}${r.n_formulas > 1 ? ` · ${r.n_formulas} possible formulas` : ""}</span><b>Change</b><span>${esc(r.derivation || "-")} <span class="mut">(${esc(r.delta || "")})</span></span>
        <b>RT</b><span>${r.rt} min${s.parent.rt ? ` (${(r.rt - s.parent.rt) >= 0 ? "+" : ""}${(r.rt - s.parent.rt).toFixed(2)} relative to the parent)` : ""}</span>
        <b>Kinetics</b><span>${esc(kin.class_text || "-")}${kin.tmax != null ? ` · maximum at ${kin.tmax} min` : ""}${kin.onset != null ? ` · appears at ${kin.onset} min` : ""}${kin.unimodal === false ? " · not unimodal" : ""}</span>
        <b>Region</b><span>${r.region ? esc(r.region) : "<span class='mut'>not localised</span>"}${loc && loc.ok ? ` <span class="mut">· margin ${loc.margin.toFixed(2)} · ${Math.round(100 * loc.fraction_explained)}% of intensity explained</span>` : (loc && loc.note ? ` <span class="mut">${esc(loc.note)}</span>` : "")}</span>
        <b>Origin</b><span>${d.predecessor && d.predecessor.predecessor != null ? `from ${d.predecessor.predecessor === "progenitore" ? "parent" : "#" + esc(d.predecessor.predecessor)}${d.predecessor.transformation ? ", " + esc(d.predecessor.transformation) : ""}${d.predecessor.cos != null ? `, cosine ${(+d.predecessor.cos).toFixed(2)}` : ""}` : "<span class='mut'>no precursor found</span>"}</span>
        ${d.isotopes ? `<b>Isotopes</b><span>${esc(d.isotopes.text || d.isotopes.status)}</span>` : ""}${r.flags.length ? `<b>Flags</b><span class="flag">${r.flags.map(esc).join("; ")}</span>` : ""}</div></div>
        <div class="cols"><div class="card"><h3>Kinetics by session</h3><canvas id="hr-kin"></canvas><div class="row" id="hr-kleg"></div></div><div class="card"><h3>Score ${r.score}</h3>${comp}
          <h3 style="margin-top:10px">Criteria</h3><ul class="crit" style="margin:0;padding-left:18px">${d.criteria.map(c => `<li class="${c.status}"><b>${c.status === "pass" ? "✓" : c.status === "fail" ? "✗" : "–"} ${esc(c.name)}</b> <span class="mut">${esc(c.text)}</span></li>`).join("")}</ul></div></div>
        ${d.ms2 ? `<div class="card"><h3>MS<sup>2</sup> of the candidate (below) and of the parent (above)</h3><canvas id="hr-ms2" style="height:300px"></canvas>
          <div class="mut"><span class="fl-s">■ shifted (S)</span>: contains the modification · <span class="fl-u">■ unshifted (U)</span>: the modified part is outside the fragment · grey: no relation to the parent</div></div>` :
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
        if (ix.length) series.push({ x: ix.map(i => t[i]), y: ix.map(i => d.areas[i]), color: hue(j, Math.max(2, labs.length)), dots: true, label: lab });
      });
      chart($("#hr-kin"), series, { xlabel: "treatment time (min)" });
      $("#hr-kleg").innerHTML = series.map(x => `<span class="mut"><span style="display:inline-block;width:14px;border-top:2px solid ${x.color};vertical-align:middle"></span> ${x.label === "?" ? "area" : "session " + esc(x.label)}</span>`).join("");
      if (d.ms2) mirror($("#hr-ms2"), d);
      const cb = $("#hr-cmp-tree");
      if (cb) cb.onclick = () => { window.TPALBERO && window.TPALBERO.compare(id); const t = $("#hr-tree"); if (t) t.scrollIntoView({ behavior: "smooth", block: "nearest" }); };
    }

    function mirror(cv, d) {
      const dpr = devicePixelRatio || 1, W = cv.clientWidth || 600, H = cv.clientHeight || 300;
      cv.width = W * dpr; cv.height = H * dpr;
      const g = cv.getContext("2d"); g.scale(dpr, dpr); g.clearRect(0, 0, W, H);
      const P = d.ms2.parent, C = d.ms2.candidate, ev = (d.localization && d.localization.evidence) || [];
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
      peaks(P, 1, () => "#7a8190");
      peaks(C, -1, m => { const e = ev.find(x => Math.abs(x.mz - m) < 0.003); return e ? (e.kind === "S" ? "#b42318" : "#2b5c8a") : "#7a8190"; });
      g.fillStyle = ink; g.textAlign = "left"; g.fillText("parent", M.l + 4, M.t + 10); g.fillText("candidate", M.l + 4, H - M.b - 4);
    }

    async function exportAll(kind) {
      const name = (s.parent.formula || "parent") + "_TPMine_HR";
      const keys = Object.keys(s.weights), times = s.times;
      const head = ["#", "Level", "Score", "m/z", "Formula", "ppm", "Possible formulas", "Change", "RT (min)", "t max (min)", "Class", "Appears at (min)", "Region", "Margin", "Precursor", "Transformation", "MS2 scans", "Modified cosine", "Matched peaks", "Flags", ...keys.map(k => "score " + k), ...times.map(t => "Area t" + t)];
      const line = (r, i) => [i + 1, r.level, r.score, r.mz, r.formula ?? "", r.ppm ?? "", r.n_formulas ?? "", r.derivation ?? r.delta ?? "", r.rt, r.tmax ?? "", r.class ?? "", r.onset ?? "", r.region ?? "", r.margin ?? "", r.predecessor ?? "", r.transformation ?? "", r.ms2_scans, r.modcos ?? "", r.n_matched, r.flags.join("; "), ...keys.map(k => r.components[k]), ...r.series];
      if (kind === "json") return dl(name + ".json", JSON.stringify(s, null, 1), "application/json");
      if (kind === "csv") return dl(name + ".csv", "sep=;\n" + [head, ...rows.map(line)].map(x => x.map(v => `"${String(v ?? "").replace(/"/g, '""')}"`).join(";")).join("\n"), "text/csv");
      toast("Preparing the Excel file...");
      const top = rows.slice(0, 50), det = []; for (const r of top) det.push(await detail(r.id));
      const sheets = [{ name: "Ranking", head, rows: rows.map(line), widths: [6, 8, 9, 10, 18, 7, 9, 28, 8, 9, 18, 9, 30, 8, 12, 24, 9, 9, 9, 40, ...keys.map(() => 10), ...times.map(() => 11)] },
        { name: "Criteria", head: ["#", "Formula", "Criterion", "Outcome", "Detail"], rows: det.flatMap((d, i) => d.criteria.map(c => [i + 1, d.row.formula ?? "", c.name, c.status, c.text])), widths: [6, 18, 32, 8, 80] },
        { name: "Localisation", head: ["#", "Formula", "Region", "Fragment m/z", "% rel.", "Fragment formula", "Evidence"], rows: det.flatMap((d, i) => ((d.localization && d.localization.evidence) || []).map(e => [i + 1, d.row.formula ?? "", d.row.region ?? "", e.mz, e.rel, e.formula, e.kind])), widths: [6, 18, 30, 12, 8, 18, 8] },
        { name: "MSn tree", head: ["Node", "Level", "Parent", "Formula", "Precursor m/z", "ppm", "Scans", "Ghost"], rows: (s.tree || []).map(t => [t.id, t.level, t.parent ?? "", t.formula ?? "", t.prec_mz ?? "", t.ppm ?? "", t.n_scans, t.ghost ? "yes" : ""]), widths: [8, 8, 8, 18, 12, 8, 10, 9] },
        { name: "Inclusion list", head: ["m/z", "RT from", "RT to", "Formula", "Score"], rows: s.inclusion.map(x => [x.mz, x.rt_from, x.rt_to, x.formula ?? "", x.score]), widths: [12, 9, 9, 18, 10] },
        { name: "Information", head: ["Item", "Value"], rows: [["Parent", s.parent.formula], ["Ion", s.parent.ion], ["m/z", s.parent.mz], ["Score weights", keys.map(k => k + " " + s.weights[k]).join(", ")], ...s.funnel.map(f => ["Filter: " + f.text, f.n]), ...s.warnings.map(x => ["Warning", x])], widths: [44, 60] }];
      dlx(name + ".xlsx", sheets);
    }
    $("#hr-x-xlsx").onclick = () => exportAll("xlsx").catch(e => toast(String(e.message || e)));
    $("#hr-x-csv").onclick = () => exportAll("csv"); $("#hr-x-json").onclick = () => exportAll("json");
    $("#hr-x-incl").onclick = async () => dl((s.parent.formula || "parent") + "_inclusion.csv", await h.callRaw("inclusion_csv", "50"), "text/csv");
    if (rows.length) show(rows[0].id);
  }

  window.TPHR = { render };
})();
