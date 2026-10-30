// @ts-strict
// «Experiments» view of the MSn tree window (COMP.tree) for a data-dependent file: every MSn scan grouped by precursor, in one virtualised table
// (only the rows in view are in the DOM). A scan opens in the DDA panels; a precursor opens its TIC (MS2 tab, `ptol` = the file's window) or its XIC.
// Loaded on the first opening of the window by composizione.js; the data are /api/dda (dda.js) and /api/scanlist (base peak intensity).
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
const H = 24;
const phys = (/** @type {Bag | null | undefined} */ f) => String((f && f.file) || "").split("#")[0];
/** @typedef {{ mz: number, idx: number[], z: number | null, lev: number[], rt0: number, rt1: number, bpi: number | null, no: number, act: string }} Gruppo */

/** The scans of D grouped by the precursor of their level-2 ancestor (an MS3 sits under the MS2 that started it), within the file's tolerance. @param {Bag} D @param {(mz: number) => number} tol @returns {Gruppo[]} */
export function groups(D, tol) {
  const n = D.sid.length, root = new Array(n).fill(-1);
  for (let i = 0; i < n; i++) {
    let j = i, hop = 0;
    while (j >= 0 && (D.lev ? D.lev[j] : 2) > 2 && hop++ < 8) { const p = D.idx.get(D.parent[j]); j = p == null ? -1 : p; }
    root[i] = j < 0 ? i : j;
  }
  const keyOf = (/** @type {number} */ i) => D.prec[root[i]] ?? D.tgt[root[i]];
  const ord = [...Array(n).keys()].filter(i => keyOf(i) != null).sort((a, b) => keyOf(a) - keyOf(b) || D.rt[a] - D.rt[b]);
  /** @type {{ anchor: number, idx: number[] }[]} */ const bins = [];
  for (const i of ord) { const m = keyOf(i), g = bins[bins.length - 1]; if (g && m - g.anchor <= tol(g.anchor)) g.idx.push(i); else bins.push({ anchor: m, idx: [i] }); }
  const mode = (/** @type {any[]} */ v) => { const c = new Map(); let b = null, bn = 0; for (const x of v) { if (x == null) continue; const k = (c.get(x) || 0) + 1; c.set(x, k); if (k > bn) { bn = k; b = x; } } return b; };
  return bins.map(g => {
    g.idx.sort((a, b) => D.rt[a] - D.rt[b] || a - b);
    let rt0 = Infinity, rt1 = -Infinity; for (const i of g.idx) { rt0 = Math.min(rt0, D.rt[i]); rt1 = Math.max(rt1, D.rt[i]); }
    return { mz: mode(g.idx.map(i => keyOf(i))), idx: g.idx, z: mode(g.idx.map(i => D.z ? D.z[i] : null)), lev: [...new Set(g.idx.map(i => (D.lev ? D.lev[i] : 2)))].sort((a, b) => a - b), rt0, rt1, bpi: null, no: D.no[g.idx[0]], act: mode(g.idx.map(i => D.act[i])) || "" };
  });
}

/** @typedef {{ f: FileAperto, D: Bag, lev: number[], levels: number[], B: (number | null)[], G: Gruppo[], dec: number }} Vista */

/** The data of the view: the DDA table, the base peak intensity of every scan (/api/scanlist per level) and the groups. @param {FileAperto} f @returns {Promise<Vista>} */
async function load(f) {
  const D = await DDA.get(f.k), dec = HR.prof(f, 2).dec, tol = (/** @type {number} */ mz) => HR.tolDa(f, mz, 2) ?? 0.5;
  const lev = D.lev || D.sid.map(() => 2), levels = [...new Set(lev)].sort((a, b) => a - b);
  const bpi = new Map();
  await Promise.all(levels.map(lv => J(`api/scanlist?k=${f.k}&level=${lv}`).then((/** @type {Bag} */ t) => t.sid.forEach((/** @type {number} */ s, /** @type {number} */ j) => bpi.set(s, t.bpint[j]))).catch(() => null)));
  const B = D.sid.map((/** @type {number} */ s) => bpi.get(s) ?? null);
  const G = groups(D, tol);
  G.forEach(g => { g.bpi = g.idx.reduce((/** @type {number | null} */ m, i) => (B[i] != null && (m == null || /** @type {number} */ (B[i]) > m) ? B[i] : m), null); });
  return { f, D, lev, levels, B, G, dec };
}

/** Sort keys of a scan (`sk`) and of a group (`gk`), per column. @param {Vista} V */
function sorters(V) {
  const { D, lev, B } = V;
  const sk = { no: (/** @type {number} */ i) => D.no[i], rt: (/** @type {number} */ i) => D.rt[i], lev: (/** @type {number} */ i) => lev[i], prec: (/** @type {number} */ i) => D.prec[i] ?? 0, z: (/** @type {number} */ i) => D.z ? D.z[i] ?? 0 : 0, act: (/** @type {number} */ i) => D.ce[i] ?? 0, bpi: (/** @type {number} */ i) => B[i] ?? -1 };
  const gk = { no: (/** @type {Gruppo} */ g) => g.no, rt: (/** @type {Gruppo} */ g) => g.rt0, lev: (/** @type {Gruppo} */ g) => g.lev[g.lev.length - 1], prec: (/** @type {Gruppo} */ g) => g.mz, z: (/** @type {Gruppo} */ g) => g.z ?? 0, act: (/** @type {Gruppo} */ g) => g.idx.length, bpi: (/** @type {Gruppo} */ g) => g.bpi ?? -1 };
  return { sk, gk };
}

const COLS = () => [["no", T("msnl.col.no")], ["rt", T("msnl.col.rt")], ["lev", "MS"], ["prec", T("msnl.col.prec")], ["z", "z"], ["act", T("msnl.col.act")], ["bpi", T("msnl.col.bpi")], ["", ""]];
/** The filters, the count and the empty grid with its header. @param {Vista} V @param {boolean} all */
function headHtml(V, all) {
  const lvOpt = V.levels.length > 1 ? h`<label>${T("msnl.f.lev")} <select data-f="lv"><option value="0">${T("msnl.f.all")}</option>${V.levels.map(l => raw(h`<option value="${l}">MS${l}</option>`))}</select></label>` : "";
  const inp = (/** @type {string} */ k, /** @type {string} */ ph, /** @type {string} */ lab, /** @type {number} */ w) => raw(h`<input data-f="${k}" inputmode="decimal" placeholder="${T(ph)}" aria-label="${T(lab)}" style="width:${w}px">`);
  const hd = COLS().map(c => raw(c[0] ? h`<span role="columnheader" data-c="${c[0]}" aria-sort="none"><button type="button">${c[1]}</button></span>` : `<span role="columnheader"></span>`));
  return h`<div class="cmp-grid msnl-f"><label>m/z ${inp("mz0", "msnl.f.min", "msnl.f.mz0", 80)}</label><label>– ${inp("mz1", "msnl.f.max", "msnl.f.mz1", 80)}</label>
    <label>RT ${inp("rt0", "msnl.f.min", "msnl.f.rt0", 64)}</label><label>– ${inp("rt1", "msnl.f.max", "msnl.f.rt1", 64)}</label>${raw(lvOpt)}
    <label><input type="checkbox" data-f="all"${raw(all ? " checked" : "")}> ${T("msnl.expand")}</label></div>
    <div class="muted sm" style="margin:4px 0" aria-live="polite" data-n></div>
    <div class="msnl" role="grid" tabindex="0" aria-label="${T("msnl.grid")}" aria-description="${T("msnl.hint")}" aria-rowcount="0"><div class="msnl-r msnl-h" role="row" aria-rowindex="1">${hd}</div><div class="msnl-b" role="rowgroup"></div></div>
    <div class="muted sm" style="margin-top:4px">${T("msnl.hint")}</div>`;
}

/** The markup of row `r` of `flat` (a precursor or one of its scans). @param {Vista} V @param {{ g: Gruppo, i?: number }} x @param {number} r @param {boolean} cur @param {boolean} ex */
function rowHtml(V, x, r, cur, ex) {
  const { D, lev, B, dec } = V, g = x.g, c = cur ? " cur" : "";
  const e = (/** @type {number | null} */ v) => (v == null ? "" : (+v).toExponential(2));
  const cells = (/** @type {any[]} */ v) => v.map(t => raw(h`<span role="gridcell">${t}</span>`));
  if (x.i == null) {
    const mz = g.mz.toFixed(dec), btn = (/** @type {string} */ a, /** @type {string} */ k) => raw(h`<button type="button" data-a="${a}" aria-label="${T(k, { mz })}" title="${T(k, { mz })}">${a.toUpperCase()}</button>`);
    return h`<div class="msnl-r msnl-g${c}" role="row" id="msnl-${r}" aria-rowindex="${r + 2}" aria-expanded="${ex}" data-r="${r}" style="top:${r * H}px">${cells([raw(h`<span aria-hidden="true">${ex ? "▾" : "▸"}</span> ×${g.idx.length}`), `${g.rt0.toFixed(2)}–${g.rt1.toFixed(2)}`, g.lev.join(","), raw(h`<b>${mz}</b>`), g.z ?? "", g.act, e(g.bpi), [btn("tic", "msnl.tic.aria"), btn("xic", "msnl.xic.aria")]])}</div>`;
  }
  const i = x.i, ce = [D.act[i], D.ce[i] == null ? "" : D.nce ? `NCE ${+D.ce[i].toFixed(2)}` : `${+D.ce[i].toFixed(2)} eV`].filter(Boolean).join(" ");
  // precursor m/z of a scan: an MS3 shows its path (first precursor → its own)
  const pt = lev[i] > 2 && D.prec[i] != null ? `${g.mz.toFixed(dec)} → ${D.prec[i].toFixed(dec)}` : (D.prec[i] ?? D.tgt[i] ?? g.mz).toFixed(dec);
  return h`<div class="msnl-r${c}" role="row" id="msnl-${r}" aria-rowindex="${r + 2}" data-r="${r}" style="top:${r * H}px">${cells([D.no[i], D.rt[i].toFixed(3), lev[i], pt, D.z ? D.z[i] ?? "" : "", ce, e(B[i]), ""])}</div>`;
}

const num = (/** @type {string} */ v, /** @type {number} */ d) => { const x = numIt(v); return Number.isFinite(x) ? x : d; };
const closeDlg = () => { const d = /** @type {HTMLDialogElement | null} */ (Q("#bigdlg")); if (d && d.open) d.close(); };
// the TIC of a precursor: a chromatogram of the MS2 tab whose precursor window holds every scan of the group, with its product-ion spectrum under it (a click on a point shows that scan)
function openTic(/** @type {Vista} */ V, /** @type {Gruppo} */ g) {
  const { f, D, lev, dec } = V;
  closeDlg();
  const v = g.idx.filter(i => lev[i] === 2).map(i => D.tgt[i] ?? D.prec[i]).filter(x => x != null); if (!v.length) return;
  const lo = v.reduce((a, b) => Math.min(a, b), Infinity), hi = v.reduce((a, b) => Math.max(a, b), -Infinity), mid = (lo + hi) / 2, t = HR.tolDa(f, mid, 2);
  const ptol = t == null ? null : +(Math.max(t, 1e-3) + (hi - lo) / 2).toPrecision(4);
  setTab(ESP.ms2, true);
  const y = tabPanels(ESP.ms2).reduce((/** @type {number} */ m, /** @type {Pannello} */ q) => Math.max(m, q.y + q.h + pnlGap()), 0);
  const c = addPanel("chrom", { tab: ESP.ms2, prec: +mid.toFixed(dec + 1), ptol, title: ptMs2(+g.mz.toFixed(dec)), x: 0, y, w: hostWidth(), h: 250, full: true });
  addMs2Spec(c, { prec: c.prec, k: (DDA.ms2Item(f) || f).k }, y + 260);
  relayout(); fitHost(); setActive(c); reveal(c); uiSave();
}
// the XIC of the precursor m/z in the Full Scan, with the Full Scan under it
function openXicOf(/** @type {Vista} */ V, /** @type {Gruppo} */ g) {
  const full = ESP.ids[0];
  closeDlg(); setTab(full, true);
  const x = xicDirect(+g.mz.toFixed(V.dec + 1), null), k1 = E.files.find((/** @type {Bag} */ q) => q.kind === full && phys(q) === phys(V.f));
  if (x && k1) apexSpectrum(x, newSpec(x, null, null, k1.k));
}

/** Mounts the view in `host` for the file `f` (an item of a DDA file). @param {HTMLElement} host @param {FileAperto} f */
export async function mount(host, f) {
  host.innerHTML = h`<div class="muted sm">${T("comp.calculating")}</div>`;
  const V = await load(f), { D, lev, G } = V, { sk, gk } = sorters(V);
  const st = { col: "prec", dir: 1, all: G.length <= 200, cur: 0, mz0: -Infinity, mz1: Infinity, rt0: -Infinity, rt1: Infinity, lv: 0 };
  /** @type {Set<Gruppo>} */ const open = new Set();
  host.innerHTML = headHtml(V, st.all);
  const L = /** @type {HTMLElement} */ (host.querySelector(".msnl")), R = /** @type {HTMLElement} */ (host.querySelector(".msnl-b")), nEl = /** @type {HTMLElement} */ (host.querySelector("[data-n]"));
  /** @type {{ g: Gruppo, i?: number }[]} */ let flat = [];
  const pass = (/** @type {number} */ i) => D.rt[i] >= st.rt0 && D.rt[i] <= st.rt1 && (!st.lv || lev[i] === st.lv);
  function build() {
    const c = /** @type {keyof typeof sk} */ (st.col), d = st.dir, s1 = sk[c], g1 = gk[c];
    const vis = G.filter(g => g.mz >= st.mz0 && g.mz <= st.mz1).map(g => ({ g, sc: g.idx.filter(pass) })).filter(x => x.sc.length);
    vis.sort((a, b) => d * (g1(a.g) - g1(b.g)) || a.g.mz - b.g.mz);
    flat = []; let n = 0;
    for (const { g, sc } of vis) {
      flat.push({ g }); n += sc.length;
      if (st.all || open.has(g)) sc.slice().sort((a, b) => d * (s1(a) - s1(b)) || D.rt[a] - D.rt[b]).forEach(i => flat.push({ g, i }));
    }
    nEl.textContent = vis.length ? T("msnl.count", { n, g: vis.length }) : T("msnl.none");
    L.setAttribute("aria-rowcount", String(flat.length + 1));
    R.style.height = flat.length * H + "px"; st.cur = Math.min(st.cur, Math.max(0, flat.length - 1)); host.querySelectorAll("[role=columnheader][data-c]").forEach(h => h.setAttribute("aria-sort", /** @type {HTMLElement} */ (h).dataset.c === st.col ? (st.dir > 0 ? "ascending" : "descending") : "none"));
    paint(true);
  }
  let pa = -1, pz = -1;
  function paint(/** @type {boolean} */ force = false) {
    const a = Math.max(0, Math.floor((L.scrollTop - H * 2) / H)), z = Math.min(flat.length, a + Math.ceil((L.clientHeight || 400) / H) + 4);
    if (!force && a === pa && z === pz) return;
    pa = a; pz = z;
    let s = ""; for (let r = a; r < z; r++) s += rowHtml(V, flat[r], r, r === st.cur, st.all || open.has(flat[r].g));
    R.innerHTML = s;
    if (flat.length) L.setAttribute("aria-activedescendant", `msnl-${st.cur}`); else L.removeAttribute("aria-activedescendant");
  }
  function act(/** @type {number} */ r, /** @type {string} */ a = "") {
    const x = flat[r]; if (!x) return; st.cur = r;
    if (a === "tic") return openTic(V, x.g);
    if (a === "xic") return openXicOf(V, x.g);
    if (x.i != null) { closeDlg(); return void DDA.showMs2(D, x.g.mz, x.i); }
    if (!st.all) { if (open.has(x.g)) open.delete(x.g); else open.add(x.g); build(); } else paint(true);
  }
  R.onclick = ev => { const t = /** @type {Element} */ (ev.target), r = /** @type {HTMLElement | null} */ (t.closest("[data-r]")); if (!r) return; const b = /** @type {HTMLElement | null} */ (t.closest("button[data-a]")); act(+(r.dataset.r || 0), b ? b.dataset.a : ""); };
  L.onscroll = () => paint();
  L.onkeydown = ev => {
    if (/** @type {Element} */ (ev.target).closest("button,input,select")) return;
    const k = ev.key, page = Math.max(1, Math.floor(L.clientHeight / H) - 2), x = flat[st.cur];
    let r = st.cur;
    if (k === "ArrowDown") r++; else if (k === "ArrowUp") r--; else if (k === "PageDown") r += page; else if (k === "PageUp") r -= page; else if (k === "Home") r = 0; else if (k === "End") r = flat.length - 1;
    else if (k === "Enter" || k === " ") { ev.preventDefault(); return act(st.cur); }
    else if ((k === "ArrowRight" || k === "ArrowLeft") && x && x.i == null && !st.all) { ev.preventDefault(); if ((k === "ArrowRight") !== open.has(x.g)) act(st.cur); return; }
    else return;
    ev.preventDefault();
    st.cur = Math.max(0, Math.min(flat.length - 1, r));
    const y = st.cur * H; if (y < L.scrollTop + H) L.scrollTop = Math.max(0, y - H); else if (y + H > L.scrollTop + L.clientHeight) L.scrollTop = y + 2 * H - L.clientHeight;
    paint(true);
  };
  host.querySelectorAll("[role=columnheader][data-c] button").forEach(b => { /** @type {HTMLElement} */ (b).onclick = () => { const c = /** @type {HTMLElement} */ (b.parentElement).dataset.c || "prec"; st.dir = st.col === c ? -st.dir : 1; st.col = c; L.scrollTop = 0; build(); }; });
  host.querySelectorAll("[data-f]").forEach(el_ => {
    const el = /** @type {HTMLInputElement} */ (el_);
    // `input` only: a `change` on blur would rebuild the rows under a click that has just started
    el.oninput = () => {
      const k = el.dataset.f;
      if (k === "all") st.all = el.checked; else if (k === "lv") st.lv = +el.value;
      else if (k === "mz0" || k === "rt0") st[k] = num(el.value, -Infinity); else if (k === "mz1" || k === "rt1") st[k] = num(el.value, Infinity);
      L.scrollTop = 0; st.cur = 0; build();
    };
  });
  build();
}

const css = document.createElement("style");
css.textContent = `.msnl{position:relative;overflow:auto;height:min(60vh,560px);border:1px solid var(--line);font-variant-numeric:tabular-nums}.msnl:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.msnl-r{display:grid;grid-template-columns:72px 92px 40px minmax(130px,1.4fr) 32px minmax(90px,1fr) 78px 104px;align-items:center;height:${H}px;line-height:${H}px;white-space:nowrap;cursor:pointer;position:absolute;left:0;right:0;min-width:640px}
.msnl-r>span{overflow:hidden;text-overflow:ellipsis;padding:0 4px;text-align:right}.msnl-r>span:nth-child(6){text-align:left}.msnl-r:hover,.msnl-r.cur{background:var(--sel)}.msnl-r.cur{box-shadow:inset 2px 0 var(--accent)}
.msnl-g{font-weight:500;border-top:1px solid var(--line)}.msnl-g>span:first-child{text-align:left}.msnl-r button{min-height:22px;min-width:40px;padding:0 6px;margin-left:4px;font-size:.85em;line-height:20px}
.msnl-h{position:sticky;top:0;z-index:1;background:var(--panel);border-bottom:1px solid var(--line);cursor:default}.msnl-h button{background:none;border:0;font-weight:600;padding:0;color:inherit;cursor:pointer;margin:0;min-width:0}
.msnl-h [aria-sort=ascending] button::after{content:" ▲"}.msnl-h [aria-sort=descending] button::after{content:" ▼"}.msnl-b{position:relative}
.mt-tabs{display:flex;gap:4px;margin-bottom:8px;border-bottom:1px solid var(--line)}.mt-tabs [role=tab]{border:1px solid var(--line);border-bottom:0;border-radius:6px 6px 0 0;min-height:28px;padding:2px 12px;background:none;color:inherit}.mt-tabs [aria-selected=true]{background:var(--sel);font-weight:600}`;
document.head.appendChild(css);

/** The tab bar «Experiments | Tree» of the MSn tree window: the tree is computed on its first showing (`goTree`). @param {FileAperto} f @param {() => void} goTree */
export async function tabs(f, goTree) {
  const bar = /** @type {HTMLElement} */ (Q("#mt-tabs")), names = ["exp", "tree"], p = /** @type {HTMLElement} */ (Q("#mt-pexp"));
  const fail = (/** @type {any} */ e) => { p.innerHTML = h`<span class="fail">${String((e && e.message) || e)}</span>`; };
  const D = await DDA.get(f.k).catch(() => null);
  // the tree needs MS3 or higher (msntree.rs refuses fewer stages): a DDA file of MS2 scans has the list alone
  if (D && !(D.lev || []).some((/** @type {number} */ l) => l > 2)) {
    bar.remove(); /** @type {HTMLElement} */ (Q("#mt-ptree")).remove(); p.removeAttribute("role"); p.removeAttribute("aria-labelledby");
    return mount(p, f).catch(fail);
  }
  bar.setAttribute("role", "tablist"); bar.setAttribute("aria-label", T("tree.title")); bar.className = "mt-tabs";
  bar.innerHTML = h`${names.map((n, j) => raw(h`<button type="button" role="tab" id="mt-t${n}" aria-controls="mt-p${n}" aria-selected="${j === 0}" tabindex="${j === 0 ? 0 : -1}">${T(n === "exp" ? "tree.tab.exp" : "tree.tab.tree")}</button>`))}`;
  let done = false;
  const show = (/** @type {string} */ n) => {
    names.forEach(m => { const b = /** @type {HTMLElement} */ (Q("#mt-t" + m)), p = /** @type {HTMLElement} */ (Q("#mt-p" + m)); b.setAttribute("aria-selected", String(m === n)); b.tabIndex = m === n ? 0 : -1; p.hidden = m !== n; });
    if (n === "tree" && !done) { done = true; goTree(); }
  };
  bar.querySelectorAll("[role=tab]").forEach((b_, j) => {
    const b = /** @type {HTMLElement} */ (b_);
    b.onclick = () => show(names[j]);
    b.onkeydown = ev => { if (ev.key !== "ArrowRight" && ev.key !== "ArrowLeft") return; ev.preventDefault(); const n = names[(j + 1) % 2]; show(n); /** @type {HTMLElement} */ (Q("#mt-t" + n)).focus(); };
  });
  return mount(p, f).catch(fail);
}
