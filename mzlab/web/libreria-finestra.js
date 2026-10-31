// @ts-check
// «What is it?» and «Identify all the MS2» (high resolution): one large resizable window, master/detail. Left the candidates (or the analogs, or the precursors
// of the file) as a sortable table, right the selected one: 2D structure, mirror plot with hover, evidence behind «Details». Rare settings in «Options…»,
// copy and Excel in the download button. A match is a candidate with evidence, never a verdict. Loaded on first use by libreria.js (LIB.whatIs, LIB.identifyAll).
import { smilesSvg } from "./mol-svg.js";

const W_ = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {any} */ a) => W_.I18N.t(k, a);
const EHt = (/** @type {unknown} */ s) => EH(s == null ? "" : s);
const LIB = () => W_.LIB, LV = () => W_.LIVELLI;
const THR_KEYS = ["entropia_min", "picchi_min", "precursore_ppm_max", "scansioni_min", "margine_min", "entropia_provvisorio", "analogo_mcos_min"];
// neutral change between a query and an analogue: the usual transformations first, then the simplest formula within 3 mDa (the composition of the engine)
const TRANSF = () => [[15.9949146, t("lib.tr.hydroxylation")], [-2.01565, t("lib.tr.dehydrogenation")], [13.97926, "+O −2H"], [-26.01565, "−C₂H₂"], [1.97926, "−CH₂ +O"], [-56.0626, t("lib.tr.dealkylation")],
  [18.010565, "+H₂O"], [-18.010565, "−H₂O"], [-27.9949146, "−CO"], [2.01565, "+2H"], [-14.01565, t("lib.tr.demethylation")], [14.01565, t("lib.tr.methylation")], [-43.98983, "−CO₂"], [31.98983, "+O₂"]];
const sgn = (/** @type {number} */ v, d = 4) => (v >= 0 ? "+" : "−") + Math.abs(v).toFixed(d);
const deltaKnown = (/** @type {number} */ dm) => { const x = TRANSF().find(([m]) => Math.abs(/** @type {number} */ (m) - dm) <= 0.003); return x ? String(x[1]) : null; };
/** @type {Map<number, Promise<string | null>>} */ const dfCache = new Map();
function deltaFormula(/** @type {number} */ dm) {
  const k = Math.round(dm * 1e4);
  if (!dfCache.has(k)) dfCache.set(k, (async () => {
    try {
      const j = await J(`api/composition?mz=${Math.abs(dm) - 0.000548579909}&ion=${encodeURIComponent("M+.")}&tol=3&unit=mda&max=30&elements=${encodeURIComponent("C:0-12,H:0-24,N:0-4,O:0-6,S:0-2")}`);
      const atoms = (/** @type {string} */ f) => [...f.matchAll(/([A-Z][a-z]?)(\d*)/g)].reduce((n, m) => n + (+m[2] || 1), 0);
      const best = j.results.map((/** @type {any} */ r) => [atoms(r.formula), r.formula]).sort((/** @type {any} */ a, /** @type {any} */ b) => a[0] - b[0])[0];
      return best ? (dm < 0 ? "−" : "+") + best[1] + " " + t("lib.simplest") : null;
    } catch (_) { return null; }
  })());
  return /** @type {Promise<string | null>} */ (dfCache.get(k));
}
const polTxt = (/** @type {number} */ v) => t(v > 0 ? "lib.what.pol.pos" : v < 0 ? "lib.what.pol.neg" : "lib.what.pol.unk");
// the same first InChIKey block (or name) = one structure: the best entry heads the group
function groupsOf(/** @type {any[]} */ rs) {
  const m = new Map(); rs.forEach((r, i) => { r._i = i; const k = LV().ikKey(r); if (!m.has(k)) m.set(k, []); m.get(k).push(r); });
  return /** @type {any[][]} */ ([...m.values()]);
}

const CSS = `#libwhat{width:96vw;height:92vh;max-width:none;max-height:none;min-width:min(640px,96vw);min-height:420px;resize:both;overflow:hidden;padding:12px 14px;box-sizing:border-box}
#libwhat[open]{display:flex;flex-direction:column;gap:6px}#libwhat.max{width:98vw!important;height:96vh!important;resize:none}#libwhat .lw-mx{min-width:24px;min-height:24px}#libwhat .lw-head{display:flex;flex-wrap:wrap;align-items:center;gap:8px 12px}#libwhat h3{margin:0;font-size:16px}
#libwhat .lw-q{flex:1;min-width:120px}#libwhat .lw-prec input{width:96px}#libwhat button,#libwhat summary,#libwhat select,#libwhat input{min-height:24px}#libwhat input{margin:0}#libwhat input[type=checkbox]{width:auto}
#libwhat .lw-pop{position:relative}#libwhat .lw-pop>summary{list-style:none;cursor:pointer;border:1px solid var(--line);border-radius:6px;padding:2px 8px;display:inline-flex;align-items:center;min-width:24px;background:var(--panel)}
#libwhat .lw-pop>summary::-webkit-details-marker{display:none}#libwhat .lw-menu{position:absolute;right:0;top:calc(100% + 4px);z-index:5;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:8px;padding:10px;min-width:260px;box-shadow:0 6px 20px color-mix(in srgb,var(--ink) 18%,transparent);display:flex;flex-direction:column;gap:6px;font-size:12px}
#libwhat .lw-menu label{display:flex;justify-content:space-between;gap:10px;align-items:center}#libwhat .lw-menu input{width:72px}#libwhat .lw-menu hr{border:0;border-top:1px solid var(--line);margin:4px 0;width:100%}
#libwhat .lw-vd{border:1px solid var(--line);border-radius:8px;padding:6px 10px;font-size:13px}#libwhat .lw-vd .lvb{font-size:13px}#libwhat .lw-vd ul{margin:4px 0;padding-left:18px}#libwhat .lw-vd summary{cursor:pointer;font-size:12px;width:max-content}
#libwhat .lw-main{flex:1;min-height:0;display:grid;grid-template-columns:minmax(380px,52%) minmax(0,1fr);gap:12px}#libwhat .lw-left,#libwhat .lw-right{min-height:0;min-width:0;display:flex;flex-direction:column;gap:6px}
#libwhat .lw-right{overflow:auto}#libwhat [role=tablist]{display:flex;gap:4px;border-bottom:1px solid var(--line)}#libwhat [role=tab]{border:1px solid transparent;border-bottom:0;border-radius:6px 6px 0 0;background:none;padding:4px 12px;cursor:pointer;color:var(--muted)}
#libwhat [role=tab][aria-selected=true]{border-color:var(--line);background:var(--panel);color:var(--ink);font-weight:600;margin-bottom:-1px}#libwhat .lw-tbl{flex:1;min-height:0;overflow:auto;border:1px solid var(--line);border-radius:6px}
#libwhat table{border-collapse:collapse;width:100%}#libwhat .lw-tbl table{table-layout:fixed}#libwhat td,#libwhat th{padding:4px 8px;border-bottom:1px solid var(--line);text-align:left;font-size:12px;vertical-align:top}
#libwhat .lw-tbl td{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}#libwhat .lw-tbl td.nm{white-space:normal;overflow-wrap:anywhere}#libwhat .num{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}
#libwhat .lw-tbl th{position:sticky;top:0;background:var(--panel);z-index:1;font-weight:600;vertical-align:bottom}#libwhat th button{all:unset;cursor:pointer;display:inline-block;max-width:100%;white-space:normal;min-height:24px;line-height:1.2}
#libwhat th button:focus-visible,#libwhat tr.row:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}#libwhat tr.row{display:table-row;cursor:pointer}#libwhat tr.row:hover,#libwhat tr.row.sel{background:var(--sel)}
#libwhat [data-grp]{font-size:11px;padding:0 6px;margin-left:4px}#libwhat .lw-id{display:flex;gap:12px;align-items:flex-start}#libwhat .lw-mol{flex:none;width:240px;height:170px;border:1px solid var(--line);border-radius:8px;display:grid;place-items:center;overflow:hidden;background:var(--panel)}
#libwhat .lw-mol svg{width:100%;height:100%}#libwhat .lw-meta{min-width:0;font-size:12px;display:flex;flex-direction:column;gap:2px}#libwhat .lw-meta h4{margin:0 0 2px;font-size:15px;overflow-wrap:anywhere}
#libwhat .lw-plot{position:relative}#libwhat canvas{width:100%;height:300px;display:block;border:1px solid var(--line);border-radius:6px}#libwhat .lw-tip{position:absolute;pointer-events:none;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:4px;padding:2px 6px;font-size:12px;white-space:nowrap;z-index:2}
#libwhat .lw-more table{width:auto}#libwhat .lw-more dl{display:grid;grid-template-columns:max-content 1fr;gap:2px 12px;margin:4px 0}#libwhat .lw-more dt{color:var(--muted)}#libwhat .lw-more dd{margin:0;overflow-wrap:anywhere}
#libwhat .lw-more summary{cursor:pointer}@media (max-width:900px){#libwhat .lw-main{grid-template-columns:minmax(0,1fr);overflow:auto}#libwhat .lw-tbl{max-height:45vh}}
#libwhat .lw-th{display:block;width:56px;height:40px;padding:0;border:1px solid var(--line);border-radius:4px;background:var(--panel);color:var(--muted);cursor:zoom-in;overflow:hidden}#libwhat .lw-th svg{width:100%;height:100%;display:block}#libwhat td.st{padding:2px 4px}
#libwhat .lw-big{position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:5;display:flex;flex-direction:column;gap:6px;align-items:flex-end;padding:10px;max-width:min(92vw,400px);background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:8px;box-shadow:0 6px 24px rgba(0,0,0,.35)}#libwhat .lw-big .lw-bigsvg{width:360px;max-width:100%;height:260px;display:grid;place-items:center}#libwhat .lw-big svg{width:100%;height:100%}#libwhat .lw-big code{align-self:stretch;user-select:text;overflow-wrap:anywhere;font-size:12px}
@media (pointer:coarse){#libwhat button,#libwhat summary,#libwhat select,#libwhat input,#libwhat tr.row td{min-height:40px}}`;
function ensureCss() {
  if (document.getElementById("lwcss")) return;
  const s = document.createElement("style"); s.id = "lwcss"; s.textContent = CSS; document.head.appendChild(s);
}

const SIZE = "mzlab.libwhat.size", CHUNK = 120;
const sizeRead = () => { try { return JSON.parse(localStorage.getItem(SIZE) || "null"); } catch (_) { return null; } };
const sizeSave = (/** @type {number} */ w, /** @type {number} */ hh) => { try { localStorage.setItem(SIZE, JSON.stringify({ w, h: hh })); } catch (_) { /* no storage */ } };
const mxSet = (/** @type {Element} */ b, /** @type {boolean} */ on) => { b.setAttribute("aria-pressed", String(on)); b.setAttribute("title", t(on ? "lib.all.restore" : "lib.all.max")); b.setAttribute("aria-label", t(on ? "lib.all.restore" : "lib.all.max")); };

/** The window (created once), emptied for a new use; Esc closes an open popup before the window. */
function dialog() {
  ensureCss();
  let d = /** @type {HTMLDialogElement | null} */ (document.getElementById("libwhat"));
  if (!d) {
    d = document.createElement("dialog"); d.id = "libwhat"; d.setAttribute("aria-labelledby", "lw-t"); document.body.appendChild(d);
    const dd = d, sz = sizeRead();
    if (sz && sz.w > 0 && sz.h > 0) { d.style.width = Math.min(sz.w, innerWidth * 0.98) + "px"; d.style.height = Math.min(sz.h, innerHeight * 0.98) + "px"; }
    // the size the user drags to is remembered (not the default, not the maximised one)
    let tm = 0; FRAME.ro(d, () => { if (!dd.open || !dd.style.width || dd.classList.contains("max")) return; clearTimeout(tm); tm = setTimeout(() => sizeSave(dd.offsetWidth, dd.offsetHeight), 300); });
    d.addEventListener("click", e => { const b = /** @type {HTMLElement} */ (e.target).closest(".lw-mx"); if (!b) return; const on = dd.classList.toggle("max"); mxSet(b, on); });
    d.addEventListener("cancel", e => { if (dd.querySelector("#li-big")) { e.preventDefault(); closeBig(dd); return; } const o = dd.querySelector(".lw-pop[open]"); if (o) { e.preventDefault(); o.removeAttribute("open"); /** @type {HTMLElement} */ (o.querySelector("summary")).focus(); } });
    // one popup at a time; a click outside closes it
    d.addEventListener("toggle", e => { const x = /** @type {HTMLDetailsElement} */ (e.target); if (x.classList && x.classList.contains("lw-pop") && x.open) dd.querySelectorAll(".lw-pop[open]").forEach(o => { if (o !== x) o.removeAttribute("open"); }); }, true);
    d.addEventListener("click", e => { const tg = /** @type {HTMLElement} */ (e.target); dd.querySelectorAll(".lw-pop[open]").forEach(o => { if (!o.contains(tg)) o.removeAttribute("open"); }); });
  }
  return d;
}
const dlIcon = () => (typeof IC_DL === "string" ? IC_DL.replace(";margin-right:4px", "") : "↓");
const pop = (/** @type {string} */ id, /** @type {string} */ face, /** @type {string} */ label, /** @type {string} */ body) =>
  `<details class="lw-pop" id="${id}"><summary title="${EHt(label)}" aria-label="${EHt(label)}">${face}</summary><div class="lw-menu">${body}</div></details>`;
const head = (/** @type {string} */ title, /** @type {string} */ sub, /** @type {string} */ mid, /** @type {string} */ opts, /** @type {string} */ dls) => `<div class="lw-head"><h3 id="lw-t">${title}</h3><span class="lw-q sm muted" id="lw-q">${sub}</span>${mid}
  ${pop("lw-opt", `⚙ ${t("lw.opt")}`, t("lw.opt.title"), opts)}${pop("lw-dl", dlIcon(), t("lw.dl"), dls)}<button type="button" class="lw-mx" aria-pressed="false">⤢</button><button type="button" class="x" id="lw-x" title="${t("common.close")}" aria-label="${t("common.close")}">&times;</button></div>`;
const thrHtml = () => { const T = LV().thresholds(); return `<b>${t("lv.thr.title")}</b>${THR_KEYS.map(k => `<label>${t(`lv.thr.${k}`)} <input data-thr="${k}" inputmode="decimal" value="${String(T[k])}"></label>`).join("")}<button type="button" id="lw-thr-reset">${t("lv.thr.reset")}</button>`; };
function thrBind(/** @type {HTMLElement} */ root, /** @type {() => void} */ again) {
  root.querySelectorAll("[data-thr]").forEach(el => { const i = /** @type {HTMLInputElement} */ (el); i.onchange = () => { const v = numIt(i.value); if (v >= 0) { LV().setThreshold(i.dataset.thr, v); again(); } }; });
  const rs = /** @type {HTMLElement | null} */ (root.querySelector("#lw-thr-reset"));
  if (rs) rs.onclick = () => { LV().reset(); const T = LV().thresholds(); root.querySelectorAll("[data-thr]").forEach(el => { const i = /** @type {HTMLInputElement} */ (el); i.value = String(T[/** @type {string} */ (i.dataset.thr)]); }); again(); };
}
async function copyTable(/** @type {string[]} */ h, /** @type {any[][]} */ rs, /** @type {any} */ meta, /** @type {(s: string) => void} */ msg) {
  if (!rs.length) return msg(t("lib.nothingCopy"));
  const f = (/** @type {unknown} */ v) => (typeof v === "number" ? String(+v.toFixed(3)) : String(v ?? ""));
  const tsv = await PROV.tsv(h.join("\t") + "\n" + rs.map(r => r.map(f).join("\t")).join("\n"), meta);
  const html = `<table><tr>${h.map(x => `<th>${EHt(x)}</th>`).join("")}</tr>${rs.map(r => `<tr>${r.map(v => `<td>${EHt(f(v))}</td>`).join("")}</tr>`).join("")}</table>`;
  try {
    if (W_.ClipboardItem && navigator.clipboard.write) await navigator.clipboard.write([new ClipboardItem({ "text/plain": new Blob([tsv], { type: "text/plain" }), "text/html": new Blob([html], { type: "text/html" }) })]);
    else await navigator.clipboard.writeText(tsv);
    msg(t("lib.copied", { n: rs.length }));
  } catch (_) { try { await navigator.clipboard.writeText(tsv); msg(t("lib.copied", { n: rs.length })); } catch (_2) { msg(t("lib.noCopy")); } }
}
/** Rows of a table move with the arrows and are chosen with Enter or Space. */
function rowKeys(/** @type {HTMLElement} */ box) {
  box.onkeydown = e => {
    const tr = /** @type {HTMLElement} */ (e.target).closest("tr.row"); if (!tr || /** @type {HTMLElement} */ (e.target).closest(".lw-th")) return;
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); /** @type {HTMLElement} */ (tr).click(); return; }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const rows = [...box.querySelectorAll("tr.row:not([hidden])")], i = rows.indexOf(tr), n = rows[i + (e.key === "ArrowDown" ? 1 : -1)];
    if (n) { /** @type {HTMLElement} */ (n).focus(); /** @type {HTMLElement} */ (n).click(); }
  };
}
const thSort = (/** @type {string} */ key, /** @type {string} */ label, /** @type {any} */ so, /** @type {string} */ cls = "") => {
  const on = so.key === key, dir = on ? (so.dir < 0 ? "descending" : "ascending") : "none";
  return `<th scope="col" class="${cls}" aria-sort="${dir}"><button type="button" data-sort="${key}" title="${EHt(t("lw.sort", { col: label.replace(/<[^>]+>/g, "") }))}">${label}<span aria-hidden="true">${on ? (so.dir < 0 ? "▾" : "▴") : ""}</span></button></th>`;
};
function sortBind(/** @type {HTMLElement} */ box, /** @type {any} */ so, /** @type {() => void} */ again) {
  box.querySelectorAll("[data-sort]").forEach(b => { const el = /** @type {HTMLElement} */ (b); el.onclick = () => { const k = String(el.dataset.sort); so.dir = so.key === k ? -so.dir : (k === "name" || k === "lib" || k === "type" || k === "dprec" || k === "rt" || k === "prec" ? 1 : -1); so.key = k; again(); const n = /** @type {HTMLElement | null} */ (box.querySelector(`[data-sort="${k}"]`)); if (n) n.focus(); }; });
}
const cmp = (/** @type {any} */ a, /** @type {any} */ b) => (typeof a === "string" || typeof b === "string" ? String(a ?? "").localeCompare(String(b ?? "")) : (a ?? -Infinity) - (b ?? -Infinity));

// the peak pairs behind the score of the selected candidate (query peak, library peak, difference); `an` adds the column of the precursor shift
function pairsTable(/** @type {any[][]} */ ps, /** @type {any} */ q, /** @type {any} */ l, /** @type {boolean} */ an) {
  if (!ps.length) return `<p class="muted">${t("lib.pairs.none")}</p>`;
  const rows = ps.slice().sort((a, b) => q.mz[a[0]] - q.mz[b[0]]).map(pr => {
    const dm = l.mz[pr[1]] - q.mz[pr[0]];
    return `<tr><td class="num">${q.mz[pr[0]].toFixed(4)}</td><td class="num">${l.mz[pr[1]].toFixed(4)}</td><td class="num">${sgn(dm)}</td><td class="num">${q.it[pr[0]].toFixed(3)}</td><td class="num">${l.it[pr[1]].toFixed(3)}</td>${an ? `<td>${pr[2] ? t("lib.pairs.yes") : ""}</td>` : ""}</tr>`;
  }).join("");
  return `<table class="lib-pairs"><caption>${t("lib.pairs.title", { n: ps.length })}</caption><tr><th scope="col" class="num">${t("lib.pairs.q")}</th><th scope="col" class="num">${t("lib.pairs.l")}</th><th scope="col" class="num">&Delta;m/z</th><th scope="col" class="num">${t("lib.pairs.qi")}</th><th scope="col" class="num">${t("lib.pairs.li")}</th>${an ? `<th scope="col">${t("lib.pairs.shift")}</th>` : ""}</tr>${rows}</table>`;
}

// hover on the mirror: the peak nearest to the pointer (experimental above the axis, library below) with its m/z, intensity and partner
function hover(/** @type {HTMLCanvasElement} */ cv, /** @type {HTMLElement} */ tip) {
  const hide = () => { tip.hidden = true; };
  cv.onpointerleave = hide;
  cv.onpointermove = e => {
    const g = /** @type {any} */ (cv)._mg; if (!g) return hide();
    const b = cv.getBoundingClientRect(), x = e.clientX - b.left, y = e.clientY - b.top, up = y < g.mid, s = up ? g.q : g.r;
    let best = -1, bd = 7;
    s.mz.forEach((/** @type {number} */ m, /** @type {number} */ i) => { const dx = Math.abs(g.X(m) - x); if (dx < bd) { bd = dx; best = i; } });
    if (best < 0) return hide();
    const pr = (g.r.pairs || []).find((/** @type {number[]} */ p) => p[up ? 0 : 1] === best), sh = (up ? g.shq : g.shl).has(best), nl = (up ? g.nlq : g.nll).has(best);
    const o = up ? g.r : g.q, mate = pr ? o.mz[pr[up ? 1 : 0]] : null;
    const what = mate != null ? t("lw.hover.pair", { mz: mate.toFixed(4), d: sgn(mate - s.mz[best]) }) + (sh ? " · " + t("lw.hover.shift") : "") : nl ? t("lw.hover.nl") : t("lw.hover.none");
    tip.textContent = t(up ? "lw.hover.exp" : "lw.hover.lib", { mz: s.mz[best].toFixed(4), it: Math.round(s.it[best] * 100) }) + " · " + what;
    tip.hidden = false;
    tip.style.left = Math.max(0, Math.min(x + 12, b.width - tip.offsetWidth - 4)) + "px"; tip.style.top = Math.max(0, y - 30) + "px";
  };
}

/** @typedef {{ peaks: number[][], prec: number | null, pol: number, f: any, rt: number | null, title: string }} Query */
/** @typedef {{ q: Query, ctx: any, d: HTMLDialogElement, L: any, st: any, $: (s: string) => any, msg: (x?: string) => void, pics: Map<string, Promise<string | null>> }} View */
const selOf = (/** @type {any} */ st) => { const A = st.sel.k === "an" ? st.an : st.res; return A && A.results[st.sel.i] ? A.results[st.sel.i] : null; };

// the headline (badge + best candidate) and, behind «Details», the evidence and the chemist's decision
function showVerdict(/** @type {View} */ V) {
  const { L, st, q, $ } = V;
  const R = st.res, vd = $("#lw-verdict");
  if (!R || !R.libs) { vd.innerHTML = ""; return; }
  const lv0 = L.assess(R.results, { nScans: st.n, analogs: st.an && st.an.results }); if (st.decided) LV().decide(lv0, st.decided);
  const r0 = st.sel.k === "id" ? R.results[st.sel.i] : null, cvs = r0 && st.lo != null ? L.cosVis({ it: R.query.it }, r0, st.lo) : null;
  // a proof only: it never changes the level
  if (cvs != null) lv0.lines = lv0.lines.concat([t("lib.what.cosVis", { lo: st.lo, cos: cvs.toFixed(2) })]);
  st.lv = lv0; if (W_.RAPPORTO) W_.RAPPORTO.note(lv0, q);
  const lv = LV().shown(lv0), b = lv0.best || lv0.nearest || lv0.analog, has = b && lv0.proposed !== "5" && lv0.proposed !== "0";
  vd.innerHTML = h`<div class="vd lw-vd"><span class="lvb ${LV().cls(lv)}" title="${LV().sentence(lv)}">${LV().tag(lv)}</span> <span class="lw-hl">${raw(has ? t("lw.best", { name: h`<b>${b.name || ""}</b>` }) : t("lw.noBest"))}</span> <span class="muted">· ${LV().name(lv)}</span>
    <details class="lw-why"${raw(st.why ? " open" : "")}><summary>${raw(t("lw.details"))}</summary><div class="sm"><b>${raw(t("lv.evidence"))}</b><ul>${lv0.lines.length ? lv0.lines.map((/** @type {string} */ x) => raw(h`<li>${x}</li>`)) : raw("<li>–</li>")}</ul><div>${lv0.decisive}</div>
    <div style="margin-top:4px">${raw(t("lv.proposed"))}: <b>${LV().tag(lv0.proposed)}</b> · <label>${raw(t("lv.decided"))} <select id="lv-dec"><option value="">${raw(t("lv.decided.none"))}</option>${LV().ORDER.map((/** @type {string} */ x) => raw(h`<option value="${x}"${raw(lv0.decided === x ? " selected" : "")}>${LV().tag(x)}</option>`))}</select></label> <span class="err" id="lv-msg" role="status"></span></div></div></details></div>`;
  $(".lw-why").ontoggle = () => { st.why = $(".lw-why").open; };
  const s = $("#lv-dec");
  // the chemist's decision: 2b is only written here
  s.onchange = () => { const r = LV().decide(lv0, s.value); if (!r.ok) { s.value = lv0.decided || ""; $("#lv-msg").textContent = r.msg; } else { st.decided = lv0.decided || ""; showVerdict(V); $("#lv-dec").focus(); } };
}

const dp = (/** @type {any} */ r) => sgn(r.dprec, 1);
const idRow = (/** @type {any} */ st, /** @type {any} */ r, /** @type {number} */ g, /** @type {number} */ k, /** @type {number} */ n) => `<tr class="row${st.sel.k === "id" && st.sel.i === r._i ? " sel" : ""}${k ? " mem" : ""}" tabindex="0" data-k="id" data-i="${r._i}"${st.sel.k === "id" && st.sel.i === r._i ? ' aria-current="true"' : ""}${k && !st.open.has(g) ? " hidden" : ""}>
  <td class="nm" title="${EHt(r.name)}">${k ? "↳ " : ""}${EHt(r.name)}${!k && n > 1 ? `<button type="button" class="sm" data-grp="${g}" aria-expanded="${st.open.has(g)}" title="${EHt(t("lib.what.group", { n: n - 1 }))}" aria-label="${EHt(t("lib.what.group", { n: n - 1 }))}">${st.open.has(g) ? "▾" : "▸"} +${n - 1}</button>` : ""}</td>
  <td class="num">${r.ent.toFixed(3)}</td><td class="num">${r.shared}</td><td class="num">${dp(r)}</td><td title="${EHt(r.type)}">${EHt(r.type)}</td><td title="${EHt(r.lib)}">${EHt(r.lib)}</td></tr>`;
const anRow = (/** @type {any} */ st, /** @type {any} */ r, /** @type {number} */ i) => `<tr class="row${st.sel.k === "an" && st.sel.i === i ? " sel" : ""}" tabindex="0" data-k="an" data-i="${i}"${st.sel.k === "an" && st.sel.i === i ? ' aria-current="true"' : ""}><td class="nm" title="${EHt(r.name)}">${EHt(r.name)}</td><td class="num">${sgn(r.delta)}</td>
  <td data-dm="${i}" title="${EHt(deltaKnown(r.delta) || "")}">${EHt(deltaKnown(r.delta) || "…")}</td><td class="num">${r.mcos.toFixed(2)}</td><td class="num">${t("lib.an.shifted", { matched: r.matched, shifted: r.shifted })}</td></tr>`;

// the left pane: the candidates (isomers grouped) or the analogs, sortable
function showList(/** @type {View} */ V) {
  const { d, L, st, $ } = V;
  const R = st.res, A = st.an, box = $("#lw-list"), note = $("#lw-note");
  $("#lw-tab-id").textContent = t("lw.tab.cand", { n: R && R.libs ? R.results.length : 0 }); $("#lw-tab-an").textContent = t("lw.tab.an", { n: A ? A.results.length : 0 });
  d.querySelectorAll("[role=tab]").forEach(b => { const on = /** @type {HTMLElement} */ (b).dataset.tab === st.tab; b.setAttribute("aria-selected", String(on)); b.setAttribute("tabindex", on ? "0" : "-1"); });
  box.setAttribute("aria-labelledby", st.tab === "an" ? "lw-tab-an" : "lw-tab-id"); note.textContent = "";
  if (!R) { box.innerHTML = ""; return; }
  if (!R.libs) { box.innerHTML = h`<p style="padding:8px">${raw(t("lib.loadFirst"))}</p><p style="padding:0 8px"><button type="button" id="lw-open">${raw(t("lib.openLibs"))}</button></p>`; $("#lw-open").onclick = () => { d.close(); L.open(); }; return; }
  if (st.tab === "id") {
    if (!R.results.length) { box.innerHTML = h`<p style="padding:8px">${raw(t("lib.what.none"))}</p>`; return; }
    const so = st.sortId, key = (/** @type {any} */ r) => (so.key === "dprec" ? Math.abs(r.dprec) : r[so.key]);
    st.groups = groupsOf(R.results).sort((a, b) => so.dir * cmp(key(a[0]), key(b[0])));
    box.innerHTML = h`<table><colgroup><col><col style="width:5.5em"><col style="width:5.5em"><col style="width:6.5em"><col style="width:6em"><col style="width:7em"></colgroup><thead><tr>${raw(thSort("name", t("lib.col.name"), so) + thSort("ent", t("lib.col.entropy"), so, "num") + thSort("shared", t("lib.col.shared"), so, "num") + thSort("dprec", t("lw.col.dppm"), so, "num") + thSort("type", t("lib.col.adduct"), so) + thSort("lib", t("lib.col.library"), so))}</tr></thead>
      <tbody>${raw(st.groups.map((g, gi) => g.map((r, k) => idRow(st, r, gi, k, g.length)).join("")).join(""))}</tbody></table>`;
    box.querySelectorAll("[data-grp]").forEach((/** @type {HTMLElement} */ b) => b.onclick = e => { e.stopPropagation(); const g = Number(b.dataset.grp); if (st.open.has(g)) st.open.delete(g); else st.open.add(g); showList(V); const n = box.querySelector(`[data-grp="${g}"]`); if (n) n.focus(); });
    sortBind(box, so, () => showList(V));
  } else {
    if (!A) { box.innerHTML = h`<p class="muted" style="padding:8px">${raw(t("lib.an.searching"))}</p>`; return; }
    if (!A.results.length) { box.innerHTML = h`<p style="padding:8px">${raw(t("lib.an.none"))}</p>`; return; }
    const so = st.sortAn, idx = A.results.map((/** @type {any} */ _, /** @type {number} */ i) => i).sort((/** @type {number} */ a, /** @type {number} */ b) => so.dir * cmp(A.results[a][so.key], A.results[b][so.key]));
    box.innerHTML = h`<table><colgroup><col><col style="width:7em"><col style="width:9em"><col style="width:6em"><col style="width:7em"></colgroup><thead><tr>${raw(thSort("name", t("lib.an.name"), so) + thSort("delta", "&Delta;m", so, "num"))}<th scope="col">${raw(t("lib.an.interp"))}</th>${raw(thSort("mcos", t("lib.an.mcos"), so, "num") + thSort("matched", t("lib.an.matched"), so, "num"))}</tr></thead>
      <tbody>${raw(idx.map((/** @type {number} */ i) => anRow(st, A.results[i], i)).join(""))}</tbody></table>`;
    note.textContent = t("lib.an.note");
    A.results.forEach(async (/** @type {any} */ r, /** @type {number} */ i) => { if (!deltaKnown(r.delta)) { const x = await deltaFormula(r.delta), c = box.querySelector(`[data-dm="${i}"]`); if (c) { c.textContent = x || t("lib.an.noFormula"); c.title = c.textContent; } } });
    sortBind(box, so, () => showList(V));
  }
  box.querySelectorAll("tr.row").forEach((/** @type {HTMLElement} */ tr) => tr.onclick = () => {
    st.sel = { k: tr.dataset.k, i: Number(tr.dataset.i) };
    box.querySelectorAll("tr.row").forEach((/** @type {HTMLElement} */ x) => { x.classList.toggle("sel", x === tr); if (x === tr) x.setAttribute("aria-current", "true"); else x.removeAttribute("aria-current"); });
    showDetail(V); if (st.sel.k === "id" && st.lo != null) showVerdict(V);
  });
}

// the right pane: structure, mirror and, behind «Details», the rare fields
function showDetail(/** @type {View} */ V) {
  const { st, L, $, pics } = V;
  const box = $("#lw-detail"), r = selOf(st), an = st.sel.k === "an", tok = ++st.tok;
  if (!r) { box.innerHTML = ""; return; }
  const q2 = an ? { mz: r.qa.mz, it: r.qa.it } : { mz: st.res.query.mz, it: st.res.query.it }, lib = an ? r.la : r;
  const facts = an ? `<div>&Delta;m ${sgn(r.delta)} · <span data-dmd>${EHt(deltaKnown(r.delta) || "…")}</span></div><div>${t("lib.an.mcos")} ${r.mcos.toFixed(2)} · ${t("lib.an.matched")} ${t("lib.an.shifted", { matched: r.matched, shifted: r.shifted })}</div>`
    : `<div>${t("lib.col.entropy")} ${r.ent.toFixed(3)} · ${t("lib.col.shared")} ${r.shared} · ${t("lw.col.dppm")} ${dp(r)}</div>`;
  box.innerHTML = h`<div class="lw-id"><div class="lw-mol" id="lw-mol" role="img" aria-label="${t("lw.mol.aria", { name: r.name || "" })}"><span class="sm muted">…</span></div>
    <div class="lw-meta"><h4>${r.name || ""}</h4>${raw(r.formula ? h`<div>${typeof fmtFormula === "function" ? raw(fmtFormula(r.formula)) : r.formula}${r.type ? " · " + r.type : ""}</div>` : r.type ? h`<div>${r.type}</div>` : "")}${raw(facts)}<div class="muted">${r.lib || ""}</div></div></div>
    <div class="lw-plot"><canvas id="lw-cv"></canvas><div class="lw-tip" hidden></div></div><div class="sm muted">${raw(t("lib.what.legend"))}</div>
    <details class="lw-more" id="lw-more"${raw(st.more ? " open" : "")}><summary class="sm">${raw(t("lw.details"))}</summary><div class="sm" id="lw-more-in"></div></details>`;
  $("#lw-more").ontoggle = () => { st.more = $("#lw-more").open; };
  const cv = $("#lw-cv");
  L.mirror(cv, { ...q2, hr: true, prec: st.prec, frag: st.frag }, an ? { mz: r.la.mz, it: r.la.it, prec: r.prec, name: r.name, shared: r.matched, pairs: r.pairs } : r);
  hover(cv, $(".lw-tip"));
  if (an && !deltaKnown(r.delta)) deltaFormula(r.delta).then(x => { const c = box.querySelector("[data-dmd]"); if (c && tok === st.tok) c.textContent = x || t("lib.an.noFormula"); });
  // behind «Details»: the rare fields, the shared neutral losses, the attribution and the peak pairs
  /** @type {number[]} */ const nls = [];
  JSON.parse(cv.dataset.nl || "[]").forEach((/** @type {[number]} */ [i]) => nls.push(+(/** @type {number} */ (st.prec) - q2.mz[i]).toFixed(4)));
  (r.pairs || []).forEach((/** @type {number[]} */ pr) => { const v = +(/** @type {number} */ (st.prec) - q2.mz[pr[0]]).toFixed(4); if (v > 0.5 && !nls.includes(v)) nls.push(v); });
  const kv = /** @type {[string, unknown][]} */ (an ? [] : [[t("lib.col.reverse"), r.reverse != null ? r.reverse.toFixed(3) : ""], [t("lib.col.explained"), r.explained != null ? Math.round(r.explained * 100) + "%" : ""], [t("lib.col.pol"), polTxt(r.polarity)], ["CE", r.ce], [t("lib.col.instrument"), r.inst], [t("lib.col.license"), r.license]]).concat([["InChIKey", r.ik]]);
  const attr = [r.authors, r.accession, r.license].some(x => x) ? `<div>${t("lib.what.attr", { authors: EHt(r.authors || "–"), accession: EHt(r.accession || "–"), license: EHt(r.license || "–") })}</div>` : "";
  $("#lw-more-in").innerHTML = h`<dl>${kv.filter(([, v]) => v != null && v !== "").map(([k, v]) => raw(h`<dt>${k}</dt><dd>${v}</dd>`))}</dl><div>${raw(nls.length ? t("lib.what.nl", { list: nls.sort((a, b) => a - b).map(v => v.toFixed(4)).join(", ") }) : t("lib.what.nlNone"))}</div>${raw(attr + pairsTable(r.pairs || [], q2, lib, an))}`;
  const smi = String(r.smiles || ""), mol = $("#lw-mol");
  const none = h`<span class="sm muted" style="padding:8px;text-align:center">${raw(t("lw.noStruct"))}</span>`;
  if (!smi) { mol.innerHTML = none; return; }
  if (!pics.has(smi)) pics.set(smi, smilesSvg(smi, 240, 170, "lwm"));
  /** @type {Promise<string | null>} */ (pics.get(smi)).then(svg => { if (tok !== st.tok) return; mol.innerHTML = svg || none; });
}

// structures of the «Identify all» rows: drawn once per SMILES and size, only for the rows in view; a click enlarges one (the SMILES stays selectable)
/** @type {Map<string, Promise<string | null>>} */ const thumbs = new Map();
const thumb = (/** @type {string} */ smi, /** @type {number} */ w, /** @type {number} */ hh, /** @type {string} */ id) => {
  const key = `${w}x${hh}:${smi}`; if (!thumbs.has(key)) thumbs.set(key, smilesSvg(smi, w, hh, id));
  return /** @type {Promise<string | null>} */ (thumbs.get(key));
};
/** Draws the thumbnails of `box` when they scroll into view; returns the hooks to watch rows added later and to stop. */
function lazyThumbs(/** @type {HTMLElement} */ box) {
  const draw = (/** @type {Element} */ el0) => {
    const el = /** @type {HTMLElement} */ (el0), smi = el.dataset.smi; if (!smi || el.dataset.done) return; el.dataset.done = "1";
    thumb(smi, 54, 38, "lwt").then(svg => { if (!el.isConnected) return; if (svg) el.innerHTML = h`${raw(svg)}`; else el.textContent = "–"; });
  };
  const watch = (/** @type {ParentNode} */ root) => root.querySelectorAll(".lw-th").forEach(el => (io ? io.observe(el) : draw(el)));
  const io = typeof IntersectionObserver === "undefined" ? null : new IntersectionObserver(es => es.forEach(e => { if (e.isIntersecting) { /** @type {IntersectionObserver} */ (io).unobserve(e.target); draw(e.target); } }), { root: box, rootMargin: "80px" });
  watch(box);
  return { watch, stop: () => { if (io) io.disconnect(); } };
}
function closeBig(/** @type {HTMLElement} */ d, back = true) {
  const w = d.querySelector("#li-big"); if (!w) return;
  const from = /** @type {HTMLElement | null} */ (d.querySelector(".lw-th[data-big]")); w.remove();
  if (from) { from.removeAttribute("data-big"); if (back) from.focus(); }
}
function bigStructure(/** @type {HTMLElement} */ d, /** @type {HTMLElement} */ from, /** @type {string} */ name) {
  closeBig(d, false); const smi = from.dataset.smi || ""; from.setAttribute("data-big", "1");
  const w = document.createElement("div"); w.className = "lw-big"; w.id = "li-big"; w.setAttribute("role", "dialog");
  w.innerHTML = h`<div class="lw-bigsvg" role="img" aria-label="${t("lw.th.bigAria", { name })}"><span class="sm muted">…</span></div><code tabindex="0">${smi}</code><button type="button" id="li-big-x">${raw(t("common.close"))}</button>`;
  d.appendChild(w);
  /** @type {HTMLElement} */ (w.querySelector("#li-big-x")).onclick = () => closeBig(d);
  /** @type {HTMLElement} */ (w.querySelector("#li-big-x")).focus();
  thumb(smi, 360, 260, "lwb").then(svg => { const slot = w.querySelector(".lw-bigsvg"); if (slot && w.isConnected) slot.innerHTML = svg ? h`${raw(svg)}` : h`<span class="sm muted">${raw(t("lw.noStruct"))}</span>`; });
  const away = (/** @type {Event} */ e) => { if (!w.isConnected) { d.removeEventListener("click", away, true); return; } const tg = /** @type {Node} */ (e.target); if (!w.contains(tg) && !(/** @type {HTMLElement} */ (tg)).closest?.(".lw-th")) { closeBig(d, false); d.removeEventListener("click", away, true); } };
  setTimeout(() => d.addEventListener("click", away, true), 0);
}

// Copy and Excel of the download button
function dlBind(/** @type {View} */ V) {
  const { q, ctx, st, $, msg } = V;
  // with a panel the provenance takes the file from it; otherwise from the query
  const p = ctx.p, file = p ? undefined : String((q.f && (q.f.label || q.f.file)) || ""), meta = { p, table: "library-analogs", file };
  const hd = ["#", t("lib.col.name"), t("lib.col.entropy"), t("lib.col.reverse"), t("lib.col.shared"), t("lib.col.explained"), t("lib.col.dprec") + " (ppm)", t("lib.col.adduct"), t("lib.col.pol"), "CE", t("lib.col.instrument"), t("lib.col.library"), t("lib.col.license"), "InChIKey", t("lv.proposed"), t("lv.decided")];
  const rowsOf = () => { const [pr, de] = st.lv ? [st.lv.proposed, st.lv.decided || ""] : ["", ""]; return st.res && st.res.libs ? st.res.results.map((/** @type {any} */ r, /** @type {number} */ i) => [i + 1, r.name, r.ent, r.reverse ?? "", r.shared, r.explained ?? "", r.dprec, r.type, polTxt(r.polarity), r.ce, r.inst, r.lib, r.license || "", r.ik || "", i === 0 ? pr : "", i === 0 ? de : ""]) : []; };
  $("#lw-copy").onclick = () => { $("#lw-dl").removeAttribute("open"); copyTable(hd, rowsOf(), meta, msg); };
  $("#lw-xlsx").onclick = () => {
    $("#lw-dl").removeAttribute("open");
    const rs = rowsOf(); if (!rs.length) return msg(t("lib.nothingSave"));
    const an = st.an ? st.an.results.map((/** @type {any} */ r, /** @type {number} */ i) => [i + 1, r.name, +r.delta.toFixed(4), deltaKnown(r.delta) || "", r.mcos, r.matched, r.shifted, r.lib]) : [];
    dlx((ctx.p && typeof plotName === "function" ? plotName(ctx.p) : t("lib.xlsx.fallback")) + t("lib.xlsx.suffix"), [{ name: t("lib.xlsx.sheet"), head: hd, rows: rs, widths: [5, 30, 10, 10, 10, 12, 14, 12, 10, 8, 14, 20, 14, 28, 10, 10] },
      { name: t("lib.what.sheetAn"), head: ["#", t("lib.an.name"), "Δm", t("lib.an.interp"), t("lib.an.mcos"), t("lib.an.matched"), t("lw.col.shifted"), t("lib.col.library")], rows: an, widths: [5, 30, 10, 24, 12, 12, 10, 20] }], { p, table: "library-analogs", file });
  };
}

/**
 * The «What is it?» view of one MS2 spectrum in the window: identity and analogs together, level and evidence.
 * `ctx.p` the spectrum panel (provenance, number of scans, scan window); `ctx.nScans` when known; `ctx.back` adds «← All MS²».
 * @param {Query} q @param {{ p?: any, k?: number, nScans?: number, back?: () => void, show?: () => void }} ctx
 */
function showQuery(q, ctx) {
  const d = dialog(), L = LIB(), prof = L.profOf(q.f), top = q.peaks.reduce((x, y) => Math.max(x, y[0]), 0);
  /** @type {any} */
  const st = { prec: q.prec != null ? +q.prec.toFixed(4) : null, v: prof.v, frag: Math.max(0.005, +(10e-6 * top).toFixed(4)), res: null, an: null, tab: "id", sel: { k: "id", i: 0 },
    n: ctx.nScans || 1, lo: null, decided: "", open: new Set(), why: false, sortId: { key: "ent", dir: -1 }, sortAn: { key: "mcos", dir: -1 }, tok: 0 };
  const opts = `<label>${t("lib.search.tol")} (ppm) <input id="lw-tol" inputmode="decimal" value="${st.v}"></label><label>${t("lib.search.peaksWithin")} (Da) <input id="lw-frag" inputmode="decimal" value="${st.frag}"></label><hr>${thrHtml()}<hr><button type="button" id="lw-libs">${t("lib.libs")}</button>`;
  const dls = `<button type="button" id="lw-copy" title="${t("lib.copy.title")}">${t("lib.copy")}</button><button type="button" id="lw-xlsx">Excel</button>`;
  const mid = `${ctx.back ? `<button type="button" id="lw-back">${t("lw.all.back")}</button>` : ""}<label class="lw-prec">${t("lib.search.prec")} <input id="lw-prec" inputmode="decimal" value="${st.prec ?? ""}"></label>
    <button type="button" id="lw-go" class="pri">${t("lib.search.go")}</button><span class="sm muted" id="lw-msg" role="status"></span>${ctx.back ? `<button type="button" id="lw-goto">${t("lw.all.show")}</button>` : ""}`;
  d.innerHTML = head(t("lib.what.title"), EHt(q.title), mid, opts, dls) + `<div id="lw-verdict"></div>
    <div class="lw-main"><section class="lw-left" aria-label="${t("lw.tabs")}"><div role="tablist"><button type="button" role="tab" id="lw-tab-id" aria-controls="lw-list" data-tab="id"></button><button type="button" role="tab" id="lw-tab-an" aria-controls="lw-list" data-tab="an"></button></div>
      <div class="lw-tbl" id="lw-list" role="tabpanel" tabindex="-1"></div><div class="sm muted" id="lw-note"></div></section>
    <section class="lw-right" id="lw-detail" aria-live="polite"></section></div>`;
  const $ = (/** @type {string} */ s) => /** @type {any} */ (d.querySelector(s));
  const msg = (/** @type {string | undefined} */ x) => { $("#lw-msg").textContent = x || ""; };
  $("#lw-x").onclick = () => d.close(); $("#lw-libs").onclick = () => { d.close(); L.open(); }; mxSet($(".lw-mx"), d.classList.contains("max"));
  if (ctx.back) { $("#lw-back").onclick = ctx.back; $("#lw-goto").onclick = ctx.show; }
  /** @type {View} */ const V = { q, ctx, d, L, st, $, msg, pics: new Map() };
  d.querySelectorAll("[role=tab]").forEach((/** @type {HTMLElement} */ b) => {
    b.onclick = () => { st.tab = String(b.dataset.tab); const A = st.tab === "an" ? st.an : st.res; if (A && A.results && A.results.length && st.sel.k !== st.tab) st.sel = { k: st.tab, i: st.tab === "id" ? (st.groups && st.groups[0] ? st.groups[0][0]._i : 0) : 0 }; showList(V); showDetail(V); };
    b.onkeydown = e => { if (e.key === "ArrowRight" || e.key === "ArrowLeft") { e.preventDefault(); const o = /** @type {HTMLElement} */ (d.querySelector(`[data-tab="${b.dataset.tab === "id" ? "an" : "id"}"]`)); o.click(); o.focus(); } };
  });
  rowKeys($("#lw-list"));

  const run = async () => {
    const num = (/** @type {string} */ id) => numIt(String($(id).value));
    st.prec = num("#lw-prec"); st.v = num("#lw-tol"); st.frag = num("#lw-frag");
    if (!(st.prec > 0) || !(st.v > 0) || !(st.frag > 0)) return msg(t("lib.checkTol"));
    msg(t("lib.searching")); const t0 = performance.now(); st.an = null; st.res = null; st.sel = { k: "id", i: 0 }; st.tab = "id";
    try {
      const [a, b] = await Promise.all([L.call("search", { peaks: q.peaks, prec: st.prec, pol: q.pol, tol: { v: st.v, unit: "ppm" }, frag: st.frag }), L.call("analogs", { peaks: q.peaks, prec: st.prec, pol: q.pol, frag: st.frag, maxDelta: 200, max: 10 })]);
      st.res = a; st.an = b; msg(a.libs ? t("lib.results", { n: a.results.length, ms: Math.round(performance.now() - t0) }) : "");
      // no identity candidate: the analogs open first
      if (a.libs && !a.results.length && b && b.results.length) { st.tab = "an"; st.sel = { k: "an", i: 0 }; }
    } catch (e) { st.res = null; msg(t("lib.error", { message: /** @type {Error} */ (e).message })); }
    showVerdict(V); showList(V); showDetail(V);
  };
  thrBind(d, () => { showVerdict(V); });
  $("#lw-go").onclick = run;
  ["#lw-prec", "#lw-tol", "#lw-frag"].forEach(id => { $(id).onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") run(); }; });
  ["#lw-tol", "#lw-frag"].forEach(id => { $(id).onchange = () => { if (st.res) run(); }; });
  dlBind(V);
  if (!d.open) d.showModal();
  showList(V);
  const redraw = () => { if (d.open && selOf(st)) showDetail(V); };
  if (!(/** @type {any} */ (d))._ro) /** @type {any} */ (d)._ro = FRAME.ro(d, () => { if (/** @type {any} */ (d)._redraw) /** @type {any} */ (d)._redraw(); });
  /** @type {any} */ (d)._redraw = redraw;
  if (ctx.p) L.scansOf(ctx.p).then((/** @type {number} */ n) => { if (n !== st.n) { st.n = n; if (st.res) showVerdict(V); } });
  const k = ctx.p ? ctx.p.k : ctx.k;
  if (k != null) J(`api/method?k=${k}`).then((/** @type {any} */ f) => { const sp = f && f.scan_params, lo = sp && sp.ms2 && sp.ms2.window_lo; if (lo > 0) { st.lo = lo; if (st.res) showVerdict(V); } }).catch(() => {});
  if (st.prec) run(); else msg(t("lib.typePrec"));
}

/** «What is it?» of the MS2 spectrum of the panel `p`. */
function whatIs(/** @type {any} */ p) {
  const q = LIB().queryOf(p);
  if (!q || !q.peaks.length) return info(t("lib.pickMs2"));
  showQuery(q, { p });
}

// «Identify all the MS2»: the MS2 of each scan are read in blocks of 60 (api/scanbin); the worker answers with the best library entry of each; the scans of
// one precursor (10 ppm) and one peak (RT within 0.2 min) are one row; a row opens «What is it?» of its best scan in the same window, «← All MS²» comes back
const phys = (/** @type {any} */ f) => (f && f.file ? f.file.split("#")[0] : "");
const lvCell = (/** @type {any} */ res) => { const lv = LV().shown(res); return `<span class="lvb ${LV().cls(lv)}" title="${EHt(LV().sentence(lv) + " · " + res.decisive)}">${EHt(LV().tag(lv))}</span>`; };
async function identifyAll(/** @type {number} */ k) {
  const L = LIB(), f = E.files[k]; if (!f) return;
  const k2 = (E.files.find((/** @type {any} */ x) => phys(x) === phys(f) && x.kind === ESP.ms2 && !x.gone) || {}).k;
  if (k2 == null) return info(t("lib.noMs2"));
  const d = dialog(), $ = (/** @type {string} */ s) => /** @type {any} */ (d.querySelector(s));
  const shell = () => {
    d.innerHTML = head(t("lib.all.title"), EHt(f.label || f.file) + " · " + t("lib.all.sub", { ppm: L.profOf(f).v }), `<span class="sm" id="li-st" role="status"></span>`,
      `<label style="justify-content:flex-start"><input type="checkbox" id="li-all"> ${t("lib.all.showAll", { n: 0 })}</label><hr>${thrHtml()}<hr><button type="button" id="lw-libs">${t("lib.libs")}</button>`,
      `<button type="button" id="li-xlsx">Excel</button><button type="button" id="li-mgf">${t("lib.all.exportMgf")}</button><button type="button" id="li-msp">${t("lib.all.exportMsp")}</button>`) + `<div id="li-body" class="lw-left" style="flex:1;min-height:0"></div>`;
    $("#lw-x").onclick = () => d.close(); $("#lw-libs").onclick = () => { d.close(); L.open(); }; mxSet($(".lw-mx"), d.classList.contains("max"));
  };
  shell(); if (!d.open) d.showModal();
  const say = (/** @type {string} */ x, /** @type {number} */ v, /** @type {number} */ m) => { $("#li-st").innerHTML = h`${x}${m ? raw(h` <progress max="${m}" value="${v}"></progress>`) : ""}`; };
  try {
    const libs = await L.call("list");
    if (!libs.libs.length) { $("#li-body").innerHTML = h`<p>${raw(t("lib.loadFirst"))}</p><p><button type="button" id="li-open">${raw(t("lib.openLibs"))}</button></p>`; $("#li-open").onclick = () => { d.close(); L.open(); }; return; }
    say(t("lib.all.reading"), 0, 1);
    const dda = await J(`api/dda?k=${k2}`), tgt = new Map(dda.sid.map((/** @type {any} */ s, /** @type {number} */ i) => [s, dda.tgt[i]])), pol = f.polarity === "negative" ? -1 : 1, /** @type {any[]} */ queries = [];
    let n = 1, i0 = 0;
    for (; i0 < n; i0 += 60) {
      const b = typeof scBin === "function" ? await scBin(`api/scanbin?k=${k2}&i0=${i0}&i1=${i0 + 59}&level=2`) : await J(`api/spectra?k=${k2}&i0=${i0}&i1=${i0 + 59}&level=2`); n = b.n;
      for (const s of b.scans) { const pr = tgt.get(s.sid); if (pr && s.mz.length) queries.push({ key: s.sid, rt: s.rt, prec: pr, pol, peaks: s.mz.map((/** @type {number} */ m, /** @type {number} */ j) => [m, s.y[j]]) }); }
      say(t("lib.all.readingN", { i: Math.min(i0 + 60, n), n }), Math.min(i0 + 60, n), n);
    }
    const t0 = performance.now();
    say(t("lib.all.comparing"), 0, queries.length);
    const r = await L.call("searchAll", { queries, tol: { v: L.profOf(f).v, unit: "ppm" }, frag: 0.01 }, (/** @type {any} */ m) => say(t("lib.all.comparingN", { i: m.read, n: m.total }), m.read, m.total));
    const ms = Math.round(performance.now() - t0);
    if (W_.LISTE) await W_.LISTE.ready();
    // contaminant exclusions (liste.js): candidates whose precursor is a contaminant the student excluded stay out of the list, counted, until «Show»
    let showHid = false, all = false, sel = -1, /** @type {{ watch: (r: ParentNode) => void, stop: () => void } | null} */ lazy = null;
    const so = { key: "rt", dir: 1 }, groups = L.groupRows(r.rows);
    const show = () => {
      const hidOf = (/** @type {any} */ g) => { const c = W_.LISTE ? W_.LISTE.compat(g.prec, f.polarity, UIP.hrPpm) : null; return !!(c && c.hid); };
      const base = groups.map((/** @type {any} */ g) => ({ g, v: L.groupAssess(g), k: W_.LISTE ? W_.LISTE.textFor(g.prec, f.polarity, UIP.hrPpm) : "", hid: hidOf(g) })).filter((/** @type {any} */ x) => all || (x.v.proposed !== "5" && x.v.proposed !== "0"));
      const nHid = base.filter((/** @type {any} */ x) => x.hid).length, rows = (showHid ? base : base.filter((/** @type {any} */ x) => !x.hid)), hasK = rows.some((/** @type {any} */ x) => x.k);
      const val = (/** @type {any} */ x) => (so.key === "rt" ? x.g.rt : so.key === "prec" ? x.g.prec : so.key === "name" ? x.g.best.name || "" : so.key === "ent" ? x.g.best.ent : so.key === "shared" ? x.g.best.shared : so.key === "dprec" ? Math.abs(x.g.best.dprec) : so.key === "n" ? x.g.n : LV().rank(LV().shown(x.v)));
      rows.sort((a, b) => so.dir * cmp(val(a), val(b)));
      const cnt = nHid ? h`<div class="sm cont-cnt" role="status">${raw(t(showHid ? "cont.cnt.shown" : "cont.cnt", { n: nHid }))} · <button type="button" id="li-chid">${raw(t(showHid ? "cont.hide" : "cont.show"))}</button></div>` : "";
      const head = thSort("rt", "RT", so, "num") + thSort("prec", "<i>m/z</i>", so, "num") + h`<th scope="col">${raw(t("lw.col.struct"))}</th>` + thSort("name", t("lib.col.name"), so) + h`<th scope="col">${raw(t("lst.col.formula"))}</th><th scope="col">${raw(t("lib.col.adduct"))}</th><th scope="col">${raw(t("lib.col.library"))}</th><th scope="col">InChIKey</th>` + thSort("ent", t("lib.all.bestEntropy"), so, "num") + thSort("shared", t("lib.col.shared"), so, "num") + thSort("dprec", t("lw.col.dppm"), so, "num") + thSort("n", t("lib.all.scans"), so, "num") + thSort("lv", t("lw.col.level"), so) + (hasK ? h`<th scope="col">${raw(t("lib.all.contaminant"))}</th>` : "");
      const cell = (/** @type {any} */ x) => { const b = x.g.best, smi = String(b.smiles || ""); return smi ? h`<td class="st"><button type="button" class="lw-th" data-smi="${smi}" aria-label="${t("lw.th.aria", { name: b.name || "" })}"><span aria-hidden="true">…</span></button></td>` : h`<td class="st">–</td>`; };
      const trs = rows.map((x, i) => h`<tr class="row${i === sel ? " sel" : ""}" tabindex="0" data-i="${i}" title="${raw(t("lib.all.openBest"))}"><td class="num">${x.g.rt.toFixed(2)}</td><td class="num">${x.g.prec.toFixed(4)}</td>${raw(cell(x))}<td class="nm">${x.g.best.name || ""}</td><td>${raw(x.g.best.formula ? fmtFormula(x.g.best.formula) : "")}</td><td title="${x.g.best.type || ""}">${x.g.best.type || ""}</td><td title="${x.g.best.lib || ""}">${x.g.best.lib || ""}</td><td title="${x.g.best.ik || ""}">${x.g.best.ik || ""}</td><td class="num">${x.g.best.ent != null ? x.g.best.ent.toFixed(2) : ""}</td><td class="num">${x.g.best.shared != null ? x.g.best.shared : ""}</td><td class="num">${x.g.best.dprec != null ? x.g.best.dprec.toFixed(1) : ""}</td><td class="num">${x.g.n}</td><td>${raw(lvCell(x.v))}</td>${raw(hasK ? h`<td title="${x.k || ""}">${x.k || ""}</td>` : "")}</tr>`);
      // hundreds of precursors: the rows arrive in chunks while scrolling (always up to the selected one)
      let n = Math.max(CHUNK, Math.ceil((sel + 1) / CHUNK) * CHUNK);
      const body = trs.slice(0, n).join("") || h`<tr><td colspan="14" class="muted">${raw(t("lib.all.noMatch"))}</td></tr>`;
      $("#li-body").innerHTML = h`${raw(cnt)}<div class="lw-tbl" id="li-list"><table><colgroup><col style="width:52px"><col style="width:72px"><col style="width:64px"><col><col style="width:96px"><col style="width:64px"><col style="width:84px"><col style="width:104px"><col style="width:58px"><col style="width:58px"><col style="width:56px"><col style="width:58px"><col style="width:58px">${raw(hasK ? "<col style=\"width:120px\">" : "")}</colgroup>
        <thead><tr>${raw(head)}</tr></thead><tbody>${raw(body)}</tbody></table></div>`;
      $("#li-st").textContent = t("lib.all.summary", { q: queries.length, ms, g: groups.length, r: rows.length });
      const ab = $("#li-all"); ab.checked = all; ab.parentElement.lastChild.textContent = " " + t("lib.all.showAll", { n: groups.length }); ab.onchange = () => { all = ab.checked; show(); };
      const chid = $("#li-chid"); if (chid) chid.onclick = () => { showHid = !showHid; show(); };
      $("#li-mgf").onclick = () => { $("#lw-dl").removeAttribute("open"); if (W_.WORKFLOW) W_.WORKFLOW.exportQueriesMgf(queries, f); };
      $("#li-msp").onclick = () => { $("#lw-dl").removeAttribute("open"); if (W_.WORKFLOW) W_.WORKFLOW.exportQueriesMsp(queries, f); };
      $("#li-xlsx").onclick = () => { $("#lw-dl").removeAttribute("open"); dlx(t("lib.all.xlsxPrefix") + String(f.label || f.file || "file").replace(/[^\w.-]+/g, "_") + ".xlsx", [
        { name: t("lib.all.sheetCand"), head: ["RT (min)", t("lib.all.precMz"), t("lib.col.name"), t("lst.col.formula"), t("lib.all.bestEntropy"), t("lib.all.scans"), t("lv.proposed"), t("lv.decided"), t("lv.evidence"), t("lib.col.adduct"), t("lib.col.library"), "InChIKey", "SMILES"].concat(hasK ? [t("lib.all.contaminant")] : []), rows: rows.map((/** @type {any} */ x) => [+x.g.rt.toFixed(3), +x.g.prec.toFixed(5), x.g.best.name || "", x.g.best.formula || "", x.g.best.ent != null ? +x.g.best.ent.toFixed(3) : "", x.g.n, x.v.proposed, x.v.decided || "", x.v.lines.concat([x.v.decisive]).join(" | "), x.g.best.type || "", x.g.best.lib || "", x.g.best.ik || "", x.g.best.smiles || ""].concat(hasK ? [x.k || ""] : [])), widths: [10, 14, 40, 20, 14, 10, 12, 12, 60, 12, 20, 28, 30] },
        { name: t("lib.all.sheetPar"), head: [t("lib.all.parameter"), t("lib.all.value")], rows: [[t("lib.table.file"), f.label || f.file || ""], [t("lib.all.precTol"), `${L.profOf(f).v} ppm`], [t("lib.all.peakTol"), t("lib.all.peakTolVal")], [t("lib.all.shown"), t("lib.all.shownVal", { n: rows.length, total: groups.length })], [t("lib.all.level"), t("lib.all.levelVal")]], widths: [28, 70] }], { table: "library-all", file: String(f.label || f.file || "") }); };
      const box = $("#li-list");
      sortBind(box, so, show); rowKeys(box);
      if (lazy) lazy.stop();
      lazy = lazyThumbs(box);
      box.onscroll = () => { if (n < trs.length && box.scrollTop + box.clientHeight > box.scrollHeight - 200) { const tb = box.querySelector("tbody"), n0 = tb.rows.length; tb.insertAdjacentHTML("beforeend", h`${raw(trs.slice(n, n + CHUNK).join(""))}`); n += CHUNK; for (let j = n0; j < tb.rows.length; j++) lazy.watch(tb.rows[j]); } };
      box.onclick = (/** @type {MouseEvent} */ e) => {
        const th = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest(".lw-th"));
        if (th) { e.stopPropagation(); const rw = /** @type {HTMLElement} */ (th.closest("tr.row")); bigStructure(d, th, rw ? String(rows[Number(rw.dataset.i)].g.best.name || "") : ""); return; }
        const tr = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("tr.row")); if (!tr) return;
        sel = Number(tr.dataset.i); const g = rows[sel].g, b = g.best, qq = queries.find(x => x.key === b.key);
        if (!qq) return;
        const back = () => { shell(); show(); const x = /** @type {HTMLElement | null} */ (d.querySelector(`#li-list tr.row[data-i="${sel}"]`)); if (x) x.focus(); };
        showQuery({ peaks: qq.peaks, prec: qq.prec, pol: qq.pol, f, rt: qq.rt, title: `${f.label || ""} · RT ${Number(qq.rt).toFixed(2)} min` }, { k: k2, nScans: g.n, back, show: () => gotoScan(b.key, d) });
      };
    };
    show();
  } catch (e) { $("#li-st").innerHTML = h`<span class="err">${/** @type {Error} */ (e).message}</span>`; }
}
function gotoScan(/** @type {string | number} */ sid, /** @type {HTMLDialogElement} */ d) {   // the MS2 panel of the DDA trio shows that scan
  try {
    const s1 = E.panels.find((/** @type {any} */ q) => q.type === "spec" && q.duo != null && q.el), s2 = s1 && E.panels.find((/** @type {any} */ q) => q.id === s1.duo && q.el);
    if (!s2 || !W_.DDA) return toast(t("lib.all.needDda"));
    d.close(); W_.DDA.selectMs2(s2, sid); s2.el.scrollIntoView({ block: "center", behavior: "smooth" });
  } catch (_) { toast(t("lib.all.cantOpen")); }
}

W_.LIBWIN = { whatIs, identifyAll };
