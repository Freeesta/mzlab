// @ts-check
// Dragging files between the user groups of a list (side list now; the loading table binds the same helper later), the «Move to group» menu and the keyboard move.
// Lazy (carica("gruppi-dnd.js")): explore.js and lista-file.js call it on the first pointer down, drag or Ctrl+Shift+Arrow. The pure parts (hit test, companions) are tested by node.
// explore.js fetches it on the first pointer down and starts a drag when it is there (a mouse drag that began earlier joins at once); a native mouse drag also fires pointercancel, so
// only finger drags are cancelled by it; over a group header the header takes the highlight, not a row; a drag held by the finger or Esc ends without a reorder.
window.GDND = (() => {
  const EDGE = 40, STEP = 14, STALE = 4000, DWELL = 600, TALL = 40;
  /** @typedef {{ kind: string, key: string, el: Element | null, top: number, bottom: number }} Box */
  /** @typedef {{ list: HTMLElement, host: HTMLElement, rowSel: string, headSel: string, keyOf: (row: Element) => string, headKey: (head: Element) => string, valid: (head: Element, bases: string[]) => boolean, ticked: () => string[], onDrop: (d: { bases: string[], target: { kind: string, key: string } }) => any }} Cfg */
  /** @type {{ base: string, touch: boolean, last: number, shown: string, cancelled: boolean, y: number, dwell: { key: string, t0: number } | null } | null} */ let drag = null;
  /** @type {Cfg | null} */ let cfg = null;
  let pending = false, scrollTimer = 0, scrollDir = 0, docBound = false;
  const hosts = new WeakSet();

  /** «before» / «onto» / «after»: the thirds of a row (the outer two reorder as ever, the middle one drops ONTO the file). @param {number} top @param {number} bottom @param {number} y */
  const zone = (top, bottom, y) => { const h = (bottom - top) / 3; return y < top + h ? "before" : y >= bottom - h ? "after" : "onto"; };
  /** What is under height `y` of a list: boxes (headers and rows in document order, folded ones have no height) → a header, a row with its zone, the gap after the last box, or null above the list. @param {Box[]} boxes @param {number} y */
  function resolve(boxes, y) {
    const vis = boxes.filter(b => b.bottom > b.top);
    if (!vis.length) return { kind: "gap", key: "" };
    if (y < vis[0].top) return null;
    if (y >= vis[vis.length - 1].bottom) return { kind: "gap", key: "" };
    const b = vis.find(x => y >= x.top && y < x.bottom) || vis.filter(x => x.bottom <= y).pop();
    if (!b) return null;
    return b.kind === "head" ? { kind: "head", key: b.key, box: b } : { kind: "row", key: b.key, zone: zone(b.top, b.bottom, y), box: b };
  }
  /** The files that travel with the dragged one: every ticked file when the dragged one is ticked, else only it. @param {string} base @param {string[]} ticked */
  const companions = (base, ticked) => ticked.includes(base) ? [...new Set(ticked)] : [base];
  /** The header `dir` places from `from` among `heads` (keys), undefined at the ends. @param {string[]} heads @param {number} from @param {number} dir */
  const pick = (heads, from, dir) => from < 0 ? undefined : heads[from + dir];

  const live = () => {
    let n = document.getElementById("gdnd-live");
    if (!n) { n = document.createElement("div"); n.id = "gdnd-live"; n.className = "sr-only"; n.setAttribute("role", "status"); n.setAttribute("aria-live", "polite"); (document.getElementById("sb-panel-file") || document.body).appendChild(n); }
    return n;
  };
  const say = (/** @type {string} */ s) => { live().textContent = s; };
  const where = (/** @type {{ kind: string, key: string }} */ t) => t.key || I18N.t("groups.move.none");

  const boxes = (/** @type {Cfg} */ c = /** @type {Cfg} */ (cfg)) => {
    return [...c.list.querySelectorAll(c.headSel + "," + c.rowSel)].map(el => {
      const r = el.getBoundingClientRect(), head = el.matches(c.headSel);
      return { kind: head ? "head" : "row", key: head ? c.headKey(el) : c.keyOf(el), el, top: r.top, bottom: r.bottom };
    });
  };
  const basesNow = () => companions(/** @type {NonNullable<typeof drag>} */ (drag).base, /** @type {Cfg} */ (cfg).ticked());
  /** The drop target under height `y`: a header the files may join, or the gap after the last group (not with a finger); null for a row or an invalid header. @param {number} y @param {boolean} touch */
  function hit(y, touch) {
    const t = resolve(boxes(), y), c = /** @type {Cfg} */ (cfg), d = /** @type {NonNullable<typeof drag>} */ (drag);
    d.y = y;
    if (t?.kind === "row") return onto(t, touch);
    d.dwell = null;
    if (!t || (t.kind === "gap" && touch)) return null;
    if (t.kind === "head" && !c.valid(/** @type {Element} */ (t.box?.el), basesNow())) return null;
    return t;
  }
  /** The middle third of another file's row is a drop ONTO it; a finger, or a row lower than TALL px, must rest DWELL ms there first (a reorder passes over it). @param {{ key: string, zone?: string, box?: Box }} t @param {boolean} touch */
  function onto(t, touch) {
    const d = /** @type {NonNullable<typeof drag>} */ (drag), b = /** @type {Box} */ (t.box);
    if (t.zone !== "onto" || basesNow().includes(t.key)) { d.dwell = null; return null; }
    if (touch || b.bottom - b.top < TALL) {
      if (d.dwell?.key !== t.key) { d.dwell = { key: t.key, t0: Date.now() }; setTimeout(() => { if (drag === d && d.dwell?.key === t.key) show(hit(d.y, d.touch)); }, DWELL + 30); }
      if (Date.now() - /** @type {{ t0: number }} */ (d.dwell).t0 < DWELL) return null;
    }
    return { kind: "onto", key: t.key, box: b };
  }
  /** @param {{ kind: string, key: string, box?: Box } | null} t */
  function mark(t, c = /** @type {Cfg} */ (cfg)) {
    c.list.querySelectorAll(".gdnd-over,.gdnd-onto").forEach(x => x.classList.remove("gdnd-over", "gdnd-onto")); c.list.classList.remove("gdnd-gap");
    if (!t) return;
    if (t.kind === "gap") c.list.classList.add("gdnd-gap"); else t.box?.el?.classList.add(t.kind === "onto" ? "gdnd-onto" : "gdnd-over");
  }
  /** Highlights the target and says it once per change. @param {{ kind: string, key: string, box?: Box } | null} t */
  function show(t) {
    if (!drag) return;
    mark(t); const id = t ? t.kind + ":" + t.key : "";
    if (id === drag.shown) return;
    drag.shown = id; if (!t) return;
    const n = basesNow().length;
    if (t.kind === "onto") { const g = window.GRUPPI?.labelOf(t.key); say(g ? I18N.t("groups.dnd.over", { g, n }) : I18N.t("groups.dnd.overOnto", { f: t.key.replace(/\.[^.]+$/, "") })); return; }
    say(t.key ? I18N.t("groups.dnd.over", { g: t.key, n }) : I18N.t("groups.dnd.overNone", { n }));
  }
  /** Scrolls the list while the pointer is within EDGE px of the top or the bottom of its scroll box. @param {number} y */
  function edge(y) {
    let sc = /** @type {HTMLElement | null} */ (cfg ? cfg.list : null);
    while (sc && !(/(auto|scroll)/.test(getComputedStyle(sc).overflowY) && sc.scrollHeight > sc.clientHeight)) sc = sc.parentElement;
    const r = sc?.getBoundingClientRect(), dir = !r ? 0 : y < r.top + EDGE ? -1 : y > r.bottom - EDGE ? 1 : 0;
    if (dir === scrollDir) return;
    scrollDir = dir; clearInterval(scrollTimer); scrollTimer = 0;
    if (dir && sc) { const s = sc; scrollTimer = window.setInterval(() => { s.scrollTop += dir * STEP; }, 16); }
  }
  const stopScroll = () => { clearInterval(scrollTimer); scrollTimer = 0; scrollDir = 0; };
  /** Leaves the drag state and applies the repaint that file arrivals asked for meanwhile. */
  function release() {
    const was = pending; drag = null; pending = false; stopScroll();
    if (cfg) mark(null);
    if (was) { renderFileList(); if (typeof renderFiles === "function") renderFiles(); }
  }
  /** @param {{ kind: string, key: string }} t */
  async function finish(t) {
    const bases = basesNow(); drag = null; pending = false; stopScroll(); mark(null);
    const g = await /** @type {Cfg} */ (cfg).onDrop({ bases, target: { kind: t.kind, key: t.key } });
    say(t.kind === "onto" ? I18N.t("groups.dnd.done", { g }) : t.key ? I18N.t("groups.dnd.done", { g: t.key }) : I18N.t("groups.dnd.doneNone"));
  }
  function cancel() { if (!drag) return; release(); say(I18N.t("groups.dnd.cancel")); }
  /** Highlights what is under `y` and says whether it is a target. @param {number} y @param {boolean} touch */
  function over(y, touch) { const t = hit(y, touch); show(t); return !!t; }
  function begin(/** @type {string} */ base, /** @type {boolean} */ touch = false) { drag = { base, touch, last: Date.now(), shown: "", cancelled: false, y: 0, dwell: null }; }

  /** @param {HTMLElement} list @param {Partial<Cfg> & Pick<Cfg, "rowSel" | "headSel" | "keyOf" | "onDrop">} o @returns {Cfg} */
  const mkCfg = (list, o) => /** @type {Cfg} */ ({ list, host: list, headKey: h => /** @type {HTMLElement} */ (h).dataset.ug || "", valid: () => true, ticked: () => [...list.querySelectorAll(o.rowSel)].filter(r => r.querySelector("input[type=checkbox]:checked")).map(o.keyOf), ...o });
  /** Binds the delegated drag listeners of a list once (a second list, the loading table, has its own host; the drag in progress belongs to the list it started in). @param {HTMLElement} list @param {Partial<Cfg> & Pick<Cfg, "rowSel" | "headSel" | "keyOf" | "onDrop">} o */
  function bind(list, o) {
    cfg = mkCfg(list, o);
    live();
    // the listeners sit on the host (the list and the free space around it): the gap after the last group is a drop target
    const host = cfg.host;
    if (hosts.has(host)) return; hosts.add(host);
    const mine = () => !!drag && !!cfg && cfg.host === host;
    host.addEventListener("dragover", e => {
      if (!mine()) return;
      /** @type {NonNullable<typeof drag>} */ (drag).last = Date.now(); edge(e.clientY);
      const t = hit(e.clientY, false);
      if (!t) { show(null); return; }
      e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = "move"; show(t);
      // ONTO a file: the row's own reorder handlers (explore.js) stay out of it
      if (t.kind === "onto") { e.stopPropagation(); t.box?.el?.classList.remove("drop-before", "drop-after"); }
    }, true);
    host.addEventListener("drop", e => {
      if (!mine()) return;
      const t = hit(e.clientY, false);
      if (!t) { release(); return; }
      e.preventDefault(); if (t.kind === "onto") e.stopPropagation(); finish(t);
    }, true);
    host.addEventListener("dragleave", e => { if (mine() && !host.contains(/** @type {Node | null} */ (e.relatedTarget))) show(null); });
    if (docBound) return; docBound = true;
    document.addEventListener("dragend", e => { if (!drag) return; if (e.dataTransfer && e.dataTransfer.dropEffect === "none") cancel(); else release(); });
    // a finger drag has no native cancel: Esc marks it, the lift of the finger ends it without a drop
    document.addEventListener("keydown", e => { if (e.key === "Escape" && drag && drag.touch && !drag.cancelled) { drag.cancelled = true; mark(null); stopScroll(); say(I18N.t("groups.dnd.cancel")); } });
  }
  /** The options of the side list. */
  function side() {
    const list = /** @type {HTMLElement} */ (Q("#flst"));
    if (cfg && cfg.list === list) return;
    carica("gruppi.js"); bind(list, sideOpts());
  }
  /** A drop (or a keyboard move): ONTO a file = new group or join, else into the header's group; returns the group. @param {{ bases: string[], target: { kind?: string, key: string } }} d */
  const drop = async d => {
    await carica("gruppi.js");
    if (d.target.kind === "onto") return GRUPPI.onto(d.bases, d.target.key);
    GRUPPI.set(d.bases, d.target.key); return d.target.key;
  };
  const sideOpts = () => {
    const autoKey = (/** @type {string} */ b) => { const f = E.files.find(/** @param {any} x */ x => !x.gone && fileBase(x.file) === b); return f ? grpOf({ ...f, ugrp: undefined }) : ""; };
    return {
      host: /** @type {HTMLElement} */ (Q("#sb-panels")), rowSel: ".fl[data-k]:not(.pr)", headSel: ".fgh[data-gt], .fgh[data-gg]",
      keyOf: row => { const f = E.files[+(/** @type {HTMLElement} */ (row.querySelector(".nm"))).dataset.k]; return f ? fileBase(f.file) : ""; },
      // a user group takes any file; an automatic header (Full Scan, MRM...) only the files that belong there, so a file never changes its experiment type
      valid: (head, bases) => !!(/** @type {HTMLElement} */ (head)).dataset.ug || bases.every(b => autoKey(b) === /** @type {HTMLElement | null} */ (head.querySelector(".gall"))?.dataset.g),
      onDrop: drop,
    };
  };
  /** The loading table: a row is dragged alone (the ticks there mean «open it»), any group header takes it, «Ungrouped» (key "") frees it. */
  const tabOpts = () => ({
    host: /** @type {HTMLElement} */ (Q("#flist")), rowSel: "tr[data-nm]", headSel: "tr.tg-head", ticked: () => /** @type {string[]} */ ([]),
    keyOf: (/** @type {Element} */ row) => /** @type {HTMLElement} */ (row).dataset.nm || "",
    onDrop: drop,
  });
  function tab() {
    const list = /** @type {HTMLElement} */ (Q("#flist"));
    if (cfg && cfg.list === list) return;
    carica("gruppi.js"); bind(list, tabOpts());
  }
  /** «Group with…»: the keyboard way to a drop ONTO a file; the file is typed with the help of the browser's own type-ahead (a datalist on the prompt). @param {string[]} bases @param {string[]} pool */
  async function groupWith(bases, pool) {
    const names = [...new Set(pool)].filter(n => !bases.includes(n)), dl = document.createElement("datalist"), i = Q("#askin");
    dl.id = "gwith-list"; names.forEach(n => { const o = document.createElement("option"); o.value = n; dl.appendChild(o); });
    document.body.appendChild(dl); i.setAttribute("list", dl.id);
    const v = ((await ask(I18N.t("groups.with.prompt"), "")) || "").toLowerCase(); i.removeAttribute("list"); dl.remove();
    const hits = v ? names.filter(n => n.toLowerCase() === v) : [], part = v ? names.filter(n => n.toLowerCase().includes(v)) : [], to = hits[0] || (part.length === 1 ? part[0] : "");
    if (!to) { if (v) say(I18N.t("groups.with.none")); return; }
    await carica("gruppi.js"); say(I18N.t("groups.dnd.done", { g: await GRUPPI.onto(bases, to) }));
  }
  /** The groups of both lists: the labels of the side list's files and of the table's rows, in the order they were first met. */
  const labels = () => [...new Set([...E.files, ...ST.files].map(/** @param {any} x */ x => x.ugrp).filter(Boolean))];
  /** Files dragged in from the desktop over a user-group header (side list or table): the header takes the highlight, the drop promises the label to the files' rows (they get it as they arrive). */
  let extOver = /** @type {{ c: Cfg, key: string } | null} */ (null);
  /** @param {DragEvent} e */
  const extHit = e => {
    if (drag || !e.dataTransfer || ![...e.dataTransfer.types].includes("Files")) return null;
    for (const [sel, opts] of /** @type {[string, () => any][]} */ ([["#flist", tabOpts], ["#flst", sideOpts]])) {
      const list = /** @type {HTMLElement | null} */ (document.querySelector(sel)); if (!list || !list.offsetParent) continue;
      const c = mkCfg(list, opts()), r = c.host.getBoundingClientRect();
      if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) continue;
      const t = resolve(boxes(c), e.clientY);
      if (t && t.kind === "head" && t.key) return { c, t };
    }
    return null;
  };
  const extClear = () => { if (extOver) mark(null, extOver.c); extOver = null; };
  // (the node tests load this file without a page)
  if (typeof document !== "undefined" && document.addEventListener) {
  document.addEventListener("dragover", e => {
    const h = extHit(e);
    if (!h) { extClear(); return; }
    if (!extOver || extOver.key !== h.t.key || extOver.c.list !== h.c.list) { extClear(); mark(h.t, h.c); extOver = { c: h.c, key: h.t.key }; say(I18N.t("groups.dnd.overFiles", { g: h.t.key })); }
  }, true);
  document.addEventListener("dragleave", e => { if (extOver && !e.relatedTarget) extClear(); }, true);
  document.addEventListener("drop", e => {
    const h = extHit(e); extClear();
    if (!h || !e.dataTransfer) return;
    const names = [...e.dataTransfer.files].map(f => f.name), g = h.t.key;
    // the drop itself goes on as usual (the page's own drop handlers load the files); the rows take the label when they arrive
    Promise.all([carica("gruppi.js"), carica("gruppi-tab.js")]).then(() => { GRUPPI.expect(names, g); say(I18N.t("groups.dnd.doneFiles", { g, n: names.length })); });
  }, true);
  }
  return {
    zone, resolve, companions, pick, bind,
    /** The drag of the row of file `k` starts (mouse dragstart or touch long press). @param {number} k @param {boolean} [touch] */
    start(k, touch) { side(); begin(fileBase(E.files[k].file), !!touch); },
    begin, over,
    /** The drag of the table row of file `name` starts (the grip's dragstart). @param {string} name */
    startT(name) { tab(); begin(name); },
    /** True while a drag holds the list: renderFileList waits (the dragged row must stay in the page); a drag silent for STALE ms no longer counts. */
    busy: () => !!drag && Date.now() - drag.last < STALE,
    defer() { pending = true; },
    cancel,
    /** Touch: the pointer moved to height `y` (true over a drop target). @param {number} y */
    move(y) { if (!drag || drag.cancelled) return false; drag.last = Date.now(); edge(y); return over(y, true); },
    /** Touch: the finger lifted; true when the drag was handled here (dropped on a header, or cancelled), false for the row reorder of the caller. @param {number} y */
    end(y) {
      if (!drag) return false;
      if (drag.cancelled) { release(); return true; }
      const t = hit(y, true);
      if (!t) { release(); return false; }
      finish(t); return true;
    },
    /** Ctrl+Shift+↑/↓ on a row: the selection moves to the previous/next group header of the list. @param {Element} row @param {number} dir */
    async step(row, dir) {
      side(); const c = /** @type {Cfg} */ (cfg), base = c.keyOf(row), bases = companions(base, c.ticked());
      const heads = [...c.list.querySelectorAll(c.headSel)].filter(h => c.valid(h, [base]));
      const from = heads.indexOf(/** @type {Element} */ (row.closest(".grp-files")?.previousElementSibling)), to = pick(heads.map(c.headKey), from, dir);
      if (to === undefined) { say(I18N.t("groups.dnd.edge")); return; }
      await c.onDrop({ bases, target: { kind: "head", key: to } });
      say(to ? I18N.t("groups.dnd.done", { g: to }) : I18N.t("groups.dnd.doneNone"));
      [...c.list.querySelectorAll(c.rowSel)].find(r => c.keyOf(r) === base)?.querySelector("input")?.focus();
    },
    /** «Move to group ▸» for a table row: the same menu, anchored to the click. @param {any} ev @param {string} name */
    async moveMenuT(ev, name) {
      tab(); await carica("gruppi.js"); const groups = labels(), some = ST.files.some(/** @param {any} x */ x => x.name === name && x.ugrp);
      menu(ev, [...groups.map(g => ({ label: g, fn: () => { GRUPPI.set([name], g); say(I18N.t("groups.dnd.done", { g })); } })), ...(groups.length ? ["-"] : []),
        { label: I18N.t("groups.with.menu"), fn: () => groupWith([name], ST.files.map(/** @param {any} x */ x => x.name)) },
        { label: I18N.t("groups.move.new"), fn: async () => { const v = ((await ask(I18N.t("groups.move.prompt"), await GRUPPI.newName([name]))) || "").trim(); if (v && GRUPPI.set([name], v)) say(I18N.t("groups.dnd.done", { g: v })); } },
        ...(some ? [{ label: I18N.t("groups.move.none"), fn: () => { GRUPPI.set([name], ""); say(I18N.t("groups.dnd.doneNone")); } }] : [])]);
    },
    /** The «Move to group ▸» entries of a row menu, as a second menu at the same place (the menu has no sub-menus). @param {any} ev @param {any} f */
    async moveMenu(ev, f) {
      side(); await carica("gruppi.js"); const c = /** @type {Cfg} */ (cfg), base = fileBase(f.file), bases = companions(base, f.vis ? c.ticked() : []), groups = [...new Set(flSide().map(/** @param {any} x */ x => x.ugrp).filter(Boolean))];
      const some = flSide().some(/** @param {any} x */ x => bases.includes(fileBase(x.file)) && x.ugrp);
      menu(ev, [...groups.map(g => ({ label: g, fn: () => { GRUPPI.set(bases, g); say(I18N.t("groups.dnd.done", { g })); } })), ...(groups.length ? ["-"] : []),
        { label: I18N.t("groups.with.menu"), fn: () => groupWith(bases, flSide().map(/** @param {any} x */ x => fileBase(x.file))) },
        { label: I18N.t("groups.move.new"), fn: async () => { const v = ((await ask(I18N.t("groups.move.prompt"), await GRUPPI.newName(bases))) || "").trim(); if (v && GRUPPI.set(bases, v)) say(I18N.t("groups.dnd.done", { g: v })); } },
        ...(some ? [{ label: I18N.t("groups.move.none"), fn: () => { GRUPPI.set(bases, ""); say(I18N.t("groups.dnd.doneNone")); } }] : [])]);
    },
  };
})();
