// Peak integration and mass spectra drawing. Split from explore.js: classic script, loaded after grafici.js.
// ------------------------------------------------------------------ integrazione dei picchi
function integ(s, a, b) {
  const xs = s.x, ys = s.y;
  let i0 = nearIdx(xs, Math.min(a, b)), i1 = nearIdx(xs, Math.max(a, b));
  if (i1 - i0 < 1) return { area: 0, height: 0, rt: xs[i0] };
  let ar = 0, mx = -Infinity, at = i0;
  for (let j = i0 + 1; j <= i1; j++) ar += (ys[j] + ys[j - 1]) / 2 * (xs[j] - xs[j - 1]) * 60;
  for (let j = i0; j <= i1; j++) if (ys[j] > mx) { mx = ys[j]; at = j; }
  const e3 = i => { let t = 0, c = 0; for (let j = i - 1; j <= i + 1; j++) if (j >= 0 && j < ys.length) { t += ys[j]; c++; } return t / c; };   // end points of the baseline: mean of 3 points
  const ya = e3(i0), yb = e3(i1), base = (ya + yb) / 2 * (xs[i1] - xs[i0]) * 60;
  const hb = ya + (yb - ya) * (xs[at] - xs[i0]) / ((xs[i1] - xs[i0]) || 1);
  return { area: Math.max(ar - base, 0), height: Math.max(mx - hb, 0), rt: xs[at] };       // conteggi x s, baseline lineare
}
// Automatic peak edges. Savitzky-Golay smoothing (quadratic polynomial fitted in a sliding window: keeps height and width of the peak, unlike a
// moving average) and its first derivative (slope of the same polynomial). The window follows the peak: about 60% of its width at half height
// (found with a coarse smoothing first), so noisy MRM traces sampled every 0.05 s and sparse TIC traces both work. The apex is where the derivative
// goes from + to -. Each edge is where the signal falls below (local baseline + 3% of the peak height, and at least 3 times the noise), or where
// the derivative changes sign again (a valley before the next peak) for a sustained stretch. Baseline = straight line between the two edges (integ()).
function sgFilter(y, m, deriv) {                         // half window m (window 2m+1), edges replicated
  const n = y.length, out = new Array(n), mm = m * (m + 1) * (2 * m + 1) / 3;
  const w = []; for (let j = -m; j <= m; j++) w.push(deriv ? j / mm : 3 * (3 * m * m + 3 * m - 1 - 5 * j * j) / ((2 * m - 1) * (2 * m + 1) * (2 * m + 3)));
  for (let i = 0; i < n; i++) { let v = 0; for (let j = -m; j <= m; j++) v += w[j + m] * y[Math.min(n - 1, Math.max(0, i + j))]; out[i] = v; }
  return out;
}
// Parameters of the automatic integration (dialog «Rilevamento dei picchi»; the defaults are the behaviour that always was). Saved in this browser.
const PK_DEF = { win: 1.2, areaNoise: 3, edgeFrac: 3, snMin: 0, noise: "mad", minHalf: 2, multi: 50, tail: 1.5, constrain: 0, nr0: 0, nr1: 0 };
const PK = (() => { let o = {}; try { o = JSON.parse(localStorage.getItem("qqq.picchi") || "{}"); } catch (e) { /* no storage */ } return { ...PK_DEF, ...o }; })();
function pkSave() { try { localStorage.setItem("qqq.picchi", JSON.stringify(PK)); } catch (e) { /* no storage */ } }
function autoEdges(s, x, ref) {
  const xs = s.x, n = xs.length, ic = nearIdx(xs, x);
  if (n < 9) return [xs[ic], xs[ic]];
  const dt = (xs[Math.min(n - 1, ic + 50)] - xs[Math.max(0, ic - 50)]) / (Math.min(n - 1, ic + 50) - Math.max(0, ic - 50) || 1) || 0.01;
  const a0 = Math.max(0, nearIdx(xs, x - 2)), a1 = Math.min(n - 1, nearIdx(xs, x + 2)), y = s.y.slice(a0, a1 + 1), N = y.length, c0 = ic - a0;
  const mCoarse = Math.max(2, Math.round(0.03 / dt / 2)), co = sgFilter(y, mCoarse, false);
  const lo = Math.max(0, nearIdx(xs, x - 0.25) - a0), hi = Math.min(N - 1, nearIdx(xs, x + 0.25) - a0);
  let m0 = lo; for (let j = lo; j <= hi; j++) if (co[j] > co[m0]) m0 = j;
  const cmin = Math.min(...co), half = cmin + (co[m0] - cmin) / 2;
  let hl = m0, hr = m0; while (hl > 0 && co[hl] > half) hl--; while (hr < N - 1 && co[hr] > half) hr++;
  const m = Math.max(PK.minHalf, Math.min(60, Math.round((hr - hl) * 0.3)));                         // half window = 30% of FWHM -> window about 60% of FWHM
  const sm = sgFilter(y, m, false), d = sgFilter(y, m, true);
  let k = Math.max(0, m0 - m); const k1 = Math.min(N - 1, m0 + m); for (let j = k; j <= k1; j++) if (sm[j] > sm[k]) k = j;   // apex of the smoothed trace
  const w0 = Math.max(0, k - Math.round(PK.win / dt)), w1 = Math.min(N - 1, k + Math.round(PK.win / dt));          // baseline window (min)
  const res = []; for (let j = w0; j <= w1; j++) res.push(Math.abs(y[j] - sm[j])); res.sort((p, q) => p - q);
  const nman = PK.noise === "manual" && PK.nr1 > PK.nr0 ? noiseSD(xs, s.y, PK.nr0, PK.nr1) : null;                    // noise from the region the student chose (no peaks there)
  const noise = nman != null && nman > 0 ? nman : Math.max(PK.noise === "rms" ? Math.sqrt(res.reduce((a, v) => a + v * v, 0) / (res.length || 1)) : 1.4826 * res[res.length >> 1], 1e-12);      // MAD of the residual = noise sigma (or its RMS)
  const low = []; for (let j = w0; j <= w1; j++) low.push(sm[j]); low.sort((p, q) => p - q);
  const base = low[Math.floor(low.length * 0.1)], H = Math.max(sm[k] - base, 0);
  const isMrm = (s.key && s.key.startsWith("m|")) || s.tr != null;
  const frac = (isMrm || (noise > 0 && H / noise > 800)) ? Math.min(PK.edgeFrac / 100, Math.max(0.005, 5 * noise / Math.max(H, 1))) : PK.edgeFrac / 100;
  const lim = base + Math.max(frac * H, PK.areaNoise * noise);
  if (PK.snMin > 0 && H < PK.snMin * noise) return null;                                  // signal/noise below the threshold: not a peak
  const need = Math.max(2, Math.round(m * PK.multi / 100)), dth = noise / Math.max(m, 1) * 0.3, maxw = Math.round(PK.tail / dt);
  const walk = dir => {
    let j = k, bad = 0;
    while (j + dir >= 0 && j + dir < N && Math.abs(j - k) < maxw) {
      const nx = j + dir; j = nx;
      if (sm[nx] <= lim) break;
      bad = (dir < 0 ? d[nx] < -dth : d[nx] > dth) ? bad + 1 : 0;                         // rising again while moving away from the apex
      if (bad >= need) { j = nx - dir * need; break; }
    }
    return j;
  };
  let e0 = xs[a0 + Math.min(walk(-1), k)], e1 = xs[a0 + Math.max(walk(1), k)];
  if (PK.constrain > 0 && ref > 0) {                                                       // «Vincola la larghezza»: the base width stays within +-constrain % of the reference peak
    const tp = xs[a0 + k], w = e1 - e0, lo = ref * (1 - PK.constrain / 100), hi = ref * (1 + PK.constrain / 100), w2 = Math.min(hi, Math.max(lo, w));
    if (w > 0 && w2 !== w) { const q = (tp - e0) / w; e0 = tp - q * w2; e1 = e0 + w2; }
  }
  return [e0, e1];
}
// trace to integrate at time x: the clicked row when stacked, otherwise the most intense one
function intSeries(p, x, py) {
  const a = p._a; if (!a || !a.sr || !a.sr.length) return null;
  if (a.stk && py != null) { const row = Math.max(0, Math.floor(a.yinv(py))); return a.sr.find(s => s.row === row) || a.sr[Math.min(a.sr.length - 1, row)]; }
  let best = null;                                       // the trace closest to the click (or the most intense one when the click position is unknown)
  a.sr.forEach(s => { const v = a.U(s, s.ys[nearIdx(s.x, x)]), d = py != null && a.Y ? Math.abs(py - a.Y(v)) : -v; if (!best || d < best.d) best = { d, s }; });
  return best && best.s;
}
// traces to integrate: the one under the click, all the visible ones, or the file chosen in the panel
function intTargets(p, x, py) {
  const a = p._a; if (!a || !a.sr || !a.sr.length) return [];
  if (p.intf === "all") return a.sr.filter(s => !isHid(p, s));
  if (p.intf) { const s = a.sr.find(q => q.key === p.intf); if (s) return [s]; }
  return [intSeries(p, x, py)].filter(Boolean);
}
function refreshIntf(p) {
  const sel = p.el.querySelector('[data-a="intf"]'); if (!sel) return;
  const on = (p.imode === "auto" || p.imode === "man") && p.type !== "chrom" && p._a && p._a.sr && p._a.sr.length > 1;
  sel.hidden = !on; if (!on) return;
  const opts = [["", I18N.t("int.target.clicked")], ["all", I18N.t("int.target.all")], ...p._a.sr.map(s => [s.key, s.name])], sig = JSON.stringify(opts);
  if (sel._sig !== sig) { sel._sig = sig; sel.innerHTML = opts.map(o => `<option value="${EH(o[0])}">${EH(o[1])}</option>`).join(""); }
  sel.value = p.intf || "";
}
// students integrate only from an XIC (or an MRM transition): the total chromatograms add up every ion, so their area belongs to no compound
function guardInt(p) {
  if (p.type !== "chrom" || p.tab === "ms2") return true;
  yesno(I18N.t("int.guard")).then(v => { if (v) openXic(null); });
  return false;
}
// integrations never overlap (same trace and file): at most two peaks touch and share the border (drop line)
const INT_EPS = 1e-6;
let HSEQ = 0;                                                             // order of the undoable actions (zoom and integrations) of the session
const iLo = it => Math.min(it.a, it.b), iHi = it => Math.max(it.a, it.b);
const intClash = (p, key, a, b, skip) => p.ints.find(i => i !== skip && i.key === key && iLo(i) < b - INT_EPS && iHi(i) > a + INT_EPS) || null;
function nearMsg(t) {                                                     // short notice next to the mouse
  let d = Q("#qnote"); if (!d) { d = document.createElement("div"); d.id = "qnote"; d.style.cssText = "position:fixed;z-index:20000;max-width:300px;padding:6px 10px;border-radius:6px;background:#333;color:#fff;font:13px system-ui;box-shadow:0 2px 8px rgba(0,0,0,.35);pointer-events:none"; document.body.appendChild(d); }
  const m = window._mxy || { x: 200, y: 200 }; d.textContent = t; d.hidden = false;
  d.style.left = Math.max(6, Math.min(innerWidth - 310, m.x + 14)) + "px"; d.style.top = Math.max(6, Math.min(innerHeight - 60, m.y + 18)) + "px";
  clearTimeout(d._t); d._t = setTimeout(() => { d.hidden = true; }, 3200);
}
document.addEventListener("mousemove", e => { window._mxy = { x: e.clientX, y: e.clientY }; }, true);
const intSnap = p => JSON.stringify(p.ints);
function pushIh(p, snap) { const h = p.ih = p.ih || []; h.push({ n: ++HSEQ, s: snap || intSnap(p) }); if (h.length > 20) h.shift(); }
function undoInt(p) { const e = p.ih && p.ih.pop(); if (!e) return; p.ints = JSON.parse(e.s); p.isel = null; draw(p); uiSave(); }
function delInts(p, list) { pushIh(p); p.ints = p.ints.filter(i => !list.includes(i)); if (list.some(i => i.id === p.isel)) p.isel = null; draw(p); uiSave(); }
function addInt(p, s, a, b, mirror = true) {
  const lo = Math.min(a, b), hi = Math.max(a, b);
  if (p.type === "mrm") { if (p.ints.some(i => i.key === s.key)) pushIh(p); p.ints = p.ints.filter(i => i.key !== s.key); }       // calibration: one peak per transition and file, a new window replaces the old one
  else {
    if (p.ints.some(i => i.key === s.key && Math.abs(i.a - lo) < 1e-6 && Math.abs(i.b - hi) < 1e-6)) return false;     // same peak clicked twice
    if (intClash(p, s.key, lo, hi)) { nearMsg(I18N.t("int.clash")); return false; }
  }
  if (p.type !== "mrm") pushIh(p);
  const f = E.files[s.k];
  p.ints.push({ id: E.seq++, k: s.k, key: s.key, a: lo, b: hi, name: s.name, ion: s.ion || s.name, file: f ? f.label : "", time: f ? f.time : null, panel: p.title });
  draw(p);
  // MRM: the same window goes to the other transition panels (quantifier and qualifier), where it can still be moved on its own
  if (p.type === "mrm" && mirror) tabPanels().filter(q => q !== p && q.type === "mrm" && q._a && q._a.sr).forEach(q => { const s2 = q._a.sr.find(x => x.k === s.k && x.ion !== s.ion); if (s2) addInt(q, s2, a, b, false); });
  return true;
}
// automatic integration at time x: refused on an integrated peak; next to one it stops at its border (the two peaks touch)
function autoInt(p, s, x) {
  const same = p.ints.filter(i => i.key === s.key).map(i => iHi(i) - iLo(i)).sort((u, v) => u - v);
  const ed = autoEdges(s, x, same.length ? same[same.length >> 1] : 0); if (!ed) { nearMsg(I18N.t("int.belowSn")); return; }
  let [l, r] = ed;
  if (p.type !== "mrm") {
    const c = p.ints.filter(i => i.key === s.key);
    if (c.some(i => iLo(i) + INT_EPS < x && x < iHi(i) - INT_EPS)) { nearMsg(I18N.t("int.clash")); return; }
    c.forEach(i => { if (iHi(i) <= x + INT_EPS) l = Math.max(l, iHi(i)); if (iLo(i) >= x - INT_EPS) r = Math.min(r, iLo(i)); });
    if (r - l < 1e-6) return;
  }
  addInt(p, s, l, r);
}
// integration whose filled area contains the pixel (used to select it with a click)
function intAt(p, px, py) {
  const a = p._a; if (!a || !a.sr || a.stk || a.cas || !a.Y || p.type === "spec" || p.type === "map") return null;
  const x = a.x0 + (px - M.l) / (a.W - M.l - M.r) * (a.x1 - a.x0);
  for (const it of [...p.ints].reverse()) {
    if (x < iLo(it) || x > iHi(it)) continue; const s = a.sr.find(q => q.key === it.key); if (!s) continue;
    if (py >= a.Y(a.U(s, s.ys[nearIdx(s.x, x)])) - 4 && py <= a.H - M.b + 3) return it;
  }
  return null;
}
// dialog of the automatic-integration parameters (applies to every panel; the defaults reproduce the behaviour that always was)
function peakParams() {
  const f = (k, hint = true) => `<label style="display:block;margin:3px 0">${I18N.t(`pk.${k}`)} <input data-pk="${k}" value="${PK[k]}" style="width:70px"> <span class="muted sm">${hint ? I18N.t(`pk.${k}.h`) : ""}</span></label>`;
  big(I18N.t("pk.title"), `<div class="muted sm" style="margin-bottom:6px">${I18N.t("pk.intro")}</div>
    ${f("win")}
    ${f("areaNoise")}
    ${f("edgeFrac")}
    ${f("snMin")}
    <label style="display:block;margin:3px 0">${I18N.t("pk.noise")} <select data-pk="noise"><option value="mad"${PK.noise === "mad" ? " selected" : ""}>${I18N.t("pk.noise.mad")}</option><option value="rms"${PK.noise === "rms" ? " selected" : ""}>${I18N.t("pk.noise.rms")}</option><option value="manual"${PK.noise === "manual" ? " selected" : ""}>${I18N.t("pk.noise.manual")}</option></select></label>
    ${f("nr0")}
    ${f("nr1", false)}
    ${f("constrain")}
    ${f("minHalf")}
    ${f("multi")}
    ${f("tail", false)}
    <div style="margin-top:8px"><button id="pk-def" type="button">${I18N.t("pk.default")}</button> <button id="pk-ok" type="button" class="go">${I18N.t("pk.apply")}</button></div>`, () => {
    const rdv = () => document.querySelectorAll("#bigbody [data-pk]").forEach(i => { i.value = PK[i.dataset.pk]; });
    Q("#pk-def").onclick = () => { Object.assign(PK, PK_DEF); pkSave(); rdv(); };
    Q("#pk-ok").onclick = () => {
      document.querySelectorAll("#bigbody [data-pk]").forEach(i => { const k = i.dataset.pk; if (k === "noise") PK[k] = i.value; else { const v = numIt(i.value); if (v >= 0) PK[k] = v; } });
      PK.minHalf = Math.max(1, Math.round(PK.minHalf)); pkSave(); Q("#bigdlg").close();
    };
  });
}
function intMenuItems(p, x, near, py) {
  const items = [], a = p._a;
  const nearInt = p.ints.find(it => x >= iLo(it) && x <= iHi(it));
  if (nearInt) items.push({ label: I18N.t("int.menu.del"), icon: IC_DEL_INT, fn: () => delInts(p, [nearInt]) });
  if (p.ints.length) items.push({ label: I18N.t("int.menu.delAll"), icon: IC_DEL_INT, fn: () => delInts(p, [...p.ints]) });
  if (nearInt || p.ints.length) items.push("-");
  items.push({ label: I18N.t("int.menu.params"), fn: peakParams });
  items.push({ label: I18N.t("int.menu.auto"), icon: IC_AUTO, fn: () => { if (!guardInt(p)) return; intTargets(p, x, py).forEach(s => autoInt(p, s, x)); } });
  if (p.sel && p.sel[1] > p.sel[0]) items.push({ label: I18N.t("int.menu.zoomSel"), icon: IC_ZOOM, fn: () => { const [s0, s1] = p.sel; const w = (s1 - s0) * 0.1; zoomTo(p, s0 - w, s1 + w); } });
  return items;
}
function allInts() {
  return E.panels.flatMap(p => p.ints.map(i => { const f = E.files[i.k], std = f && f.type === "standard" && f.conc != null; return { ...i, _p: p, _it: i, panel: ptitle(p.title), conc: std ? f.conc : null, cunit: std ? f.cunit || "" : "" }; }));      // the concentration is only for standards: the calibration line is made by the student in Excel
}
// optional columns of the integration table (off by default): S/N and the figures of the peak (width at half height, plates, tailing)
const INTOPT = { sn: false, par: false };
function intExtra(r) {
  const p = r._p, it = r._it, s = p && p._a && p._a.sr && p._a.sr.find(q => q.key === it.key);
  const pk = s ? chromPeak(s.x, s.y, r.a, r.b) : null, sd = s && it.noise ? noiseSD(s.x, s.y, it.noise[0], it.noise[1]) : null;
  return { sn: pk && sd ? pk.height / sd : null, noise: !!it.noise, w50: pk ? pk.w50 : null, N: pk ? pk.N : null, T: pk ? pk.tailing : null };
}
function showInts() {
  const rows = allInts().map(r => ({ ...r, ...intExtra(r) }));
  const fx = (v, d) => v == null || !isFinite(v) ? "" : String(+v.toFixed(d));
  const head = [INT_HEADS[0], INT_HEADS[1], INT_HEADS[2], I18N.t("int.tbl.time"), I18N.t("int.tbl.conc"), I18N.t("int.tbl.rt0"), I18N.t("int.tbl.rt1"), I18N.t("int.tbl.apex"), I18N.t("int.tbl.area"), I18N.t("int.tbl.height")], num = [0, 0, 0, 1, 1, 1, 1, 1, 1, 1];
  const th = (t, n) => `<th${n ? ' class="num"' : ""}>${t}</th>`;
  const hx = (INTOPT.sn ? th("S/N", 1) : "") + (INTOPT.par ? th("w<sub>1/2</sub> (min)", 1) + th(I18N.t("int.tbl.plates"), 1) + th(I18N.t("int.tbl.tail"), 1) : "");
  big(I18N.t("int.tbl.title"), rows.length ? `<div class="bar"><button id="ig-xlsx">${IC_DL}Excel</button>
    <label title="${I18N.t("int.tbl.sn.title")}"><input type="checkbox" id="ig-sn" ${INTOPT.sn ? "checked" : ""}> S/N</label>
    <label title="${I18N.t("int.tbl.par.title")}"><input type="checkbox" id="ig-par" ${INTOPT.par ? "checked" : ""}> ${I18N.t("int.tbl.par")}</label></div>
    <table><tr>${head.map((t, i) => th(t, num[i])).join("")}${hx}<th>${I18N.t("int.tbl.ibk")}</th><th></th></tr>` +
    rows.map((r, i) => `<tr><td>${EH(r.panel)}</td><td>${EH(r.ion)}</td><td>${EH(r.file)}</td><td class="num">${r.time ?? ""}</td><td class="num">${r.conc != null ? r.conc + " " + EH(r.cunit) : ""}</td><td class="num">${r.a.toFixed(2)}</td><td class="num">${r.b.toFixed(2)}</td><td class="num">${(r.rt ?? 0).toFixed(2)}</td><td class="num" title="${fmtFull(r.area ?? 0)}">${fmtA(r.area ?? 0)}</td><td class="num">${fmt(r.height ?? 0)}</td>` +
      (INTOPT.sn ? `<td class="num">${r.sn != null ? fx(r.sn, 1) : `<button data-nz="${i}" title="${I18N.t("int.tbl.pickNoise.title")}">${I18N.t("int.tbl.pickNoise")}</button>`}</td>` : "") +
      (INTOPT.par ? `<td class="num">${fx(r.w50, 3)}</td><td class="num">${r.N != null ? Math.round(r.N) : ""}</td><td class="num">${fx(r.T, 2)}</td>` : "") + `<td>${EH(r.ibk || I18N.t("ibk.no"))}</td><td>${intClash(r._p, r.key, r.a, r.b, r._it) ? `<span class="warn" title="${I18N.t("int.tbl.overlap.title")}">${I18N.t("int.tbl.overlap")}</span> ` : ""}<button data-del="${i}" title="${I18N.t("int.tbl.del.title")}">×</button></td></tr>`).join("") +
    `</table><div class="muted sm">${I18N.t("int.tbl.note")}</div>`
    : `<div class="muted">${I18N.t("int.tbl.none")}</div>`, () => {
    if (!rows.length) return;
    Q("#ig-sn").onchange = e => { INTOPT.sn = e.target.checked; showInts(); };
    Q("#ig-par").onchange = e => { INTOPT.par = e.target.checked; showInts(); };
    Q("#bigbody").querySelectorAll("button[data-del]").forEach(b => b.onclick = () => { const r = rows[+b.dataset.del]; delInts(r._p, [r._it]); showInts(); });
    Q("#bigbody").querySelectorAll("button[data-nz]").forEach(b => b.onclick = () => {
      const r = rows[+b.dataset.nz]; r._p._pickNoise = r._it; Q("#bigdlg").close(); reveal(r._p);
      stepMsg(r._p, I18N.t("int.pickNoise.msg"), true);
    });
    Q("#ig-xlsx").onclick = () => dlx(I18N.t("int.xlsx.file"), [{ name: I18N.t("int.xlsx.sheet"), head: [...INT_HEADS, ...(INTOPT.sn ? [I18N.t("int.xlsx.sn")] : []), ...(INTOPT.par ? ["w1/2 (min)", I18N.t("int.xlsx.plates"), I18N.t("int.xlsx.tail")] : [])],
      rows: rows.map(r => [...INT_COLS.map(c => r[c]), ...(INTOPT.sn ? [r.sn] : []), ...(INTOPT.par ? [r.w50, r.N, r.T] : [])]), widths: [16, 18, 22, 14, 16, 8, 14, 14, 14, 18, 14, 34, 18, 12, 14, 14] }]);
  });
}

// ------------------------------------------------------------------ spettri
// The window of a spectrum holds no scan of the kind shown (DDA, MS1/MS2 filters): the window moves to the nearest scan of that kind (on a tie the earlier one)
// and the cursor of the chromatogram it follows snaps to it. False only when the file has no such scan at all.
async function snapNearest(p, f) {
  try {
    const pr = p.prec != null && p.prec !== "" ? `&precursor=${p.prec}` : "", ft = p.filt ? `&filter=${encodeURIComponent(p.filt)}` : "";
    const n = await J(`api/nearest_scan?k=${f.k}&rt=${(p.r0 + p.r1) / 2}&level=${p.level || 1}${pr}${ft}`);
    if (!n || n.rt == null) return false;
    p.r0 = n.rt - NEAR; p.r1 = n.rt + NEAR;
    const c = E.panels.find(q => q.id === p.link); if (c) { c.cur = n.rt; cursorLine(c); }
    return true;
  } catch (e) { return false; }
}
async function drawSpec(p, snapped) {
  if (p.iso && p._isoKey != null && p._isoKey !== JSON.stringify(p.zoom)) { p.iso = null; p._isoKey = null; }      // the student moved the zoom: the isotope simulation switches off
  const tok = p._tok = (p._tok || 0) + 1;                // "last one wins": a slower, older request never paints over a newer one
  const say = t => { const { g, W, H: HF } = setup(p.cv); g.clearRect(0, 0, W, HF); g.fillStyle = css("--muted"); g.fillText(t, M.l, 30); p.leg.innerHTML = ""; if (p.leg2) p.leg2.innerHTML = ""; p._a = null; };
  const isDda = p.dda != null && !!window.DDA;                // MS2 panel of a DDA trio: shows one MS2 scan of the file (dda.js), whatever file is selected in the toolbar
  const targetK = p.link ? p.k : (E.browse && (!p.tab || p.tab === E.tab) ? E.cur : p.k);
  const base = isDda ? [E.files[p.k]] : p.all ? shownFor(p) : [E.files[targetK]];
  const files = scanFiles(base.filter(Boolean));
  if (!files.length) return say(I18N.t("spec.isMrm"));
  if (isDda) { const m = DDA.waiting(p); if (m) return say(m); }
  else if (p.r0 == null) return say(I18N.t("spec.pickRange"));
  const sref = window.SUBREF ? SUBREF.eff(p) : null;          // the reference of the session (or the file chosen in «fondo»): same window of time, factor of the session
  const bgOf = f => p.bg === "w" ? (p.bw0 != null && p.bw1 != null && p.bw1 > p.bw0 ? [f.k, p.bw0, p.bw1] : null)
    : (sref && sref.k !== f.k ? [sref.k, p.r0, p.r1, sref.factor, 1] : null);
  // scan-by-scan walk (arrows): one file, no background subtraction -> the scan comes from the cache of neighbouring scans, not from a window request
  const one = !isDda && files.length === 1 && p.si != null && !bgOf(files[0]) && !p._exp;
  if (one && p.lock && p.lock.fresh) await scFetch(files[0].k, p.level, prk(p.prec, p.filt), p.si - SC_AHEAD, p.si + SC_AHEAD).catch(() => {});   // the axes of the walk look at +-20 scans
  const rel = specRel(p);                                // y axis in % of the highest peak of each scan (default for MS2) or in cps
  const hrp = !!window.HR && HR.anyHr(files, p.level), DECP = p.dec ?? (window.HR ? HR.dec(files, p.level) : 1);       // high resolution: the m/z are the centroids of the file, written with the decimals of the profile
  const relScale = d => { let mx = 1e-9; for (const v of d.y) if (v > mx) mx = v; const k = 100 / mx; return { ...d, y0: d.y, y: d.y.map(v => v * k), py: d.py ? d.py.map(v => v * k) : d.py }; };
  const reqs = [];
  files.forEach(f => {
    const isHr = !!window.HR && HR.isHr(f);
    const split = p.level === 2 && !isHr && f.precursors && f.precursors.length > 1 && (p.prec == null || p.prec === "");
    if (split) {
      const pset = [...new Set(f.precursors)].sort((a, b) => a - b);
      pset.forEach((pr, i) => reqs.push({ f, prec: pr, multi: true, idx: i }));
    } else {
      reqs.push({ f, prec: p.prec, multi: false });
    }
  });
  const data = await Promise.all(reqs.map(q => (isDda ? DDA.scanData(p) : one ? scData(p, q.f.k) : getSpec(q.f.k, p.r0, p.r1, p.level, prk(q.prec, p.filt), bgOf(q.f))).then(d => ({ f: q.f, prec: q.prec, multi: q.multi, idx: q.idx, d: rel ? relScale(d) : d }))));
  if (tok !== p._tok) return false;                      // a newer request is on its way: this one is dropped
  if (p.iso && p.iso.only && hrp && window.QQQRef) data.forEach(x => { try { x.d = HR.isoAsSpectrum(p, x.f, x.d); } catch (e) { /* the observed spectrum stays */ } });      // «Sostituisci»: the simulation takes the place of the spectrum
  const mzs = data.flatMap(x => x.d.mz);
  if (!mzs.length) {
    if (!snapped && !isDda && files.length === 1 && p.r0 != null && await snapNearest(p, files[0])) return drawSpec(p, true);      // the cursor fell between two scans of this kind: the nearest one is shown
    return say(I18N.t("spec.noScan"));
  }
  // everything is known: only now the canvas is cleared and drawn again, in one go (until then the previous spectrum stays on the screen)
  const { g, W, H: HF } = setup(p.cv);
  const H = HF;
  const lineOf = x => x.d.pmz && !p.sticks;                  // a profile file is drawn as a line (the peaks are drawn as bars only if the student asks)
  const inRange = (lo, hi) => Math.max(1e-9, ...data.flatMap(x => { let m = 0; const [mz, y] = lineOf(x) ? [x.d.pmz, x.d.py] : [x.d.mz, x.d.y]; y.forEach((v, i) => { if (mz[i] >= lo && mz[i] <= hi && v > m) m = v; }); return [m]; }));
  // the m/z axis is always the widest range of all the scans of the file(s) (level / precursor of the panel): it only changes with a zoom, another file or another precursor
  const wide = files.reduce((r, f) => { const a = p.level === 2 ? f.mz2_min : f.mz_min, b = p.level === 2 ? f.mz2_max : f.mz_max; return a != null && b != null ? [Math.min(r[0], a), Math.max(r[1], b)] : r; }, [Infinity, -Infinity]);
  const full = wide[0] < wide[1] ? [Math.max(0, wide[0] - 1), wide[1] + 1] : [Math.min(...mzs) - 2, Math.max(...mzs) + 2];
  const x0 = p.zoom ? p.zoom[0] : full[0], x1 = p.zoom ? p.zoom[1] : full[1];
  let ymax = inRange(x0, x1) * 1.12;
  const yfull = ymax;                                     // automatic top of the intensity axis (y zoom is relative to it)
  const lk = p.zoomY ? null : lockSync(p, files[0].k, x0, x1, ymax, inRange, one);   // the lock freezes ONLY the intensity axis while walking over the scans
  if (lk) ymax = lk.ymax;
  let ymin = 0;
  if (p.zoomY) { ymin = p.zoomY[0]; ymax = p.zoomY[1]; }   // zoom of the intensity axis (drag on the numbers at the left of the axis, or box)
  const { X, Y } = axes(g, W, H, x0, x1, ymax, fmt, { xt: "m/z", yt: rel ? I18N.t("axis.relPct") : YT_I, xn: Math.max(10, Math.round((W - M.l - M.r) / (48 * fz()))), ymin });
  const ph = H - M.t - M.b;
  p._a = { x0, x1, X, Y, W, H, data, ymax, ymin, yfull: [0, yfull], yinv: py => ymin + (H - M.b - py) / ph * (ymax - ymin), full };
  p._a.hrp = hrp; p._a.dec = DECP;
  p._a.snap = px => {                                      // the peak nearest to a pixel: the top bin around it and the centroid (intensity weighted) of that peak
    if (hrp) {                                             // high resolution: the nearest centroid itself (never an average of neighbours); on a tie the taller one
      let b = null;
      data.forEach(x => x.d.mz.forEach((m, j) => { if (m < x0 || m > x1 || x.d.y[j] <= 0) return; const dd = Math.abs(X(m) - px); if (dd < 14 && (!b || dd < b.dd - 0.5 || (Math.abs(dd - b.dd) <= 0.5 && x.d.y[j] > b.y))) b = { dd, m, y: x.d.y[j], y0: (x.d.y0 || x.d.y)[j], f: x.f, multi: x.multi, prec: x.prec }; }));
      return b && { m: b.m, y: b.y, y0: b.y0, f: b.f, multi: b.multi, prec: b.prec };
    }
    let best = null;
    data.forEach(x => x.d.mz.forEach((m, j) => { if (m < x0 || m > x1 || x.d.y[j] <= 0) return; const dd = Math.abs(X(m) - px); if (dd < 14 && (!best || dd < best.dd)) best = { dd, m, j, x }; }));
    if (!best) return null;
    const { mz, y } = best.x.d; let jb = best.j;
    for (let j = 0; j < mz.length; j++) if (Math.abs(mz[j] - best.m) <= 0.35 && y[j] > y[jb]) jb = j;
    let sw = 0, sm = 0; for (let j = 0; j < mz.length; j++) if (Math.abs(mz[j] - mz[jb]) <= 0.5 && y[j] > 0) { sw += y[j]; sm += mz[j] * y[j]; }
    return { m: sw > 0 ? sm / sw : mz[jb], y: y[jb], y0: (best.x.d.y0 || y)[jb], f: best.x.f, multi: best.x.multi, prec: best.x.prec };
  };
  const cont = hrp && window.LISTE ? LISTE.forSpec(data[0].d.mz, data[0].d.y, files[0] && files[0].polarity, UIP.hrPpm) : null;       // known contaminants: only in the box that opens on a peak (high resolution only)
  p._a.cont = cont;
  const cst = x => { if (!cont) return () => null; const c = x.cs = x.cs || new Map(); return j => { let r = c.get(j); if (r === undefined) { r = cont.state(x.d.mz[j]); c.set(j, r); } return r; }; };     // contaminant state of a peak (HR only): compatible entries, hidden or not
  const hidOf = x => { const f = cst(x), t1 = 0.01 * Math.max(...x.d.y, 1e-9); return j => !p.showHid && x.d.y[j] >= t1 && !!(f(j) && f(j).hid); };
  p._a.hov = px => {                                       // peak nearest to the mouse
    const b = p._a.snap(px); if (!b) return null;
    const rf = p.meas && p.meas.ref != null ? `<div>Δ<i>m/z</i> <b>${Math.abs(b.m - p.meas.ref).toFixed(hrp ? DECP : 1)}</b> <span class="sm">da ${p.meas.ref.toFixed(hrp ? DECP : 1)}</span>${hrp ? ` <span class="sm">(${((b.m - p.meas.ref) * 1000).toFixed(1)} mDa)</span>` : ""}</div>` : "";
    return { px: X(b.m), html: I18N.t("tip.peak", { mz: b.m.toFixed(hrp ? DECP : 2), y: rel ? b.y.toFixed(1) + " %" : fmt(b.y), abs: rel ? ` <span class="sm">(${fmt(b.y0)} cps)</span>` : "" }) + `${data.length > 1 ? `<div class="sm">${EH(b.f.label)}${b.multi ? ` (${I18N.t("ctl.prec.one", { v: b.prec })})` : ""}</div>` : ""}${rf}${cont ? cont.tip(b.m) : ""}` };
  };
  g.save(); g.beginPath(); g.rect(M.l, M.t - 1, W - M.l - M.r, H - M.t - M.b + 1); g.clip();     // bars taller than the (magnified) graph end at the top edge
  data.forEach(x => {
    const color = x.multi ? PAL[(x.idx + 3) % PAL.length] : x.f.color;
    g.strokeStyle = color; g.lineWidth = 1.4 + LWX();
    if (x.multi) g.globalAlpha = 0.55; else g.globalAlpha = 1.0;
    g.beginPath();
    if (lineOf(x)) {                                                     // profile: continuous line with a light fill under it
      const pm = x.d.pmz, py = x.d.py; let on = false;
      let curX = -999, minY, maxY, firstY, lastY, minFirst = true;
      const flush = (cx) => { if (curX === -999) return; if (!on) { g.moveTo(cx, firstY); on = true; } else g.lineTo(cx, firstY); if (minFirst) { g.lineTo(cx, minY); g.lineTo(cx, maxY); } else { g.lineTo(cx, maxY); g.lineTo(cx, minY); } g.lineTo(cx, lastY); };
      pm.forEach((m, j) => { if (m < x0 - 0.5 || m > x1 + 0.5) return; const px = Math.round(X(m)), pvy = Y(py[j]);
        if (px !== curX) { flush(curX); curX = px; minY = maxY = firstY = lastY = pvy; minFirst = true; }
        else { if (pvy < minY) { minY = pvy; if (maxY === firstY) minFirst = true; } if (pvy > maxY) { maxY = pvy; if (minY === firstY) minFirst = false; } lastY = pvy; }
      });
      flush(curX);
      g.stroke(); if (on) { g.save(); g.lineTo(X(Math.min(x1 + 0.5, pm[pm.length - 1])), Y(0)); g.lineTo(X(Math.max(x0 - 0.5, pm[0])), Y(0)); g.closePath(); g.globalAlpha = 0.12; g.fillStyle = color; g.fill(); g.restore(); }
      g.beginPath();
    } else { const hid = cont ? hidOf(x) : null, dim = []; x.d.mz.forEach((m, j) => { if (m < x0 || m > x1) return; if (hid && hid(j)) { dim.push(j); return; } g.moveTo(X(m), Y(0)); g.lineTo(X(m), Y(x.d.y[j])); }); g.stroke();
      if (dim.length) { g.save(); g.strokeStyle = css("--line"); g.globalAlpha = 1; g.beginPath(); dim.forEach(j => { g.moveTo(X(x.d.mz[j]), Y(0)); g.lineTo(X(x.d.mz[j]), Y(x.d.y[j])); }); g.stroke(); g.restore(); } }
    g.fillStyle = color;                                          // a small triangle on every cut bar
    x.d.mz.forEach((m, j) => { if (m < x0 || m > x1 || x.d.y[j] <= ymax) return; g.beginPath(); g.moveTo(X(m) - 4, M.t + 7); g.lineTo(X(m) + 4, M.t + 7); g.lineTo(X(m), M.t); g.closePath(); g.fill(); });
  });
  g.restore();
  const d0 = data[0].d, pk = [];
  const top0 = Math.max(...d0.y, 1e-9), thr = (p.thr ?? 5) / 100 * top0, NL = p.nlab ?? 10, DEC = DECP;       // labels: only the peaks above the threshold, at most NL
  const hid0 = cont ? hidOf(data[0]) : null;
  d0.mz.forEach((m, j) => { if (m >= x0 && m <= x1 && d0.y[j] >= thr && d0.y[j] <= ymax && !(hid0 && hid0(j))) pk.push([m, d0.y[j]]); });
  pk.sort((a, b) => b[1] - a[1]); const used = [];
  g.fillStyle = css("--ink"); g.textAlign = "center"; g.font = fpx(12);
  const lbls = []; p._a.lbls = lbls;                      // clickable labels: hover draws a small box, right click opens the menu
  for (const [m, y] of pk.slice(0, 40)) { const px = X(m); if (used.some(u => Math.abs(u - px) < 26) || used.length >= NL) continue; used.push(px); const t = m.toFixed(DEC), w = g.measureText(t).width, fo = p.fm && p.fm.find(z => Math.abs(z.m - m) < 1e-4), up = fo ? 13 * fz() : 0;   // a formula applied to the peak (candidate, italic) sits between the m/z and the bar
    g.fillText(t, px, Y(y) - 4 - up);
    if (fo) { g.save(); g.font = fpx(11, "italic "); g.fillStyle = css("--accent"); g.fillText(subText(fo.text), px, Y(y) - 4); g.restore(); }
    lbls.push({ m, x: px - w / 2 - 3, y: Y(y) - 4 - up - 12 * fz(), w: w + 6, h: 3 + 12 * fz(), tip: I18N.t("tip.peakLabel", { mz: m.toFixed(DEC) }) + (cont ? cont.tip(m) : "") }); }
  g.font = fpx(12);
  contMarks(p, g, X, Y, W, data, cst, hidOf, x0, x1, ymax);
  drawMeas(p, g, X, Y, W);                                 // ruler: reference and measured differences (before the annotations: a label hides the dashed line under it)
  for (const a of p.anns) {                                        // the point is the top of the nearest observed peak
    let ay = 0, bd = 0.7; d0.mz.forEach((m, j) => { const dd = Math.abs(m - a.x); if (dd < bd) { bd = dd; ay = d0.y[j]; } });
    drawAnn(p, g, a, X(a.x), Math.max(M.t + 1, Math.min(H - M.b, Y(Math.min(ay, ymax)))), W, 28);
  }
  if (window.DDA) DDA.decorate(p, g, X, Y, W, { x0, x1, ymax, d0, data, files });       // DDA: flags of the precursors, isolation band, line of the precursor
  // theoretical isotope pattern of a formula chosen by the student (red circles), aligned on the nearest observed peak
  let isoNote = "";
  if (p.iso && !p.iso.only && window.QQQRef) {
    try {
      if (hrp) isoNote = HR.drawIso(p, g, X, Y, d0, ymax, W, files[0]);          // high resolution: fine structure, error in ppm (hr.js)
      else {
      const ion = QQQRef.ionCounts(p.iso.formula, p.iso.ad), pat = QQQRef.isoPattern(ion.n, ion.z), top = pat.reduce((a, b) => (b.rel > a.rel ? b : a));
      let obs = null; d0.mz.forEach((m, j) => { if (Math.abs(m - top.mz) <= 1.5 && d0.y[j] > 0 && (!obs || d0.y[j] > obs.y)) obs = { m, y: d0.y[j] }; });
      if (obs) {                                         // profile data: several points per peak -> use the centroid of the peak (+-0.5 Da)
        let sw = 0, sm = 0; d0.mz.forEach((m, j) => { if (Math.abs(m - obs.m) <= 0.5 && d0.y[j] > 0) { sw += d0.y[j]; sm += m * d0.y[j]; } });
        if (sw > 0) obs.c = sm / sw;
      }
      const shift = obs ? (obs.c ?? obs.m) - top.mz : 0, h = obs ? obs.y : ymax / 1.12 * 0.9;
      g.save(); g.strokeStyle = g.fillStyle = "#d62728"; g.lineWidth = 1.5;
      for (const r of pat) {
        const px = X(r.mz + shift) + 3, py = Y(h * r.rel / 100); if (px < M.l || px > W - M.r) continue;
        g.setLineDash([3, 2]); g.beginPath(); g.moveTo(px, Y(0)); g.lineTo(px, py); g.stroke(); g.setLineDash([]);
        g.beginPath(); g.arc(px, py, 3.5, 0, 7); g.stroke();
        if (r.rel >= 1) { g.font = fpx(12); g.textAlign = "left"; g.fillText((r.off ? "M+" + r.off : "M") + " " + (r.rel < 10 ? r.rel.toFixed(1) : Math.round(r.rel)) + "%", px + 5, py - 4); }
      }
      g.restore(); g.font = fpx(12);
      isoNote = `<span><i style="background:#d62728"></i>${I18N.t("iso.note", { formula: fmtFormula(p.iso.formula), adduct: fmtAdduct(p.iso.ad), align: obs ? I18N.t("iso.align", { mz: (obs.c ?? obs.m).toFixed(mzd(p, p.level)), shift: Math.abs(shift) >= 0.05 ? I18N.t("iso.shift", { v: (shift > 0 ? "+" : "") + shift.toFixed(1) }) : "" }) : I18N.t("iso.noPeak", { mz: top.mz.toFixed(mzd(p)) }) })}</span>`;
      }
    } catch (e) { isoNote = `<span class="sm">profilo isotopico: ${EH(e.message)}</span>`; }
  }
  p.leg.innerHTML = `<span style="font-variant-numeric:tabular-nums">${scanLine(p, data, d0)}${bgOf(files[0]) ? I18N.t("spec.bgSubtracted") : ""}</span>`;       // on the row of the x axis title, to the right
  if (isDda) DDA.caption(p);                                // scan, parent, activation, NCE, isolation
  p.leg.title = p.leg.textContent;
  if (window.SUBREF && p.bg !== "w") SUBREF.chip(p);
  const multiFiles = scanFiles(tabFiles(p.tab)).length > 1;
  const showLabel = data.length > 1 || multiFiles;
  if (p.leg2) p.leg2.innerHTML = (showLabel ? data.map(x => {
    const lab = x.f.label.length > 25 ? x.f.label.slice(0, 24) + "…" : x.f.label;
    const color = x.multi ? PAL[(x.idx + 3) % PAL.length] : x.f.color;
    return `<span><i style="background:${color}"></i>${EH(lab)}${legPol(x.f)}${x.multi ? ` (prec. ${x.prec})` : ""}</span>`;
  }).join("") : "") + isoNote;
  contLegend(p);
}

// Known contaminants on a high-resolution spectrum: a small diamond over the peaks whose m/z is COMPATIBLE with an entry of the lists (a sign, not a verdict).
// A peak whose entries are all excluded is dimmed (drawn in draw()) and counted; "Show" draws it again with an empty diamond. Click on the diamond: exclude / include.
function contMarks(p, g, X, Y, W, data, cst, hidOf, x0, x1, ymax) {
  const a = p._a; a.cmarks = []; a.chid = 0; a.cvis = 0;
  if (!a.cont) return;
  const d = data[0].d, f = cst(data[0]), top = Math.max(...d.y, 1e-9), list = [];
  d.mz.forEach((m, j) => { if (d.y[j] < 0.01 * top) return; const s = f(j); if (!s || !s.all.length) return; if (s.hid) a.chid++; else a.cvis++; if (m >= x0 && m <= x1 && d.y[j] <= ymax && (!s.hid || p.showHid)) list.push({ m, j, y: d.y[j], s }); });
  list.sort((u, v) => v.y - u.y);
  g.save(); g.lineWidth = 1.4;
  for (const r of list.slice(0, 80)) {
    const cx = X(r.m), cy = Math.max(M.t + 6, Y(r.y) - 24), off = r.s.hid, sz = 5;
    g.beginPath(); g.moveTo(cx, cy - sz); g.lineTo(cx + sz, cy); g.lineTo(cx, cy + sz); g.lineTo(cx - sz, cy); g.closePath();
    g.strokeStyle = css("--ink"); g.fillStyle = off ? css("--panel") : css("--warn"); g.fill(); g.stroke();
    a.cmarks.push({ x: cx, y: cy, m: r.m, s: r.s });
  }
  g.restore();
}
function contMenu(p, e, mk) {
  const L = window.LISTE, all = mk.s.all, seen = new Set(), items = [];
  const ms = all.filter(x => { const k = x.list + ":" + x.id + ":" + (x.series ? x.series.n : ""); if (seen.has(k)) return false; seen.add(k); return true; }).slice(0, 4);
  ms.forEach((x, i) => {
    if (i) items.push("-");
    items.push({ label: x.name + (x.adduct ? " " + x.adduct : ""), dim: true });
    if (L.isExcluded(x)) {
      const ex = L.exclItems().filter(q => q.key === L.keyFor(q.kind, x));
      ex.forEach(q => items.push({ label: I18N.t(q.kind === "entry" ? "cont.in.one" : q.kind === "cls" ? "cont.in.cls" : "cont.in.list", { n: q.kind === "entry" ? x.name : q.kind === "cls" ? (x.cls || "") : x.listName }), fn: () => L.exclSet(q.kind, q.key, false) }));
    } else {
      items.push({ label: I18N.t("cont.ex.one"), fn: () => L.exclSet("entry", L.keyFor("entry", x), true) });
      if (x.clsKey) items.push({ label: I18N.t("cont.ex.cls", { n: x.cls || x.clsKey }), fn: () => L.exclSet("cls", L.keyFor("cls", x), true) });
      if (!x.listBuiltin) items.push({ label: I18N.t("cont.ex.list", { n: x.listName }), fn: () => L.exclSet("list", x.list, true) });
    }
  });
  menu(e, items);
}
// legend: button (keyboard) listing the marked peaks, and the counter of the hidden ones with "Show"
function contLegend(p) {
  const a = p._a; if (!a || !a.cont || !p.leg2) return;
  const n = a.chid || 0, c = a.cmarks.length;
  let h = "";
  if (a.cvis || c) h += `<button type="button" class="cont-lb" aria-label="${EH(I18N.t("cont.lb.aria", { n: a.cvis }))}">◆ ${I18N.t("cont.lb", { n: a.cvis })}</button>`;
  if (n) h += `<span class="cont-cnt" role="status">${I18N.t(p.showHid ? "cont.cnt.shown" : "cont.cnt", { n })} · <button type="button" class="cont-show">${I18N.t(p.showHid ? "cont.hide" : "cont.show")}</button></span>`;
  if (!h) return;
  p.leg2.insertAdjacentHTML("beforeend", h);
  const b = p.leg2.querySelector(".cont-show"); if (b) b.onclick = () => { p.showHid = !p.showHid; draw(p); };
  const l = p.leg2.querySelector(".cont-lb");
  if (l) l.onclick = e => {
    e.stopPropagation();
    const L = window.LISTE, f = a.cont, d = a.data[0].d, top = Math.max(...d.y, 1e-9), rows = [];
    d.mz.forEach((m, j) => { if (d.y[j] < 0.01 * top) return; const s = f.state(m); if (s.all.length && !s.hid) rows.push({ m, y: d.y[j], s }); });
    rows.sort((u, v) => v.y - u.y);
    const b = l.getBoundingClientRect(), ev = { preventDefault() {}, clientX: b.left, clientY: b.bottom, kbd: e.detail === 0 };
    menu(ev, rows.length ? rows.slice(0, 15).map(r => ({ label: `m/z ${r.m.toFixed(a.dec ?? 4)} · ${r.s.vis[0].name}`, fn: () => setTimeout(() => contMenu(p, ev, { s: r.s }), 0) })) : [{ label: I18N.t("cont.none"), dim: true }]);
  };
}

