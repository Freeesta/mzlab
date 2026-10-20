// @ts-check
// Report page of one Flow run (`corsa/1`, written by `mzlab-cli flow report`): lays out the numbers Rust made, in one column that prints well in black and white.
// English only (Flow/Tabui rule: no lang/ keys). Every figure has its data table right under it. Same JSON, same HTML: nothing here reads a clock or a random value.
window.RAPPORTO_FLOW = (() => {
  /** @typedef {{name:string,bytes:number,sha256:string,type:string}} RFFile */
  /** @typedef {{name:string,value:unknown,state:string,proof?:unknown}} RFParam */
  /** @typedef {{instance:string,step:string,algorithm?:string,preset?:string,status:string,error_key?:string,ms:number,params_sha256:string,params:RFParam[]}} RFStep */
  /** @typedef {{schema:string,engine:Record<string,string>,plan_sha256?:string,inputs:RFFile[],lists:RFFile[],steps:RFStep[],total_ms:number,levels:{files:number,rows:number,auto:Record<string,number>,other:Record<string,number>},report?:any}} Corsa */

  const SECTIONS = [
    ["summary", "Summary"], ["inputs", "Inputs and lists"], ["plan", "Plan and parameters"],
    ["timing", "Timing"], ["counts", "Counts"], ["qc", "Quality control"],
  ];
  const STYLE = `
.rf{max-width:60rem;margin:0 auto;padding:0 16px;color:var(--ink);font:14px/1.45 system-ui,sans-serif}
.rf h2{margin:1.4em 0 .4em;font-size:1.15em;border-bottom:1px solid var(--line)}
.rf h3{margin:1em 0 .3em;font-size:1em}
.rf table{border-collapse:collapse;width:100%;margin:.4em 0}
.rf th,.rf td{border:1px solid var(--line);padding:3px 6px;text-align:left;vertical-align:top}
.rf td.n,.rf th.n{text-align:right;font-variant-numeric:tabular-nums}
.rf .h{font-family:ui-monospace,monospace;font-size:.85em;word-break:break-all}
.rf .bar{display:flex;flex-wrap:wrap;gap:6px 14px;align-items:center;padding:6px 0}
.rf fieldset{border:1px solid var(--line);padding:4px 10px}
.rf label{display:inline-flex;align-items:center;gap:4px;min-height:24px}
.rf button{min-height:24px;min-width:24px}
.rf svg{max-width:100%;height:auto}
.rf .bk{fill:var(--accent);stroke:var(--ink);stroke-width:1}
.rf .bk.err{fill:var(--bad);stroke-dasharray:4 2}
.rf .tx{fill:var(--ink);font-size:11px}
.rf [hidden]{display:none}
@media print{.rf .bar{display:none}.rf{max-width:none;padding:0;color:#000}.rf h2,.rf h3{break-after:avoid}.rf table,.rf svg{break-inside:avoid}.rf .bk{fill:#888}.rf .bk.err{fill:#fff}}
`;

  /** @param {unknown} s */
  const esc = s => String(s).replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c] || c));
  /** @param {number} ms */
  const fmtMs = ms => (ms >= 1000 ? (ms / 1000).toFixed(2) + " s" : (Math.round(ms * 10) / 10).toFixed(1) + " ms");
  /** @param {number} b */
  const fmtBytes = b => (b >= 1048576 ? (b / 1048576).toFixed(1) + " MB" : b >= 1024 ? (b / 1024).toFixed(1) + " kB" : b + " B");
  /** @param {unknown} v */
  const val = v => (typeof v === "string" ? v : JSON.stringify(v));
  /** @param {unknown} v */
  const num = v => (typeof v === "number" && isFinite(v) ? String(Math.round(v * 1e4) / 1e4) : "–");
  /** @param {string[]} head @param {string[][]} rows @param {string} caption @param {number[]=} right */
  const table = (head, rows, caption, right = []) =>
    `<table><caption>${esc(caption)}</caption><thead><tr>${head.map((h, i) => `<th scope="col"${right.includes(i) ? ' class="n"' : ""}>${esc(h)}</th>`).join("")}</tr></thead><tbody>${
      rows.map(r => `<tr>${r.map((c, i) => (i === 0 ? `<th scope="row">${c}</th>` : `<td${right.includes(i) ? ' class="n"' : ""}>${c}</td>`)).join("")}</tr>`).join("")}</tbody></table>`;
  /** @param {RFFile[]} list @param {string} caption */
  const files = (list, caption) => (list.length
    ? table(["Name", "Type", "Size", "SHA-256"], list.map(f => [esc(f.name), esc(f.type), fmtBytes(f.bytes), `<span class="h">${esc(f.sha256)}</span>`]), caption, [2])
    : `<p>No ${esc(caption.toLowerCase())}.</p>`);

  /** @param {Corsa} c */
  function summary(c) {
    const by = /** @type {Record<string,number>} */ ({});
    for (const s of c.steps) by[s.status] = (by[s.status] || 0) + 1;
    const ok = Object.keys(by).sort().map(k => `${by[k]} ${esc(k)}`).join(", ") || "none";
    const eng = Object.keys(c.engine).map(k => `${esc(k)} ${esc(c.engine[k])}`).join(" · ");
    return table(["Item", "Value"], [
      ["Engine", esc(eng)], ["Plan SHA-256", c.plan_sha256 ? `<span class="h">${esc(c.plan_sha256)}</span>` : "not recorded"],
      ["Input files", String(c.inputs.length)], ["Lists", String(c.lists.length)], ["Steps", `${c.steps.length} (${ok})`], ["Total time", fmtMs(c.total_ms)],
      ["Annotated rows", `${c.levels.rows} in ${c.levels.files} file(s)`],
    ], "Summary of the run");
  }

  /** @param {Corsa} c */
  function plan(c) {
    return c.steps.map(s => {
      const head = `${esc(s.instance)} — ${esc(s.step)}${s.algorithm ? " · " + esc(s.algorithm) : ""}${s.preset ? " · preset " + esc(s.preset) : ""}`;
      const rows = s.params.map(p => [esc(p.name), `<code>${esc(val(p.value))}</code>`, p.state === "estimated" ? "estimated" : "fixed", p.proof === undefined ? "" : `<code>${esc(val(p.proof))}</code>`]);
      return `<h3>${head}</h3><p>Status ${esc(s.status)}${s.error_key ? " (" + esc(s.error_key) + ")" : ""} · parameters SHA-256 <span class="h">${esc(s.params_sha256)}</span></p>` +
        (rows.length ? table(["Parameter", "Value", "State", "Proof of the estimate"], rows, "Parameters of " + s.instance) : "<p>No parameters.</p>");
    }).join("") || "<p>No steps.</p>";
  }

  /** @param {Corsa} c */
  function timing(c) {
    const max = Math.max(1, ...c.steps.map(s => s.ms)), row = 22, left = 150, w = 520;
    const bars = c.steps.map((s, i) => {
      const bw = Math.max(1, Math.round((s.ms / max) * (w - left - 70)));
      return `<text class="tx" x="${left - 6}" y="${i * row + 15}" text-anchor="end">${esc(s.instance)}</text><rect class="bk${s.status === "error" ? " err" : ""}" x="${left}" y="${i * row + 3}" width="${bw}" height="${row - 8}"/>` +
        `<text class="tx" x="${left + bw + 4}" y="${i * row + 15}">${esc(fmtMs(s.ms))}</text>`;
    }).join("");
    const fig = c.steps.length ? `<svg role="img" aria-label="Time of each step; the same numbers are in the table below" viewBox="0 0 ${w} ${c.steps.length * row + 4}" width="${w}">${bars}</svg>` : "";
    const rows = c.steps.map(s => [esc(s.instance), esc(s.step), esc(s.status), fmtMs(s.ms), c.total_ms > 0 ? (100 * s.ms / c.total_ms).toFixed(1) + " %" : "–"]);
    return fig + (rows.length ? table(["Instance", "Step", "Status", "Time", "Share"], rows, "Time of each step", [3, 4]) : "<p>No steps.</p>");
  }

  /** @param {Corsa} c */
  function counts(c) {
    const levels = [...new Set([...Object.keys(c.levels.auto), ...Object.keys(c.levels.other)])].sort();
    const lv = levels.length
      ? table(["Level", "auto", "Other"], levels.map(l => [esc(l), String(c.levels.auto[l] || 0), String(c.levels.other[l] || 0)]), "Rows per identification level (auto = given by the program, not confirmed)", [1, 2])
      : "<p>No annotated rows in this run.</p>";
    const st = /** @type {Record<string,number>} */ ({});
    for (const s of c.steps) st[s.status] = (st[s.status] || 0) + 1;
    return lv + table(["Status", "Steps"], Object.keys(st).sort().map(k => [esc(k), String(st[k])]), "Steps per status", [1]);
  }

  /** @param {any} r */
  function qc(r) {
    if (!r || !r.qc) return "<p>This run has no table report.</p>";
    const q = r.qc, cells = r.celle || {};
    let h = table(["Item", "Value"], [["Rows", String(r.righe ?? "–")], ["Files", String(r.file ?? "–")], ["Detected cells", num(cells.rilevate)], ["Targeted cells", num(cells.mirate)],
      ["Filled cells", num(cells.riempite)], ["Absent cells", num(cells.assenti)]], "Table report");
    const f = q.file || [];
    if (f.length) h += table(["File", "Detected", "Targeted", "Filled", "Absent", "Total area"], f.map((/** @type {any} */ x) => [esc(x.file), num(x.rilevati), num(x.mirati), num(x.riempiti), num(x.assenti), num(x.area_totale)]), "Cells per file", [1, 2, 3, 4, 5]);
    const g = q.gruppi || [];
    if (g.length) h += table(["Type", "Group", "Time (min)", "Files", "Rows", "Median CV"], g.map((/** @type {any} */ x) => [esc(x.tipo), esc(x.gruppo), num(x.tempo_min), num(x.file), num(x.righe), num(x.cv_mediano)]), "Replicate variation per condition", [2, 3, 4, 5]);
    if (q.standard_cv != null) h += `<p>Standard CV: ${num(q.standard_cv)}</p>`;
    const sc = q.scartate || [];
    if (sc.length) h += table(["Rule", "Rows"], sc.map((/** @type {any} */ x) => [esc(x.regola), num(x.righe)]), "Rows dropped by each rule", [1]);
    return h;
  }

  /** @param {Corsa} c */
  const bodies = c => ({ summary: summary(c), inputs: files(c.inputs, "Input files") + files(c.lists, "Lists"), plan: plan(c), timing: timing(c), counts: counts(c), qc: qc(c.report) });

  /**
   * Draws the report into `el`. `opts.sections` turns a section off (all on by default); the checkboxes of the toolbar do the same by hand.
   * @param {HTMLElement} el @param {Corsa} corsa @param {{sections?:Record<string,boolean>}=} opts
   */
  function render(el, corsa, opts = {}) {
    if (!document.getElementById("rf-style")) {
      const st = document.createElement("style"); st.id = "rf-style"; st.textContent = STYLE; document.head.appendChild(st);
    }
    const on = (/** @type {string} */ k) => !opts.sections || opts.sections[k] !== false;
    const b = bodies(corsa);
    el.innerHTML = `<article class="rf"><h1>Flow run report</h1><div class="bar"><fieldset><legend>Sections</legend>${
      SECTIONS.map(([k, t]) => `<label><input type="checkbox" data-sec="${k}"${on(k) ? " checked" : ""}> ${esc(t)}</label>`).join(" ")}</fieldset>` +
      `<button type="button" data-rf="json">Download JSON</button><button type="button" data-rf="print">Print</button></div>` +
      SECTIONS.map(([k, t]) => `<section id="rf-${k}" aria-labelledby="rf-${k}-h"${on(k) ? "" : " hidden"}><h2 id="rf-${k}-h">${esc(t)}</h2>${b[/** @type {keyof typeof b} */ (k)]}</section>`).join("") + `</article>`;
    for (const cb of el.querySelectorAll("input[data-sec]")) {
      cb.addEventListener("change", () => { const s = el.querySelector("#rf-" + /** @type {HTMLElement} */ (cb).dataset.sec); if (s) /** @type {HTMLElement} */ (s).hidden = !/** @type {HTMLInputElement} */ (cb).checked; });
    }
    const btn = (/** @type {string} */ k) => /** @type {HTMLButtonElement|null} */ (el.querySelector(`[data-rf="${k}"]`));
    btn("json")?.addEventListener("click", () => download(corsa));
    btn("print")?.addEventListener("click", () => window.print());
  }

  /** @param {Corsa} corsa */
  function download(corsa) {
    const url = URL.createObjectURL(new Blob([JSON.stringify(corsa, null, 1) + "\n"], { type: "application/json" }));
    const a = document.createElement("a"); a.href = url; a.download = "corsa.json"; document.body.appendChild(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  return { render, SECTIONS: SECTIONS.map(s => s[0]) };
})();
