// @ts-strict
"use strict";
// The Method window shows here what the file itself says about its acquisition: the `metodo/1` document (docs/agenti/metodo.md) that /api/metodo gives for the open item
// (the engine reads it from the .raw, the .wiff, the .D, the .dam or the mzML that carries it). Only what the file holds is drawn, as the file says it; a value the file
// lacks has no row, and the parts the file does not decode are not listed. Everything that comes from a file goes through EH().

const METODO_DOC = (() => {
  /** @param {string} k @param {any} [p] */
  const T = (k, p) => I18N.t(k, p);
  const n = (/** @type {any} */ v) => (v == null ? "" : String(+(+v).toFixed(4)));
  const rowOf = (/** @type {string} */ a, /** @type {any} */ b) => (b == null || b === "" ? "" : `<tr><th scope="row" class="muted" style="font-weight:normal;text-align:left">${EH(a)}</th><td>${EH(String(b))}</td></tr>`);
  const fold = (/** @type {string} */ title, /** @type {string} */ body) => (body ? `<details style="margin-top:8px"><summary>${EH(title)}</summary>${body}</details>` : "");
  const withUnit = (/** @type {any} */ p) => `${n(p.valore)}${p.unita ? " " + p.unita : ""}`;
  const join = (/** @type {any[]} */ a, /** @type {string} */ sep = " · ") => a.filter(x => x != null && x !== "").join(sep);

  function head(/** @type {any} */ d) {
    const o = d.origine || {}, c = d.campione || {}, s = d.strumento || {}, u = d.operatore || {};
    const files = (o.file || []).map((/** @type {any} */ f) => rowOf(f.nome, join([o.formato, f.byte != null ? Number(f.byte) + " B" : "", f.sha1 ? "SHA-1 " + f.sha1 : ""]))).join("");
    return files + rowOf(T("method.doc.created"), o.file_creato)
      + rowOf(T("method.doc.name"), c.nome) + rowOf("Vial", c.vial) + rowOf("ID", c.id) + rowOf(T("method.doc.inject"), c.iniezione_uL != null ? n(c.iniezione_uL) + " µL" : "") + rowOf(T("method.doc.methodFile"), c.metodo_file)
      + rowOf(T("method.doc.instrument"), join([s.produttore, s.modello], " ")) + rowOf(T("method.doc.serial"), s.serie) + rowOf("Hardware", s.hardware) + rowOf("Firmware", s.firmware)
      + (s.software || []).map((/** @type {any} */ x) => rowOf(x.nome, x.versione)).join("")
      + rowOf("Computer", s.computer || u.computer) + rowOf(T("method.doc.user"), u.utente);
  }

  function ms(/** @type {any} */ d) {
    const m = d.ms || {};
    const exps = (m.periodi || []).flatMap((/** @type {any} */ p) => p.esperimenti || []);
    const line = (/** @type {any} */ e) => join([e.tipo, e.mz ? `m/z ${n(e.mz[0])}–${n(e.mz[1])}` : "", (e.transizioni || []).length ? (e.transizioni.length + " MRM") : "", (e.ce_eV || []).length ? "CE " + e.ce_eV.map(n).join(", ") + " eV" : "",
      (e.nce || []).length ? "NCE " + e.nce.map(n).join(", ") : "", e.isolamento_mz != null ? `${T("method.doc.isol")} ${n(e.isolamento_mz)} m/z` : ""]);
    const sets = (m.impostazioni || []).map((/** @type {any} */ x) => rowOf(x.etichetta || x.id, join([(x.valori || []).map(n).join(", "), x.valore, x.unita], " ")));
    return rowOf(T("method.doc.source"), (m.sorgente || {}).tipo)
      + exps.map((/** @type {any} */ e, /** @type {number} */ i) => rowOf(`${T("method.sec.experiment")} ${e.indice ?? i + 1}`, line(e))).join("") + sets.join("");
  }

  function lc(/** @type {any} */ d) {
    const l = d.lc || {}, g = l.gradiente || [], p = l.pressione_max;
    const solv = (l.solventi || []).map((/** @type {any} */ x) => rowOf(T("method.doc.solvent", { ch: x.canale || "" }), x.nome)).join("");
    const rows = solv + rowOf(T("method.flow"), n(l.flusso_mL_min)) + rowOf(T("method.oven"), l.forno_C != null ? n(l.forno_C) + " °C" : "") + rowOf(T("method.runTime"), l.durata_min != null ? n(l.durata_min) + " min" : "")
      + rowOf(T("method.doc.pmax"), p ? withUnit(p) : "");
    const tbl = g.length ? `<table><tr><th scope="col" class="num">${EH(T("method.time"))}</th><th scope="col" class="num">% B</th><th scope="col" class="num">${EH(T("method.flow"))}</th></tr>${g.map((/** @type {any} */ q) => `<tr><td class="num">${EH(n(q.t_min))}</td><td class="num">${EH(n(q.B_pct))}</td><td class="num">${EH(n(q.flusso_mL_min))}</td></tr>`).join("")}</table>` : "";
    return fold(T("method.lc.title"), (rows ? `<table>${rows}</table>` : "") + tbl);
  }

  function detectors(/** @type {any} */ d) {
    const r = (d.rivelatori || []).map((/** @type {any} */ x) => rowOf(x.tipo || "", join([x.nm ? `${x.nm.map(n).join("–")} nm` : "", x.passo_nm != null ? n(x.passo_nm) + " nm" : "", x.frequenza_Hz != null ? n(x.frequenza_Hz) + " Hz" : "", x.lampada, x.cella_C != null ? n(x.cella_C) + " °C" : ""]))).join("");
    return r ? `<table>${r}</table>` : "";
  }

  function all(/** @type {any} */ d, /** @type {{ k: number }} */ f) {
    const c = d.campi || [], reg = d.registro || [];
    const fields = c.length ? `<table>${c.map((/** @type {any} */ f) => rowOf(`${f.origine || ""} · ${f.chiave || ""}`, String(f.valore ?? "") + (f.unita ? " " + f.unita : ""))).join("")}</table>` : "";
    const log = window.METODO_REGISTRO && METODO_REGISTRO.timed(reg).length ? METODO_REGISTRO.html(reg, f) : reg.length ? `<table>${reg.map((/** @type {any} */ x, /** @type {number} */ i) => rowOf(String(i + 1), x.testo)).join("")}</table>` : "";
    return fold(T("method.doc.fields", { n: c.length }), fields) + fold(T("method.doc.log"), log);
  }


  /** The section for the open item `f`. */
  async function html(/** @type {{ k: number, dda?: any }} */ f) {
    const title = `<h4>${EH(T("method.doc.title"))}</h4>`;
    let d;
    try { d = ((await J("api/metodo?k=" + f.k)) || {}).metodo; } catch (_) { d = null; }
    if (!d) return `<section class="msec">${title}<div class="muted sm">${EH(T("method.doc.none"))}</div></section>`;
    const rows = head(d) + ms(d);
    return `<section class="msec">${title}${METODO_INCL.html(d, f)}${rows ? `<table>${rows}</table>` : ""}${lc(d)}${detectors(d)}${all(d, f)}</section>`;
  }
  return { html, bind: METODO_INCL.bind };
})();
