// @ts-check
// The right-click menu of a panel kind, built from the actions/1 registry (azioni.js): point actions, a separator, then panel actions with their key,
// the same commands as the panel bar (#pbar); at most 12 rows at the top level, the tail in ONE submenu. Lazy (loadScript("panel-menu.js")): azioni-barra.js asks
// for it when the page is idle after start-up; a right click that comes first waits for it (ctxFor in eventi.js, the map menu in mappa.js), one tick later.
/** @typedef {{ view: string, [k: string]: any }} Ctx */
window.PANELMENU = (() => {
  const MAX_ROWS = 12;
  /** the bar tools the menu repeats (one registry, two views): [short menu label, icon]; the ones that open a popover on their own button stay in the bar */
  const MENU = /** @type {Record<string, [string, string]>} */ ({
    izoom: ["ctx.panel.izoom", "fit"], iauto: ["ctx.panel.iauto", "area"], iman: ["ctx.panel.iman", "measure"], idel: ["ctx.panel.idel", "trash"], tlink: ["ctx.panel.tlink", "link"],
    smooth: ["ctx.panel.smooth", "filter"], log: ["ctx.panel.log", "gauge"], rul: ["ctx.panel.rul", "measure"], rel: ["ctx.panel.rel", "flag"], lock: ["ctx.panel.lock", "lock"],
    tabp: ["ctx.panel.tabp", "list"], dl: ["ctx.panel.dl", "save"], ms2tri: ["ctx.panel.ms2tri", "ms2"], area: ["ctx.panel.area", "area"] });
  const T = (/** @type {string} */ k) => I18N.t(k), A = /** @type {any} */ (AZIONI);
  /** the download menu opens where the right click was; run from the menu it is deferred (the click on the entry would close it at once) @param {any} c */
  const dlAt = c => { const e = c.e; if (e && e.type === "click") return dlMenu(c.p, e); setTimeout(() => dlMenu(c.p, /** @type {any} */ ({ preventDefault() {}, stopPropagation() {}, clientX: e ? e.clientX : 0, clientY: e ? e.clientY : 0, kbd: !!(e && e.kbd) })), 0); };
  /** scope "point" (under the pointer, the default) or "panel" (the whole panel); a bar tool listed in MENU also gets its place in the menu @param {any} a */
  const decorate = a => {
    if (!["point", "panel"].includes(a.scope ||= "point")) throw new Error("AZIONI: unknown scope " + a.scope + " of " + a.id);
    const m = a.id.startsWith("panel.") && MENU[a.id.slice(6)];
    if (!m) return;
    const lab = () => T(m[0]);
    Object.assign(a, { menu: true, menuLabel: lab, aria: lab, tip: () => T(a.ti), iconKey: m[1] });
    if (a.id === "panel.dl") a.run = dlAt;
  };
  /** full screen is a panel action in the menu, except on the maps (they have their own) */
  Object.assign(A.get("view.fullscreen") || {}, { scope: "panel", menu: true, visible: (/** @type {Ctx} */ c) => c.view !== "map" && c.view !== "map3d" });
  A.actions.forEach(decorate);
  const register = A.register;
  A.register = (/** @type {any} */ a) => { decorate(a); return register(a); };
  /** block of an action in the menu: 0 = point actions, 1 = panel commands of the menu, 2 = panel commands that come from the bar @param {any} a */
  const block = a => (a.scope === "panel" ? (a.barra ? 2 : 1) : 0);
  /** @param {any} it an entry, or the icon row of several @returns {any[]} */
  const parts = it => it.fila || [it];
  const forContext = A.forContext;
  A.forContext = (/** @type {Ctx} */ c) => {
    const rows = forContext(c).filter((/** @type {any} */ it) => it !== "-").map((/** @type {any} */ it) => {
      const a = A.get(parts(it)[0].id);
      parts(it).forEach((/** @type {any} */ e) => { e.scope = a.scope; if (a.shortcut) e.key = A.keyText(a.shortcut); });
      return { it, sec: block(a) + a.group, b: block(a) };
    }).sort((x, y) => x.b - y.b);
    const out = /** @type {any[]} */ ([]);
    rows.forEach((r, i) => { if (i && r.sec !== rows[i - 1].sec) out.push("-"); out.push(r.it); });
    return out;
  };
  const clean = A.clean;
  A.clean = (/** @type {any[]} */ items) => {
    let out = clean(items);
    const rows = out.filter(it => it !== "-").length;
    // only a menu built from the registry (its entries carry a `scope`) is capped: the menu of the drawing has its own rows
    if (rows <= MAX_ROWS || !out.some(it => it !== "-" && parts(it)[0].scope)) return out;
    // from the end: only a plain entry can move (an entry that already has a submenu, a grey row or the readout row stays)
    const move = new Set(), need = rows - (MAX_ROWS - 1);
    for (let i = out.length - 1; i >= 0 && move.size < need; i--) { const it = out[i]; if (it !== "-" && !it.sub && !it.dim && (it.fn || it.fila)) move.add(it); }
    if (move.size < 2) return out;
    const sub = /** @type {any[]} */ ([]), rest = /** @type {any[]} */ ([]), name = T("ctx.more");
    for (const it of out) {
      if (!move.has(it)) { rest.push(it); continue; }
      if (!sub.length) rest.push({ id: "menu.more", scope: "panel", label: name, aria: name, icon: AZIONI_ICONE.svg("list"), check: null, dim: false, off: false, fn: () => {}, sub: () => sub.flatMap(m => parts(m).map(x => ({ name: x.label, aria: x.aria, tip: x.tip, check: x.check, key: x.key, run: () => x.fn() }))) });
      sub.push(it);
    }
    return clean(rest);
  };
  return { MENU, MAX_ROWS };
})();
