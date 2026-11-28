// @ts-strict
"use strict";
// Panel header buttons as registry actions (barra, scope "panel"): ONE renderer AZIONI.bar, state by AZIONI.refresh.
// The shared bar (pbar.js) draws them too (`pb` groups them); `bonly` rows exist only there.
(() => {
  const T = (/** @type {string} */ k) => I18N.t(k), G = ["dd", "xb", "v", "l", "s", "i", "t", "x", "tb", "o", "xp", "tp"], LENS = ["chrom", "xic", "mrm"], ALL = [...LENS, "spec", "map"], MS = (/** @type {Ctx} */ c) => ms2Chrom(c.p);
  /** map zoom and gestures (the hidden header twin is .mznav) @param {string} id */
  const nav = id => ({ bonly: 1, run: (/** @type {Ctx} */ c) => MAPPA.runCmd(id, MAPPA.ad2(c.p, null)) });
  const navIc = (/** @type {string} */ id) => () => MAPPA.SV(MAPPA.ICO[/** @type {any} */ (AZIONI.get(id)).ic]);
  const ic = (/** @type {string} */ w, /** @type {string} */ d) => `<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="${w}" aria-hidden="true" focusable="false">${d}</svg>`;
  const IC_LENS = ic("1.5", '<circle cx="8" cy="8" r="4.5"/><path d="M8 1v4M8 11v4M1 8h4M11 8h4"/>'), IC_TAB = ic("1.4", '<rect x="1.5" y="2.5" width="13" height="11" rx="1.2"/><path d="M1.5 6h13M1.5 9.7h13M6 6v7.5"/>');
  const shift = (/** @type {Ctx} */ c) => !!(c.e && c.e.shiftKey), uv = (/** @type {Ctx} */ c) => !!c.p.uv, xw = (/** @type {Ctx} */ c) => xwc().then((/** @type {any} */ m) => m.press(c.p, shift(c))), lens = (/** @type {Ctx} */ c) => !!window.XICBTN && XICBTN.lens.ok(c.p);
  const pop = (/** @type {Ctx} */ c) => xwc().then((/** @type {any} */ m) => m.pop(c.p, c.b)), B = ' type="button"', P = ' aria-haspopup="dialog" aria-expanded="false"';
  const W = (/** @type {string} */ n) => /** @type {any} */ (window)[n];
  const mode = (/** @type {string} */ m) => (/** @type {Ctx} */ c) => c.p._mode(m);
  /** @typedef {any} Ctx
   * @typedef {(c: Ctx) => any} F @typedef {{ toggle?: number, bonly?: number, hid?: number, gh?: number, bid?: string, visible?: F, at?: string, nodata?: number, tag?: string, after?: () => string, ti0?: F, tip?: F, checked?: F, available?: F, run?: F }} Opt */
  // [data-a, views, group, title key, aria key ("=": title), text or icon, class, Opt]
  /** @type {[string, string[], string, string, string, string | ((c: Ctx) => string), string, Opt][]} */
  const ROWS = [
    ["ddaw", ["spec"], "dd", "dda.win.wait", "", "", "bt ddaw", { hid: 1, gh: 1, at: ' aria-haspopup="listbox" aria-expanded="false"', available: c => !!c.p._dws, run: c => W("DDAELENCO").open(c.p, c.b) }],
    ["ms2tri", ["chrom", "xic"], "dd", "ctl.ms2tri.title", "ctl.ms2tri", () => T("dda.tri.btn"), "bt tgl", { hid: 1, gh: 1, toggle: 1, at: B, available: c => !!c.p._tri, checked: c => !!c.p.ms2tri, run: c => W("DDAELENCO").flip(c.p) }],
    ["xicb", ["spec"], "xb", "xicbtn.title", "xicbtn.aria", "XIC", "bt xbb", { toggle: 1, run: c => XICBTN.press(c.p, shift(c)) }],
    ["xicl", ["spec"], "xb", "xicbtn.ion.title", "", "", "bt xbl", { at: " hidden" + P, run: c => XICBTN.openPop(c.p) }],
    ["xwc", ["spec"], "xb", "panel.wl.title", "", "XWC", "bt xbb", { visible: uv, at: B, run: xw }],
    ["xwcs", ["spec"], "xb", "xwc.pop.open", "=", "▾", "bt xbl", { bid: "xwcl", visible: uv, at: B + " hidden" + P, run: pop }],
    ["izoom", LENS, "v", "panel.izoom.title", "", () => IC_ZOOM, "bt", { checked: c => c.p.imode === "zoom", run: mode("zoom") }],
    ["fit", ALL, "v", "panel.fit.title", "", () => IC_FIT, "bt", { at: " disabled", run: c => { const p = c.p; pushZh(p); p.zoom = null; p.zoomY = null; if (p.type === "spec") p.lock = null; draw(p); uiSave(); } }],
    ...["mapzin", "mapzout", "maphelp"].map(id => /** @type {any} */ ([id, ["map"], "v", `cmd.${id}.tip`, `cmd.${id}.name`, navIc(id), "bt", nav(id)])),
    ["tlink", LENS, "l", "panel.tlink.title", "panel.tlink.aria", () => IC_SYNC, "bt", { toggle: 1, checked: c => !!c.p.tl, run: c => toggleTl(c.p) }],
    ["rul", ["spec"], "s", "panel.rul.title", "", () => IC_RULER, "bt", { checked: c => !!c.p.rul, run: c => { const p = c.p; COMMANDS.run("spec.ruler", p, () => { p.rul = !p.rul; if (!p.rul && p.meas && p.meas.ref != null && !p.meas.list.length) { p.meas = null; draw(p); } }); AZIONI.refresh(c); } }],
    ["par", ["spec"], "s", "panel.par.title", "panel.par.aria", "&#9662;", "bt", { run: c => specParams(c.p, c.b) }],
    ["rel", ["spec"], "s", "panel.rel.title", "", "%", "bt", { checked: c => !!specRel(c.p), run: c => { const p = c.p; COMMANDS.run("spec.display", p, () => { p.rel = !specRel(p); }); AZIONI.refresh(c); draw(p); uiSave(); } }],
    ["lock", ["spec"], "s", "lock.off.title", "=", () => IC_UNLOCK, "bt", { toggle: 1, run: c => toggleLock(c.p) }],
    ["iauto", LENS, "i", "panel.iauto.title", "", () => IC_AUTO, "bt", { checked: c => c.p.imode === "auto", run: mode("auto") }],
    ["iall", LENS, "i", "panel.iall.title", "=", "&#9662;", "bt iall", { at: B + ' aria-haspopup="true"', run: c => intMenu(c.p, c.e) }],
    ["iman", LENS, "i", "panel.iman.title", "", () => IC_MAN, "bt", { checked: c => c.p.imode === "man", run: mode("man") }],
    ["idel", LENS, "i", "panel.idel.title", "", () => IC_DEL_INT, "bt", { checked: c => c.p.imode === "idel", run: mode("idel") }],
    ["intf", LENS, "i", "panel.intf.title", "", "", "bt", { tag: "select", at: " hidden", run: c => { c.p.intf = c.e.target.value; } }],
    ["iclr", LENS, "i", "panel.iclr.title", "", () => T("panel.iclr"), "bt", { at: " hidden", run: c => { if (c.p.ints.length) delInts(c.p, [...c.p.ints]); } }],
    ["smooth", LENS, "t", "ctl.smooth.title", "ctl.smooth", () => IC_SMOOTH, "bt tgs", { toggle: 1, at: B, checked: c => !!c.p.smooth, tip: c => T(ESP.mrm({ type: c.p.type }) ? "ctl.mrm.smooth.title" : "ctl.smooth.title"),
      run: c => { const p = c.p; pushFields(p, "history.smooth"); p.smooth = !p.smooth; AZIONI.refresh(c); draw(p); uiSave(); } }],
    ["log", LENS, "t", "ctl.log.title", "ctl.log.aria", () => T("ctl.log"), "bt tgl", { toggle: 1, at: B, checked: c => !!c.p.log, available: c => !(MS(c) || (c.p.type === "chrom" && c.p.kind === "pda")), tip: c => T(MS(c) ? "ctl.noMs2" : "ctl.log.title"),
      run: c => { const p = c.p; COMMANDS.run("panel.source", p, () => { p.log = !p.log; }); AZIONI.refresh(c); draw(p); uiSave(); } }],
    ["xic", ["chrom"], "x", "panel.xic.title", "", c => c.p.kind === "pda" ? "XWC" : "XIC", "bt", { available: c => !MS(c), ti0: c => T(c.p.kind === "pda" ? "panel.wl.title" : MS(c) ? "panel.xic.off" : "panel.xic.title"), run: c => c.p.kind === "pda" ? xw(c) : openXic(null, { after: c.p }) }],
    ["xwcl", ["chrom"], "x", "xwc.pop.open", "=", "▾", "bt xbl", { at: B + " hidden" + P, run: pop }],
    ["ticbg", ["chrom"], "tb", "ctl.ticbg.on", "=", "", "bt tgl", { hid: 1, gh: 1, toggle: 1, at: B + P, available: c => W("TICBG")?.applies(c.p), checked: c => !!W("TICBG")?.on(c.p), run: c => W("TICBG").press(c.p) }],
    ["dl", ALL, "o", "panel.dl.title", "=", () => IC_DL.replace(";margin-right:4px", ""), "bt", { run: c => dlMenu(c.p, c.e) }],
    ["xicp", ["spec"], "xp", "xiclens.toggle", "=", IC_LENS, "bt xbp", { toggle: 1, hid: 1, visible: c => !uv(c), checked: c => !!UIP.xicLens && lens(c), available: lens, run: () => XICBTN.setLens(!UIP.xicLens) }],
    ["tabp", ["spec"], "tp", "tabp.btn.title", "=", IC_TAB, "bt tpb", { at: B + P, run: c => TABPICCHI.open(c.p) }],
    ["more", ALL, "e", "panel.more.title", "=", "⋯", "bt hmb", { at: B + ' hidden aria-haspopup="true" aria-expanded="false"', after: () => h`<span class="hmore" hidden role="group" aria-label="${T("panel.more.title")}"></span>` }],
    ["max", ALL, "e", "panel.max.title", "", () => IC_FS, "bt fsb", { run: c => c.p._setMax(!c.p.el.classList.contains("max")) }],
    ["close", ALL, "e", "panel.close", "", "&times;", "x", { nodata: 1, run: c => closePanelWithUndo(c.p) }],
  ];
  for (const [id, views, bar, ti, al, tx, cl, o] of ROWS) AZIONI.register({ id: "panel." + id, group: "tools", views, menu: false, barra: true, scope: "panel", bar, bid: id, ti, al, tx, cl, name: () => T(ti), svg: () => "", ...o });
  const html = (a, c, shared) => {
    const ok = !a.available || a.available(c), on = a.checked ? !!a.checked(c) : false, tip = (a.tip || a.ti0 || (() => T(a.ti)))(c), al = a.al === "=" ? tip : a.al ? T(a.al) : "", tag = a.tag || "button", tx = typeof a.tx === "function" ? a.tx(c) : a.tx;
    return h`<${raw(tag)} class="${a.cl + (on ? " on" : "")}"${raw((a.nodata ? "" : ` data-a="${a.bid}"`) + (a.toggle ? ` aria-pressed="${on}"` : "") + (a.at || "") + (ok ? "" : shared || a.hid ? " hidden" : " disabled"))}${raw(al ? h` aria-label="${al}"` : "")} title="${tip}">${raw(tx)}</${raw(tag)}>${raw(a.after ? a.after() : "")}`;
  };
  const wire = (l, c, host) => {
    for (const a of l) {
      const b = /** @type {HTMLElement | null} */ (host.querySelector(a.nodata ? ".x" : `[data-a="${a.bid}"]`));
      if (b && a.run) b[a.tag ? "onchange" : "onclick"] = (/** @type {Event} */ e) => AZIONI.run(a.id, { ...c, e, b });
    }
  };
  /** the header buttons into `host` (`.tbs`); with `pb` ({of, groups, wrap}) the groups of the shared bar, unavailable ones hidden @param {any} c {view, p} @param {HTMLElement} host @param {any} [pb] */
  AZIONI.bar = (c, host, pb) => {
    const l = AZIONI.list(c, "barra").filter(a => pb ? pb.of(a) : !a.bonly), g = (/** @type {string} */ k) => l.filter(a => (pb ? pb.of(a) : a.bar) === k).map(a => html(a, c, pb)).join("");
    host.innerHTML = pb ? pb.groups.map((/** @type {string[]} */ [k, t]) => { const s = g(k); return s ? pb.wrap(k, t, s) : ""; }).join("") : G.map(k => { const s = g(k); return s ? h`<span class="tbg">${raw(s)}</span>` : ""; }).join("") + g("e");
    wire(l, c, host);
  };
  AZIONI.refresh = c => {
    const el = c.p.el; if (!el) return;
    for (const a of AZIONI.list(c, "barra")) {
      const b = /** @type {HTMLButtonElement | null} */ (el.querySelector(a.nodata ? ".x" : `.tbs [data-a="${a.bid}"]`)); if (!b) continue;
      const ok = !a.available || a.available(c), on = !!a.checked && a.checked(c) && ok;
      if (a.available) { if (a.hid) b.hidden = !ok; else b.disabled = !ok; }
      const g = a.gh && b.parentElement; if (g && g.hidden === !!ok) { g.hidden = !ok; fitHdSoon(c.p); }   // the group goes with its only button
      if (a.checked) { b.classList.toggle("on", on); if (a.toggle) b.setAttribute("aria-pressed", String(on)); }
      if (a.tip) b.title = a.tip(c);
    }
  };
  setTimeout(() => loadScript("panel-menu.js"), 1000);
})();
