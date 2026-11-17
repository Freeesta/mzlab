// @ts-nocheck
// The rows of the card «Manual integrations» for the windows drawn on the traces the page makes itself (chromatogram, XIC, DAD): the panels that are not
// transition panels. The engine measures them (POST api/mantraces, mzlab_core::manint::table_text, the same bytes as `mzlab-cli manual-table --set traces=`),
// with the panel, the concentration of the sample, the correction note and what lies under the baseline. Loaded by integrazioni-manuali.js, which draws the card.
const G = window;
const t = (k, p = {}) => G.I18N.t(k, p);
const st = { sig: "", recs: [], csv: "", busy: 0 };
const UV = "mAU·s";
const isXr = p => p && p.ints && p.ints.length && !ESP.mrm(p) && p._a && p._a.sr;
/** The window of a record as the engine reads it: the baseline it was drawn with (older records: its zero window or its two-point line, else the edges). */
const baseOf = it => it.bm === "line" && it.bl ? { baseline: "line", line: it.bl } : it.bm === "edges" && it.base ? { baseline: "line", line: it.base }
  : it.bm === "zero" || it.bm === "edges" ? { baseline: it.bm } : it.zero ? { baseline: "zero" } : it.base ? { baseline: "line", line: it.base } : { baseline: "edges" };
const sampleOf = n => n.replace(/\.mzml$/i, "");

/** The files (the samples of the windows with the traces they need) and the windows of the panels, with the record of each window. */
function build() {
  const files = new Map(), wins = [], recs = [];
  for (const p of E.panels) {
    if (!isXr(p)) continue;
    for (const it of p.ints) {
      const s = p._a.sr.find(q => q.key === it.key), f = E.files[it.k];
      if (!s || !f || f.gone || !s.x || s.x.length < 2) continue;
      const kind = CAMPIONI.kindOf(f), conc = (kind === "std" || kind === "qc") && f.conc != null;
      let e = files.get(it.k);
      if (!e) { e = { i: files.size, name: f.file || "", time: f.time ?? null, conc: conc ? f.conc : null, cunit: conc ? f.cunit || "" : "", trs: [], idx: new Map() }; files.set(it.k, e); }
      const panel = ptitle(p.title), label = String(it.ion || it.name || ""), tk = `${panel}|${it.key}`;
      let ti = e.idx.get(tk);
      if (ti == null) { ti = e.trs.length; e.idx.set(tk, ti); e.trs.push({ panel, label, unit: p.kind === "pda" ? UV : "", s }); }
      wins.push({ file: e.i, tr: ti, a: Math.min(it.a, it.b), b: Math.max(it.a, it.b), ...baseOf(it), ...(it.corr ? { corr: it.corr } : {}) });
      recs.push({ p, it, panel, label, sample: sampleOf(e.name) });
    }
  }
  return { files: [...files.values()], wins, recs };
}
/** What decides a new table: the windows, the samples and a cheap fingerprint of each trace (a smoothing or an other ion changes it). */
const sigOf = b => JSON.stringify([b.wins, b.files.map(e => [e.name, e.time, e.conc, e.cunit, e.trs.map(x => [x.panel, x.label, x.unit, x.s.x.length, x.s.y.reduce((a, v) => a + v, 0)])])]);
const filesJson = b => JSON.stringify(b.files.map(e => ({ name: e.name, time: e.time, ...(e.conc != null ? { conc: e.conc, conc_unit: e.cunit } : {}),
  transitions: e.trs.map(x => ({ panel: x.panel, label: x.label, unit: x.unit, rt: Array.from(x.s.x), y: Array.from(x.s.y) })) })));

/** Measures the windows again when they, the samples or the traces changed; true when the table changed. */
async function sync() {
  const b = build(), sig = sigOf(b);
  if (sig === st.sig) { st.recs = b.recs; return false; }
  st.sig = sig; st.recs = b.recs;
  const n = ++st.busy;
  if (!b.wins.length) { st.csv = ""; return true; }
  try {
    const r = await fetch("api/mantraces", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ files: filesJson(b), windows: JSON.stringify(b.wins), sums: false }) });
    if (n === st.busy) st.csv = r.ok ? await r.text() : "";
  } catch (_) { if (n === st.busy) st.csv = ""; }
  return true;
}

const parsed = () => { const all = G.MANINT.rows(st.csv), head = all[0] || []; return { head, body: all.slice(1), c: n => head.indexOf(n) }; };
const count = () => st.csv ? parsed().body.length : 0;
/** The record of a row: same panel, channel and sample, the edges the engine wrote (three decimals). */
function recOf(r, c) {
  return st.recs.find(x => x.panel === r[c("panel")] && x.label === r[0] && x.sample === r[c("sample")] && +Math.min(x.it.a, x.it.b).toFixed(3) === +r[c("start_min")] && +Math.max(x.it.a, x.it.b).toFixed(3) === +r[c("end_min")]) || null;
}
const COLS = ["panel", "channel", "sample", "time_min", "conc", "area", "start_min", "end_min", "apex_rt_min", "height", "snr", "baseline", "below", "correction"];
const NUM = ["time_min", "conc", "area", "start_min", "end_min", "apex_rt_min", "height", "snr", "below"];
const HEAD = { panel: "canali.manint.h.panel", channel: "canali.manint.h.channel", sample: "canali.manint.h.sample", time_min: "canali.manint.h.time", conc: "canali.manint.h.conc",
  area: "canali.manint.h.area", start_min: "canali.manint.h.start", end_min: "canali.manint.h.end", apex_rt_min: "canali.manint.h.apex", height: "canali.manint.h.height",
  snr: "canali.manint.h.snr", baseline: "canali.manint.h.base", below: "canali.manint.h.below", correction: "canali.manint.h.corr" };
const sci = v => /^-?\d(\.\d+)?E[+-]\d+$/.test(v) ? G.I18N.sciE(Number(v)) : v;
const BASES = () => ({ zero: t("int.base.zero"), edges: t("int.base.edges"), line: t("int.base.line") });
/** One cell as shown; apex, area and height copy their full-precision CSV value. */
function cell(r, c, col, uv) {
  const v = col === "channel" ? r[0] : col === "conc" ? (r[c("conc")] !== "" ? `${r[c("conc")]} ${r[c("conc_unit")]}`.trim() : "") : col === "correction" ? (r[c("correction")] || t("ibk.no")) : r[c(col)];
  const shown = col === "time_min" ? G.timeTag(v === "" ? null : +v) : col === "baseline" ? BASES()[v] || v : col === "area" || col === "height" || col === "below" ? sci(v) : v;
  const copy = ["apex_rt_min", "area", "height"].includes(col) && v !== "";
  return copy ? h`<span class="manint-cp" role="button" tabindex="0" data-cp="${v}" aria-label="${t("canali.manint.cp.aria", { c: t(HEAD[col]), v: shown })}" title="${t("canali.manint.cp")}">${shown}</span>` : h`${shown}`;
}
/** The table of these rows (empty text without any). */
function html() {
  if (!st.csv) return "";
  const { head, body, c } = parsed(); if (!body.length) return "";
  const uv = body.every(r => r[c("area_unit")] === UV), mix = !uv && body.some(r => r[c("area_unit")] === UV);
  const area = t(uv ? "canali.manint.h.areaUv" : mix ? "canali.manint.h.areaMix" : "canali.manint.h.area");
  const ths = COLS.map(k => h`<th scope="col"${raw(NUM.includes(k) ? ' class="num"' : "")}>${k === "area" ? area : t(HEAD[k])}</th>`).join("");
  const trs = body.map((r, i) => {
    const rec = recOf(r, c), who = { ch: r[0], s: r[c("sample")] };
    const tds = COLS.map(k => h`<td${raw(NUM.includes(k) ? ' class="num"' : "")}>${raw(cell(r, c, k, uv))}</td>`).join("");
    const act = rec ? h`<button type="button" class="manint-x" data-xdel="${i}" aria-label="${t("canali.manint.del.aria", who)}" title="${t("canali.manint.del")}">×</button>` : "";
    return h`<tr class="manint-go" data-xgo="${i}" tabindex="0" aria-label="${t("canali.manint.show.aria", who)}">${raw(tds)}<td>${raw(act)}</td></tr>`;
  }).join("");
  return h`<table class="qq-tab" id="manint-x"><thead><tr>${raw(ths)}<th scope="col"><span class="sr-only">${t("canali.manint.h.act")}</span></th></tr></thead><tbody>${raw(trs)}</tbody></table>`;
}
/** Wires the rows of the table inside `el`: a click shows the integration on its chart, the X deletes it. */
function bind(el) {
  if (!st.csv) return;
  const { body, c } = parsed(), recAt = b => recOf(body[Number(b.dataset.xgo ?? b.dataset.xdel)], c);
  const show = b => { const rec = recAt(b); if (rec) { G.reveal(rec.p); G.draw(rec.p); } };
  el.querySelectorAll("tr[data-xgo]").forEach(b => {
    b.addEventListener("click", e => { if (!e.target.closest("button, [data-cp]")) show(b); });
    b.addEventListener("keydown", e => { if (e.target === b && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); show(b); } });
  });
  el.querySelectorAll("[data-xdel]").forEach(b => b.addEventListener("click", () => { const rec = recAt(b); if (rec) G.delInts(rec.p, [rec.it]); }));
}
G.MANTR = { sync, html, bind, count, csv: () => st.csv, state: () => ({ csv: st.csv }) };
