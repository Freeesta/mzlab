// @ts-strict
"use strict";
// The inclusion list of a Thermo method (metodo/1: ms.periodi[].esperimenti[].inclusione) in the Method window, read only, at the head of "Metodo registrato nel file" (metodo-doc.js).
// Each row has two short buttons, «XIC» and «MS2», that run the actions "metodo.xic" and "metodo.ms2" of the registry azioni/1 (so the right-click menu and Shift+F10 have them too);
// the XIC is the usual one (openXic with `diretto`, pannelli.js), the MS2 is the one DDA.ms2For finds for the precursor inside the RT window of the entry (dda.js). Nothing here is turned into expected products.

const METODO_INCL = (() => {
  /** @param {string} k @param {any} [p] */
  const T = (k, p) => I18N.t(k, p);
  const n = (/** @type {any} */ v) => (v == null ? "" : String(+(+v).toFixed(4)));
  /** @type {{ f: any, items: any[], ms2: Map<number, any> }} */
  let cur = { f: null, items: [], ms2: new Map() };
  const win = (/** @type {any} */ v) => (v.t_inizio_min != null && v.t_fine_min != null ? [v.t_inizio_min, v.t_fine_min] : null);
  const rtText = (/** @type {any} */ v) => (v.t_inizio_min != null || v.t_fine_min != null ? `${n(v.t_inizio_min)}–${n(v.t_fine_min)}` : "");
  const polText = (/** @type {any} */ v) => (v.polarita === "positive" ? T("method.doc.incl.pos") : v.polarita === "negative" ? T("method.doc.incl.neg") : "");
  const noMs2 = () => (cur.f && cur.f.dda ? T("method.doc.btn.ms2.none") : T("method.doc.btn.ms2.nodda"));
  // text of an attribute: the Method window writes every "m/z" in italics with a replace over the whole page, which must not reach inside a tag
  const attr = (/** @type {string} */ t) => EH(t).replace(/m\/z/g, "m&#47;z");
  function html(/** @type {any} */ d, /** @type {any} */ f) {
    const items = ((d.ms || {}).periodi || []).flatMap((/** @type {any} */ p) => p.esperimenti || []).flatMap((/** @type {any} */ e) => e.inclusione || []);
    cur = { f, items, ms2: new Map() };
    if (!items.length) return "";
    const has = (/** @type {string} */ k) => items.some(x => x[k] != null && x[k] !== "");
    const cols = [["mz", "m/z", (/** @type {any} */ v) => n(v.mz)], ["rt", T("method.doc.incl.rt"), rtText], ["pol", T("method.doc.incl.pol"), polText],
      ["nce", T("method.doc.incl.ce"), (/** @type {any} */ v) => (v.nce || []).map(n).join(", ")], ["formula", T("method.doc.incl.formula"), (/** @type {any} */ v) => v.formula], ["specie", T("method.doc.incl.species"), (/** @type {any} */ v) => v.specie],
      ["nome", T("method.doc.incl.name"), (/** @type {any} */ v) => v.nome]].filter(c => c[0] === "mz" || (c[0] === "rt" ? has("t_inizio_min") || has("t_fine_min") : c[0] === "pol" ? has("polarita") : has(c[0])));
    const head = cols.map(c => `<th scope="col"${c[0] === "mz" || c[0] === "rt" ? ' class="num"' : ""}>${EH(String(c[1]))}</th>`).join("") + `<th scope="col">${EH(T("method.doc.incl.act"))}</th>`;
    const btn = "min-width:36px;min-height:26px;margin-right:4px";
    const body = items.map((v, i) => `<tr data-i="${i}">${cols.map(c => `<td${c[0] === "mz" || c[0] === "rt" ? ' class="num"' : ""}>${EH(String(c[2](v) || ""))}</td>`).join("")}<td>`
      + `<button type="button" class="bt" style="${btn}" data-ia="xic" title="${attr(T("method.doc.btn.xic.tip", { mz: n(v.mz) }))}" aria-label="${attr(T("method.doc.btn.xic.tip", { mz: n(v.mz) }))}">${EH(T("method.doc.btn.xic"))}</button>`
      + `<button type="button" class="bt" style="${btn}" data-ia="ms2" disabled title="${attr(noMs2())}" aria-label="${attr(noMs2())}">${EH(T("method.doc.btn.ms2"))}</button></td></tr>`).join("");
    return `<div class="sm" style="margin:2px 0 4px"><b>${EH(T("method.doc.incl.title"))}</b> <span class="muted">${EH(T("method.doc.incl.note"))}</span></div><table id="mincl"><caption class="sr-only">${EH(T("method.doc.incl.title"))}</caption><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`
      + `<div style="margin:4px 0 8px"><button type="button" class="bt" style="min-height:26px" data-ia="tab">${EH(T("tdati.title"))}</button></div>`;
  }
  /** The context of the actions of an entry. */
  const ctxOf = (/** @type {number} */ i) => ({ vista: "metodo", f: cur.f, v: cur.items[i], i, ms2: cur.ms2.get(i) || null });
  const closeDlg = () => { const d = /** @type {HTMLDialogElement | null} */ (document.getElementById("bigdlg")); if (d && d.open) d.close(); };
  if (typeof AZIONI !== "undefined") {
    AZIONI.registra({ id: "metodo.xic", gruppo: "xic", viste: ["metodo"], nome: (/** @type {any} */ c) => T("method.doc.btn.xic.tip", { mz: n(c.v.mz) }),
      esegui: (/** @type {any} */ c) => { closeDlg(); return openXic(null, { mz: c.v.mz, diretto: true }); } });
    AZIONI.registra({ id: "metodo.ms2", gruppo: "ms2", viste: ["metodo"], disponibile: (/** @type {any} */ c) => !!c.ms2,
      nome: (/** @type {any} */ c) => (c.ms2 ? T("method.doc.btn.ms2.tip", { mz: n(c.v.mz), rt: n(c.ms2.rt) }) : noMs2()),
      esegui: (/** @type {any} */ c) => { closeDlg(); return DDA.openEntry(c.f, c.v.mz, win(c.v)); } });
  }
  /** Wires the buttons, the right-click menu and Shift+F10 of the list once the dialog is on the page; the MS2 buttons wake up when the file says an MS2 scan fits the entry. */
  function bind(/** @type {any} */ f) {
    const root = document.getElementById("mincl");
    if (!root || !cur.items.length) return;
    const sec = /** @type {HTMLElement} */ (root.closest(".msec"));
    const at = (/** @type {Element} */ el) => { const tr = el.closest("tr[data-i]"); return tr ? Number(/** @type {HTMLElement} */ (tr).dataset.i) : -1; };
    sec.addEventListener("click", ev => {
      const b = /** @type {HTMLElement | null} */ ((/** @type {Element} */ (ev.target)).closest("button[data-ia]"));
      if (!b || /** @type {HTMLButtonElement} */ (b).disabled) return;
      if (b.dataset.ia === "tab") {
        TDATI.table({ title: T("method.doc.incl.title"), head: ["m/z", T("method.doc.incl.rt"), T("method.doc.incl.pol"), T("method.doc.incl.ce"), T("method.doc.incl.formula"), T("method.doc.incl.species"), T("method.doc.incl.name")],
          rows: cur.items.map(v => [n(v.mz), rtText(v), polText(v), (v.nce || []).map(n).join(", "), v.formula || "", v.specie || "", v.nome || ""]) });
        return;
      }
      const i = at(b); if (i >= 0) AZIONI.esegui("metodo." + b.dataset.ia, ctxOf(i));
    });
    const open = (/** @type {number} */ i, /** @type {any} */ ev) => {
      const dlg = document.getElementById("bigdlg"), ctx = document.getElementById("ctx");
      if (dlg && ctx && ctx.parentElement !== dlg) { dlg.appendChild(ctx); dlg.addEventListener("close", () => document.body.appendChild(ctx), { once: true }); }      // a modal dialog leaves nothing else above it
      menu(ev, [{ label: "m/z " + n(cur.items[i].mz), dim: true }, "-", ...AZIONI.per(ctxOf(i))]);
    };
    sec.addEventListener("contextmenu", ev => { const i = at(/** @type {Element} */ (ev.target)); if (i >= 0) open(i, ev); });
    sec.addEventListener("keydown", ev => {
      if (!((ev.key === "F10" && ev.shiftKey) || ev.key === "ContextMenu")) return;
      const el = /** @type {HTMLElement} */ (ev.target), i = at(el); if (i < 0) return;
      const r = el.getBoundingClientRect();
      open(i, { preventDefault: () => ev.preventDefault(), clientX: r.left, clientY: r.bottom, kbd: true });
    });
    if (f.dda && window.DDA) DDA.get(f.k).then(() => {
      cur.items.forEach((v, i) => {
        const q = DDA.ms2For({ k: f.k }, v.mz, { rt: win(v) })[0], b = /** @type {HTMLButtonElement | null} */ (root.querySelector(`tr[data-i="${i}"] button[data-ia=ms2]`));
        if (!q || !b) return;
        cur.ms2.set(i, q);
        const tip = T("method.doc.btn.ms2.tip", { mz: n(v.mz), rt: n(q.rt) });
        b.disabled = false; b.title = tip; b.setAttribute("aria-label", tip);
      });
    }).catch(() => { /* the buttons stay grey with their reason */ });
  }

  return { html, bind };
})();
