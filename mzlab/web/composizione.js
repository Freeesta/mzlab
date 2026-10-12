"use strict";
// «Formule compatibili»: elemental composition of an ion from its exact m/z (server: mzlab/chem/composition.py, /api/composition). High-resolution world only.
// The answer is a list of CANDIDATES with their errors and checks, never an identification. Classic script, loaded after hr.js.
const COMP = (() => {
  const KEY = "qqq.comp";                                           // saved presets of the elements in use (localStorage, this browser only)
  const P_CHNO = "C H N O S P", BUILTIN = { [I18N.t("comp.preset.organic")]: "C:0-60,H:0-120,N:0-10,O:0-12", [P_CHNO]: "C:0-60,H:0-120,N:0-10,O:0-12,S:0-3,P:0-3",
    [I18N.t("comp.preset.halogens")]: "C:0-60,H:0-120,N:0-10,O:0-12,S:0-3,P:0-3,F:0-6,Cl:0-3,Br:0-2" };
  const IONS = [["[M+H]+", "[M+H]⁺"], ["[M+NH4]+", "[M+NH₄]⁺"], ["[M+Na]+", "[M+Na]⁺"], ["[M+K]+", "[M+K]⁺"], ["[M]+", I18N.t("comp.ion.plus")], ["M+.", I18N.t("comp.ion.radical")],
    ["[M-H]-", "[M−H]⁻"], ["[M+Cl]-", "[M+Cl]⁻"], ["[M+HCOO]-", "[M+HCOO]⁻"], ["[M]-", I18N.t("comp.ion.minus")]];
  const RULES = [["HC", I18N.t("comp.rule.HC")], ["NC", I18N.t("comp.rule.NC")], ["OC", I18N.t("comp.rule.OC")], ["PC", I18N.t("comp.rule.PC")], ["SC", I18N.t("comp.rule.SC")], ["FC", I18N.t("comp.rule.FC")], ["ClC", I18N.t("comp.rule.ClC")], ["BrC", I18N.t("comp.rule.BrC")], ["LEWIS", "Lewis"], ["SENIOR", "Senior"]];
  const rd = () => { return lsObj(KEY); };
  const wr = o => { try { localStorage.setItem(KEY, JSON.stringify(o)); } catch (e) { /* not available */ } };
  const num = v => { const x = numIt(v); return Number.isFinite(x) ? x : null; };
  // M+1/M and M+2/M seen in a spectrum d = {mz, y} around the peak at m/z `mz` (null when the peaks are not there)
  function observed(mz, d, tolDa = 0.006) {
    if (!d || !d.mz) return { m1: null, m2: null };
    const at = t => { let best = 0; d.mz.forEach((m, i) => { if (Math.abs(m - t) <= tolDa && d.y[i] > best) best = d.y[i]; }); return best; };
    const i0 = at(mz); if (!i0) return { m1: null, m2: null };
    const a = at(mz + 1.0033548), b = at(mz + 2.0067096);
    return { m1: a ? a / i0 : null, m2: b ? b / i0 : null };
  }
  // isotope peaks around the peak at `mz` (from -0.5 to +6.5 m/z) as "mz:intensity;..." for the isotope score; "" when there is no spectrum
  function obsText(mz, d) {
    if (!d || !d.mz || mz == null) return "";
    const out = []; d.mz.forEach((m, i) => { if (m >= mz - 0.5 && m <= mz + 6.5 && d.y[i] > 0) out.push(`${m.toFixed(5)}:${d.y[i].toPrecision(6)}`); });
    return out.join(";");
  }
  // the MS2 spectrum on screen (active panel first) as "mz intensity" lines; "" when there is none
  function ms2Text() {
    const ps = (E.panels || []).filter(p => p.type === "spec" && p._a && p._a.data && p._a.data[0] && p._a.data[0].d && p._a.data[0].f && p._a.data[0].f.kind === "ms2");
    const p = ps.find(x => x.id === E.active) || ps[0]; if (!p) return "";
    const d = p._a.data[0].d; return d.mz.map((m, i) => `${m.toFixed(5)} ${d.y[i].toPrecision(6)}`).join("\n");
  }
  const halo = f => { const m = {}; String(f).replace(/(Cl|Br)(\d*)/g, (_, e, n) => { m[e] = +n || 1; }); return Object.entries(m).map(([e, n]) => e + (n > 1 ? "×" + n : "")).join(" "); };
  const f2 = v => (v == null ? "–" : (+v).toFixed(2));
  // ---------------------------------------------------------------- helpers shared with the compact panel (elementare.js)
  const qs = q => Object.entries(q).filter(([, v]) => v != null && v !== "").map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join("&");
  const median = a => { const s = a.slice().sort((x, y) => x - y), n = s.length; return n ? (n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2) : 0; };
  // centroid algorithm: "file" keeps the points as they are; "max" replaces every local maximum of a profile by the vertex of the parabola through its three points
  function centroidMax(d) {
    const mz = [], y = [];
    for (let i = 0; i < d.mz.length; i++) {
      const a = d.y[i - 1], b = d.y[i], c = d.y[i + 1];
      if (i > 0 && i < d.mz.length - 1 && b > a && b >= c) {
        const den = a - 2 * b + c, sh = den ? 0.5 * (a - c) / den : 0, step = sh >= 0 ? d.mz[i + 1] - d.mz[i] : d.mz[i] - d.mz[i - 1];
        mz.push(d.mz[i] + sh * step); y.push(b - 0.25 * (a - c) * sh);
      } else if ((i === 0 || i === d.mz.length - 1) && b > 0 && (i === 0 ? b >= c : b > a)) { mz.push(d.mz[i]); y.push(b); }
    }
    return { mz, y };
  }
  // S/N threshold: the noise is the median intensity of the spectrum; peaks below sn × noise are not given to the score
  const snKeep = (d, sn) => { if (!(sn > 0) || !d || !d.y || !d.y.length) return d; const lim = sn * median(d.y.filter(v => v > 0)), mz = [], y = []; d.mz.forEach((m, i) => { if (d.y[i] >= lim) { mz.push(m); y.push(d.y[i]); } }); return { mz, y }; };
  const prepSpec = (d, s) => { if (!d || !d.mz) return d; let x = d; if (s && s.cent === "max") x = centroidMax(x); return snKeep(x, s && s.sn); };
  // the parameters of the isotope score and of the fragment constraint, from the saved settings `s`; q gets obs/frags only when there is something to compare
  function scoreParams(q, d, ms2, s) {
    s = s || {};
    const sint = (s.sint ?? 30) / 100, ms = ms2 ? snKeep(peaksOf(ms2), s.sn) : null;
    Object.assign(q, { r200: s.r200 ?? 70000, sppm: s.sppm ?? 3, sint, isotol: sint, imin: s.ithr ?? 0.1, inoise: s.ithr ?? 0.1, fppm: s.ftol || "", norm: s.norm && s.norm !== "fit" ? s.norm : "",
      w: [s.w0 ?? 1, s.w1 ?? 2, s.w2 ?? 1].join(","), obs: d && q.mz != null ? obsText(q.mz, d) : "", frags: ms ? ms.mz.map((m, i) => `${m}:${ms.y[i]}`).join(";") : "" });
    if (!q.obs && !q.frags) delete q.w;
    return q;
  }
  const peaksOf = t => { const mz = [], y = []; String(t || "").split(/\n/).forEach(l => { const a = l.trim().split(/[\s;,:\t]+/).map(x => numIt(x)); if (a.length >= 2 && a[0] > 0 && a[1] > 0) { mz.push(a[0]); y.push(a[1]); } }); return { mz, y }; };
  // «Min spectral fit»: candidates whose combined score is below it are not listed (only when there is a score)
  const afterFit = (j, s) => { const mf = s && s.minfit; if (mf > 0) j.results = j.results.filter(r => !r.score || r.score.total * 100 >= mf); return j; };
  const DEF = { ithr: 0.1, sint: 30, minfit: 0, sn: 0, ftol: 5, norm: "fit", cent: "file", rep: false };
  function open(o = {}) {
    const st = { ...DEF, ...rd() }, pol = o.polarity === "negative" ? "-" : "+", presets = { ...BUILTIN, ...(st.presets || {}) };
    const ob = observed(o.mz, o.spec), f4 = v => (v == null ? "" : (+v).toFixed(4));
    const opt = (v, cur, l) => `<option value="${EH(v)}"${v === cur ? " selected" : ""}>${l}</option>`;
    const els0 = st.els || (window.ELEM ? ELEM.DEFAULT_ELS : presets[P_CHNO]);
    big(I18N.t("comp.title"), `<div class="muted sm" style="margin-bottom:6px">${I18N.t("comp.note")}</div>
      <h4>${I18N.t("cx.pred")}</h4>
      <div class="cmp-grid">
        <label>m/z <input id="cmp-mz" inputmode="decimal" value="${o.mz != null ? (+o.mz).toFixed(5) : ""}" style="width:110px"></label>
        <label>${I18N.t("comp.ion")} <select id="cmp-ion">${IONS.filter(i => pol === "+" ? !/-$/.test(i[0]) : /-$/.test(i[0])).map(i => opt(i[0], o.ion || st["ion" + pol] || (pol === "+" ? "[M+H]+" : "[M-H]-"), i[1])).join("")}</select></label>
        <label>${I18N.t("comp.tol")} <input id="cmp-tol" value="${st.tol ?? 5}" style="width:56px"> <select id="cmp-unit">${opt("ppm", st.unit || "ppm", "ppm")}${opt("mda", st.unit || "ppm", "mDa")}</select></label>
        <label>${I18N.t("comp.max")} <input id="cmp-max" value="${st.max ?? 10}" style="width:48px"></label>
        <label>${I18N.t("comp.nitrogen")} <select id="cmp-n">${opt("none", st.n || "none", I18N.t("comp.n.none"))}${opt("even", st.n || "none", I18N.t("comp.n.even"))}${opt("odd", st.n || "none", I18N.t("comp.n.odd"))}</select></label>
        <label>${I18N.t("comp.rdbFrom")} <input id="cmp-r0" value="${st.r0 ?? -1}" style="width:50px"> ${I18N.t("comp.rdbTo")} <input id="cmp-r1" value="${st.r1 ?? 100}" style="width:50px"></label>
        <label>${I18N.t("cx.cent")} <select id="cmp-cent">${opt("file", st.cent, I18N.t("cx.cent.file"))}${opt("max", st.cent, I18N.t("cx.cent.max"))}</select></label>
        <label>${I18N.t("cx.norm")} <select id="cmp-norm">${[["fit", "cx.norm.fit"], ["mono", "cx.norm.mono"], ["max", "cx.norm.max"], ["sum", "cx.norm.sum"]].map(([v, k]) => opt(v, st.norm, I18N.t(k))).join("")}</select></label>
        <label><input type="checkbox" id="cmp-rep"${st.rep ? " checked" : ""}> ${I18N.t("cx.rep")}</label>
      </div>
      <h4>${I18N.t("comp.elements")} <span class="muted sm" style="font-weight:400">${I18N.t("comp.elements.hint")}</span></h4>
      <div class="cmp-grid"><select id="cmp-pre">${Object.keys(presets).map(k => `<option>${EH(k)}</option>`).join("")}</select><button id="cmp-load" type="button">${I18N.t("comp.load")}</button><button id="cmp-save" type="button">${I18N.t("comp.saveAs")}</button></div>
      <div id="cmp-elt" style="margin-top:4px"></div>
      <details style="margin-top:4px"><summary>${I18N.t("cx.pt")}</summary><div id="cmp-ptb"></div></details>
      <textarea id="cmp-els" rows="2" aria-label="${I18N.t("comp.elements")}" style="width:100%;box-sizing:border-box;margin-top:4px">${EH(els0)}</textarea>
      <h4>${I18N.t("comp.filters")} <span class="muted sm" style="font-weight:400">${I18N.t("comp.filters.hint")}</span></h4>
      <div class="cmp-grid">${RULES.map(([k, l]) => `<label><input type="checkbox" data-r="${k}"${(st.rules || []).includes(k) ? " checked" : ""}> ${EH(l)}</label>`).join("")}</div>
      <div class="cmp-grid" style="margin-top:6px"><label>${I18N.t("comp.parent")} <input id="cmp-par" value="${EH(o.parent || "")}" placeholder="${I18N.t("comp.parent.ph")}" style="width:150px"></label>
        <label>${I18N.t("comp.isoObs")} M+1/M <input id="cmp-m1" value="${f4(ob.m1)}" style="width:62px"> M+2/M <input id="cmp-m2" value="${f4(ob.m2)}" style="width:62px"></label>
        <button id="cmp-go" type="button" class="go">${I18N.t("comp.go")}</button></div>
      <details id="cmp-sc" style="margin-top:6px"${o.spec || o.ms2 ? " open" : ""}><summary><b>${I18N.t("cx.pat")}</b> · <b>${I18N.t("cx.frg")}</b></summary>
        <div class="muted sm">${I18N.t("comp.sc.note")}</div>
        <h4>${I18N.t("cx.pat")}</h4>
        <div class="cmp-grid"><label>${I18N.t("cx.ithr")} <input id="cmp-ithr" value="${st.ithr}" style="width:52px"></label>
          <label>${I18N.t("cx.itol")} <input id="cmp-sint" value="${st.sint}" style="width:44px"></label>
          <label>${I18N.t("comp.sc.r200")} <input id="cmp-r200" value="${st.r200 ?? o.r200 ?? 70000}" style="width:70px"></label>
          <label>${I18N.t("comp.sc.sppm")} <input id="cmp-sppm" value="${st.sppm ?? 3}" style="width:44px"></label>
          <span>${I18N.t("comp.sc.weights")} <label>${I18N.t("comp.sc.wm")} <input id="cmp-w0" value="${st.w0 ?? 1}" style="width:36px"></label>
          <label>${I18N.t("comp.sc.wi")} <input id="cmp-w1" value="${st.w1 ?? 2}" style="width:36px"></label>
          <label>${I18N.t("comp.sc.wf")} <input id="cmp-w2" value="${st.w2 ?? 1}" style="width:36px"></label></span></div>
        <div class="muted sm" id="cmp-nobs">${o.spec ? I18N.t("comp.sc.obs", { n: obsText(o.mz, o.spec).split(";").filter(Boolean).length }) : I18N.t("comp.sc.noobs")}</div>
        <h4>${I18N.t("cx.frg")}</h4>
        <div class="cmp-grid"><label>${I18N.t("cx.minfit")} <input id="cmp-minfit" value="${st.minfit}" style="width:44px"></label>
          <label>${I18N.t("cx.sn")} <input id="cmp-sn" value="${st.sn}" style="width:44px"></label>
          <label>${I18N.t("cx.ftol")} <input id="cmp-ftol" value="${st.ftol}" style="width:44px"></label></div>
        <div style="margin-top:4px">${I18N.t("comp.sc.ms2")} <button id="cmp-ms2get" type="button">${I18N.t("comp.sc.ms2get")}</button>
          <textarea id="cmp-ms2" rows="3" aria-label="${I18N.t("comp.sc.ms2")}" placeholder="${I18N.t("comp.sc.ms2ph")}" style="width:100%;box-sizing:border-box;margin-top:2px">${EH(o.ms2 && o.ms2.mz ? o.ms2.mz.map((m, i) => `${m.toFixed(5)} ${o.ms2.y[i].toPrecision(6)}`).join("\n") : "")}</textarea></div></details>
      <div id="cmp-known" class="sm" style="margin-top:6px"></div><div id="cmp-out" style="margin-top:8px"></div>`, () => {
      const $ = s => Q(s);
      $("#cmp-ms2get").onclick = () => { const t = ms2Text(); if (t) $("#cmp-ms2").value = t; else toast(I18N.t("comp.sc.ms2none")); };
      $("#cmp-load").onclick = () => { $("#cmp-els").value = presets[$("#cmp-pre").value] || $("#cmp-els").value; $("#cmp-els").dispatchEvent(new Event("input")); };
      $("#cmp-save").onclick = async () => { const n = await ask(I18N.t("comp.presetName"), ""); if (!n) return; const s = rd(); s.presets = { ...(s.presets || {}), [n]: $("#cmp-els").value }; wr(s); toast(I18N.t("comp.saved")); };
      if (window.ELEM) { ELEM.bindEls($("#cmp-elt"), $("#cmp-els")); ELEM.bindPeriodic($("#cmp-ptb"), $("#cmp-els")); }
      $("#cmp-rep").onchange = () => { if ($("#cmp-rep").checked) { $("#cmp-els").value = presets[P_CHNO]; $("#cmp-els").dispatchEvent(new Event("input")); } };      // «representative elements»: the usual set of organic molecules
      const go = async () => {
        const rules = [...document.querySelectorAll("#bigbody [data-r]")].filter(x => x.checked).map(x => x.dataset.r);
        const q = { mz: num($("#cmp-mz").value), ion: $("#cmp-ion").value, tol: num($("#cmp-tol").value) ?? 5, unit: $("#cmp-unit").value, max: parseInt($("#cmp-max").value) || 10,
          n: $("#cmp-n").value, rdb: `${num($("#cmp-r0").value) ?? -1},${num($("#cmp-r1").value) ?? 100}`, elements: $("#cmp-els").value.replace(/\s+/g, ""), rules: rules.join(","), parent: $("#cmp-par").value.trim(), m1: num($("#cmp-m1").value), m2: num($("#cmp-m2").value) };
        const sNow = { ...DEF, ...rd(), r200: num($("#cmp-r200").value) ?? 70000, sppm: num($("#cmp-sppm").value) ?? 3, sint: num($("#cmp-sint").value) ?? 30, ithr: num($("#cmp-ithr").value) ?? 0.1, minfit: num($("#cmp-minfit").value) ?? 0, sn: num($("#cmp-sn").value) ?? 0,
          ftol: num($("#cmp-ftol").value) ?? 5, norm: $("#cmp-norm").value, cent: $("#cmp-cent").value, rep: $("#cmp-rep").checked, w0: num($("#cmp-w0").value), w1: num($("#cmp-w1").value), w2: num($("#cmp-w2").value) };
        const sc = { r200: sNow.r200, sppm: sNow.sppm, sint: sNow.sint / 100 };
        scoreParams(q, o.spec ? prepSpec(o.spec, sNow) : null, $("#cmp-ms2").value, sNow);
        if (q.mz == null) { $("#cmp-out").innerHTML = `<span class="fail">${I18N.t("comp.typeMz")}</span>`; return; }
        if (window.LISTE) await LISTE.ready();
        { const kn = window.LISTE && o.polarity ? LISTE.textFor(q.mz, o.polarity, UIP.hrPpm) : ""; $("#cmp-known").innerHTML = kn ? `<b>${I18N.t("comp.known")}</b> ${EH(kn)} <span class="muted">${I18N.t("comp.known.note")}</span>` : ""; }
        wr({ ...sNow, tol: q.tol, unit: q.unit, max: q.max, n: q.n, r0: num($("#cmp-r0").value), r1: num($("#cmp-r1").value), els: $("#cmp-els").value, rules, ["ion" + (o.polarity === "negative" ? "-" : "+")]: q.ion });
        $("#cmp-out").textContent = I18N.t("comp.calculating");
        try {
          const j = afterFit(await J("api/composition?" + qs(q)), sNow);
          if (!j.results.length) { $("#cmp-out").innerHTML = `<div class="muted">${I18N.t("comp.none")}</div>`; return; }
          const hasIso = j.results.some(r => r.iso && r.iso.ok != null), hasSc = j.results.some(r => r.score), hasFr = j.results.some(r => r.frag);
          $("#cmp-out").innerHTML = `<div class="bar"><button id="cmp-xlsx" type="button">${typeof IC_DL !== "undefined" ? IC_DL : ""}Excel</button></div><div class="muted sm">${I18N.t("comp.count", { n: j.n_candidates })}${j.n_candidates > j.results.length ? " " + I18N.t("comp.first", { n: j.results.length }) : ""}. ${I18N.t("comp.confidence")}</div>
            <table class="lct"><tr><th>#</th><th>${I18N.t("comp.col.ion")}</th><th>${I18N.t("comp.col.neutral")}</th><th class="num">${I18N.t("comp.col.rdb")}</th><th class="num">Δ mDa</th><th class="num">Δ ppm</th><th>${I18N.t("comp.col.iso12")}</th>${hasIso ? "<th>" + I18N.t("comp.col.iso") + "</th>" : ""}<th>${I18N.t("comp.col.hal")}</th>${hasSc ? `<th class="num">${I18N.t("comp.col.score")}</th><th>${I18N.t("comp.col.terms")}</th>` : ""}${hasFr ? `<th class="num">${I18N.t("comp.col.frag")}</th>` : ""}<th>${I18N.t("comp.col.rules")}</th></tr>
            ${j.results.map((r, i) => `<tr data-i="${i}"${hasSc ? ' style="cursor:pointer" tabindex="0"' : ""}><td>${i + 1}</td><td><b>${fmtFormula(r.formula)}</b></td><td>${r.neutral ? fmtFormula(r.neutral) : ""}</td><td class="num">${r.rdb}</td><td class="num">${r.delta_mda}</td><td class="num">${r.delta_ppm}</td>
              <td class="num">${(r.iso.m1 * 100).toFixed(1)}% / ${(r.iso.m2 * 100).toFixed(1)}%</td>${hasIso ? `<td>${r.iso.ok == null ? "" : r.iso.ok ? I18N.t("comp.compatible") : I18N.t("comp.incompatible")}</td>` : ""}<td><b>${EH(halo(r.formula))}</b></td>${hasSc ? `<td class="num"><b>${f2(r.score && r.score.total)}</b></td><td class="sm">${r.score ? I18N.t("comp.terms", { m: f2(r.score.mass), i: f2(r.score.iso), f: f2(r.score.frag) }) : ""}</td>` : ""}${hasFr ? `<td class="num">${r.frag && r.frag.explained != null ? `${(r.frag.explained * 100).toFixed(0)} % (${r.frag.n_explained}/${r.frag.n_peaks})` : "–"}</td>` : ""}<td>${r.rules_failed.length ? I18N.t("comp.fails") + " " + r.rules_failed.join(", ") : I18N.t("comp.allPass")}</td></tr>`).join("")}</table><div id="cmp-det" style="margin-top:8px"></div>`;
          if (hasSc) {
            const det = i => {
              const r = j.results[i], pk = r.iso.peaks || [];
              $("#cmp-out").querySelectorAll("tr[data-i]").forEach(t => { t.style.background = +t.dataset.i === i ? "var(--sel, rgba(120,160,255,.2))" : ""; });
              $("#cmp-det").innerHTML = `<div><b>${fmtFormula(r.formula)}</b> · ${I18N.t("comp.det.title")}</div><div class="muted sm">${I18N.t("comp.det.limits", { r: sc.r200, p: sc.sppm, s: Math.round(sc.sint * 100), mn: 1, w: q.w || "1,2,1" })}</div>
                <table class="lct"><tr><th class="num">m/z</th><th class="num">${I18N.t("comp.det.th")}</th><th class="num">${I18N.t("comp.det.obs")}</th><th class="num">Δ ppm</th><th class="num">${I18N.t("comp.det.tm")}</th><th class="num">${I18N.t("comp.det.ti")}</th><th class="num">${I18N.t("comp.det.w")}</th><th class="num">${I18N.t("comp.det.contrib")}</th></tr>
                ${pk.map(x => `<tr><td class="num">${x.mz.toFixed(4)}</td><td class="num">${x.rel.toFixed(1)}</td><td class="num">${x.obs_rel == null ? I18N.t("comp.det.missing") : x.obs_rel.toFixed(1)}</td><td class="num">${x.d_ppm == null ? "" : x.d_ppm.toFixed(1)}</td><td class="num">${f2(x.t_mass)}</td><td class="num">${f2(x.t_int)}</td><td class="num">${f2(x.w)}</td><td class="num">${f2(x.w * x.t)}</td></tr>`).join("")}
                ${(r.iso.extra || []).map(x => `<tr><td class="num">${x.mz.toFixed(4)}</td><td class="num">–</td><td class="num">${x.rel.toFixed(1)}</td><td></td><td colspan="3" class="muted sm">${I18N.t("comp.det.extra")}</td><td class="num">${f2(x.w)}</td></tr>`).join("")}</table>`;
            };
            $("#cmp-out").querySelectorAll("tr[data-i]").forEach(t => { t.onclick = () => det(+t.dataset.i); t.onkeydown = ev => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); det(+t.dataset.i); } }; });
            det(0);
          }
          $("#cmp-xlsx").onclick = () => dlx(I18N.t("comp.xlsx.prefix") + (+q.mz).toFixed(4) + ".xlsx", [{ name: I18N.t("comp.xlsx.sheet"), head: ["#", I18N.t("comp.col.ion"), I18N.t("comp.col.neutral"), I18N.t("comp.col.rdb"), "Δ mDa", "Δ ppm", "M+1/M (%)", "M+2/M (%)", I18N.t("comp.col.iso"), I18N.t("comp.col.score"), I18N.t("comp.sc.wm"), I18N.t("comp.sc.wi"), I18N.t("comp.sc.wf"), I18N.t("comp.col.frag"), I18N.t("comp.col.failed")],
            rows: j.results.map((r, i) => [i + 1, r.formula, r.neutral || "", r.rdb, r.delta_mda, r.delta_ppm, +(r.iso.m1 * 100).toFixed(2), +(r.iso.m2 * 100).toFixed(2), r.iso.ok == null ? "" : r.iso.ok ? I18N.t("comp.compatible") : I18N.t("comp.incompatible"),
              r.score ? r.score.total : "", r.score ? r.score.mass : "", r.score && r.score.iso != null ? r.score.iso : "", r.score && r.score.frag != null ? r.score.frag : "", r.frag && r.frag.explained != null ? `${(r.frag.explained * 100).toFixed(0)} % (${r.frag.n_explained}/${r.frag.n_peaks})` : "", r.rules_failed.join(", ")]), widths: [5, 24, 22, 8, 10, 10, 12, 12, 16, 10, 8, 8, 8, 14, 30] },
            { name: I18N.t("lib.all.sheetPar"), head: [I18N.t("lib.all.parameter"), I18N.t("lib.all.value")], rows: [["m/z", q.mz], [I18N.t("comp.ion"), q.ion], [I18N.t("comp.tol"), `${q.tol} ${q.unit}`], [I18N.t("comp.xlsx.elements"), q.elements], [I18N.t("comp.col.rdb"), q.rdb], [I18N.t("comp.nitrogen"), q.n], [I18N.t("comp.filters"), q.rules || ""], [I18N.t("comp.parent"), q.parent || ""], [I18N.t("lib.all.level"), I18N.t("comp.xlsx.level")]], widths: [28, 60] }]);
        } catch (e) { $("#cmp-out").innerHTML = `<span class="fail">${EH(e.message)}</span>`; }
      };
      $("#cmp-go").onclick = go;
      if (o.mz != null) go();
    });
  }
  // ---------------------------------------------------------------- «Albero MSn» (WP-H4): the fragmentation paths of an MSn file (server: mzlab/chem/msntree.py, /api/msntree)
  async function tree(k, formula) {
    const f = E.files[k];
    big(I18N.t("tree.title"), `<div class="muted sm" style="margin-bottom:6px">${I18N.t("tree.note")}</div>
      <div class="cmp-grid"><label>${I18N.t("tree.formula")} <input id="mt-f" value="${EH(formula || "")}" placeholder="${I18N.t("tree.formula.ph")}" style="width:200px"></label>
        <button id="mt-go" type="button" class="go">${I18N.t("comp.go")}</button></div><div id="mt-out" style="margin-top:8px"></div>`, () => {
      const go = async () => {
        const fm = Q("#mt-f").value.replace(/\s+/g, "");
        Q("#mt-out").textContent = I18N.t("comp.calculating");
        try {
          const j = await J(`api/msntree?k=${k}${fm ? "&formula=" + encodeURIComponent(fm) : ""}`);
          const nd = j.nodes, depth = n => (n.parent == null ? 0 : 1 + depth(nd[n.parent]));
          Q("#mt-out").innerHTML = `<div class="bar"><button id="mt-xlsx" type="button">${typeof IC_DL !== "undefined" ? IC_DL : ""}Excel</button></div>${j.formula_given ? "" : `<div class="muted sm">${I18N.t("tree.noFormula")}${j.root_candidates.length ? `: ${j.root_candidates.slice(0, 6).map(x => `<a href="#" data-rf="${EH(x)}">${fmtFormula(x)}</a>`).join(", ")}` : ""} ${I18N.t("tree.clickFix")}</div>`}
            <div style="display:flex;gap:10px;flex-wrap:wrap"><div style="flex:1 1 380px;max-height:360px;overflow:auto"><table class="lct" id="mt-tbl"><tr><th>${I18N.t("tree.col.path")}</th><th>MS</th><th class="num">${I18N.t("lib.all.scans")}</th><th class="num">${I18N.t("tree.col.prec")}</th><th>${I18N.t("lst.col.formula")}</th><th class="num">ppm</th><th></th></tr>
            ${nd.map(n => `<tr data-n="${n.id}" style="cursor:pointer"><td style="padding-left:${6 + 14 * depth(n)}px">${EH(n.path[n.path.length - 1].join("@").replace(/@([A-Za-z]+)@/, "@$1"))}</td><td>${n.level}</td><td class="num">${n.scans}</td>
              <td class="num">${n.prec == null ? "" : n.prec.toFixed(4)}</td><td>${n.formula ? fmtFormula(n.formula) : "–"}</td><td class="num">${n.ppm == null ? "" : n.ppm.toFixed(1)}</td><td class="muted sm">${n.empty ? I18N.t("tree.empty") : ""}</td></tr>`).join("")}</table></div>
            <div id="mt-pk" style="flex:1 1 300px;max-height:360px;overflow:auto"><span class="muted sm">${I18N.t("tree.clickNode")}</span></div></div>`;
          Q("#mt-xlsx").onclick = () => dlx(I18N.t("tree.xlsx.prefix") + (f ? String(f.label || "file").replace(/[^\w.-]+/g, "_") : "file") + ".xlsx", [
            { name: I18N.t("tree.xlsx.paths"), head: [I18N.t("tree.col.path"), "MS", I18N.t("lib.all.scans"), I18N.t("tree.xlsx.peaks"), I18N.t("tree.col.prec"), I18N.t("lst.col.formula"), "Δ ppm", I18N.t("lst.col.note")], rows: nd.map(n => [n.label, n.level, n.scans, n.n_peaks, n.prec == null ? "" : +n.prec.toFixed(5), n.formula || "", n.ppm == null ? "" : +n.ppm.toFixed(2), n.empty ? I18N.t("tree.empty") : ""]), widths: [40, 6, 10, 8, 14, 20, 8, 14] },
            { name: I18N.t("tree.xlsx.peaks"), head: [I18N.t("tree.col.path"), "m/z", I18N.t("tree.pct"), I18N.t("lst.col.formula"), "Δ ppm", I18N.t("lst.col.note")], rows: nd.flatMap(n => n.peaks.slice().sort((a, b) => b.rel - a.rel).map(p => [n.label, +p.mz.toFixed(5), p.rel, p.formula || "", p.ppm == null ? "" : +p.ppm.toFixed(2), p.contam ? I18N.t("tree.contam") : ""])), widths: [40, 12, 12, 20, 8, 16] }]);
          Q("#mt-out").querySelectorAll("[data-rf]").forEach(a => { a.onclick = ev => { ev.preventDefault(); Q("#mt-f").value = a.dataset.rf; go(); }; });
          Q("#mt-tbl").querySelectorAll("tr[data-n]").forEach(tr => {
            tr.onclick = () => {
              const n = nd[+tr.dataset.n];
              Q("#mt-tbl").querySelectorAll("tr").forEach(x => { x.style.background = ""; }); tr.style.background = "var(--sel, rgba(120,160,255,.2))";
              Q("#mt-pk").innerHTML = `<div><b>${EH(n.label)}</b></div><div class="muted sm">${I18N.t("tree.nodeInfo", { scans: n.scans, peaks: n.n_peaks })} ${I18N.t("tree.clickPeak")}</div>
                <span id="mt-cnt"></span><table class="lct"><tr><th class="num">m/z</th><th class="num">%</th><th>Formula</th><th class="num">ppm</th><th></th><th>${I18N.t("cont.col")}</th></tr>
                ${n.peaks.slice().sort((a, b) => b.rel - a.rel).map(p => { const c = window.LISTE ? LISTE.compat(p.mz, f && f.polarity, UIP.hrPpm) : null; return `<tr data-mz="${p.mz}"${c && c.hid ? ' data-hid="1"' : ""} style="cursor:pointer"><td class="num">${p.mz.toFixed(4)}</td><td class="num">${p.rel}</td><td>${p.formula ? fmtFormula(p.formula) : "–"}</td><td class="num">${p.ppm == null ? "" : p.ppm.toFixed(1)}</td><td class="muted sm">${p.contam ? I18N.t("tree.contam") : ""}</td><td class="sm">${c && c.vis.length ? `<span aria-hidden="true">◆</span> ${EH(c.vis[0].name)}${c.vis.length > 1 ? " +" + (c.vis.length - 1) : ""}` : ""}</td></tr>`; }).join("")}</table>`;
              { const hids = Q("#mt-pk").querySelectorAll("tr[data-hid]"), cn = Q("#mt-cnt"); let show = false;
                const sync = () => { hids.forEach(r => { r.hidden = !show; }); cn.innerHTML = hids.length ? `<div class="sm">${I18N.t(show ? "cont.cnt.shown" : "cont.cnt", { n: hids.length })} · <button type="button" id="mt-cshow">${I18N.t(show ? "cont.hide" : "cont.show")}</button></div>` : ""; const bt = Q("#mt-cshow"); if (bt) bt.onclick = () => { show = !show; sync(); }; };
                sync(); }
              Q("#mt-pk").querySelectorAll("tr[data-mz]").forEach(r => { r.onclick = () => open({ mz: +r.dataset.mz, parent: n.formula || "", polarity: f && f.polarity, ion: j.ion }); });
            };
          });
        } catch (e) { Q("#mt-out").innerHTML = `<span class="fail">${EH(e.message)}</span>`; }
      };
      Q("#mt-go").onclick = go;
      go();
    });
  }
  return { open, observed, tree, rd, wr, num, IONS, BUILTIN, qs, prepSpec, scoreParams, afterFit, obsText };
})();
window.COMP = COMP;
