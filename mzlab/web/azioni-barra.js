// @ts-strict
"use strict";
// Panel header buttons as registry actions (barra, scope "panel") and ONE renderer, AZIONI.bar; AZIONI.refresh (was syncTb) redraws their state from the panel p.
(() => {
  const T = (/** @type {string} */ k) => I18N.t(k), G = ["xb", "v", "l", "s", "i", "t", "x", "o"], LENS = ["chrom", "xic", "mrm"], ALL = [...LENS, "spec", "map"], MS = (/** @type {Ctx} */ c) => ms2Chrom(c.p);
  const mode = (/** @type {string} */ m) => (/** @type {Ctx} */ c) => c.p._mode(m);
  /** @typedef {any} Ctx
   * @typedef {{ toggle?: number, at?: string, nodata?: number, tag?: string, after?: () => string, ti0?: (c: Ctx) => string, tip?: (c: Ctx) => string, checked?: (c: Ctx) => any, available?: (c: Ctx) => boolean, run?: (c: Ctx) => any }} Opt */
  // [data-a, views, header group, title key, aria key ("=": the title), text or icon, class, Opt]
  /** @type {[string, string[], string, string, string, string | ((c: Ctx) => string), string, Opt][]} */
  const ROWS = [
    ["xicb", ["spec"], "xb", "xicbtn.title", "xicbtn.aria", "XIC", "bt xbb", { toggle: 1 }],
    ["xicl", ["spec"], "xb", "xicbtn.ion.title", "", "", "bt xbl", { at: ' hidden aria-haspopup="dialog" aria-expanded="false"' }],
    ["izoom", LENS, "v", "panel.izoom.title", "", () => IC_ZOOM, "bt", { checked: c => c.p.imode === "zoom", run: mode("zoom") }],
    ["fit", ALL, "v", "panel.fit.title", "", () => IC_FIT, "bt", { at: " disabled", run: c => { const p = c.p; pushZh(p); p.zoom = null; p.zoomY = null; if (p.type === "spec") p.lock = null; draw(p); uiSave(); } }],
    ["tlink", LENS, "l", "panel.tlink.title", "panel.tlink.aria", () => IC_SYNC, "bt", { toggle: 1, checked: c => !!c.p.tl, run: c => toggleTl(c.p) }],
    ["rul", ["spec"], "s", "panel.rul.title", "", () => IC_RULER, "bt", { checked: c => !!c.p.rul, run: c => { const p = c.p; COMMANDS.run("spec.ruler", p, () => { p.rul = !p.rul; if (!p.rul && p.meas && p.meas.ref != null && !p.meas.list.length) { p.meas = null; draw(p); } }); AZIONI.refresh(c); } }],
    ["par", ["spec"], "s", "panel.par.title", "panel.par.aria", "&#9662;", "bt", { run: c => specParams(c.p, c.b) }],
    ["rel", ["spec"], "s", "panel.rel.title", "", "%", "bt", { checked: c => !!specRel(c.p), run: c => { const p = c.p; COMMANDS.run("spec.display", p, () => { p.rel = !specRel(p); }); AZIONI.refresh(c); draw(p); uiSave(); } }],
    ["lock", ["spec"], "s", "lock.off.title", "=", () => IC_UNLOCK, "bt", { toggle: 1, run: c => toggleLock(c.p) }],
    ["iauto", LENS, "i", "panel.iauto.title", "", () => IC_AUTO, "bt", { checked: c => c.p.imode === "auto", run: mode("auto") }],
    ["iall", LENS, "i", "panel.iall.title", "=", "&#9662;", "bt iall", { at: ' type="button" aria-haspopup="true"', run: c => intMenu(c.p, c.e) }],
    ["iman", LENS, "i", "panel.iman.title", "", () => IC_MAN, "bt", { checked: c => c.p.imode === "man", run: mode("man") }],
    ["idel", LENS, "i", "panel.idel.title", "", () => IC_DEL_INT, "bt", { checked: c => c.p.imode === "idel", run: mode("idel") }],
    ["intf", LENS, "i", "panel.intf.title", "", "", "bt", { tag: "select", at: " hidden", run: c => { c.p.intf = c.e.target.value; } }],
    ["iclr", LENS, "i", "panel.iclr.title", "", () => T("panel.iclr"), "bt", { at: " hidden", run: c => { if (c.p.ints.length) delInts(c.p, [...c.p.ints]); } }],
    ["smooth", LENS, "t", "ctl.smooth.title", "ctl.smooth", () => IC_SMOOTH, "bt tgs", { toggle: 1, at: ' type="button"', checked: c => !!c.p.smooth, tip: c => T(ESP.mrm({ type: c.p.type }) ? "ctl.mrm.smooth.title" : "ctl.smooth.title"),
      run: c => { const p = c.p; pushFields(p, "history.smooth"); p.smooth = !p.smooth; AZIONI.refresh(c); draw(p); uiSave(); } }],
    ["log", LENS, "t", "ctl.log.title", "ctl.log.aria", () => T("ctl.log"), "bt tgl", { toggle: 1, at: ' type="button"', checked: c => !!c.p.log, available: c => !(MS(c) || (c.p.type === "chrom" && c.p.kind === "pda")), tip: c => T(MS(c) ? "ctl.noMs2" : "ctl.log.title"),
      run: c => { const p = c.p; COMMANDS.run("panel.source", p, () => { p.log = !p.log; }); AZIONI.refresh(c); draw(p); uiSave(); } }],
    ["xic", ["chrom"], "x", "panel.xic.title", "", "XIC", "bt", { available: c => !MS(c), ti0: c => T(MS(c) ? "panel.xic.off" : "panel.xic.title"), run: c => c.p.kind === "pda" ? wlOpen(c.p) : openXic(null, { after: c.p }) }],
    ["dl", ALL, "o", "panel.dl.title", "=", () => IC_DL.replace(";margin-right:4px", ""), "bt", { run: c => dlMenu(c.p, c.e) }],
    ["more", ALL, "e", "panel.more.title", "=", "⋯", "bt hmb", { at: ' type="button" hidden aria-haspopup="true" aria-expanded="false"', after: () => h`<span class="hmore" hidden role="group" aria-label="${T("panel.more.title")}"></span>` }],
    ["max", ALL, "e", "panel.max.title", "", () => IC_FS, "bt fsb", { run: c => c.p._setMax(!c.p.el.classList.contains("max")) }],
    ["close", ALL, "e", "panel.close", "", "&times;", "x", { nodata: 1, run: c => closePanelWithUndo(c.p) }],
  ];
  for (const [id, views, bar, ti, al, tx, cl, o] of ROWS) AZIONI.register({ id: "panel." + id, group: "tools", views, menu: false, barra: true, scope: "panel", bar, bid: id, ti, al, tx, cl, name: () => T(ti), svg: () => "", ...o });
  /** the attributes and the content of one button @param {any} a @param {any} c */
  const html = (a, c) => {
    const ok = !a.available || a.available(c), on = a.checked ? !!a.checked(c) : false, tip = (a.tip || a.ti0 || (() => T(a.ti)))(c), al = a.al === "=" ? tip : a.al ? T(a.al) : "", tag = a.tag || "button", tx = typeof a.tx === "function" ? a.tx(c) : a.tx;
    return h`<${raw(tag)} class="${a.cl + (on ? " on" : "")}"${raw((a.nodata ? "" : ` data-a="${a.bid}"`) + (a.toggle ? ` aria-pressed="${on}"` : "") + (a.at || "") + (ok ? "" : " disabled"))}${raw(al ? h` aria-label="${al}"` : "")} title="${tip}">${raw(tx)}</${raw(tag)}>${raw(a.after ? a.after() : "")}`;
  };
  /** writes the buttons of the panel header into `host` (the `.tbs` span) and wires them to the actions @param {any} c {view, p} @param {HTMLElement} host */
  AZIONI.bar = (c, host) => {
    const l = AZIONI.list(c, "barra"), g = (/** @type {string} */ k) => l.filter(a => a.bar === k).map(a => html(a, c)).join("");
    host.innerHTML = G.map(k => { const s = g(k); return s ? h`<span class="tbg">${raw(s)}</span>` : ""; }).join("") + g("e");
    for (const a of l) {
      const b = /** @type {HTMLElement | null} */ (host.querySelector(a.nodata ? ".x" : `[data-a="${a.bid}"]`));
      if (b && a.run) b[a.tag ? "onchange" : "onclick"] = (/** @type {Event} */ e) => AZIONI.run(a.id, { ...c, e, b });
    }
  };
  /** the state of the buttons from the panel (checked → on, aria-pressed; available → disabled; tip → title) @param {any} c {view, p} */
  AZIONI.refresh = c => {
    const el = c.p.el; if (!el) return;
    for (const a of AZIONI.list(c, "barra")) {
      const b = /** @type {HTMLButtonElement | null} */ (el.querySelector(a.nodata ? ".x" : `.tbs [data-a="${a.bid}"]`)); if (!b) continue;
      const ok = !a.available || a.available(c), on = !!a.checked && a.checked(c) && ok;
      if (a.available) b.disabled = !ok;
      if (a.checked) { b.classList.toggle("on", on); if (a.toggle) b.setAttribute("aria-pressed", String(on)); }
      if (a.tip) b.title = a.tip(c);
    }
  };
})();
