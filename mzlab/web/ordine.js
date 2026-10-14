// @ts-check
// @ts-strict
"use strict";
// Order and unit of the file times: pure functions, also run by node in tests/test_ordine.py. Times are always stored in MINUTES; the unit
// (min, h, d) only changes what is shown and typed. The order of the files has ONE rule, here: by time if the names give a time, else standards
// by growing concentration, else natural alphabetical order (file2 before file10).

const TIME_UNITS = { s: 1 / 60, min: 1, h: 60, d: 1440 };
const CONC_MGL = { "mg/L": 1, "µg/L": 1e-3, "ng/L": 1e-6, "g/L": 1e3 };       // concentration labels of the load table -> factor to mg/L

// natural comparison of two strings: digit runs compare as numbers, the rest case-insensitively by code point (no locale: same order everywhere)
function naturalCmp(a, b) {
  const sa = String(a).toLowerCase().match(/\d+|\D+/g) || [], sb = String(b).toLowerCase().match(/\d+|\D+/g) || [];
  for (let i = 0; i < Math.min(sa.length, sb.length); i++) {
    const x = sa[i], y = sb[i], dx = /^\d/.test(x), dy = /^\d/.test(y);
    if (dx && dy) { const d = Number(x) - Number(y); if (d) return d < 0 ? -1 : 1; if (x.length !== y.length) return x.length < y.length ? -1 : 1; }
    else if (x !== y) return x < y ? -1 : 1;
  }
  return sa.length === sb.length ? 0 : sa.length < sb.length ? -1 : 1;
}

// concentration of a file in mg/L (null if none); cunit is the label of CONC_MGL, or a key of the name units (ppm, ugl...)
function concMgL(f) {
  if (f.conc == null) return null;
  const fac = CONC_MGL[f.cunit] ?? { ppm: 1, ppb: 1e-3, ppt: 1e-6, mgl: 1, ugl: 1e-3, ngml: 1e-3, ngl: 1e-6, gl: 1e3 }[f.cunit] ?? 1;
  return f.conc * fac;
}

// comparator for the files of a list: the rule is decided on the whole list, so the order is the same whatever the order they arrive in
function fileComparator(list, conc = concMgL) {
  const byTime = list.some(f => f.time != null), byConc = !byTime && list.some(f => conc(f) != null);
  const key = f => byTime ? f.time : byConc ? conc(f) : null, big = Infinity;
  return (a, b) => {
    const ka = key(a), kb = key(b);
    if (ka !== kb) return (ka ?? big) < (kb ?? big) ? -1 : (ka ?? big) > (kb ?? big) ? 1 : 0;
    return naturalCmp(a.name ?? a.label ?? a.file ?? "", b.name ?? b.label ?? b.file ?? "");
  };
}
// a sorted copy (stable; the input is not changed)
function orderFiles(list) { const c = fileComparator(list); return list.map((f, i) => [f, i]).sort((x, y) => c(x[0], y[0]) || x[1] - y[1]).map(x => x[0]); }

// group of a high-resolution file in the file list (one place): a single scan or constant RT -> infusion; MS1 + the MS2 it triggered in the same
// file (both parts of the file) -> dda; only scans with a precursor -> ms2; otherwise (no MS2, scans without precursor) -> full
function hrGroup(f) { return f.infusion ? "infusion" : f.dda ? "dda" : f.kind === "ms2" ? "ms2" : "full"; }
// the physical file of an entry of the session ("x.mzML#MS2" is a part of "x.mzML")
const fileBase = name => String(name).split("#")[0];

// time in minutes <-> value shown in a unit
const timeShown = (min, unit) => min == null ? null : +(min / (TIME_UNITS[unit] || 1)).toPrecision(6);
const timeStored = (val, unit) => val == null ? null : +(val * (TIME_UNITS[unit] || 1)).toPrecision(12);
// download a Blob as a file (the object URL is released after `wait` ms)
function saveBlob(blob, name, wait = 4000) {
  const a = document.createElement("a"), u = URL.createObjectURL(blob);
  a.href = u; a.download = name; document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(u), wait);
}
// decimal text typed by the student ("0,5" or "0.5") -> number | NaN; lenient like parseFloat (a trailing unit is ignored)
const numIt = t => parseFloat(String(t).replace(",", "."));
// text typed in the time field: "", "12", "0,5", "1.25" -> number | null (empty) | NaN (not a number); no exponent, no sign
function parseDecimal(text) {
  const s = String(text ?? "").trim().replace(",", ".");
  if (s === "") return null;
  return /^\d+(\.\d*)?$|^\.\d+$/.test(s) ? Number(s) : NaN;
}

// the unit chosen by the student (localStorage "qqq.tunit"; in memory if the storage is not available); a time is shown only when the file name gave one
let _tunit = "min";
function tunitGet() {
  try { const u = localStorage.getItem("qqq.tunit"); if (u && TIME_UNITS[u]) _tunit = u; } catch (_) { /* no storage */ }
  return _tunit;
}
function tunitSet(u) {
  if (!TIME_UNITS[u]) return;
  _tunit = u;
  try { localStorage.setItem("qqq.tunit", u); } catch (_) { /* no storage */ }
}
// "30 min", "0.5 h", "2 d" for a time in minutes
const tunitLabel = u => typeof I18N !== "undefined" ? { s: I18N.t("unit.t.s"), min: I18N.t("unit.t.min"), h: I18N.t("unit.t.h"), d: I18N.t("unit.t.d") }[u] : u;
const fmtTime = min => min == null ? "" : timeShown(min, tunitGet()) + " " + tunitLabel(tunitGet());

if (typeof module !== "undefined") module.exports = { TIME_UNITS, naturalCmp, concMgL, fileComparator, orderFiles, hrGroup, fileBase, timeShown, timeStored, parseDecimal, tunitGet, tunitSet, tunitLabel, fmtTime };
