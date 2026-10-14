// @ts-strict
"use strict";
// The Agilent method folders *.m loaded outside any run (AgilentD.held, cartella-carica.js) in the Method window: the engine (Rust, rust-metodo.js) turns the folder into its
// `metodo/1` document (docs/agenti/metodo.md) and this file only draws the section «quantificazione»: what the method looks for, compound by compound, as the file says.
// Everything that comes from a file goes through EH().

const METODO_M = (() => {
  /** @type {Map<string, any>} */
  const docs = new Map();           // folder root -> metodo/1 document (or { error })
  /** @param {string} k @param {any} [p] */
  const T = (k, p) => I18N.t(k, p);
  const any = () => AgilentD.held.length > 0;
  const n = (/** @type {any} */ v) => (v == null ? "" : String(+(+v).toFixed(4)));

  // the document of a held folder, read once: its files as the folder holds them (path inside the folder)
  async function doc(/** @type {{ name: string, root: string, files: File[] }} */ m) {
    if (docs.has(m.root)) return docs.get(m.root);
    let d;
    try {
      const rb = window.MZLAB_RUST_READY ? await window.MZLAB_RUST_READY : null;
      if (!rb || rb.disabled || !rb.metodoM) throw new Error(T("err.file.agilentNoEngine", { name: m.name }));
      const dirOf = (/** @type {File} */ f) => (typeof CartCarica !== "undefined" ? CartCarica.dir(f) : (f.webkitRelativePath || "").replace(/[^/]*$/, ""));
      d = await rb.metodoM(m.name, m.files.map(f => ({ rel: (dirOf(f) + f.name).slice(m.root.length), size: f.size, arrayBuffer: () => f.arrayBuffer() })));
    } catch (e) { d = { error: String(/** @type {Error} */ (e).message || e) }; }
    docs.set(m.root, d);
    return d;
  }

  function compound(/** @type {any} */ c) {
    const ions = (c.ioni || []).map((/** @type {any} */ q) => `${n(q.mz)} (${n(q.risposta_relativa_pct)} %${q.min_pct != null ? `, ${n(q.min_pct)}–${n(q.max_pct)}` : ""})`).join("; ");
    const lv = (c.livelli || []).map((/** @type {any} */ l) => n(l.conc)).join(", ");
    const w = c.finestra_rt || {}, z = c.finestra_mz || {}, k = c.curva || {};
    const cell = (/** @type {any} */ v) => `<td>${EH(String(v ?? ""))}</td>`;
    return `<tr>${cell(c.nome)}${cell(c.tipo)}<td class="num">${EH(n(c.rt_min))}</td>${cell(w.valore != null ? `${n(w.valore)} ${w.unita || ""}` : "")}${cell(z.sinistra != null ? `−${n(z.sinistra)} / +${n(z.destra)} ${z.unita || ""}` : "")}${cell(ions)}${cell(lv ? `${lv} ${c.unita_conc || ""}` : "")}${cell([k.modello, k.origine, k.peso].filter(Boolean).join(", "))}</tr>`;
  }

  function section(/** @type {{ name: string }} */ m, /** @type {any} */ d) {
    const title = `<h4>${EH(T("method.m.title", { name: m.name }))}</h4>`;
    if (d.error) return `<section class="msec">${title}<div class="fail">${EH(T("method.m.error", { name: m.name, message: d.error }))}</div></section>`;
    const q = d.quantificazione;
    if (!q || !(q.composti || []).length) return `<section class="msec">${title}<div class="muted sm">${EH(T("method.m.none"))}</div></section>`;
    const f = q.finestre || {};
    const win = f.riferimento != null ? `<div class="sm muted" style="margin:4px 0">${EH(T("method.m.windows", { ref: n(f.riferimento), non: n(f.non_riferimento), unit: f.unita === "Percent" ? "%" : f.unita || "", corr: n(f.correlazione) }))}</div>` : "";
    const th = ["method.m.col.name", "method.m.col.type", "method.m.col.rt", "method.m.col.rtwin", "method.m.col.mzwin", "method.m.col.ions", "method.m.col.levels", "method.m.col.curve"].map(k => `<th>${EH(T(k))}</th>`).join("");
    return `<section class="msec">${title}<div class="sm hw">${EH(T("method.m.about"))}</div>${win}<table><tr>${th}</tr>${q.composti.map(compound).join("")}</table></section>`;
  }

  // the sections of all the held folders (a folder dropped twice counts once)
  async function html() {
    const seen = new Set(), out = [];
    for (const m of AgilentD.held) { if (seen.has(m.root)) continue; seen.add(m.root); out.push(section(m, await doc(m))); }
    return out.join("");
  }
  return { any, html, docs };
})();
