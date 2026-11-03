// @ts-check
// Progress rows of the side list: from the drop on, one grey row per file being read or converted (the state of CONVROWS: phase and percent; an error with «Details»;
// × cancels the file or closes its error), in the order of arrival, under the open files; a row goes when its file is open or set aside. While the loading runs, a line
// over the rows gives the count and the clock of the loading («3 of 5 ready · 0:12», CONVROWS.head keeps it) and «Cancel» for the whole loading (CONVROWS.onCancel).
// renderFileList (explore.js) draws them with the list; every repaint and every new percent of CONVROWS refreshes them here.
"use strict";

const UPROWS = /** @type {any} */ ((() => {
  const T = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
  const listed = (/** @type {string} */ n) => ST.files.find((/** @type {FileRiga} */ f) => f.name === n);
  // a row read but not open yet stays until it opens (OPENNOW); a file set aside or not to be opened goes
  const shown = (/** @type {ConvRow} */ r) => {
    if (r.state === "cancel" || OPENNOW.isOpen(r.name)) return false;
    if (r.state !== "done") return true;
    const f = listed(r.name);
    return !!f && f.use && f.kind in GORD && OPENNOW.busy() > 0;
  };
  const rows = () => CONVROWS.rows.filter(shown).sort((/** @type {ConvRow} */ a, /** @type {ConvRow} */ b) => CONVROWS.rank(a.name) - CONVROWS.rank(b.name));
  const pcOf = (/** @type {ConvRow} */ r) => r.state === "done" ? 100 : r.pc || 0;
  const txt = (/** @type {ConvRow} */ r) => { const s = CONVROWS.short(r); return r.state === "done" ? T("files.up.opening") : s.txt + (s.num ? " " + s.num : ""); };
  function one(/** @type {ConvRow} */ r) {
    const fail = r.state === "fail", lab = EH(r.name), x = T(fail ? "files.up.close" : "files.up.cancel", { name: r.name });
    return `<div class="fl pend up${fail ? " err" : ""}" data-up="${lab}" title="${EH(CONVROWS.label(r))}"${fail ? "" : ' aria-busy="true"'}><i></i><div class="fi"><b>${EH(r.name.replace(/\.(mzml|mzxml|wiff|raw|d)$/i, ""))}</b><small data-uptxt>${EH(txt(r))}</small>` +
      (fail ? (r.diag && typeof DIAG !== "undefined" ? DIAG.html(r.diag, EH, T) : "") : `<div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${pcOf(r)}" aria-label="${EH(r.name + ": " + CONVROWS.label(r))}"><i style="width:${pcOf(r)}%"></i></div>`) +
      `</div><button type="button" data-up-x="${lab}" aria-label="${EH(x)}" title="${EH(x)}">&times;</button></div>`;
  }
  const head = () => CONVROWS.running().length && CONVROWS.onCancel ? `<div class="fl-uphead sm muted"><span class="cp-num" data-cvhead>${EH(CONVROWS.ready())}</span><button type="button" class="sec sm" data-up-all title="${EH(T("load.cancel.title", {}))}">${EH(T("conv.cancel", {}))}</button></div>` : "";
  /** the rows, for renderFileList */
  const html = () => `<div id="fl-up">${head()}${rows().map(one).join("")}</div>`;
  // × on a running row: the engine stops that file (lr_cancel) and it is not opened; on an error: the row goes
  async function cancel(/** @type {string} */ name) {
    const r = CONVROWS.find(name); if (!r) return;
    if (r.state === "fail") { CONVROWS.drop(name); return; }
    OPENNOW.hold([name]); FILETAB.soon();
    if (ConvUI.stop(name)) { paint(); return; }
    const rb = window.MZLAB_RUST;
    if (rb && rb.rpc) try { await rb.rpc({ op: "lr_cancel", names: [name] }); } catch (_) { /* already read: it stays in the table, unticked */ }
    paint();
  }
  function wire(/** @type {HTMLElement} */ box) {
    const all = /** @type {HTMLElement | null} */ (box.querySelector("[data-up-all]")); if (all) all.onclick = () => { if (CONVROWS.onCancel) CONVROWS.onCancel(); };
    box.querySelectorAll("[data-up-x]").forEach(b => { /** @type {HTMLElement} */ (b).onclick = () => cancel(/** @type {HTMLElement} */ (b).dataset.upX || ""); });
    const v = window.MZLAB_VERSION;
    box.querySelectorAll(".fl.up.err").forEach(el => { const r = CONVROWS.find(/** @type {HTMLElement} */ (el).dataset.up || ""), d = el.querySelector(".diag"); if (r && r.diag && d) DIAG.cabla(d, r.diag, v, T); });
  }
  /** after a repaint of the loading table: the rows again, without rebuilding the list */
  function paint() {
    let box = document.getElementById("fl-up");
    // before the first list is drawn (no session yet) the rows get their box at the end of the side list
    if (!box) { const fl = document.getElementById("flst"); if (!fl || !CONVROWS.rows.length) return; box = document.createElement("div"); box.id = "fl-up"; fl.appendChild(box); }
    const h = head() + rows().map(one).join("");
    if (box.innerHTML !== h) { box.innerHTML = h; wire(box); }
    const fl = document.getElementById("flst"); if (fl) fl.setAttribute("aria-busy", String(!!box.querySelector("[aria-busy=true]")));
  }
  /** a new percent: only its bar and its text */
  function fill(/** @type {ConvRow} */ r) {
    const el = document.querySelector(`#fl-up [data-up="${CSS.escape(r.name)}"]`); if (!el) { paint(); return; }
    const t = el.querySelector("[data-uptxt]"), s = txt(r), l = CONVROWS.label(r); if (t && t.textContent !== s) t.textContent = s;
    if (/** @type {HTMLElement} */ (el).title !== l) /** @type {HTMLElement} */ (el).title = l;
    const b = el.querySelector(".prog"), i = b && /** @type {HTMLElement | null} */ (b.querySelector("i"));
    if (b && i) { b.setAttribute("aria-valuenow", String(pcOf(r))); i.style.width = pcOf(r) + "%"; }
  }
  // CONVROWS updates the cells of the loading table in place and repaints it on a new row: the side rows follow the same calls
  const f0 = CONVROWS.fill.bind(CONVROWS), p0 = CONVROWS.paint.bind(CONVROWS);
  CONVROWS.fill = (/** @type {ConvRow} */ r) => { f0(r); fill(r); };
  CONVROWS.paint = () => { p0(); paint(); };
  return { html, paint, wire, any: () => rows().length > 0 };
})());
