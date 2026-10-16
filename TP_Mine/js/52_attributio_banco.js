// @ts-check
// TPMINE-PRIVATE  «Banco 2b» of Attributio (admin mode): `ATTRIBUTIO_BANCO.open(ctx)` reviews ONE feature of the table of levels at a time. The engine (the .wasm `attributio_rev`, or the native
// route `feat/attributio/review` in the app) gives the proposal with its reasons, the proofs (mirror spectrum, fragments, isotopes, purity, RT versus logP), the checklist, the alternatives with the
// same formula and the decisions already taken; the chemist ticks the proofs he or she has seen (nothing is pre-ticked), decides and saves. The program proposes, it never decides a 2b.
// A decision is added to `feature_decisions.csv` (history in append-only: only the row with the same date and reviewer is replaced) in the campaign folder when the browser may write there,
// else downloaded; it also goes to the notebook and the .mzflow (operation `attributio`, field `feature_decisions`). Texts in English, without keys of lang/ (the exception of mzFinder).
// Colours: Okabe-Ito through the CSS variables of the levels; every chart has its table of data (TDATI); zoom with buttons, no drag; arrows go to the next and the previous feature.
(() => {
  "use strict";
  const REVIEWER_KEY = "qqq.attributio.reviewer";
  const LEVELS = [["", "keep the cascade's level"], ["2a", "2a"], ["2b", "2b"], ["3a", "3a"], ["3b", "3b"]];
  const EVIDENCE = [["", "none"], ["fragments", "explained fragments"], ["library", "library spectrum"], ["analog", "analog"], ["in_silico", "in silico"], ["context", "context"]];
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  const num = (/** @type {any} */ v, n = 4) => (v == null || !isFinite(v) ? "" : String(+Number(v).toFixed(n)));
  /** @type {any} */
  const W = window;

  /** @typedef {{ pool: any, files: {path: string, file: File}[], root: string, list: {pol: string, id: number}[], index: number, host: HTMLElement,
   *   dir?: any, dirPath?: string, onSaved: (text: string, path: string) => Promise<void> | void, say?: (m: string, err?: boolean) => void }} Ctx */
  /** @type {{ ctx: Ctx | null, rev: any, zoom: {lo: number, hi: number} | null, span: [number, number], busy: boolean }} */
  const st = { ctx: null, rev: null, zoom: null, span: [0, 1], busy: false };

  function css() {
    if (document.getElementById("bn-css")) return;
    const s = document.createElement("style"); s.id = "bn-css";
    s.textContent = ".bn{border:1px solid var(--line);border-radius:6px;padding:12px;margin:10px 0;background:var(--panel)}.bn h3,.bn h4{margin:6px 0}.bn .row{display:flex;flex-wrap:wrap;gap:8px;align-items:center}"
      + ".bn button,.bn select,.bn input[type=text],.bn textarea{min-height:28px;min-width:28px;font:inherit}.bn input[type=checkbox]{width:24px;height:24px;margin:0}.bn label{display:inline-flex;gap:6px;align-items:center}"
      + ".bn-chk{display:grid;gap:6px;margin:6px 0;padding:0;list-style:none}.bn-chk li{display:flex;gap:8px;align-items:flex-start}.bn-proof{color:var(--muted);font-size:12px}"
      + ".bn svg{max-width:100%;height:auto;background:var(--panel)}.bn svg text{fill:var(--ink);font-size:11px}.bn .bn-ax{stroke:var(--muted)}.bn .bn-exp{stroke:var(--lv1)}.bn .bn-ref{stroke:var(--lv3b)}"
      + ".bn .bn-hit{stroke:var(--lv2a)}.bn .bn-dash{stroke-dasharray:4 3}.bn table{border-collapse:collapse;font-size:12px}.bn th,.bn td{padding:3px 8px;text-align:left;border-bottom:1px solid var(--line)}"
      + ".bn-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:10px}.bn textarea{width:100%;box-sizing:border-box}.bn .err{color:var(--bad)}"
      + "@media (prefers-reduced-motion:no-preference){.bn{transition:border-color .15s}}";
    document.head.appendChild(s);
  }

  // ---- the rules of the note: a copy of rules.rs for the live state of Save; the engine checks again when it saves
  /** @param {any} cur @param {{livello: string, group: string, nome: string}} d @returns {string[]} */
  function changes(cur, d) {
    const out = [], to = { "2b": "2b", "3": "3", "3a": "3", "3b": "3", "2": "2a", "2a": "2a" }[/** @type {"2b"} */ (d.livello)];
    if (to && to !== cur.level) out.push(`level ${cur.level} -> ${to}`);
    if (d.group === "ambiguous" && (cur.group || "") !== "ambiguous") out.push("identity rejected");
    if (d.nome.trim() && d.nome.trim() !== (cur.name || "")) out.push(`name -> ${d.nome.trim()}`);
    return out;
  }

  // ---- charts
  /** @param {number} lo @param {number} hi @param {number} w @param {number} l @param {number} r */
  const xs = (lo, hi, w, l, r) => (/** @type {number} */ v) => l + (v - lo) / (hi - lo || 1) * (w - l - r);

  /** the mirror spectrum: the feature's MS2 up, the library's down; the explained peaks are labelled and drawn with a solid stroke @param {any} view */
  function mirrorSvg(view) {
    const peaks = view.peaks || [], ref = view.mirror ? view.mirror.peaks : [];
    const all = [...peaks.map((/** @type {any} */ p) => p.mz), ...ref.map((/** @type {number[]} */ p) => p[0])];
    if (!all.length) return "";
    const full = [Math.min(...all) - 5, Math.max(...all) + 5];
    st.span = /** @type {[number, number]} */ (full);
    const [lo, hi] = st.zoom ? [st.zoom.lo, st.zoom.hi] : full, W_ = 640, H = 300, L = 40, R = 10, mid = H / 2, X = xs(lo, hi, W_, L, R);
    let g = `<line class="bn-ax" x1="${L}" x2="${W_ - R}" y1="${mid}" y2="${mid}"/>`;
    const ticks = 6;
    for (let i = 0; i <= ticks; i++) { const v = lo + (hi - lo) * i / ticks; g += `<text x="${X(v)}" y="${H - 4}" text-anchor="middle">${num(v, 1)}</text><line class="bn-ax" x1="${X(v)}" x2="${X(v)}" y1="${mid - 3}" y2="${mid + 3}"/>`; }
    for (const p of peaks) if (p.mz >= lo && p.mz <= hi) {
      const y = mid - 8 - p.rel / 100 * (mid - 34), hit = !!p.frag;
      g += `<line class="bn-exp${hit ? "" : " bn-dash"}" stroke-width="${hit ? 3 : 2}" x1="${X(p.mz)}" x2="${X(p.mz)}" y1="${mid}" y2="${y}"/>`;
      if (hit) g += `<text x="${X(p.mz)}" y="${y - 3}" text-anchor="middle">${esc(p.frag.formula)}</text>`;
    }
    for (const p of ref) if (p[0] >= lo && p[0] <= hi) g += `<line class="bn-ref" stroke-width="2" x1="${X(p[0])}" x2="${X(p[0])}" y1="${mid}" y2="${mid + 8 + p[1] / 100 * (mid - 34)}"/>`;
    g += `<text x="${L}" y="12">feature (solid: explained)</text><text x="${L}" y="${H - 18}">${ref.length ? "library" : "no library spectrum"}</text>`;
    return `<svg id="bn-mirror" viewBox="0 0 ${W_} ${H}" width="${W_}" role="img" aria-label="Mirror spectrum of the feature and its best library match">${g}</svg>`;
  }

  /** the theoretical isotope pattern @param {number[][]} iso */
  function isotopeSvg(iso) {
    if (!iso.length) return "";
    const W_ = 320, H = 150, L = 36, R = 10, lo = iso[0][0] - 0.5, hi = iso[iso.length - 1][0] + 0.5, X = xs(lo, hi, W_, L, R);
    let g = `<line class="bn-ax" x1="${L}" x2="${W_ - R}" y1="${H - 24}" y2="${H - 24}"/>`;
    for (const [mz, rel] of iso) g += `<line class="bn-exp" stroke-width="4" x1="${X(mz)}" x2="${X(mz)}" y1="${H - 24}" y2="${H - 24 - rel / 100 * (H - 50)}"/><text x="${X(mz)}" y="${H - 8}" text-anchor="middle">${num(mz, 3)}</text><text x="${X(mz)}" y="${H - 28 - rel / 100 * (H - 50)}" text-anchor="middle">${num(rel, 1)}%</text>`;
    return `<svg id="bn-iso" viewBox="0 0 ${W_} ${H}" width="${W_}" role="img" aria-label="Theoretical isotope pattern of the best formula">${g}</svg>`;
  }

  /** observed RT (or organic fraction) against the window the logP predicts @param {any} r */
  function rtSvg(r) {
    if (!r || r.pred == null || r.y == null || r.window == null) return "";
    const W_ = 320, H = 90, L = 20, R = 20, lo = Math.min(r.y, r.pred - r.window) - 0.5, hi = Math.max(r.y, r.pred + r.window) + 0.5, X = xs(lo, hi, W_, L, R);
    const g = `<rect fill="none" class="bn-hit" stroke-width="2" x="${X(r.pred - r.window)}" y="30" width="${X(r.pred + r.window) - X(r.pred - r.window)}" height="20"/>`
      + `<line class="bn-ref" stroke-width="3" x1="${X(r.pred)}" x2="${X(r.pred)}" y1="26" y2="54"/><circle class="bn-exp" fill="var(--lv1)" cx="${X(r.y)}" cy="40" r="6"/>`
      + `<text x="${L}" y="14">observed (dot), predicted (bar) and window (box)</text><text x="${L}" y="${H - 8}">${num(lo, 2)}</text><text x="${W_ - R}" y="${H - 8}" text-anchor="end">${num(hi, 2)}</text>`;
    return `<svg id="bn-rt" viewBox="0 0 ${W_} ${H}" width="${W_}" role="img" aria-label="Observed ${esc(r.y_label)} against the window predicted from logP">${g}</svg>`;
  }

  /** the tables of data of the charts (TDATI) @param {HTMLElement} box @param {any} rev */
  function attachData(box, rev) {
    const T = W.TDATI, ev = rev.proposal.evidence, view = ev.view || {};
    if (!T) return;
    const q = (/** @type {string} */ s) => box.querySelector(s);
    if (q("#bn-mirror")) T.addButton(q("#bn-mirror"), () => ({
      title: "Mirror spectrum", head: ["m/z", "relative intensity (%)", "fragment", "error (ppm)", "neutral loss", "library m/z", "library relative intensity (%)"],
      rows: (view.peaks || []).map((/** @type {any} */ p, /** @type {number} */ i) => {
        const pair = (view.mirror?.pairs || []).find((/** @type {any[]} */ x) => x[0] === i), lp = pair ? view.mirror.peaks[pair[1]] : null;
        return [num(p.mz), num(p.rel, 1), p.frag ? p.frag.formula : "", p.frag ? num(p.frag.ppm, 2) : "", p.frag ? p.frag.loss : "", lp ? num(lp[0]) : "", lp ? num(lp[1], 1) : ""];
      }),
    }));
    if (q("#bn-iso")) T.addButton(q("#bn-iso"), () => ({ title: "Theoretical isotope pattern", head: ["m/z", "relative intensity (%)"], rows: view.isotopes.map((/** @type {number[]} */ p) => [num(p[0]), num(p[1], 2)]) }));
    if (q("#bn-rt")) T.addButton(q("#bn-rt"), () => {
      const r = rev.rtlogp;
      return { title: "RT versus logP", head: ["quantity", "value"], rows: [[r.y_label, num(r.y, 3)], ["logP", num(r.logp, 2)], ["predicted", num(r.pred, 3)], ["residual", num(r.resid, 3)], ["window", num(r.window, 3)], ["inside the window", r.ok == null ? "" : r.ok ? "yes" : "no"]] };
    });
  }

  // ---- the page
  /** @param {any} rev */
  function evidenceOptions(rev) {
    const refs = (rev.proposal.evidence.references || []).filter((/** @type {any} */ r) => r.origin === "library" && r.library);
    return refs.map((/** @type {any} */ r, /** @type {number} */ i) => `<option value="lib${i}">${esc(r.name || r.entry || r.library)} (${esc(r.library)}, cos ${num(r.cos, 3)})</option>`).join("");
  }

  /** @param {any} rev */
  function body(rev) {
    const p = rev.proposal, ev = p.evidence, view = ev.view || {}, cur = rev.current;
    const reasons = (p.reasons || []).map((/** @type {string} */ r) => `<li>${esc(r)}</li>`).join("") || "<li>no reasons recorded</li>";
    const frag = (view.peaks || []).filter((/** @type {any} */ x) => x.frag).map((/** @type {any} */ x) => `<tr><td>${num(x.mz)}</td><td>${num(x.rel, 1)}</td><td>${esc(x.frag.formula)}</td><td>${num(x.frag.ppm, 2)}</td><td>${esc(x.frag.loss)}</td></tr>`).join("");
    const alts = (rev.alternatives || []).map((/** @type {any} */ a) => `<tr><td>${esc(a.name)}${a.is_best ? " (proposed)" : ""}</td><td>${esc(a.source)}</td><td>${a.score == null ? "" : num(a.score, 3)}</td><td>${a.matched ?? ""}</td><td>${a.logp == null ? "" : num(a.logp, 2)}</td></tr>`).join("");
    const pur = ev.purity || {}, rt = rev.rtlogp || {};
    const checks = (rev.checklist || []).map((/** @type {any} */ c) => `<li><label><input type="checkbox" data-chk="${esc(c.key)}" aria-label="${esc(c.label)}"> ${esc(c.label)}</label>`
      + `${c.proof ? `<span class="bn-proof">${esc(c.proof)}</span>` : ""}</li>`).join("");
    return `<div class="row"><span class="lvb lv-${esc(cur.level)}" aria-label="Current level ${esc(cur.level)}${cur.sub && cur.sub !== "a" ? esc(cur.sub) : ""}">L${esc(cur.level)}${cur.sub && cur.sub !== "a" ? esc(cur.sub) : ""}</span>
        <span>${esc(cur.basis)}</span><strong>${esc(cur.name || "")}</strong><span>${esc(cur.identity || "")}</span></div>
      <p><strong>Proposal: ${esc(p.class)}</strong> (proposed level ${esc(p.proposed)}). The program proposes; the level is yours to decide.</p><ul>${reasons}</ul>
      ${p.best ? `<p>Best structure: ${esc(p.best.formula)} ${esc(p.best.adduct)} from ${esc(p.best.structure_from)}${p.best.ion_ppm != null ? `, ${num(p.best.ion_ppm, 2)} ppm` : ""}</p>` : ""}
      <div class="bn-grid"><div><h4>Mirror spectrum</h4>
        <div class="row"><button type="button" data-zoom="in" aria-label="Zoom in">+</button><button type="button" data-zoom="out" aria-label="Zoom out">−</button>
        <button type="button" data-zoom="left" aria-label="Move left">←</button><button type="button" data-zoom="right" aria-label="Move right">→</button><button type="button" data-zoom="reset" aria-label="Reset zoom">Reset</button></div>
        <div id="bn-mirror-box">${rev.has_ms2 ? mirrorSvg(view) : "<p>No MS2 for this feature.</p>"}</div></div>
        <div><h4>Isotopes</h4>${isotopeSvg(view.isotopes || []) || "<p>No isotope pattern.</p>"}<h4>Precursor purity</h4>
        <p>${esc(pur.badge || "")}${pur.purity != null ? ` (${num(pur.purity, 2)})` : ""}${(pur.badge_reasons || []).length ? ": " + esc(pur.badge_reasons.join("; ")) : ""}</p>
        <h4>RT versus logP</h4>${rtSvg(rt) || ""}<p>${esc(rt.note || (rt.ok == null ? "" : rt.ok ? "within the window" : "outside the window"))}</p></div></div>
      <h4>Fragments explained (${ev.n_explained ?? 0} of ${ev.n_peaks ?? 0})</h4>
      ${frag ? `<table><caption class="sr-only">Fragments explained</caption><thead><tr><th scope="col">m/z</th><th scope="col">%</th><th scope="col">fragment ion formula</th><th scope="col">ppm</th><th scope="col">loss</th></tr></thead><tbody>${frag}</tbody></table>` : "<p>none</p>"}
      <h4>Alternatives with the same formula</h4>
      ${alts ? `<table><caption class="sr-only">Alternatives with the same formula</caption><thead><tr><th scope="col">name</th><th scope="col">source</th><th scope="col">score</th><th scope="col">peaks</th><th scope="col">logP</th></tr></thead><tbody>${alts}</tbody></table>` : "<p>none found in the libraries of the campaign</p>"}
      <fieldset class="row"><legend>Checklist (tick what you have seen)</legend><ul class="bn-chk">${checks}</ul></fieldset>
      <fieldset><legend>Decision</legend><div class="row">
        <label>Level <select id="bn-lvl" aria-label="Level">${LEVELS.map(([v, t]) => `<option value="${v}">${esc(t)}</option>`).join("")}</select></label>
        <label>Name <input type="text" id="bn-name" aria-label="Name" value=""></label>
        <label>Identity <input type="text" id="bn-id" aria-label="Identity" value=""></label>
        <label><input type="checkbox" id="bn-amb" aria-label="Reject the identity (ambiguous)"> reject the identity</label>
        <label>Evidence <select id="bn-ev" aria-label="Evidence">${EVIDENCE.map(([v, t]) => `<option value="${v}">${esc(t)}</option>`).join("")}${evidenceOptions(rev)}</select></label></div>
        <label>Motivation <textarea id="bn-note" rows="3" aria-label="Motivation"></textarea></label>
        <div class="row"><label>Reviewer <input type="text" id="bn-by" aria-label="Reviewer" autocomplete="off"></label>
        <button type="button" id="bn-save" aria-label="Save the decision" disabled>Save</button><span id="bn-msg" role="status" class="mut"></span></div></fieldset>
      ${rev.decision_rows ? `<p>${rev.decision_rows} earlier decision row(s) for this feature in ${esc(rev.decisions_path)}.</p>` : ""}`;
  }

  /** @param {HTMLElement} sec */
  function wire(sec) {
    const q = (/** @type {string} */ s) => /** @type {any} */ (sec.querySelector(s));
    const rev = st.rev;
    const decision = () => ({ livello: q("#bn-lvl").value, nome: q("#bn-name").value, group: q("#bn-amb").checked ? "ambiguous" : "", note: q("#bn-note").value.trim(), by: q("#bn-by").value.trim() });
    const refresh = () => {
      const d = decision(), ch = changes(rev.current, { livello: d.livello, group: d.group, nome: d.nome });
      const need = (ch.length > 0 || d.livello === "2b") && !d.note, nothing = !d.livello && !d.nome && !d.group && !q("#bn-id").value.trim() && !q("#bn-ev").value;
      q("#bn-save").disabled = st.busy || !d.by || need || nothing;
      q("#bn-msg").textContent = need ? `A motivation is required: ${ch.join(", ") || "a 2b row"}.` : (!d.by ? "Write the reviewer." : "");
    };
    try { q("#bn-by").value = localStorage.getItem(REVIEWER_KEY) || ""; } catch (_) { /* no storage: the reviewer is typed every time */ }
    for (const id of ["#bn-lvl", "#bn-name", "#bn-id", "#bn-amb", "#bn-note", "#bn-by", "#bn-ev"]) { q(id).addEventListener("input", refresh); q(id).addEventListener("change", refresh); }
    q("#bn-save").onclick = () => save(sec);
    for (const b of sec.querySelectorAll("[data-zoom]")) b.addEventListener("click", () => zoom(sec, /** @type {HTMLElement} */ (b).dataset.zoom || ""));
    refresh();
  }

  /** @param {HTMLElement} sec @param {string} how */
  function zoom(sec, how) {
    const [flo, fhi] = st.span, cur = st.zoom || { lo: flo, hi: fhi }, w = cur.hi - cur.lo, c = (cur.lo + cur.hi) / 2;
    if (how === "reset") st.zoom = null;
    else if (how === "in" || how === "out") { const nw = Math.min(fhi - flo, Math.max(2, w * (how === "in" ? 0.5 : 2))); st.zoom = { lo: Math.max(flo, c - nw / 2), hi: Math.min(fhi, c + nw / 2) }; }
    else { const d = w * 0.25 * (how === "left" ? -1 : 1), lo = Math.max(flo, Math.min(fhi - w, cur.lo + d)); st.zoom = { lo, hi: lo + w }; }
    const box = /** @type {HTMLElement} */ (sec.querySelector("#bn-mirror-box"));
    box.innerHTML = mirrorSvg(st.rev.proposal.evidence.view || {});
    attachData(sec, st.rev);
  }

  /** the folder handle's file at `path` written with `text`; false when the browser may not write there @param {any} dir @param {string} path @param {string} text */
  async function writeInFolder(dir, path, text) {
    try {
      if (!dir || typeof dir.getFileHandle !== "function") return false;
      if (typeof dir.requestPermission === "function" && (await dir.requestPermission({ mode: "readwrite" })) !== "granted") return false;
      const parts = path.split("/"), name = /** @type {string} */ (parts.pop());
      let d = dir;
      for (const part of parts) d = await d.getDirectoryHandle(part, { create: true });
      const w = await (await d.getFileHandle(name, { create: true })).createWritable();
      await w.write(text); await w.close();
      return true;
    } catch (_) { return false; }
  }

  /** @param {string} name @param {string} text */
  function download(name, text) {
    const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([text], { type: "text/csv;charset=utf-8" })); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }

  /** the decision JSON the engine reads @param {HTMLElement} sec */
  function build(sec) {
    const q = (/** @type {string} */ s) => /** @type {any} */ (sec.querySelector(s)), rev = st.rev, ev = rev.proposal.evidence, sel = q("#bn-ev").value;
    const lib = sel.startsWith("lib") ? (ev.references || []).filter((/** @type {any} */ r) => r.origin === "library" && r.library)[Number(sel.slice(3))] : null;
    const evidence = lib ? { type: "library", library: lib.library, entry: lib.entry || "", cosine: lib.cos, matched: lib.matched, delta_mz: lib.dm ?? "", delta_formula: "", note: "" }
      : { type: sel, library: "", entry: "", cosine: sel === "fragments" ? ev.frac_explained ?? "" : "", matched: sel === "fragments" ? ev.n_explained ?? "" : "", delta_mz: "", delta_formula: "", note: "" };
    const t = new Date(), day = `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}-${String(t.getDate()).padStart(2, "0")}`;
    return {
      pol: rev.pol, id: rev.id, livello: q("#bn-lvl").value, nome: q("#bn-name").value.trim(), identita: q("#bn-id").value.trim(), group: q("#bn-amb").checked ? "ambiguous" : "",
      motivazione: q("#bn-note").value.trim(), decided_on: day, decided_by: q("#bn-by").value.trim(), rt_check: "", evidence,
      checklist: [...sec.querySelectorAll("[data-chk]")].filter(c => /** @type {HTMLInputElement} */ (c).checked).map(c => /** @type {HTMLElement} */ (c).dataset.chk),
    };
  }

  /** @param {HTMLElement} sec */
  async function save(sec) {
    const ctx = /** @type {Ctx} */ (st.ctx), q = (/** @type {string} */ s) => /** @type {any} */ (sec.querySelector(s)), say = (/** @type {string} */ m, /** @type {boolean} */ e = false) => { q("#bn-msg").textContent = m; q("#bn-msg").className = e ? "err" : "mut"; };
    const d = build(sec);
    try { localStorage.setItem(REVIEWER_KEY, d.decided_by); } catch (_) { /* optional */ }
    st.busy = true; q("#bn-save").disabled = true; say("Saving…");
    try {
      const r = await ctx.pool.attributioDecision(ctx.files, ctx.root, JSON.stringify(d), ctx.dirPath || "");
      const path = r.summary.path, name = path.split("/").pop() || "feature_decisions.csv";
      const wrote = r.summary.written || await writeInFolder(ctx.dir, path, r.text);
      if (!wrote) download(name, r.text);
      const op = { decision: { pol: d.pol, id: d.id, livello: d.livello, checklist: d.checklist, by: d.decided_by, on: d.decided_on }, feature_decisions: r.text };
      if (W.WORKFLOW && W.WORKFLOW.logOp) W.WORKFLOW.logOp("attributio", { attributio: op });
      await ctx.onSaved(r.text, path);
      say(wrote ? `Saved in ${path}.` : `Downloaded ${name}; put it in ${path.split("/").slice(0, -1).join("/")}.`);
    } catch (e) {
      const x = /** @type {any} */ (e);
      say("Not saved: " + ((x && x.params && x.params.detail) || (x && x.message) || String(e)), true);
    } finally { st.busy = false; }
  }

  /** the review of the feature `i` of the list, drawn in the host @param {number} i */
  async function show(i) {
    const ctx = /** @type {Ctx} */ (st.ctx), n = ctx.list.length;
    if (!n) return;
    ctx.index = (i + n) % n;
    const f = ctx.list[ctx.index];
    let sec = /** @type {HTMLElement | null} */ (ctx.host.querySelector("#at-banco"));
    if (!sec) {
      sec = document.createElement("section"); sec.id = "at-banco"; sec.className = "bn"; sec.setAttribute("aria-label", "Banco 2b"); ctx.host.textContent = ""; ctx.host.appendChild(sec);
      sec.addEventListener("keydown", ev => {
        const t = /** @type {HTMLElement} */ (ev.target), typing = /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) && !(t.tagName === "INPUT" && /** @type {HTMLInputElement} */ (t).type === "checkbox");
        if (ev.key === "Escape") close();
        else if (!typing && !ev.altKey && !ev.ctrlKey && !ev.metaKey && (ev.key === "ArrowRight" || ev.key === "ArrowLeft") && t.tagName !== "BUTTON") { ev.preventDefault(); show(ctx.index + (ev.key === "ArrowRight" ? 1 : -1)); }
      });
    }
    sec.setAttribute("aria-busy", "true");
    sec.innerHTML = `<h3>Banco 2b · ${esc(f.pol)} ${esc(f.id)} <span class="mut">(${ctx.index + 1} of ${n})</span></h3><p role="status">Reviewing…</p>`;
    try {
      st.rev = await ctx.pool.attributioReview(ctx.files, ctx.root, f.pol, f.id); st.zoom = null;
    } catch (e) {
      const x = /** @type {any} */ (e);
      sec.innerHTML = `<h3>Banco 2b · ${esc(f.pol)} ${esc(f.id)}</h3><p class="err" role="alert">${esc((x && x.params && x.params.detail) || (x && x.message) || String(e))}</p><button type="button" data-bn="close" aria-label="Close the Banco">Close</button>`;
      sec.querySelector("[data-bn=close]")?.addEventListener("click", close); sec.removeAttribute("aria-busy"); return;
    }
    const rev = st.rev;
    sec.innerHTML = `<div class="row"><h3 tabindex="-1" id="bn-h">Banco 2b · ${esc(rev.pol)} ${esc(rev.id)} · m/z ${num(rev.mz)} · RT ${num(rev.rt, 2)} <span class="mut">(${ctx.index + 1} of ${n})</span></h3>
      <button type="button" data-bn="prev" aria-label="Previous feature">←</button><button type="button" data-bn="next" aria-label="Next feature">→</button><button type="button" data-bn="close" aria-label="Close the Banco">Close</button></div>`
      + body(rev);
    sec.removeAttribute("aria-busy");
    for (const b of sec.querySelectorAll("[data-bn]")) b.addEventListener("click", () => { const k = /** @type {HTMLElement} */ (b).dataset.bn; if (k === "close") close(); else show(ctx.index + (k === "next" ? 1 : -1)); });
    wire(sec); attachData(sec, rev);
    /** @type {HTMLElement} */ (sec.querySelector("#bn-h")).focus();
  }

  function close() {
    const ctx = st.ctx; if (!ctx) return;
    ctx.host.textContent = "";
    st.rev = null;
  }

  /** @param {Ctx} ctx */
  function open(ctx) {
    css(); st.ctx = ctx; st.busy = false;
    return show(ctx.index);
  }

  W.ATTRIBUTIO_BANCO = { open, close, changes };
})();
