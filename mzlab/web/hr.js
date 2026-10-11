"use strict";
// High resolution: how precisely m/z values are written and compared (classic script, loaded after explore.js).
// The server sends, for every file, the mass profile of its survey scans (`prof1`) and of its product-ion scans (`prof2`):
// {hr, an, dec, tol, unit}. Everything in the page that decides decimals, a tolerance or the grouping of m/z goes through HR.prof().
// The 3200 QTRAP files have the low-resolution profile (1 decimal, the unit window of the XIC): nothing changes for them.
const HR = (() => {
  const LOW = { hr: false, an: "?", dec: 1, tol: 0.5, unit: "Da" };
  const num = (v, lo, hi, d) => { v = +v; return Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : d; };
  // profile of file f at a level (1 survey, 2 product ions); `f` may be undefined (no file yet): low resolution
  function prof(f, level) {
    const lv = level || (f && f.lv) || 1, p = f && (lv === 2 ? f.prof2 : f.prof1);
    if (!p) return LOW;
    if (!p.hr) return p;                                                     // ion trap: 2 decimals, 0.5 Da (as Thermo's default), not high resolution
    return { ...p, dec: Math.round(num(UIP.hrDec, 3, 5, p.dec)), tol: num(UIP.hrPpm, 1, 50, p.tol), unit: "ppm" };
  }
  const isHr = (f, level) => prof(f, level).hr;
  // decimals for a group of files (a graph with several files): the largest of their profiles
  const dec = (files, level) => Math.max(1, ...(files || []).filter(Boolean).map(f => prof(f, level).dec));
  const anyHr = (files, level) => (files || []).some(f => f && isHr(f, level));
  // half width (Da) of an ion window at m/z for file f: ppm for high resolution, null = "use the unit window of today"
  const tolDa = (f, mz, level) => { const q = prof(f, level); return q.hr ? mz * q.tol * 1e-6 : null; };
  const fmt = (f, level, v) => (v == null || !Number.isFinite(v)) ? "" : v.toFixed(prof(f, level).dec);
  const ppm = (obs, ref) => (obs - ref) / ref * 1e6;
  // "tolerance 5 ppm" or "0.5 Da" as text for titles and tooltips
  const tolText = (f, level) => { const q = prof(f, level); return q.unit === "ppm" ? `${+q.tol.toFixed(1)} ppm` : `${+q.tol.toFixed(2)} Da`; };
  // appended to server requests: the server must also behave as today when the switch is off
  const q = () => "";
  // short label of the instrument, for lists and the method window: "Orbitrap Exploris 120 · R 45 000 / 15 000"
  const rtxt = r => r ? Math.round(r).toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ") : "";
  function label(f) {
    if (!f || !(f.prof1 && f.prof1.hr) && !(f.prof2 && f.prof2.hr)) return "";
    const r = [f.res1, f.res2].filter(Boolean).map(rtxt); const rr = r.length === 2 && r[0] === r[1] ? [r[0]] : r;
    return [f.instrument, rr.length ? "R " + rr.join(" / ") : "", f.dda ? "DDA" : ""].filter(Boolean).join(" · ");
  }
  // a file whose high-resolution reading failed on the server opens as low resolution: say it once
  const told = new Set();
  function notice(files) {
    const bad = (files || []).filter(f => f && f.hr_err && !told.has(f.file));
    if (!bad.length) return;
    bad.forEach(f => { told.add(f.file); console.info("HR", f.file, f.hr_err); });
    let d = document.querySelector("#hrerr");
    if (!d) { d = document.createElement("div"); d.id = "hrerr"; d.style.cssText = "position:fixed;z-index:20000;left:50%;bottom:18px;transform:translateX(-50%);max-width:520px;padding:8px 14px;border-radius:6px;background:#333;color:#fff;font:13px system-ui;box-shadow:0 2px 8px rgba(0,0,0,.35)"; document.body.appendChild(d); }
    d.textContent = I18N.t("hr.notice", { files: bad.map(f => f.label || f.file).join(", ") }); d.hidden = false;
    clearTimeout(d._t); d._t = setTimeout(() => { d.hidden = true; }, 7000);
  }
  // ---------------------------------------------------------------- isotope pattern with the fine structure (high resolution)
  // isoPattern() of tables.js groups isotopologues by nominal mass (right for unit resolution). Here every different exact mass stays a peak
  // (13C and 34S at M+2 are 2.7 mDa apart); masses closer than 0.5 mDa are one peak. n = element counts of the ion, z = charge.
  const ELECTRON = 0.00054858, FINE_TOL = 0.0005;
  let ELMAP = null;
  const elIso = s => {
    if (!ELMAP) ELMAP = Object.fromEntries(ELEMENTS.map(e => [e.s, e]));
    const e = ELMAP[s]; if (!e || !e.iso.length) throw new Error(I18N.t("tbl.iso.unavailable", { s }));
    return e.iso.filter(i => i[1] != null && i[2] > 0).map(i => [i[1], i[2] / 100]);
  };
  const joinMasses = list => {                             // list of [mass, probability]: sort, and merge masses within FINE_TOL
    list.sort((a, b) => a[0] - b[0]);
    const out = []; let cur = null;
    for (const [m, p] of list) {
      if (cur && m - cur.m0 <= FINE_TOL) { cur.s += m * p; cur.p += p; } else { cur = { m0: m, s: m * p, p }; out.push(cur); }
    }
    return out.map(c => [c.s / c.p, c.p]);
  };
  const convMass = (a, b) => { const r = []; for (const [ma, pa] of a) for (const [mb, pb] of b) { const p = pa * pb; if (p > 1e-10) r.push([ma + mb, p]); } return joinMasses(r); };
  function isoFine(n, z = 1, minRel = 0.1) {
    let dist = [[0, 1]];
    for (const [s, k] of Object.entries(n)) {
      if (!(k > 0)) continue;
      let base = elIso(s), res = [[0, 1]], c = k;
      while (c > 0) { if (c & 1) res = convMass(res, base); c >>= 1; if (c) base = convMass(base, base); }
      dist = convMass(dist, res);
    }
    const top = Math.max(...dist.map(d => d[1]));
    return dist.map(([m, p]) => ({ mz: (m - z * ELECTRON) / Math.abs(z), rel: 100 * p / top })).filter(r => r.rel >= minRel).sort((a, b) => a.mz - b.mz);
  }
  // Peaks closer than their width are not resolved: at resolving power R the fine structure of the pattern fuses into one peak. R200 = resolving power at
  // m/z 200 as Thermo writes it (Orbitrap: R falls with 1/sqrt(m/z)); two neighbours merge when their distance is below the FWHM = m/R(m).
  const resAt = (R200, mz) => R200 * Math.sqrt(200 / mz);
  // generic: two neighbours merge when their distance is below the width fw(m) at the peak
  function mergeBy(pat, fw) {
    if (!fw) return pat;
    const out = []; let cur = null;
    for (const r of pat) {
      if (cur && r.mz - cur.last <= fw(r.mz)) { cur.s += r.mz * r.rel; cur.rel += r.rel; cur.last = r.mz; } else { cur = { s: r.mz * r.rel, rel: r.rel, last: r.mz }; out.push(cur); }
    }
    const top = Math.max(...out.map(c => c.rel));
    return out.map(c => ({ mz: c.s / c.rel, rel: 100 * c.rel / top }));
  }
  const mergeRes = (pat, R200) => (R200 > 0 ? mergeBy(pat, m => m / resAt(R200, m)) : pat);
  // the width of a peak of the simulation: from the resolving power (at m/z 200, scaling with 1/sqrt(m/z)), or fixed in Da, or in ppm; given as FWHM or as the width at 10% / 5% of the height
  const WDEF = { fwhm: 1, "10": 1.8227, "5": 2.3223 };            // width at that fraction of a Gaussian / FWHM
  function fwhmFn(iso, f, level) {
    const k = WDEF[iso.def || "fwhm"] || 1;
    if (iso.fw && iso.fw.v > 0) return iso.fw.mode === "ppm" ? (m => m * iso.fw.v * 1e-6 / k) : (() => iso.fw.v / k);
    const R = iso.R === 0 ? 0 : (iso.R || (f && (level === 2 ? f.res2 : f.res1)) || 0);
    return R > 0 ? (m => m / resAt(R, m)) : null;
  }
  // the Gaussian profile of the merged peaks (points = points per FWHM): {x, y}, highest point = 100
  function isoCurve(pat, fw, pts = 10) {
    const sg = m => fw(m) / 2.3548, a = pat[0].mz - 5 * sg(pat[0].mz), b = pat[pat.length - 1].mz + 5 * sg(pat[pat.length - 1].mz);
    const step = Math.min(...pat.map(r => fw(r.mz))) / Math.max(3, pts), n = Math.min(6000, Math.max(20, Math.ceil((b - a) / step))), x = [], y = [];
    for (let i = 0; i <= n; i++) { const m = a + (b - a) * i / n; let v = 0; for (const r of pat) { const d = (m - r.mz) / sg(r.mz); if (Math.abs(d) < 6) v += r.rel * Math.exp(-0.5 * d * d); } x.push(m); y.push(v); }
    const mx = Math.max(...y) || 1; return { x, y: y.map(v => 100 * v / mx) };
  }
  // ions of a simulation: one per adduct (with its concentration, %), or, with a charge distribution {z0, hw}, one per charge state (protonated / deprotonated)
  // weighted by 2^(-4 ((z - z0) / hw)^2) (hw = full width at half height, in charge units); the isotope calculation itself is the one of isoFine
  function isoSeries(iso) {
    const adds = iso.adds && iso.adds.length ? iso.adds : [{ ad: iso.ad, c: 100 }], cd = iso.cd, out = [];
    adds.forEach(a => {
      const base = QQQRef.ionCounts(iso.formula, a.ad), w0 = (a.c == null ? 100 : a.c) / 100;
      if (cd && (cd.z0 > 1 || cd.hw > 0)) {
        const sg = base.z < 0 ? -1 : 1, M0 = QQQRef.ionCounts(iso.formula, sg > 0 ? "[M]+" : "[M]-"), hw = cd.hw || 0;
        for (let z = Math.max(1, Math.floor(cd.z0 - 1.5 * hw)); z <= Math.max(1, Math.ceil(cd.z0 + 1.5 * hw)); z++) {
          const w = hw > 0 ? Math.pow(2, -4 * ((z - cd.z0) / hw) ** 2) : (z === Math.round(cd.z0) ? 1 : 0), n = { ...M0.n };
          n.H = (n.H || 0) + sg * z; if (w < 0.02 || n.H < 0) continue;
          out.push({ ion: { n, z: sg * z }, w: w * w0, tag: `${z}${sg > 0 ? "+" : "−"}` });
        }
      } else out.push({ ion: base, w: w0, tag: a.ad });
    });
    if (!out.length) throw new Error(I18N.t("bn.iso.noSeries"));
    return out;
  }
  // the combined pattern of the simulation, 100 = its most intense peak; every peak knows the first peak (m0) of its own ion, and a tag when there are several ions
  function isoPat(iso, fw) {
    const parts = isoSeries(iso), out = [];
    parts.forEach(sr => { let pt = isoFine(sr.ion.n, sr.ion.z); if (fw) pt = mergeBy(pt, fw); const m0 = pt[0].mz; pt.forEach(r => out.push({ ...r, rel: r.rel * sr.w, m0, tag: parts.length > 1 ? sr.tag : "" })); });
    out.sort((a, b) => a.mz - b.mz);
    const mx = Math.max(...out.map(r => r.rel)) || 1;
    return out.map(r => ({ ...r, rel: 100 * r.rel / mx }));
  }
  // the simulation in place of the spectrum («Sostituisci»): centroids (and the Gaussian line when the style is «profilo»), height of the observed top
  function isoAsSpectrum(p, f, dObs) {
    const fw = fwhmFn(p.iso, f, p.level), pat = isoPat(p.iso, fw);
    const top = Math.max(0, ...(dObs && dObs.y ? dObs.y : [0])) || 1e6, o = { mz: pat.map(r => r.mz), y: pat.map(r => top * r.rel / 100), mode: "centroid" };
    if (p.iso.style === "profilo" && fw) { const c = isoCurve(pat, fw, p.iso.pts || 10); o.pmz = c.x; o.py = c.y.map(v => top * v / 100); o.mode = "profile"; }
    return o;
  }
  // the theoretical pattern on a spectrum (red circles): each circle sits on the observed centroid within the tolerance of the profile (and says the
  // error in ppm), otherwise on the calculated m/z with "non trovato". Returns the note for the legend.
  function drawIso(p, g, X, Y, d0, ymax, W, f) {
    const tol = prof(f, p.level).tol, st = p.iso.style || "centroidi";
    const fw = fwhmFn(p.iso, f, p.level), R = p.iso.R === 0 ? 0 : (p.iso.R || (p.level === 2 ? f.res2 : f.res1) || 0), pat = isoPat(p.iso, fw);          // no width: whole fine structure
    const top = pat.reduce((a, b) => (b.rel > a.rel ? b : a));
    const find = mz => { let best = null; d0.mz.forEach((m, j) => { if (d0.y[j] <= 0) return; const e = ppm(m, mz); if (Math.abs(e) <= tol && (!best || d0.y[j] > best.y)) best = { m, y: d0.y[j], e }; }); return best; };
    const obsTop = find(top.mz), h = obsTop ? obsTop.y : ymax / 1.12 * 0.9;
    g.save(); g.strokeStyle = g.fillStyle = "#d62728"; g.lineWidth = 1.5;
    if (st === "profilo" && fw) {                                      // Gaussian profile at the resolution of the scan
      const c = isoCurve(pat, fw, p.iso.pts || 10); g.beginPath(); let on = false;
      c.x.forEach((m, i) => { const px = X(m), py = Y(h * c.y[i] / 100); if (px < M.l - 2 || px > W - M.r + 2) { on = false; return; } if (on) g.lineTo(px, py); else { g.moveTo(px, py); on = true; } }); g.stroke();
    }
    for (const r of pat) {
      const ob = find(r.mz), px = X(ob ? ob.m : r.mz), py = Y(h * r.rel / 100); if (px < M.l || px > W - M.r) continue;
      if (st === "barre") { g.lineWidth = 3; g.beginPath(); g.moveTo(px, Y(0)); g.lineTo(px, py); g.stroke(); g.lineWidth = 1.5; }
      else if (st === "centroidi") { g.setLineDash([3, 2]); g.beginPath(); g.moveTo(px, Y(0)); g.lineTo(px, py); g.stroke(); g.setLineDash([]); g.beginPath(); g.arc(px, py, 3.5, 0, 7); g.stroke(); }
      if (r.rel >= 1) {
        const off = Math.round(r.mz - r.m0), t = (r.tag ? r.tag + " " : "") + (off ? "M+" + off : "M") + " " + (r.rel < 10 ? r.rel.toFixed(1) : Math.round(r.rel)) + "%" + (ob ? ` · ${ob.e >= 0 ? "+" : "−"}${Math.abs(ob.e).toFixed(1)} ppm` : " · " + I18N.t("hr.iso.notFound"));
        g.font = fpx(10); g.textAlign = "left"; g.fillText(t, px + 5, py - 4);
      }
    }
    g.restore(); g.font = fpx(11);
    const wtxt = p.iso.fw && p.iso.fw.v > 0 ? " " + (p.iso.def && p.iso.def !== "fwhm" ? I18N.t("hr.iso.widthDef", { v: p.iso.fw.v, unit: p.iso.fw.mode === "ppm" ? "ppm" : "Da", def: p.iso.def }) : I18N.t("hr.iso.width", { v: p.iso.fw.v, unit: p.iso.fw.mode === "ppm" ? "ppm" : "Da" })) : R ? " " + I18N.t("hr.iso.res", { R: Math.round(R) }) : "";
    return `<span><i style="background:#d62728"></i>${I18N.t("hr.iso.legend", { formula: fmtFormula(p.iso.formula), adduct: fmtAdduct(p.iso.ad), w: wtxt, obs: obsTop ? ` (M: ${obsTop.e >= 0 ? "+" : "−"}${Math.abs(obsTop.e).toFixed(1)} ppm)` : " " + I18N.t("hr.iso.noPeak", { tol: +tol.toFixed(1), mz: top.mz.toFixed(HR.prof(f, p.level).dec) }) })}</span>`;
  }
  // ---------------------------------------------------------------- XIC of an ion
  // A trace {mz, ion: true} is an ion: its exact m/z. Each file reads it with ITS tolerance: ppm of the profile for a high-resolution file,
  // the unit window of today (xicWin, explore.js) for the others. A classic trace {mz, w} is an explicit window in Da (all files alike).
  const xicFiles = tab => scanFiles(tabFiles(tab || E.tab));
  const ionText = (mz, f) => { const q = prof(f, f.lv); return `m/z ${(+mz).toFixed(q.dec)} \u00b1 ${+q.tol.toFixed(1)} ppm`; };
  // the trace for an ion if a high-resolution file is among the files the XIC is for; otherwise null (the caller makes the classic unit-window trace)
  function ionTrace(mz, o = {}) {
    const fs = xicFiles(o.tab), hf = fs.find(f => isHr(f, f.lv));
    if (!hf) return null;
    return { id: E.seq++, mz: +(+mz).toFixed(5), ion: true, obs: !!o.obs && !fs.every(f => isHr(f, f.lv)), label: (o.prefix ? o.prefix + " \u00b7 " : "") + ionText(mz, hf) };
  }
  // [centre, half width] in Da of trace t for file f
  function xicArgs(f, t, p) {
    if (!t.ion) return [t.mz, t.w ?? p.tol];
    const q = prof(f, f.lv);
    if (q.hr) return [t.mz, t.mz * q.tol * 1e-6];
    const [a, b] = xicWin(t.mz, t.obs);
    return [rh((a + b) / 2, 2), rh((b - a) / 2, 2)];
  }
  // lower and upper m/z written in the header of an XIC panel for trace t (ion traces: the window of the first high-resolution file)
  function xicEdges(t, p) {
    const f = xicFiles(p.tab).find(x => isHr(x, x.lv)), q = f && prof(f, f.lv);
    if (!t.ion || !q) return null;
    const d = q.dec, w = t.mz * q.tol * 1e-6; return [+(t.mz - w).toFixed(d + 1), +(t.mz + w).toFixed(d + 1)];
  }
  return { isoSeries, isoPat, mergeRes, mergeBy, resAt, fwhmFn, isoCurve, isoAsSpectrum, WDEF, LOW, prof, isHr, dec, anyHr, tolDa, fmt, ppm, tolText, q, label, notice, isoFine, drawIso, ionTrace, ionText, xicArgs, xicEdges, xicFiles };
})();
window.HR = HR;
