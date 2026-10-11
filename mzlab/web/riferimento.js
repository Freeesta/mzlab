"use strict";
// «Difference from the reference» (high resolution only; «Statistica» panel): for every feature of the feature table (TABELLA) the numbers against a
// reference chosen by the user (default: the files at time <= 0, blanks and dark controls of the sample table). Highest treated / reference, log2 fold
// change with a visible pseudocount, absent in the reference, the shape in time (descriptive labels used ONLY to sort and filter), replicates, and the
// two lists (kept, discarded) with the reason. Never a verdict: the program sorts and filters, the chemist decides. The work is done by
// mzlab/statistica.py (reference_difference, identity_links; /api/stat/reference); this file draws the table and opens the evidence of a row.
const RIFERIMENTO = (() => {
  const KEY = "qqq.riferimento", MAX_ROWS = 500;
  const t = (k, p) => I18N.t(k, p);
  const eh = s => String(s == null ? "" : s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const SHAPES = ["rises", "rises_falls", "falls", "flat"];
  let st = { factor: 3, stat: "max", min: 1, pseudo: "auto", floor: 1000, sort: "log2fc", dir: -1, show: "kept", shape: "all", ref: null };
  let res = null;                      // {r (server answer), tb (table), names, rows: [{i, f, q, idn, lst}]}
  let busy = null;
  try { Object.assign(st, JSON.parse(localStorage.getItem(KEY) || "{}")); } catch (_) { /* none */ }
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };
  const fmt = (v, d = 4) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
  const num = v => (v == null || !isFinite(v) ? "–" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? v.toExponential(2) : v.toFixed(1));

  // ---------------------------------------------------------------- the numbers
  async function run() {
    const tb = TABELLA.table;
    if (!tb) throw new Error(t("blank.err.table"));
    if (!(st.factor >= 0) || !(st.min >= 1)) throw new Error(t("blank.err.param"));
    const fo = TABELLA.fileObjs(), names = fo.map(f => f.file);
    const pseudo = String(st.pseudo).trim().toLowerCase() === "auto" ? null : numIt(st.pseudo);
    if (pseudo != null && !(pseudo >= 0)) throw new Error(t("blank.err.param"));
    const chosen = Array.isArray(st.ref) ? st.ref.map(n => names.indexOf(n)).filter(i => i >= 0) : null;
    const body = { matrix: TABELLA.matrix(), roles: TABELLA.roles(), times: fo.map(f => (f.time == null ? null : +f.time)), groups: fo.map(f => CAMPIONI.repOf(f)),
      reference: chosen && chosen.length ? chosen : null, factor: st.factor, stat: st.stat, min_detect: st.min, pseudo, floor: +st.floor || 1000,
      mz: tb.features.map(f => f.mz), rt: tb.features.map(f => f.rt) };
    const j = await fetch("api/stat/reference", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) }).then(r => r.json());
    if (j.error) throw I18N.err(j);
    // the lists the user loaded (never the built-in ones): name and Δppm of the entries that match; null when there is none
    let lst = null;
    if (window.LISTE && LISTE.ready && LISTE.hasUser) {
      await LISTE.ready();
      if (LISTE.hasUser()) {
        const ppm = (window.UIP && UIP.hrPpm) || 5, pol = (fo[0] && fo[0].polarity) || "positive";
        lst = tb.features.map(f => LISTE.userMatches(f.mz, pol, ppm) || []);
      }
    }
    res = { r: j, tb, names, hasList: !!lst, rows: tb.features.map((f, i) => ({ i, f, q: j.rows[i], idn: (j.identity || [])[i] || [], lst: lst ? lst[i] : null })) };
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_reference", { reference: j.reference.map(i => names[i]), factor: st.factor, stat: st.stat, min: st.min, pseudo: j.params.pseudo });
    return res;
  }

  // ---------------------------------------------------------------- the table
  const keyOf = {
    id: x => x.f.id, mz: x => x.f.mz, rt: x => x.f.rt, ref: x => x.q.ref_level, max: x => x.q.treated_max, ratio: x => (x.q.ratio == null ? 1e300 : x.q.ratio),
    log2fc: x => x.q.log2fc, shape: x => (x.q.shape ? SHAPES.indexOf(x.q.shape) : 99), rho: x => x.q.rho, tmax: x => x.q.t_max, cv: x => x.q.rep_cv,
    state: x => (x.q.kept ? 0 : 1)
  };
  const COLS = [["id", "ref.col.id", true], ["mz", "ref.col.mz", true], ["rt", "ref.col.rt", true], ["ref", "ref.col.ref", true], ["max", "ref.col.max", true],
    ["ratio", "ref.col.ratio", true], ["log2fc", "ref.col.log2fc", true], ["absent", "ref.col.absent", false], ["shape", "ref.col.shape", true], ["rho", "ref.col.rho", true],
    ["tmax", "ref.col.tmax", true], ["rep", "ref.col.rep", false], ["cv", "ref.col.cv", true], ["state", "ref.col.state", true], ["reason", "ref.col.reason", false],
    ["ident", "ref.col.ident", false]];
  const identText = (res0, l) => (l.kind === "adduct" ? t("ref.ident.adduct", { as: l.as, other: res0.tb.features[l.other].id, oas: l.other_as }) : t("ref.ident.isf", { other: res0.tb.features[l.other].id, loss: l.loss }));
  const listText = lst => lst.map(m => `${m.name}, ${m.err >= 0 ? "+" : ""}${m.err.toFixed(1)} ppm`).join("; ");
  const cells = (x, forFile) => {
    const q = x.q, ex = (v, d) => (v == null ? "" : +v.toFixed(d));
    const c = { id: x.f.id, mz: forFile ? +x.f.mz.toFixed(5) : fmt(x.f.mz), rt: forFile ? +x.f.rt.toFixed(3) : fmt(x.f.rt, 2), ref: forFile ? q.ref_level : num(q.ref_level), max: forFile ? q.treated_max : num(q.treated_max),
      ratio: forFile ? ex(q.ratio, 3) : (q.ratio == null ? "–" : fmt(q.ratio, 1)), log2fc: forFile ? ex(q.log2fc, 3) : fmt(q.log2fc, 2),
      absent: t(q.absent_in_reference ? "ref.yes" : "ref.no"), shape: q.shape ? t(`ref.shape.${q.shape}`) : "–", rho: forFile ? ex(q.rho, 3) : fmt(q.rho, 2),
      tmax: forFile ? ex(q.t_max, 3) : fmt(q.t_max, 1), rep: `${num(q.rep_mean)} (${q.rep_n_detected}/${q.rep_n})`, cv: forFile ? ex(q.rep_cv, 1) : fmt(q.rep_cv, 0),
      state: t(q.kept ? "ref.kept" : "ref.discarded"), reason: t(`ref.reason.${q.reason}`), ident: x.idn.map(l => identText(res, l)).join("; ") };
    if (res.hasList) c.list = listText(x.lst);
    return c;
  };
  const visible = () => {
    let v = res.rows.filter(x => (st.show === "all" || (st.show === "kept") === x.q.kept) && (st.shape === "all" || (st.shape === "new" ? x.q.absent_in_reference && x.q.kept : x.q.shape === st.shape)));
    const k = keyOf[st.sort] || keyOf.log2fc;
    v = v.slice().sort((a, b) => { const p = k(a), q = k(b); if (p == null && q == null) return a.i - b.i; if (p == null) return 1; if (q == null) return -1; return (p < q ? -1 : p > q ? 1 : a.i - b.i) * (st.dir < 0 ? -1 : 1) || a.i - b.i; });
    return v;
  };
  function tableHtml() {
    const v = visible(), shown = v.slice(0, MAX_ROWS), cols = COLS.slice();
    if (res.hasList) cols.push(["list", "ref.col.list", false]);
    const th = cols.map(([id, key, sortable]) => sortable
      ? `<th scope="col" aria-sort="${st.sort === id ? (st.dir < 0 ? "descending" : "ascending") : "none"}"><button type="button" class="ref-sort" data-sort="${id}">${eh(t(key))}${st.sort === id ? (st.dir < 0 ? " ▼" : " ▲") : ""}</button></th>`
      : `<th scope="col">${eh(t(key))}</th>`).join("");
    const tr = shown.map(x => { const c = cells(x, false); return `<tr data-i="${x.i}" data-mz="${x.f.mz}" data-rt="${x.f.rt}" tabindex="0" role="button" aria-label="${eh(t("ref.row.aria", { id: x.f.id, mz: fmt(x.f.mz), rt: fmt(x.f.rt, 2) }))}">${cols.map(([id]) => `<td${["id", "absent", "shape", "rep", "state", "reason", "ident", "list"].includes(id) ? "" : ' class="num"'}>${eh(c[id])}</td>`).join("")}</tr>`; }).join("");
    return `<div class="sm" role="status" id="ref-count">${t("ref.count", { n: shown.length, tot: v.length, all: res.rows.length })}${v.length > shown.length ? " " + t("ref.cap", { n: MAX_ROWS }) : ""}</div>
<div class="qq-scroll"><table class="qq-tab" id="ref-tab" aria-label="${eh(t("ref.title"))}"><thead><tr>${th}</tr></thead><tbody>${tr}</tbody></table></div>`;
  }
  const refLine = () => {
    const r = res.r, p = r.params, n = i => res.names[i];
    return `<p class="sm" id="ref-used" role="status"><b>${t("ref.used")}</b> ${r.reference.map(i => eh(n(i))).join(", ")} <span class="muted">(${t(r.reference_rule === "explicit" ? "ref.rule.explicit" : "ref.rule.meta")})</span><br>
<b>${t("ref.treated")}</b> ${r.treated.map(i => eh(n(i))).join(", ")}<br>
<b>${t("ref.pseudo")}</b> ${num(p.pseudo)} <span class="muted">(${t(`ref.pseudo.src.${p.pseudo_source}`)})</span></p>`;
  };
  function paintRes(box) {
    const el = box.querySelector("#ref-res");
    if (!el) return;
    if (!res) { el.innerHTML = ""; return; }
    const r = res.r;
    el.innerHTML = `${refLine()}<div class="sm" role="status">${t("ref.sum", { k: r.kept.length, d: r.discarded.length, thr: r.params.factor, min: r.params.min_detect })}</div>
<div class="qq-form"><label>${t("ref.show")} <select id="ref-show"><option value="kept"${st.show === "kept" ? " selected" : ""}>${t("ref.show.kept")}</option><option value="discarded"${st.show === "discarded" ? " selected" : ""}>${t("ref.show.discarded")}</option><option value="all"${st.show === "all" ? " selected" : ""}>${t("ref.show.all")}</option></select></label>
<label>${t("ref.shapef")} <select id="ref-shape"><option value="all">${t("ref.shape.all")}</option><option value="new"${st.shape === "new" ? " selected" : ""}>${t("ref.shape.new")}</option>${SHAPES.map(s => `<option value="${s}"${st.shape === s ? " selected" : ""}>${t(`ref.shape.${s}`)}</option>`).join("")}</select></label></div>
<div id="ref-tbl">${tableHtml()}</div>
<p class="sm" id="ref-note">${t("ref.note")}</p>
<div class="qq-form"><button type="button" id="ref-tsv">TSV</button><button type="button" id="ref-xlsx">Excel</button></div><div id="ref-open" class="sm" role="status"></div>`;
  }
  function chooser() {
    const names = TABELLA.fileObjs().map(f => f.file), sel = new Set(res ? res.r.reference.map(i => res.names[i]) : []);
    return `<fieldset class="ref-fs"><legend class="sm">${t("ref.choose")}</legend>${names.map((n, i) => `<label class="sm ref-chk"><input type="checkbox" data-ref="${eh(n)}"${sel.has(n) ? " checked" : ""}> ${eh(n)}</label>`).join(" ")}
<button type="button" id="ref-default">${t("ref.default")}</button></fieldset>`;
  }
  function paint(box) {
    const tb = TABELLA.table;
    box.innerHTML = `<p class="sm">${t("ref.intro")}</p>
${tb ? `<div id="ref-chooser">${chooser()}</div>` : `<p class="sm">${t("blank.err.table")}</p>`}
<form id="ref-form" class="qq-form"><label>${t("ref.factor")} <input id="ref-f" value="${st.factor}" inputmode="decimal" size="4"></label>
<label>${t("blank.level")} <select id="ref-s"><option value="max"${st.stat === "max" ? " selected" : ""}>${t("blank.stat.max")}</option><option value="mean"${st.stat === "mean" ? " selected" : ""}>${t("blank.stat.mean")}</option></select></label>
<label>${t("blank.min")} <input id="ref-n" value="${st.min}" inputmode="numeric" size="3"></label>
<label>${t("ref.pseudo.in")} <input id="ref-p" value="${eh(st.pseudo)}" inputmode="decimal" size="7" aria-describedby="ref-pnote"></label><label>${t("ref.floor")} <input id="ref-fl" value="${st.floor}" inputmode="decimal" size="6"></label>
<button type="submit" id="ref-go">${t("blank.go")}</button></form><p class="sm muted" id="ref-pnote">${t("ref.pseudo.note")}</p>
<div id="ref-msg" class="sm bad" role="alert"></div><div id="ref-res"></div>`;
    paintRes(box);
  }
  const msgOf = (box, err) => { const m = box.querySelector("#ref-msg"); if (m) m.textContent = err ? String(err.message || err) : ""; };
  let seq = 0;
  async function refresh(box) {
    const my = ++seq;
    msgOf(box, null);
    let err = null;
    try { await run(); } catch (e) { res = null; err = e; }
    if (my !== seq) return;                                 // a newer run started meanwhile
    paint(box); if (err) msgOf(box, err);
  }

  // ---------------------------------------------------------------- the evidence of a row: XIC in the files, MS1 at the apex, MS2, profile in time
  async function inspect(x, box) {
    const note = box.querySelector("#ref-open"), mz = x.f.mz, rt = x.f.rt;
    const say = k => { if (note) note.textContent = k ? t(k) : ""; };
    try {
      const prof = window.PROFILO ? await PROFILO.show(mz, rt) : null;                // fills the profile, which orders the files by time
      if (typeof xicDirect !== "function") return;
      const np = xicDirect(mz, null, null);
      const name = res.names[x.q.max_file], f = E.files.find(q => q.file === name);
      const row = prof && prof.rows ? prof.rows.find(q => q.file === name) : null;
      const apex = row && row.rt != null ? row.rt : rt;
      if (!f || typeof newSpec !== "function" || !np) return;
      const near = await nearScan(f.k, apex);
      const s1 = newSpec(np, near ? near.rt - NEAR : apex - NEAR, near ? near.rt + NEAR : apex + NEAR, f.k);
      if (window.DDA && f.dda && s1) {                                                // a data-dependent file: the MS2 scans of this ion, if the instrument took any
        DDA.make(np, s1);
        setTimeout(() => { try { const q = DDA.ms2For(s1, mz)[0]; if (q) DDA.openMs2(s1, q.j); } catch (_) { /* no MS2 */ } }, 600);
      }
      say("ref.opened");
    } catch (err) { if (note) note.textContent = String(err && err.message || err); }
  }

  // ---------------------------------------------------------------- export (parameters and reference included: reproducibility)
  const head = () => ["row ID", "row m/z", "row retention time", t("ref.col.ref"), t("ref.col.max"), t("ref.col.ratio"), "log2FC", t("ref.col.absent"), t("ref.col.shape"), t("ref.col.rho"),
    t("ref.col.tmax"), t("ref.col.rep"), t("ref.col.cv"), t("ref.col.state"), t("ref.col.reason"), t("ref.col.ident")].concat(res.hasList ? [t("ref.col.list")] : []);
  const body = () => visible().map(x => { const c = cells(x, true); return ["id", "mz", "rt", "ref", "max", "ratio", "log2fc", "absent", "shape", "rho", "tmax", "rep", "cv", "state", "reason", "ident"].concat(res.hasList ? ["list"] : []).map(k => c[k]); });
  const params = () => {
    const r = res.r, p = r.params, n = i => res.names[i];
    return [[t("ref.used"), r.reference.map(n).join("; ")], [t("ref.treated"), r.treated.map(n).join("; ")], [t("ref.rule"), t(r.reference_rule === "explicit" ? "ref.rule.explicit" : "ref.rule.meta")],
      [t("ref.factor"), p.factor], [t("blank.level"), p.ref_stat === "mean" ? t("blank.stat.mean") : t("blank.stat.max")], [t("blank.min"), p.min_detect],
      [t("ref.pseudo"), p.pseudo], [t("ref.pseudo.srcrow"), t(`ref.pseudo.src.${p.pseudo_source}`)], [t("ref.floor"), p.floor], [t("ref.noisef"), p.noise_factor], ["ρ", p.rho_min], [t("ref.drop"), p.drop],
      [t("ref.show"), t(`ref.show.${st.show}`)], [t("ref.shapef"), t(`ref.shape.${st.shape}`)], [t("ref.sorted"), `${st.sort} ${st.dir < 0 ? "↓" : "↑"}`], [t("ref.table.src"), res.tb.source || ""],
      [t("mappa.meta.date"), new Date().toISOString()]];
  };
  const tsv = () => params().map(l => "# " + l.join("\t")).concat([head().join("\t")], body().map(l => l.map(c => String(c).replace(/[\t\r\n]+/g, " ")).join("\t"))).join("\n") + "\n";

  let live = null;                                              // the box of the section, repainted when the table or the sample table changes
  function mount(box) {
    live = box;
    paint(box);
    if (TABELLA.table && !res) refresh(box);
    box.onsubmit = e => {
      e.preventDefault();
      st.factor = numIt(box.querySelector("#ref-f").value); st.stat = box.querySelector("#ref-s").value; st.min = parseInt(box.querySelector("#ref-n").value, 10);
      st.pseudo = box.querySelector("#ref-p").value.trim() || "auto"; st.floor = numIt(box.querySelector("#ref-fl").value) || 1000; save();
      refresh(box);
    };
    box.onchange = e => {
      if (e.target.id === "ref-show") { st.show = e.target.value; save(); box.querySelector("#ref-tbl").innerHTML = tableHtml(); }
      else if (e.target.id === "ref-shape") { st.shape = e.target.value; save(); box.querySelector("#ref-tbl").innerHTML = tableHtml(); }
      else if (e.target.dataset && e.target.dataset.ref != null) {
        st.ref = [...box.querySelectorAll("input[data-ref]")].filter(c => c.checked).map(c => c.dataset.ref); save(); refresh(box);
      }
    };
    box.onclick = e => {
      const b = e.target.closest("button");
      if (b && b.id === "ref-default") { st.ref = null; save(); refresh(box); return; }
      if (b && b.dataset.sort) { st.dir = st.sort === b.dataset.sort ? -st.dir : (b.dataset.sort === "id" || b.dataset.sort === "shape" || b.dataset.sort === "state" ? 1 : -1); st.sort = b.dataset.sort; save(); box.querySelector("#ref-tbl").innerHTML = tableHtml(); const nb = box.querySelector(`button[data-sort="${st.sort}"]`); if (nb) nb.focus(); return; }
      if (b && b.id === "ref-xlsx" && res) { const w = [10, 12, 10, 14, 14, 10, 10, 10, 16, 8, 8, 18, 8, 10, 24, 40, 30]; dlx("reference-difference.xlsx", [{ name: "Features", head: head(), rows: body(), widths: w }, { name: "Parameters", head: [t("mappa.meta.what"), t("mappa.meta.value")], rows: params(), widths: [34, 70] }]); return; }
      if (b && b.id === "ref-tsv" && res) { dl("reference-difference.tsv", tsv(), "text/tab-separated-values"); return; }
      const tr = e.target.closest("#ref-tab tbody tr");
      if (tr && res) inspect(res.rows[+tr.dataset.i], box);
    };
    box.onkeydown = e => { const tr = e.target.closest && e.target.closest("#ref-tab tbody tr"); if (tr && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); tr.click(); } };
  }
  const again = () => { res = null; if (live && live.isConnected && live.querySelector("#ref-res")) { if (TABELLA.table) refresh(live); else paint(live); } };
  document.addEventListener("tabella", again);
  document.addEventListener("campioni", again);
  if (typeof STAT !== "undefined") STAT.add({ id: "reference", title: "ref.title", hr: true, visible: () => true, mount });
  return { mount, run, inspect, tsv, get params() { return st; }, get result() { return res; } };
})();
window.RIFERIMENTO = RIFERIMENTO;
