// @ts-check
// Dragging files between the user groups of a list (side list now; the loading table binds the same helper later), the «Move to group» menu and the keyboard move.
// Lazy (carica("gruppi-dnd.js")): explore.js and lista-file.js call it on the first pointer down, drag or Ctrl+Shift+Arrow. The pure parts (hit test, companions) are tested by node.
window.GDND = (() => {
  const EDGE = 40, STEP = 14, STALE = 4000;
  /** @typedef {{ kind: string, key: string, el: Element | null, top: number, bottom: number }} Box */
  /** @typedef {{ list: HTMLElement, host: HTMLElement, rowSel: string, headSel: string, keyOf: (row: Element) => string, headKey: (head: Element) => string, valid: (head: Element, bases: string[]) => boolean, ticked: () => string[], onDrop: (d: { bases: string[], target: { kind: string, key: string } }) => any }} Cfg */
  /** @type {{ base: string, touch: boolean, last: number, shown: string, cancelled: boolean } | null} */ let drag = null;
  /** @type {Cfg | null} */ let cfg = null;
  let pending = false, scrollTimer = 0, scrollDir = 0, bound = false;

  /** «before» / «onto» / «after»: the thirds of a row (03 reorders on the outer two and on the middle one alike; 05 will drop ONTO the file). @param {number} top @param {number} bottom @param {number} y */
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

  const boxes = () => {
    const c = /** @type {Cfg} */ (cfg);
    return [...c.list.querySelectorAll(c.headSel + "," + c.rowSel)].map(el => {
      const r = el.getBoundingClientRect(), head = el.matches(c.headSel);
      return { kind: head ? "head" : "row", key: head ? c.headKey(el) : c.keyOf(el), el, top: r.top, bottom: r.bottom };
    });
  };
  const basesNow = () => companions(/** @type {NonNullable<typeof drag>} */ (drag).base, /** @type {Cfg} */ (cfg).ticked());
  /** The drop target under height `y`: a header the files may join, or the gap after the last group (not with a finger); null for a row or an invalid header. @param {number} y @param {boolean} touch */
  function hit(y, touch) {
    const t = resolve(boxes(), y), c = /** @type {Cfg} */ (cfg);
    if (!t || t.kind === "row" || (t.kind === "gap" && touch)) return null;
    if (t.kind === "head" && !c.valid(/** @type {Element} */ (t.box?.el), basesNow())) return null;
    return t;
  }
  /** @param {{ kind: string, key: string, box?: Box } | null} t */
  function mark(t) {
    const c = /** @type {Cfg} */ (cfg);
    c.list.querySelectorAll(".gdnd-over").forEach(x => x.classList.remove("gdnd-over")); c.list.classList.remove("gdnd-gap");
    if (!t) return;
    if (t.kind === "gap") c.list.classList.add("gdnd-gap"); else t.box?.el?.classList.add("gdnd-over");
  }
  /** Highlights the target and says it once per change. @param {{ kind: string, key: string, box?: Box } | null} t */
  function show(t) {
    if (!drag) return;
    mark(t); const id = t ? t.kind + ":" + t.key : "";
    if (id === drag.shown) return;
    drag.shown = id; if (!t) return;
    const n = basesNow().length;
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
    if (was) renderFileList();
  }
  /** @param {{ kind: string, key: string }} t */
  async function finish(t) {
    const bases = basesNow(); drag = null; pending = false; stopScroll(); mark(null);
    await /** @type {Cfg} */ (cfg).onDrop({ bases, target: { kind: t.kind, key: t.key } });
    say(t.key ? I18N.t("groups.dnd.done", { g: t.key }) : I18N.t("groups.dnd.doneNone"));
  }
  function cancel() { if (!drag) return; release(); say(I18N.t("groups.dnd.cancel")); }
  /** Highlights what is under `y` and says whether it is a target. @param {number} y @param {boolean} touch */
  function over(y, touch) { const t = hit(y, touch); show(t); return !!t; }
  function begin(/** @type {string} */ base, /** @type {boolean} */ touch = false) { drag = { base, touch, last: Date.now(), shown: "", cancelled: false }; }

  /** Binds the delegated drag listeners of a list once. @param {HTMLElement} list @param {Partial<Cfg> & Pick<Cfg, "rowSel" | "headSel" | "keyOf" | "onDrop">} o */
  function bind(list, o) {
    cfg = { list, host: list, headKey: h => /** @type {HTMLElement} */ (h).dataset.ug || "", valid: () => true, ticked: () => [...list.querySelectorAll(o.rowSel)].filter(r => r.querySelector("input[type=checkbox]:checked")).map(o.keyOf), ...o };
    live();
    if (bound) return; bound = true;
    // the listeners sit on the host (the list and the free space around it): the gap after the last group is a drop target
    const host = cfg.host;
    host.addEventListener("dragover", e => {
      if (!drag) return;
      drag.last = Date.now(); edge(e.clientY);
      const t = hit(e.clientY, false);
      if (!t) { show(null); return; }
      e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = "move"; show(t);
    });
    host.addEventListener("drop", e => {
      if (!drag) return;
      const t = hit(e.clientY, false);
      if (!t) { release(); return; }
      e.preventDefault(); finish(t);
    });
    host.addEventListener("dragleave", e => { if (drag && !host.contains(/** @type {Node | null} */ (e.relatedTarget))) show(null); });
    document.addEventListener("dragend", e => { if (!drag) return; if (e.dataTransfer && e.dataTransfer.dropEffect === "none") cancel(); else release(); });
    // a finger drag has no native cancel: Esc marks it, the lift of the finger ends it without a drop
    document.addEventListener("keydown", e => { if (e.key === "Escape" && drag && drag.touch && !drag.cancelled) { drag.cancelled = true; mark(null); stopScroll(); say(I18N.t("groups.dnd.cancel")); } });
  }
  /** The options of the side list. */
  function side() {
    const list = /** @type {HTMLElement} */ (Q("#flst"));
    if (cfg && cfg.list === list) return;
    carica("gruppi.js");
    const autoKey = (/** @type {string} */ b) => { const f = E.files.find(/** @param {any} x */ x => !x.gone && fileBase(x.file) === b); return f ? grpOf({ ...f, ugrp: undefined }) : ""; };
    bind(list, {
      host: /** @type {HTMLElement} */ (Q("#sb-panels")), rowSel: ".fl[data-k]:not(.pr)", headSel: ".fgh[data-gt], .fgh[data-gg]",
      keyOf: row => { const f = E.files[+(/** @type {HTMLElement} */ (row.querySelector(".nm"))).dataset.k]; return f ? fileBase(f.file) : ""; },
      // a user group takes any file; an automatic header (Full Scan, MRM...) only the files that belong there, so a file never changes its experiment type
      valid: (head, bases) => !!(/** @type {HTMLElement} */ (head)).dataset.ug || bases.every(b => autoKey(b) === /** @type {HTMLElement | null} */ (head.querySelector(".gall"))?.dataset.g),
      onDrop: async d => { await carica("gruppi.js"); GRUPPI.set(d.bases, d.target.key); },
    });
  }
  /** The name of a new group: the common tokens of the names (engine), else «Group N». @param {string[]} names */
  async function newName(names) {
    const used = new Set(E.files.map(/** @param {any} x */ x => x.ugrp));
    const ans = await fetch("api/campioni/nome_gruppo", { method: "POST", body: JSON.stringify({ names }) }).then(r => r.json()).catch(() => null);
    let n = 1; while (used.has(I18N.t("files.ugrp.name", { n }))) n++;
    return ans && !ans.error && ans.nome ? String(ans.nome) : I18N.t("files.ugrp.name", { n });
  }
  return {
    zone, resolve, companions, pick,
    /** The drag of the row of file `k` starts (mouse dragstart or touch long press). @param {number} k @param {boolean} [touch] */
    start(k, touch) { side(); begin(fileBase(E.files[k].file), !!touch); },
    begin, over,
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
    /** The «Move to group ▸» entries of a row menu, as a second menu at the same place (the menu has no sub-menus). @param {any} ev @param {any} f */
    async moveMenu(ev, f) {
      side(); await carica("gruppi.js"); const c = /** @type {Cfg} */ (cfg), base = fileBase(f.file), bases = companions(base, f.vis ? c.ticked() : []), groups = [...new Set(flSide().map(/** @param {any} x */ x => x.ugrp).filter(Boolean))];
      const some = flSide().some(/** @param {any} x */ x => bases.includes(fileBase(x.file)) && x.ugrp);
      menu(ev, [...groups.map(g => ({ label: g, fn: () => { GRUPPI.set(bases, g); say(I18N.t("groups.dnd.done", { g })); } })), ...(groups.length ? ["-"] : []),
        { label: I18N.t("groups.move.new"), fn: async () => { const v = ((await ask(I18N.t("groups.move.prompt"), await newName(bases))) || "").trim(); if (v) { GRUPPI.set(bases, v); say(I18N.t("groups.dnd.done", { g: v })); } } },
        ...(some ? [{ label: I18N.t("groups.move.none"), fn: () => { GRUPPI.set(bases, ""); say(I18N.t("groups.dnd.doneNone")); } }] : [])]);
    },
  };
})();
