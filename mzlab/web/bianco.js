// @ts-strict
"use strict";
// Blank subtraction of the feature table (high resolution only; «Statistica» panel): threshold = blank level (maximum or mean of the blank files)
// x a factor chosen by the user; a feature is kept when at least N sample files are above its threshold. Two lists, both visible: kept and
// discarded, with the blank level, the threshold and the highest sample for each feature. Nothing is hidden; the numbers come from /api/stat/blank
// (mzlab/statistica.py). The same parameters feed the first filter of the PCA.
const BIANCO = (() => {
  const KEY = "qqq.bianco";
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  let st = { factor: 3, stat: "max", min: 1 };
  let res = null;                                              // {r (server answer), features (the rows of the table it was computed on)}
  Object.assign(st, lsObj(KEY));
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };
  const fmt = (v, d = 4) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
  const num = v => (v == null || !isFinite(v) ? "–" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? v.toExponential(2) : v.toFixed(1));

  async function run() {
    const tb = TABELLA.table;
    if (!tb) throw new Error(t("blank.err.table"));
    const roles = TABELLA.roles();
    if (!roles.includes("blank")) throw new Error(t("blank.err.noblank"));
    if (!(st.factor >= 0) || !(st.min >= 1)) throw new Error(t("blank.err.param"));
    const j = await fetch("api/stat/blank", { method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ matrix: TABELLA.matrix(), roles, factor: st.factor, stat: st.stat, min_detect: st.min }) }).then(r => r.json());
    if (j.error) throw I18N.err(j);
    res = { r: j, features: tb.features };
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_blank", { factor: st.factor, stat: st.stat, min: st.min });
    return res;
  }
  const rowsOf = idxs => idxs.map(i => ({ f: res.features[i], q: res.r.rows[i] }));
  const head = () => ["ID", "m/z", "RT", t("blank.col.blank"), t("blank.col.thr"), t("blank.col.max"), t("blank.col.n")];
  const body = idxs => rowsOf(idxs).map(({ f, q }) => [f.id, +f.mz.toFixed(5), +f.rt.toFixed(3), q.blank, q.threshold, q.max_sample, q.n_above]);
  function list(title, idxs, id) {
    const rows = rowsOf(idxs);
    const tr = rows.map(({ f, q }) => `<tr data-mz="${f.mz}" data-rt="${f.rt}" tabindex="0" role="button" aria-label="${eh(t("feat.row.aria", { mz: fmt(f.mz), rt: fmt(f.rt, 2) }))}"><td class="num">${eh(f.id)}</td><td class="num">${fmt(f.mz)}</td><td class="num">${fmt(f.rt, 2)}</td><td class="num">${num(q.blank)}</td><td class="num">${num(q.threshold)}</td><td class="num">${num(q.max_sample)}</td><td class="num">${q.n_above}/${q.n_samples}</td></tr>`).join("");
    return `<h4 class="sm" style="margin:8px 0 2px">${eh(title)} (${idxs.length})</h4><div class="qq-scroll"><table class="qq-tab" id="${id}"><thead><tr>${head().map(h => `<th scope="col">${eh(h)}</th>`).join("")}</tr></thead><tbody>${tr}</tbody></table></div>`;
  }
  function paintRes(box) {
    const el = box.querySelector("#blank-res");
    if (!el) return;
    if (!res) { el.innerHTML = ""; return; }
    const r = res.r;
    el.innerHTML = `<div class="sm" role="status">${t("blank.sum", { k: r.kept.length, d: r.discarded.length, thr: r.factor, stat: r.stat === "mean" ? t("blank.stat.mean") : t("blank.stat.max"), nb: r.n_blank, min: r.min_detect })}</div>
${list(t("blank.kept"), r.kept, "blank-kept")}${list(t("blank.discarded"), r.discarded, "blank-disc")}
<div class="qq-form"><button type="button" id="blank-xlsx">Excel</button></div>`;
  }
  function paint(box) {
    const tb = TABELLA.table;
    box.innerHTML = `<p class="sm">${t("blank.intro")}</p>
<form id="blank-form" class="qq-form"><label>${t("blank.factor")} <input id="blank-f" value="${st.factor}" inputmode="decimal" size="4"></label>
<label>${t("blank.level")} <select id="blank-s"><option value="max"${st.stat === "max" ? " selected" : ""}>${t("blank.stat.max")}</option><option value="mean"${st.stat === "mean" ? " selected" : ""}>${t("blank.stat.mean")}</option></select></label>
<label>${t("blank.min")} <input id="blank-n" value="${st.min}" inputmode="numeric" size="3"></label><button type="submit" id="blank-go">${t("blank.go")}</button></form>
${tb ? "" : `<p class="sm">${t("blank.err.table")}</p>`}<div id="blank-msg" class="sm bad" role="alert"></div><div id="blank-res"></div>`;
    paintRes(box);
  }
  function mount(box) {
    paint(box);
    box.onsubmit = async e => {
      e.preventDefault();
      const f = numIt(box.querySelector("#blank-f").value), n = parseInt(box.querySelector("#blank-n").value, 10), msg = box.querySelector("#blank-msg");
      st.factor = f; st.stat = box.querySelector("#blank-s").value; st.min = n; save(); msg.textContent = "";
      try { await run(); paintRes(box); } catch (err) { msg.textContent = String(err && err.message || err); res = null; paintRes(box); }
    };
    box.onclick = e => {
      if (e.target.id === "blank-xlsx" && res) dlx("blank-subtraction.xlsx", [{ name: "Kept", head: head(), rows: body(res.r.kept), widths: [10, 12, 8, 14, 14, 14, 8] }, { name: "Discarded", head: head(), rows: body(res.r.discarded), widths: [10, 12, 8, 14, 14, 14, 8] }], { table: "blank" });
      const tr = e.target.closest("tbody tr[data-mz]");
      if (tr && window.PROFILO) PROFILO.show(tr.dataset.mz, tr.dataset.rt);
    };
    box.onkeydown = e => { const tr = e.target.closest && e.target.closest("tbody tr[data-mz]"); if (tr && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); tr.click(); } };
  }
  document.addEventListener("tabella", () => { res = null; });
  if (typeof STAT !== "undefined") STAT.add({ id: "blank", title: "blank.title", hr: true, visible: () => true, mount });
  return { mount, run, get params() { return st; }, get result() { return res; } };
})();
window.BIANCO = BIANCO;
