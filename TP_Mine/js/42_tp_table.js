// @ts-check
// TPMINE-PRIVATE  TPTAB: the table of the products of mzFinder, a list that sorts and filters with the main numbers of every row and opens the sheet of the product
// right under the row (one row open at a time). The page of the engine (21_tpmine_hr.js) gives the columns, the rows and what to put in the sheet; this file only shows.
// Keyboard: Tab to a row button, Enter or Space opens and closes, ↑ ↓ Home End move between the rows, Esc closes; the headers are buttons (aria-sort). Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const CSS = `
.tp .tpt-wrap{overflow-x:auto;border:1px solid var(--line);border-radius:6px}.tp table.tpt{border-collapse:collapse;width:100%;font-size:12px}
.tp table.tpt th{position:static;white-space:nowrap}.tp table.tpt th button.tpt-sh{all:unset;box-sizing:border-box;cursor:pointer;display:inline-flex;gap:4px;align-items:center;min-height:24px;min-width:24px;font-weight:600;color:var(--muted);padding:0 2px}
.tp table.tpt th button.tpt-sh:hover,.tp table.tpt th[aria-sort="ascending"] button.tpt-sh,.tp table.tpt th[aria-sort="descending"] button.tpt-sh{color:var(--ink)}
.tp table.tpt button:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.tp table.tpt td.tpt-c0{white-space:nowrap}.tp table.tpt button.tpt-tg{all:unset;box-sizing:border-box;cursor:pointer;display:inline-flex;align-items:center;justify-content:center;width:24px;height:24px;border-radius:4px;color:var(--muted);margin-right:2px}
.tp table.tpt button.tpt-tg:hover{background:var(--soft);color:var(--ink)}
@media(pointer:coarse){.tp table.tpt button.tpt-tg,.tp table.tpt th button.tpt-sh{min-width:40px;min-height:40px}}
.tp table.tpt tr.tpt-exp>td{padding:0;border-bottom:2px solid var(--accent);background:var(--bg)}
.tp table.tpt tr.tpt-exp>td>.tpt-slot{position:sticky;left:0;box-sizing:border-box;width:var(--tpt-w,100%);padding:8px 10px}
.tp table.tpt .tpt-no{color:var(--muted)}
.tp .tpt-colm{margin:0 0 6px;font-size:12px}.tp .tpt-colm summary{cursor:pointer;min-height:24px}.tp .tpt-colm>div{display:flex;flex-wrap:wrap;gap:4px 14px;padding:6px 0}.tp .tpt-colm label{display:inline-flex;align-items:center;gap:4px;min-height:24px}`;

  /**
   * @typedef {{ key: string, label: string, title?: string, num?: boolean, sort?: (r: any) => number | string | null, cell: (r: any) => string }} Col
   * @typedef {{ id: string, cols: Col[], rowId: (r: any) => number, name: (r: any) => string, onOpen: (id: number, panel: HTMLElement) => void, onSort?: (key: string, dir: number) => void, esc: (s: any) => string, label?: string, panelId?: string }} Opts
   */

  /**
   * The table in `host`.
   * @param {HTMLElement} host
   * @param {Opts} o
   */
  /** rows ordered by the sort value of a column (empty values last, ties in the original order) */
  function sortRows(/** @type {any[]} */ rows, /** @type {any} */ c, /** @type {number} */ dir) {
    if (!c || !c.sort) return rows;
    const f = c.sort, v = new Map(rows.map((r, i) => [r, { v: f(r), i }]));
    return [...rows].sort((a, b) => {
      const x = /** @type {any} */ (v.get(a)), y = /** @type {any} */ (v.get(b));
      if (x.v == null || y.v == null) return x.v == null && y.v == null ? x.i - y.i : x.v == null ? 1 : -1;
      const d = typeof x.v === "string" ? x.v.localeCompare(y.v) : x.v - y.v;
      return d ? d * dir : x.i - y.i;
    });
  }
  function mount(host, o) {
    if (!document.getElementById("tpt-css")) { const s = document.createElement("style"); s.id = "tpt-css"; s.textContent = CSS; document.head.appendChild(s); }
    /** @type {{ rows: any[], open: number | null, sort: string | null, dir: number }} */
    const st = { rows: [], open: null, sort: null, dir: -1 };
    const panel = document.createElement("div"); panel.id = o.panelId || "tp-det";
    host.innerHTML = `<div class="tpt-wrap"><table class="tpt" id="${o.id}" aria-label="${o.esc(o.label || "Products")}"><thead><tr></tr></thead><tbody></tbody></table></div>`;
    const wrap = /** @type {HTMLElement} */ (host.querySelector(".tpt-wrap")), tbl = /** @type {HTMLTableElement} */ (host.querySelector("table")), head = /** @type {HTMLElement} */ (tbl.querySelector("thead tr")), body = /** @type {HTMLElement} */ (tbl.querySelector("tbody"));
    const fitSlot = () => wrap.style.setProperty("--tpt-w", Math.max(280, wrap.clientWidth - 2) + "px");
    if (typeof ResizeObserver !== "undefined") new ResizeObserver(fitSlot).observe(wrap); else fitSlot();

    function drawHead() {
      head.innerHTML = cols.map(c => `<th scope="col" aria-sort="${st.sort === c.key ? (st.dir > 0 ? "ascending" : "descending") : "none"}"${c.title ? ` title="${o.esc(c.title)}"` : ""}>${c.sort ? `<button type="button" class="tpt-sh" data-sh="${c.key}">${c.label}<span aria-hidden="true">${st.sort === c.key ? (st.dir > 0 ? "▲" : "▼") : ""}</span></button>` : c.label}</th>`).join("");
    }
    let cols = o.cols.filter(c => !c.off);                 // the visible columns (the others are added from the menu of the page)
    const sorted = () => sortRows(st.rows, o.cols.find(x => x.key === st.sort), st.dir);
    function draw() {
      const had = document.activeElement instanceof HTMLElement && host.contains(document.activeElement) ? document.activeElement.getAttribute("data-tg") : null;
      panel.remove(); drawHead();
      body.innerHTML = sorted().map(r => {
        const id = o.rowId(r), on = st.open === id;
        return `<tr class="clk${on ? " sel" : ""}" data-id="${id}">${cols.map((c, i) => `<td${c.num || i === 0 ? ` class="${[c.num ? "n" : "", i === 0 ? "tpt-c0" : ""].filter(Boolean).join(" ")}"` : ""}>${i === 0 ? `<button type="button" class="tpt-tg" data-tg="${id}" aria-expanded="${on}" aria-controls="${panel.id}" aria-label="Details of ${o.esc(o.name(r))}"><span aria-hidden="true">${on ? "▾" : "▸"}</span></button>` : ""}${c.cell(r)}</td>`).join("")}</tr>${on ? `<tr class="tpt-exp"><td colspan="${o.cols.length}"><div class="tpt-slot"></div></td></tr>` : ""}`;
      }).join("");
      const slot = body.querySelector(".tpt-slot"); if (slot) slot.appendChild(panel);
      if (had != null) { const b = body.querySelector(`[data-tg="${had}"]`); if (b instanceof HTMLElement) b.focus(); }
    }
    /** @param {number} id */
    function open(id) {
      if (!st.rows.some(r => o.rowId(r) === id)) return false;
      const fresh = st.open !== id;
      st.open = id; if (fresh) panel.innerHTML = ""; draw();
      if (fresh) o.onOpen(id, panel);
      return true;
    }
    function close() { st.open = null; panel.innerHTML = ""; draw(); }
    /** @param {number} id */
    function toggle(id) { if (st.open === id) close(); else open(id); }

    body.addEventListener("click", e => {
      const tr = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("tr.clk"));
      if (tr && !(/** @type {HTMLElement} */ (e.target).closest("a,input,select"))) toggle(+/** @type {string} */ (tr.dataset.id));
    });
    head.addEventListener("click", e => {
      const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("[data-sh]")); if (!b) return;
      sortBy(/** @type {string} */ (b.dataset.sh), undefined, true);
    });
    host.addEventListener("keydown", e => {
      const t = /** @type {HTMLElement} */ (e.target);
      if (e.key === "Escape" && st.open != null && (t.closest("tr.tpt-exp") || t.closest("[data-tg]"))) { const id = st.open; e.preventDefault(); close(); const b = body.querySelector(`[data-tg="${id}"]`); if (b instanceof HTMLElement) b.focus(); return; }
      if (!t.matches("[data-tg]") || !["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
      const all = /** @type {HTMLElement[]} */ ([...body.querySelectorAll("[data-tg]")]), i = all.indexOf(t);
      const n = e.key === "Home" ? 0 : e.key === "End" ? all.length - 1 : Math.max(0, Math.min(all.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)));
      e.preventDefault(); all[n].focus();
    });

    /** sort by a column; `dir` +1 / -1, else the next one (a number column starts with the largest value)
     * @param {string} key @param {number} [dir] @param {boolean} [byUser] */
    function sortBy(key, dir, byUser) {
      const c = o.cols.find(x => x.key === key); if (!c || !c.sort) return;
      st.dir = dir || (st.sort === key ? -st.dir : (c.num ? -1 : 1)); st.sort = key; draw();
      if (byUser && o.onSort) o.onSort(key, st.dir);
    }
    drawHead();
    /** @param {string[]} keys */
    function setCols(keys) { cols = o.cols.filter(c => keys.includes(c.key)); draw(); }
    return { setCols,
      /** the rows to show (already filtered); the open row stays open if it is still there, else it is closed @param {any[]} rows */
      setRows(rows) { st.rows = rows; if (st.open != null && !rows.some(r => o.rowId(r) === st.open)) { st.open = null; panel.innerHTML = ""; } draw(); },
      open, close, toggle, sortBy,
      get openId() { return st.open; },
      panel,
    };
  }

  /** a small line chart of the areas along the treatment, readable for a screen reader (values in the label); the data are also in the row and in the sheet
   * @param {number[]} series @param {string} [what] */
  function spark(series, what) {
    const mx = Math.max(...series) || 1, w = 70, ht = 18;
    const pts = series.map((v, i) => `${(i / Math.max(1, series.length - 1) * (w - 4) + 2).toFixed(1)},${(ht - 2 - v / mx * (ht - 4)).toFixed(1)}`).join(" ");
    const lab = `${what || "area"} along the treatment, from 0 to ${Math.round(mx)}: ${series.map(v => Math.round(v)).join(", ")}`;
    return `<svg width="${w}" height="${ht}" role="img" aria-label="${lab}"><polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;
  }

  G.TPTAB = { mount, spark };
})();
