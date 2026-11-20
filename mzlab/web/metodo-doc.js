// @ts-strict
"use strict";
// The Method window shows here what the file itself says about its acquisition: the `method/1` document (docs/agenti/method.md) that /api/metodo gives for the open item
// (the engine reads it from the .raw, the .wiff, the .D, the .dam or the mzML that carries it). Only what the file holds is drawn, as the file says it; a value the file
// lacks has no row. What the reader could not read (`avvisi`: parts not decoded, missing, cut) is listed first, in plain words.
// Everything that comes from a file goes through EH().

const METODO_DOC = (() => {
  /** @param {string} k @param {any} [p] */
  const T = (k, p) => I18N.t(k, p);
  const n = (/** @type {any} */ v) => (v == null ? "" : String(+(+v).toFixed(4)));
  const rowOf = (/** @type {string} */ a, /** @type {any} */ b) => (b == null || b === "" ? "" : `<tr><th scope="row" class="muted" style="font-weight:normal;text-align:left">${EH(a)}</th><td>${EH(String(b))}</td></tr>`);
  const fold = (/** @type {string} */ title, /** @type {string} */ body) => (body ? `<details style="margin-top:8px"><summary>${EH(title)}</summary>${body}</details>` : "");
  const withUnit = (/** @type {any} */ p) => `${n(p.value)}${p.unit ? " " + p.unit : ""}`;
  const join = (/** @type {any[]} */ a, /** @type {string} */ sep = " · ") => a.filter(x => x != null && x !== "").join(sep);

  function head(/** @type {any} */ d) {
    const o = d.origin || {}, c = d.sample || {}, s = d.instrument || {}, u = d.operator || {};
    const files = (o.file || []).map((/** @type {any} */ f) => rowOf(f.name, join([o.format, f.byte != null ? Number(f.byte) + " B" : "", f.sha1 ? "SHA-1 " + f.sha1 : ""]))).join("");
    return files + rowOf(T("method.doc.created"), o.file_created)
      + rowOf(T("method.doc.name"), c.name) + rowOf("Vial", c.vial) + rowOf("ID", c.id) + rowOf(T("method.doc.inject"), c.injection_uL != null ? n(c.injection_uL) + " µL" : "") + rowOf(T("method.doc.methodFile"), c.method_file)
      + rowOf(T("method.doc.instrument"), join([s.manufacturer, s.model], " ")) + rowOf(T("method.doc.serial"), s.series) + rowOf("Hardware", s.hardware) + rowOf("Firmware", s.firmware)
      + (s.software || []).map((/** @type {any} */ x) => rowOf(x.name, x.version)).join("")
      + rowOf("Computer", s.computer || u.computer) + rowOf(T("method.doc.user"), u.user);
  }

  function ms(/** @type {any} */ d) {
    const m = d.ms || {};
    const exps = (m.periods || []).flatMap((/** @type {any} */ p) => p.experiments || []);
    const line = (/** @type {any} */ e) => join([e.type, e.mz ? `m/z ${n(e.mz[0])}–${n(e.mz[1])}` : "", (e.transitions || []).length ? (e.transitions.length + " MRM") : "", (e.ce_eV || []).length ? "CE " + e.ce_eV.map(n).join(", ") + " eV" : "",
      (e.nce || []).length ? "NCE " + e.nce.map(n).join(", ") : "", e.isolation_mz != null ? `${T("method.doc.isol")} ${n(e.isolation_mz)} m/z` : ""]);
    const sets = (m.settings || []).map((/** @type {any} */ x) => rowOf(x.label || x.id, join([(x.values || []).map(n).join(", "), x.value, x.unit], " ")));
    return rowOf(T("method.doc.source"), (m.source || {}).type)
      + exps.map((/** @type {any} */ e, /** @type {number} */ i) => rowOf(`${T("method.sec.experiment")} ${e.index ?? i + 1}`, line(e))).join("") + sets.join("");
  }

  function lc(/** @type {any} */ d) {
    const l = d.lc || {}, g = l.gradient || [], p = l.pressure_max;
    const solv = (l.solvents || []).map((/** @type {any} */ x) => rowOf(T("method.doc.solvent", { ch: x.channel || "" }), x.name)).join("");
    const rows = solv + rowOf(T("method.flow"), n(l.flow_mL_min)) + rowOf(T("method.oven"), l.oven_C != null ? n(l.oven_C) + " °C" : "") + rowOf(T("method.runTime"), l.duration_min != null ? n(l.duration_min) + " min" : "")
      + rowOf(T("method.doc.pmax"), p ? withUnit(p) : "");
    const tbl = g.length ? `<table><tr><th scope="col" class="num">${EH(T("method.time"))}</th><th scope="col" class="num">% B</th><th scope="col" class="num">${EH(T("method.flow"))}</th></tr>${g.map((/** @type {any} */ q) => `<tr><td class="num">${EH(n(q.t_min))}</td><td class="num">${EH(n(q.B_pct))}</td><td class="num">${EH(n(q.flow_mL_min))}</td></tr>`).join("")}</table>` : "";
    return fold(T("method.lc.title"), (rows ? `<table>${rows}</table>` : "") + tbl);
  }

  function detectors(/** @type {any} */ d) {
    const r = (d.detectors || []).map((/** @type {any} */ x) => rowOf(x.type || "", join([x.nm ? `${x.nm.map(n).join("–")} nm` : "", x.step_nm != null ? n(x.step_nm) + " nm" : "", x.frequency_Hz != null ? n(x.frequency_Hz) + " Hz" : "", x.lamp, x.cell_C != null ? n(x.cell_C) + " °C" : ""]))).join("");
    return r ? `<table>${r}</table>` : "";
  }

  // the `method/1.warnings` keys of the readers; another is shown as it is, with its place
  const NOTE_KEYS = ["meta.notDecoded", "meta.missing", "meta.truncated", "meta.damaged", "meta.ionDiscarded", "meta.fromTable"];
  function notes(/** @type {any} */ d) {
    const a = (d.warnings || []).filter((/** @type {any} */ x) => x && x.key);
    if (!a.length) return "";
    const line = (/** @type {any} */ x) => NOTE_KEYS.includes(x.key) ? T(x.key, { place: x.place || "" }) : join([x.key, x.place], ": ");
    return `<div class="sm" role="note" aria-label="${EH(T("method.doc.notes"))}" data-mnotes="${a.length}"><strong>${EH(T("method.doc.notes"))}</strong><ul style="margin:4px 0 8px">${a.map((/** @type {any} */ x) => `<li>${EH(line(x))}</li>`).join("")}</ul></div>`;
  }

  function all(/** @type {any} */ d, /** @type {{ k: number }} */ f) {
    const c = d.fields || [], reg = d.log || [];
    const fields = c.length ? `<table>${c.map((/** @type {any} */ f) => rowOf(`${f.origin || ""} · ${f.key || ""}`, String(f.value ?? "") + (f.unit ? " " + f.unit : ""))).join("")}</table>` : "";
    const log = window.METODO_REGISTRO && METODO_REGISTRO.timed(reg).length ? METODO_REGISTRO.html(reg, f) : reg.length ? `<table>${reg.map((/** @type {any} */ x, /** @type {number} */ i) => rowOf(String(i + 1), x.text)).join("")}</table>` : "";
    return fold(T("method.doc.fields", { n: c.length }), fields) + fold(T("method.doc.log"), log);
  }


  /** The section for the open item `f`. */
  async function html(/** @type {{ k: number, dda?: any }} */ f) {
    const title = `<h4>${EH(T("method.doc.title"))}</h4>`;
    let d;
    try { d = ((await J("api/metodo?k=" + f.k)) || {}).method; } catch (_) { d = null; }
    if (!d) return `<section class="msec">${title}<div class="muted sm">${EH(T("method.doc.none"))}</div></section>`;
    const rows = head(d) + ms(d);
    return `<section class="msec">${title}${notes(d)}${METODO_INCL.html(d, f)}${rows ? `<table>${rows}</table>` : ""}${lc(d)}${detectors(d)}${all(d, f)}</section>`;
  }
  return { html, bind: METODO_INCL.bind };
})();
