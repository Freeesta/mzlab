// @ts-check
// TPMINE-PRIVATE  TPTAB: the one table component of Tabui. A closed row has few columns (the first ones can stay fixed), a click opens the row on the spot (one row open at a time):
// a head (filled by the page), tabs and a panel. The page of the engine gives the columns, the rows and what to put in the panel; this file only shows.
// Also: filter chips and search, "Columns" menu, pinned row, state symbols, level badges, virtual rows, "Export view". Every option beyond cols/rowId/name/onOpen is optional.
// Keyboard: Tab to a row button, Enter or Space opens and closes, ↑ ↓ PgUp PgDn Home End move between the rows, Esc closes; ← → Home End move between the tabs. Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const CSS = `
.tp{--tpt-rh:30px}.tp .pd-1{--lvc:#009E73}.tp .pd-2{--lvc:#0072B2}.tp .pd-3{--lvc:#E69F00}.tp .pd-4{--lvc:#999999}
.tp .tpt-wrap{overflow-anchor:none;contain:inline-size;max-width:100%;overflow-x:auto;border:1px solid var(--line);border-radius:6px;position:relative}.tp .tpt-wrap.tpt-h{overflow:auto}
.tp table.tpt{border-collapse:separate;border-spacing:0;width:100%;font-size:12px}
.tp table.tpt th{position:sticky;top:0;z-index:2;background:var(--panel);white-space:nowrap;text-align:left;border-bottom:1px solid var(--line)}
.tp table.tpt th.n{text-align:right}
.tp table.tpt td{height:var(--tpt-rh);box-sizing:border-box;padding:0 6px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:26em;vertical-align:middle;border-bottom:1px solid var(--line)}
.tp table.tpt td.n{text-align:right;font-variant-numeric:tabular-nums}
.tp table.tpt .tpt-stk{position:sticky;z-index:1;background:var(--panel)}.tp table.tpt th.tpt-stk{z-index:3}.tp table.tpt tr.sel td.tpt-stk{background:var(--sel)}
.tp table.tpt th button.tpt-sh{all:unset;box-sizing:border-box;cursor:pointer;display:inline-flex;gap:4px;align-items:center;min-height:24px;min-width:24px;font-weight:600;color:var(--muted);padding:0 2px}
.tp table.tpt th button.tpt-sh:hover,.tp table.tpt th[aria-sort="ascending"] button.tpt-sh,.tp table.tpt th[aria-sort="descending"] button.tpt-sh{color:var(--ink)}
.tp table.tpt th>span[tabindex]{display:inline-flex;align-items:center;min-height:24px;font-weight:600;color:var(--muted)}
.tp table.tpt button:focus-visible,.tp .tpt-bar button:focus-visible,.tp .tpt-bar summary:focus-visible,.tp .tpt-tabs button:focus-visible,.tp table.tpt [tabindex]:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.tp table.tpt td.tpt-c0{white-space:nowrap}.tp table.tpt button.tpt-tg{all:unset;box-sizing:border-box;cursor:pointer;display:inline-flex;align-items:center;justify-content:center;width:24px;height:24px;border-radius:4px;color:var(--muted);margin-right:2px;vertical-align:middle}
.tp table.tpt button.tpt-tg:hover{background:var(--soft);color:var(--ink)}
.tp table.tpt .tpt-pin{display:inline-flex;align-items:center;justify-content:center;min-width:24px;min-height:24px;color:var(--warn)}
.tp table.tpt tr.tpt-sp td{padding:0;border:0;height:auto}
@media(pointer:coarse){.tp{--tpt-rh:40px}.tp table.tpt button.tpt-tg,.tp table.tpt th button.tpt-sh,.tp .tpt-chip,.tp .tpt-bar summary,.tp .tpt-tabs button{min-width:40px;min-height:40px}}
.tp table.tpt tr.tpt-exp>td{padding:0;height:auto;max-width:none;overflow:visible;white-space:normal;border-bottom:2px solid var(--accent);background:var(--bg)}
.tp table.tpt tr.tpt-exp>td>.tpt-slot{position:sticky;left:0;box-sizing:border-box;width:var(--tpt-w,100%);padding:8px 10px}
.tp table.tpt .tpt-no{color:var(--muted)}
.tp .tpt-bar{display:flex;flex-wrap:wrap;gap:6px 10px;align-items:center;margin:0 0 6px;font-size:12px}
.tp .tpt-chip{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;padding:0 10px;border:1px solid var(--line);border-radius:12px;background:var(--panel);color:var(--ink);font:inherit}
.tp .tpt-chip[aria-pressed="true"]{background:var(--sel);border-color:var(--accent);font-weight:600}.tp .tpt-chip[aria-pressed="true"]::before{content:"✓ "}
.tp .tpt-q{box-sizing:border-box;min-height:24px;max-width:180px}
.tp .tpt-n{color:var(--muted)}.tp .tpt-live{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}
.tp .tpt-colm,.tp .tpt-xm{position:relative;margin:0 0 6px;font-size:12px}.tp .tpt-bar .tpt-colm,.tp .tpt-bar .tpt-xm{margin:0}.tp .tpt-colm summary,.tp .tpt-xm summary{cursor:pointer;min-height:24px;display:inline-flex;align-items:center}
.tp .tpt-colm>div,.tp .tpt-xm>div{display:flex;flex-wrap:wrap;gap:4px 14px;padding:6px 0}.tp .tpt-colm label{display:inline-flex;align-items:center;gap:4px;min-height:24px}
.tp .tpt-xm button{min-height:24px;min-width:24px;cursor:pointer}
.tp .tpt-head{display:flex;flex-wrap:wrap;gap:4px 10px;align-items:center;margin-bottom:6px}
.tp .tpt-tabs{display:flex;gap:2px;border-bottom:1px solid var(--line);overflow-x:auto;margin-bottom:6px}
.tp .tpt-tabs button{box-sizing:border-box;cursor:pointer;min-height:28px;min-width:24px;padding:0 12px;border:0;border-bottom:3px solid transparent;background:none;color:var(--muted);font:inherit;white-space:nowrap}
.tp .tpt-tabs button[aria-selected="true"]{color:var(--ink);font-weight:600;border-bottom-color:var(--accent)}
.tp .tpt-body .tpt-scroll{max-height:420px;overflow:auto}
.tp .tpt-st{display:inline-block;min-width:1em;text-align:center;font-family:"Segoe UI Symbol","DejaVu Sans","Noto Sans Symbols 2",sans-serif}.tp .tpt-st-detected{color:var(--ink)}.tp .tpt-st-filled{color:var(--accent)}.tp .tpt-st-targeted{color:var(--warn)}.tp .tpt-st-absent,.tp .tpt-st-na{color:var(--muted)}
.tp .tpt-cells{letter-spacing:1px;white-space:nowrap}.tp .tpt-cells .tpt-more{font-size:11px;color:var(--muted);margin-left:3px;letter-spacing:0}
.tp .tpt-leg{display:flex;flex-wrap:wrap;gap:2px 14px;font-size:11px;margin-top:6px;color:var(--muted)}.tp .tpt-leg span[tabindex]{min-height:24px;display:inline-flex;align-items:center;gap:4px}
.tp .lvb.tpt-dec{border-style:double;border-width:4px;padding:0 4px}
@media(prefers-reduced-motion:no-preference){.tp .tpt-chip,.tp .tpt-tabs button{transition:background .12s,border-color .12s}}`;

  /**
   * @typedef {{ key: string, label: string, title?: string, help?: string, num?: boolean, off?: boolean, sticky?: boolean, width?: number, sort?: (r: any) => number | string | null, cell: (r: any) => string, text?: (r: any) => string }} Col
   * @typedef {{ key: string, label: string, help?: string, test: (r: any) => boolean, on?: boolean }} Chip
   * @typedef {{ key: string, label: string, help?: string }} Tab
   * @typedef {{ id: string, cols: Col[], rowId: (r: any) => number, name: (r: any) => string, onOpen: (id: any, panel: HTMLElement, tab?: string) => void, onSort?: (key: string, dir: number) => void, esc: (s: any) => string, label?: string, panelId?: string,
   *   chips?: Chip[], search?: (r: any) => string, onFilter?: (keys: string[]) => void, pin?: (r: any) => boolean, tabs?: Tab[], head?: (r: any, box: HTMLElement) => void, height?: number, columns?: boolean,
   *   virtual?: number, rowHeight?: number, exportName?: string, status?: boolean }} Opts
   */

  const STATES = /** @type {Record<string, [string, string]>} */ ({ detected: ["●", "detected"], filled: ["◐", "filled"], targeted: ["◌", "targeted"], absent: ["○", "absent"], na: ["–", "n/a"] });
  const lsGet = (/** @type {string} */ k) => { try { return localStorage.getItem(k); } catch (_) { return null; } };
  const lsSet = (/** @type {string} */ k, /** @type {string} */ v) => { try { localStorage.setItem(k, v); } catch (_) { /* private mode: the choice is not kept */ } };
  /** the help of a key: only a marker, the popover of the page (TPHELP) opens it; no `title` */
  const tt = (/** @type {string | undefined} */ k) => k && G.TPHELP ? ` data-tt="${k}"` : "";
  const plain = (/** @type {string} */ html) => { const t = document.createElement("template"); t.innerHTML = html; return (t.content.textContent || "").replace(/\s+/g, " ").trim(); };

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

  /**
   * The table in `host`.
   * @param {HTMLElement} host
   * @param {Opts} o
   */
  function mount(host, o) {
    if (!document.getElementById("tpt-css")) { const s = document.createElement("style"); s.id = "tpt-css"; s.textContent = CSS; document.head.appendChild(s); }
    const coarse = typeof matchMedia !== "undefined" && matchMedia("(pointer:coarse)").matches;
    const rowH = coarse ? 40 : (o.rowHeight || 30), virtualN = o.virtual ?? 300, tabs = o.tabs || [], esc = o.esc;
    const kCols = "qqq.tabui.cols." + o.id, kChips = "qqq.tabui.chips." + o.id;
    /** @type {{ rows: any[], list: any[], pinned: any[], rest: any[], open: number | null, sort: string | null, dir: number, tab: string, q: string, chips: Set<string>, vis: Set<string>, expH: number, a: number, b: number }} */
    const st = { rows: [], list: [], pinned: [], rest: [], open: null, sort: null, dir: -1, tab: tabs.length ? tabs[0].key : "", q: "", chips: new Set((o.chips || []).filter(c => c.on).map(c => c.key)), vis: new Set(o.cols.filter(c => !c.off || c.sticky).map(c => c.key)), expH: 380, a: 0, b: 0 };
    if (o.columns) {                                         // the choice of columns and chips is kept per table
      try { const k = JSON.parse(lsGet(kCols) || "null"); if (Array.isArray(k)) st.vis = new Set(o.cols.filter(c => c.sticky || k.includes(c.key)).map(c => c.key)); } catch (_) { /* ignore */ }
    }
    if (o.chips) { try { const k = JSON.parse(lsGet(kChips) || "null"); if (Array.isArray(k)) st.chips = new Set(o.chips.filter(c => k.includes(c.key)).map(c => c.key)); } catch (_) { /* ignore */ } }

    const panel = document.createElement("div"); panel.id = o.panelId || "tp-det";
    /** @type {HTMLElement | null} */ let headBox = null, tabBox = null, bodyBox = null;
    const hasBar = !!(o.chips || o.search || o.columns || o.exportName || o.status);
    const bar = !hasBar ? "" : `<div class="tpt-bar">${(o.chips || []).map(c => `<button type="button" class="tpt-chip" data-chip="${c.key}" aria-pressed="${st.chips.has(c.key)}"${tt(c.help)}>${c.label}</button>`).join("")}${o.search ? `<input type="search" class="tpt-q" aria-label="Search the rows" placeholder="Search">` : ""}
      <span class="tpt-n" role="status"></span>${o.columns ? `<details class="tpt-colm"><summary${tt("ft.cols")}>Columns</summary><div>${o.cols.filter(c => !c.sticky).map(c => `<label><input type="checkbox" data-vc="${c.key}"${st.vis.has(c.key) ? " checked" : ""}> ${c.label}</label>`).join("")}</div></details>` : ""}${o.exportName ? `<details class="tpt-xm"><summary>Export view</summary><div><button type="button" data-x="csv">CSV</button><button type="button" data-x="xlsx">Excel</button></div></details>` : ""}</div>`;
    host.innerHTML = `${bar}<div class="tpt-live" role="status"></div><div class="tpt-wrap"><table class="tpt" id="${o.id}" aria-label="${esc(o.label || "Products")}"><thead><tr></tr></thead><tbody></tbody></table></div>`;
    const wrap = /** @type {HTMLElement} */ (host.querySelector(".tpt-wrap")), tbl = /** @type {HTMLTableElement} */ (host.querySelector("table")), head = /** @type {HTMLElement} */ (tbl.querySelector("thead tr")), body = /** @type {HTMLElement} */ (tbl.querySelector("tbody"));
    const live = /** @type {HTMLElement} */ (host.querySelector(".tpt-live")), count = /** @type {HTMLElement | null} */ (host.querySelector(".tpt-n"));
    wrap.style.setProperty("--tpt-rh", rowH + "px");
    if (o.height) { wrap.classList.add("tpt-h"); wrap.style.maxHeight = o.height + "px"; }
    const fitSlot = () => { wrap.style.setProperty("--tpt-w", Math.max(280, wrap.clientWidth - 2) + "px"); wrap.classList.toggle("tpt-narrow", wrap.clientWidth < 700); };
    if (typeof ResizeObserver !== "undefined") new ResizeObserver(() => { fitSlot(); fixSticky(); }).observe(wrap); else fitSlot();
    if (typeof ResizeObserver !== "undefined") new ResizeObserver(() => { if (st.open != null && panel.offsetHeight > 40) st.expH = panel.offsetHeight; }).observe(panel);

    let cols = o.cols.filter(c => st.vis.has(c.key));
    const idOf = (/** @type {any} */ r) => o.rowId(r);
    const isVirt = () => st.rest.length > virtualN;
    const scrollFrame = (/** @type {() => void} */ f) => G.FRAME && G.FRAME.next ? G.FRAME.next(f) : requestAnimationFrame(f);

    function drawHead() {
      head.innerHTML = cols.map((c, i) => `<th scope="col"${c.sticky ? ` class="tpt-stk${c.num ? " n" : ""}"` : c.num ? ` class="n"` : ""} aria-sort="${st.sort === c.key ? (st.dir > 0 ? "ascending" : "descending") : "none"}"${c.title && !c.help ? ` title="${esc(c.title)}"` : ""}${c.width || c.sticky ? ` style="${c.sticky ? `left:var(--tpt-l${i},0);` : ""}${c.width ? `width:${c.width}px;min-width:${c.width}px` : ""}"` : ""}>${
        c.sort ? `<button type="button" class="tpt-sh" data-sh="${c.key}"${tt(c.help)}>${c.label}<span aria-hidden="true">${st.sort === c.key ? (st.dir > 0 ? "▲" : "▼") : ""}</span></button>` : c.help && G.TPHELP ? `<span tabindex="0"${tt(c.help)}>${c.label}</span>` : c.label}</th>`).join("");
      head.setAttribute("aria-rowindex", "1");
    }
    /** the left offset of the fixed columns: the sum of the widths of the fixed ones before them */
    function fixSticky() {
      let x = 0;
      cols.forEach((c, i) => {
        if (!c.sticky) return;
        tbl.style.setProperty("--tpt-l" + i, x + "px");
        x += /** @type {HTMLElement} */ (head.children[i]).offsetWidth;
      });
    }
    const stk = (/** @type {Col} */ c, /** @type {number} */ i) => c.sticky || c.width ? ` style="${c.sticky ? `left:var(--tpt-l${i},0);` : ""}${c.width ? `max-width:${c.width}px` : ""}"` : "";
    /** @param {any} r @param {number} idx the index in the list of rows */
    function rowHtml(r, idx) {
      const id = idOf(r), on = st.open === id, pinned = o.pin ? o.pin(r) : false;
      const idCol = cols.findIndex(c => c.key === "identity");
      return `<tr class="clk${on ? " sel" : ""}" data-id="${id}" aria-rowindex="${idx + 2}">${cols.map((c, i) => {
        const k = [c.num ? "n" : "", i === 0 ? "tpt-c0" : "", c.sticky ? "tpt-stk" : ""].filter(Boolean).join(" ");
        const tg = i === 0 ? `<button type="button" class="tpt-tg" data-tg="${id}" aria-expanded="${on}" aria-controls="${panel.id}" aria-label="Details of ${esc(o.name(r))}"><span aria-hidden="true">${on ? "▾" : "▸"}</span></button>${pinned ? `<span class="tpt-pin" tabindex="0" role="img" aria-label="Pinned: the parent"${tt("ft.pin.parent")}>★</span>` : ""}` : "";
        return `<td${k ? ` class="${k}"` : ""}${i === (idCol < 0 ? 0 : idCol) ? ` id="${o.id}-i-${id}"` : ""}${stk(c, i)}>${tg}${c.cell(r)}</td>`;
      }).join("")}</tr>${on ? `<tr class="tpt-exp"><td colspan="${cols.length}"><div class="tpt-slot"></div></td></tr>` : ""}`;
    }
    const spacer = (/** @type {number} */ n) => n > 0 ? `<tr class="tpt-sp" aria-hidden="true" style="height:${n * rowH}px"><td colspan="${cols.length}"></td></tr>` : "";

    /** the window of the virtual list: the rows in view ± 20, the pinned ones and the open one always */
    function windowOf() {
      const n = st.rest.length, p = st.pinned.length;
      if (!isVirt()) return [0, n];
      let pos = wrap.scrollTop - (head.offsetHeight || 30) - p * rowH;
      const oi = st.open == null ? -1 : st.rest.findIndex(r => idOf(r) === st.open);
      if (oi >= 0 && pos > (oi + 1) * rowH + st.expH) pos -= st.expH; else if (oi >= 0 && pos > oi * rowH) pos = oi * rowH;
      const view = wrap.clientHeight || (o.height || 520);
      const a = Math.max(0, Math.floor(pos / rowH) - 20), b = Math.min(n, Math.ceil((pos + view) / rowH) + 20);
      return [a, Math.max(a, b)];
    }
    function draw() {
      const had = document.activeElement instanceof HTMLElement && host.contains(document.activeElement) ? document.activeElement.getAttribute("data-tg") : null;
      panel.remove(); drawHead();
      const p = st.pinned.length, n = st.rest.length, [a, b] = windowOf(); st.a = a; st.b = b;
      const oi = st.open == null ? -1 : st.rest.findIndex(r => idOf(r) === st.open);
      const idx = [];
      for (let i = a; i < b; i++) idx.push(i);
      if (oi >= 0 && (oi < a || oi >= b)) idx.splice(oi < a ? 0 : idx.length, 0, oi);
      let html = st.pinned.map((r, i) => rowHtml(r, i)).join(""), prev = -1;
      if (isVirt()) { for (const i of idx) { html += spacer(i - prev - 1) + rowHtml(st.rest[i], p + i); prev = i; } html += spacer(n - 1 - prev); }
      else html += st.rest.map((r, i) => rowHtml(r, p + i)).join("");
      body.innerHTML = html;
      tbl.setAttribute("aria-rowcount", String(st.list.length + 1));
      fixSticky();
      const slot = body.querySelector(".tpt-slot"); if (slot) slot.appendChild(panel);
      if (had != null) { const bt = body.querySelector(`[data-tg="${had}"]`); if (bt instanceof HTMLElement) bt.focus(); }
    }
    let raf = 0;
    wrap.addEventListener("scroll", () => {
      if (!isVirt() || raf) return;
      raf = scrollFrame(() => { raf = 0; const [a, b] = windowOf(); if (Math.abs(a - st.a) >= 10 || Math.abs(b - st.b) >= 10 || (a === 0) !== (st.a === 0) || (b === st.rest.length) !== (st.b === st.rest.length)) draw(); });
    });

    // ---- the filter, the order and the pinned row
    const chipTests = () => (o.chips || []).filter(c => st.chips.has(c.key));
    function derive() {
      const ts = chipTests(), q = st.q.trim().toLowerCase();
      const ok = (/** @type {any} */ r) => ts.every(c => c.test(r)) && (!q || !o.search || o.search(r).toLowerCase().includes(q));
      const pinned = o.pin ? st.rows.filter(r => /** @type {Function} */ (o.pin)(r)) : [];
      const rest = (o.chips || o.search) ? st.rows.filter(r => !(o.pin && o.pin(r)) && ok(r)) : st.rows.filter(r => !(o.pin && o.pin(r)));
      st.pinned = pinned; st.rest = sortRows(rest, o.cols.find(x => x.key === st.sort), st.dir); st.list = [...pinned, ...st.rest];
      if (count) count.textContent = `${st.list.length} of ${st.rows.length} rows`;
      if (isVirt() && !o.height) { wrap.classList.add("tpt-h"); wrap.style.maxHeight = "520px"; }
    }
    function refresh() {
      derive();
      if (st.open != null && !st.list.some(r => idOf(r) === st.open)) { st.open = null; panel.innerHTML = ""; }
      draw();
    }

    // ---- the open row
    /** @param {number} id */ const rowOf = id => st.list.find(r => idOf(r) === id);
    const trOf = (/** @type {number} */ id) => /** @type {HTMLElement | null} */ (body.querySelector(`tr.clk[data-id="${id}"]`));
    const scroller = () => wrap.classList.contains("tpt-h") ? wrap : null;
    /** brings the row of the list into the DOM (virtual lists) */
    function reveal(/** @type {number} */ i) {
      if (!isVirt()) return;
      const p = st.pinned.length, ri = i - p; if (ri < 0) return;
      const oi = st.open == null ? -1 : st.rest.findIndex(r => idOf(r) === st.open);
      wrap.scrollTop = p * rowH + ri * rowH + (oi >= 0 && oi < ri ? st.expH : 0);
      draw();
    }
    /** runs `f` and keeps the row `id` where it was on the screen */
    function anchored(/** @type {number} */ id, /** @type {() => void} */ f) {
      const t0 = trOf(id)?.getBoundingClientRect().top;
      f();
      const t1 = trOf(id)?.getBoundingClientRect().top;
      if (t0 == null || t1 == null || Math.abs(t1 - t0) < 1) return;
      const s = scroller(); if (s) s.scrollTop += t1 - t0; else window.scrollBy(0, t1 - t0);
    }
    function showPanel(/** @type {number} */ id) {            // scroll only when the panel would end below the visible part
      const tr = trOf(id), ex = /** @type {HTMLElement | null} */ (tr?.nextElementSibling); if (!tr || !ex || !ex.classList.contains("tpt-exp")) return;
      const s = scroller(), view = s ? s.getBoundingClientRect() : { top: 0, bottom: window.innerHeight }, over = ex.getBoundingClientRect().bottom - view.bottom;
      if (over <= 0) return;
      const room = tr.getBoundingClientRect().top - view.top - (head.offsetHeight || 28), by = Math.max(0, Math.min(over, room));
      if (by > 0) { if (s) s.scrollTop += by; else window.scrollBy(0, by); }
    }
    function buildPanel(/** @type {any} */ r) {
      panel.innerHTML = "";
      panel.setAttribute("role", "region");
      const lab = `${o.id}-i-${idOf(r)}`; panel.setAttribute("aria-labelledby", lab);
      if (!tabs.length) { headBox = tabBox = bodyBox = null; return; }
      panel.innerHTML = `<div class="tpt-head"></div><div class="tpt-tabs" role="tablist" aria-label="Sections of the row">${tabs.map(t => `<button type="button" role="tab" id="${o.id}-tab-${t.key}" data-tab="${t.key}" aria-selected="${t.key === st.tab}" aria-controls="${o.id}-tp" tabindex="${t.key === st.tab ? 0 : -1}"${tt(t.help)}>${t.label}</button>`).join("")}</div><div class="tpt-body" id="${o.id}-tp" role="tabpanel" aria-labelledby="${o.id}-tab-${st.tab}"></div>`;
      headBox = panel.querySelector(".tpt-head"); tabBox = panel.querySelector(".tpt-tabs"); bodyBox = panel.querySelector(".tpt-body");
      if (o.head && headBox) o.head(r, headBox);
    }
    function fill(/** @type {any} */ r) {
      if (tabs.length && bodyBox) { bodyBox.innerHTML = ""; o.onOpen(r, bodyBox, st.tab); } else o.onOpen(idOf(r), panel);
    }
    /** @param {number} id */
    function open(id) {
      const r = rowOf(id); if (!r) return false;
      const fresh = st.open !== id;
      if (fresh && isVirt() && !trOf(id)) reveal(st.list.indexOf(r));
      anchored(id, () => { st.open = id; if (fresh) { buildPanel(r); draw(); fill(r); } else draw(); });
      if (fresh) { live.textContent = `Details of ${o.name(r)} opened`; }
      showPanel(id);
      return true;
    }
    function close() {
      const id = st.open; if (id == null) return;
      anchored(id, () => { st.open = null; panel.innerHTML = ""; draw(); });
      live.textContent = "Details closed";
    }
    /** @param {number} id */
    function toggle(id) { if (st.open === id) close(); else open(id); }
    /** @param {string} key @param {boolean} [focus] */
    function selectTab(key, focus) {
      if (st.open == null || !tabBox || !bodyBox) return;
      st.tab = key;
      tabBox.querySelectorAll("[role=tab]").forEach(b => { const on = /** @type {HTMLElement} */ (b).dataset.tab === key; b.setAttribute("aria-selected", String(on)); b.setAttribute("tabindex", on ? "0" : "-1"); if (on && focus) /** @type {HTMLElement} */ (b).focus(); });
      bodyBox.setAttribute("aria-labelledby", `${o.id}-tab-${key}`);
      const r = rowOf(st.open); if (r) { bodyBox.innerHTML = ""; o.onOpen(r, bodyBox, key); }
    }

    body.addEventListener("click", e => {
      const t = /** @type {HTMLElement} */ (e.target), tr = /** @type {HTMLElement | null} */ (t.closest("tr.clk"));
      if (tr && !t.closest("a,input,select,textarea,button:not(.tpt-tg)")) toggle(+/** @type {string} */ (tr.dataset.id));
    });
    head.addEventListener("click", e => {
      const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("[data-sh]")); if (!b) return;
      sortBy(/** @type {string} */ (b.dataset.sh), undefined, true);
    });
    host.addEventListener("keydown", e => {
      const t = /** @type {HTMLElement} */ (e.target);
      if (e.key === "Escape" && st.open != null && (t.closest("tr.tpt-exp") || t.closest("[data-tg]"))) { const id = st.open; e.preventDefault(); close(); const b = body.querySelector(`[data-tg="${id}"]`); if (b instanceof HTMLElement) b.focus(); return; }
      if (t.matches("[role=tab]") && ["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
        const ks = tabs.map(x => x.key), i = ks.indexOf(/** @type {string} */ (t.dataset.tab));
        const n = e.key === "Home" ? 0 : e.key === "End" ? ks.length - 1 : (i + (e.key === "ArrowRight" ? 1 : ks.length - 1)) % ks.length;
        e.preventDefault(); selectTab(ks[n], true); return;
      }
      if (!t.matches("[data-tg]") || !["ArrowDown", "ArrowUp", "Home", "End", "PageDown", "PageUp"].includes(e.key)) return;
      const i = st.list.findIndex(r => String(idOf(r)) === t.dataset.tg), last = st.list.length - 1;
      const n = e.key === "Home" ? 0 : e.key === "End" ? last : Math.max(0, Math.min(last, i + ({ ArrowDown: 1, ArrowUp: -1, PageDown: 10, PageUp: -10 }[/** @type {"ArrowDown"} */ (e.key)])));
      e.preventDefault(); if (!st.list[n]) return;
      if (!trOf(idOf(st.list[n]))) reveal(n);
      const b = body.querySelector(`[data-tg="${idOf(st.list[n])}"]`); if (b instanceof HTMLElement) { b.focus(); b.scrollIntoView({ block: "nearest" }); }
    });
    panel.addEventListener("click", e => { const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("[role=tab]")); if (b && b.dataset.tab) selectTab(b.dataset.tab, false); });

    // ---- chips, search, "Columns" and "Export view"
    host.querySelectorAll("[data-chip]").forEach(b => b.addEventListener("click", () => {
      const k = /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.chip), on = !st.chips.has(k);
      if (on) st.chips.add(k); else st.chips.delete(k);
      b.setAttribute("aria-pressed", String(on)); lsSet(kChips, JSON.stringify([...st.chips]));
      if (o.onFilter) o.onFilter([...st.chips]);
      if (isVirt()) wrap.scrollTop = 0;
      refresh();
    }));
    const q = /** @type {HTMLInputElement | null} */ (host.querySelector(".tpt-q"));
    if (q) q.addEventListener("input", () => { st.q = q.value; refresh(); });
    host.querySelectorAll("[data-vc]").forEach(b => b.addEventListener("change", () => {
      const k = /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.vc);
      if (/** @type {HTMLInputElement} */ (b).checked) st.vis.add(k); else st.vis.delete(k);
      lsSet(kCols, JSON.stringify([...st.vis])); cols = o.cols.filter(c => st.vis.has(c.key)); draw();
    }));
    const filtersText = () => `# filters: ${[...chipTests().map(c => c.label), ...(st.q.trim() ? [`search "${st.q.trim()}"`] : [])].join(", ") || "none"}`;
    /** the text of a cell of the view for the export (the model, not the DOM) */
    const textOf = (/** @type {Col} */ c, /** @type {any} */ r) => c.text ? c.text(r) : plain(c.cell(r));
    const exported = () => ({ head: cols.map(c => plain(c.label)), rows: st.list.map(r => cols.map(c => textOf(c, r))) });
    host.querySelectorAll("[data-x]").forEach(b => b.addEventListener("click", async () => {
      const kind = /** @type {HTMLElement} */ (b).dataset.x, v = exported(), name = o.exportName || "table";
      if (kind === "csv") {
        const cell = (/** @type {string} */ s) => /[";\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
        G.dl(name + ".csv", "﻿" + [filtersText(), v.head.map(cell).join(";"), ...v.rows.map(r => r.map(cell).join(";"))].join("\r\n") + "\r\n", "text/csv;charset=utf-8", false);
      } else G.dlx(name + ".xlsx", [{ name: "View", head: v.head, rows: v.rows }], false);
    }));

    /** sort by a column; `dir` +1 / -1, else the next one (a number column starts with the largest value)
     * @param {string} key @param {number} [dir] @param {boolean} [byUser] */
    function sortBy(key, dir, byUser) {
      const c = o.cols.find(x => x.key === key); if (!c || !c.sort) return;
      st.dir = dir || (st.sort === key ? -st.dir : (c.num ? -1 : 1)); st.sort = key; refresh();
      if (byUser && o.onSort) o.onSort(key, st.dir);
    }
    drawHead();
    /** @param {string[]} keys */
    function setCols(keys) { cols = o.cols.filter(c => keys.includes(c.key) || c.sticky); draw(); }
    fitSlot();
    return { setCols,
      /** all the rows (the chips and the search filter them here); the open row stays open if it is still shown, else it is closed @param {any[]} rows */
      setRows(rows) { st.rows = rows; refresh(); },
      open, close, toggle, sortBy,
      /** the rows as shown, pinned first (the model of the view) */
      get view() { return st.list; },
      get openId() { return st.open; },
      get tab() { return st.tab; },
      selectTab: (/** @type {string} */ k) => selectTab(k, false),
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

  /** the symbol of the state of an area: ● detected, ◐ filled, ◌ targeted, ○ absent, – n/a (the shape says it, not the colour)
   * @param {string} s */
  function state(s) {
    const [sym, word] = STATES[s] || STATES.na;
    return `<span class="tpt-st tpt-st-${STATES[s] ? s : "na"}" role="img" aria-label="${word}"${tt("ft.state." + s)}>${sym}</span>`;
  }
  /** a row of symbols, at most `max` and then «+N»; the label sums them up («5 detected, 1 filled»)
   * @param {string[]} states @param {number} [max] */
  function cells(states, max = 12) {
    const n = /** @type {Record<string, number>} */ ({}); for (const s of states) n[s] = (n[s] || 0) + 1;
    const lab = Object.keys(STATES).filter(k => n[k]).map(k => `${n[k]} ${STATES[k][1]}`).join(", ") || "no samples";
    const sym = (/** @type {string} */ s) => `<span class="tpt-st tpt-st-${STATES[s] ? s : "na"}" aria-hidden="true">${(STATES[s] || STATES.na)[0]}</span>`;
    return `<span class="tpt-cells" role="img" aria-label="${lab}">${states.slice(0, max).map(sym).join("")}${states.length > max ? `<span class="tpt-more">+${states.length - max}</span>` : ""}</span>`;
  }
  /** the legend of the symbols under the table @param {string[]} keys */
  function legend(keys) {
    return `<div class="tpt-leg">${keys.map(k => `<span${G.TPHELP ? ` tabindex="0"` : ""}${tt("ft.state." + k)}>${state(k)} ${(STATES[k] || STATES.na)[1]}</span>`).join("")}</div>`;
  }
  /** the badge of the Schymanski level («L3», «L2a»); a level decided by the chemist has the double border and the proposed one in the label
   * @param {string | number} lv @param {{ decided?: string | number | null }} [opt] */
  function level(lv, opt) {
    const L = G.LIVELLI, tag = (/** @type {any} */ x) => L ? L.tag(x) : "L" + x, cls = (/** @type {any} */ x) => L ? L.cls(x) : "lv-" + x;
    const d = opt && opt.decided;
    if (d != null && d !== "") return `<span class="lvb ${cls(d)} tpt-dec" role="img" aria-label="${tag(d)}, decided by the chemist; proposed ${tag(lv)}"${tt("ft.col.level")}>${tag(d)}</span>`;
    return `<span class="lvb ${cls(lv)}" role="img" aria-label="${tag(lv)}, proposed"${tt("ft.col.level")}>${tag(lv)}</span>`;
  }
  /** the badge of the prediction scale («Prev. 2»): only the outline, so it is not taken for a level @param {number} n */
  function pred(n) {
    return `<span class="lvb pd-${n}" role="img" aria-label="Prev. ${n}, prediction scale"${tt("ft.col.pred")}>Prev. ${n}</span>`;
  }

  G.TPTAB = { mount, spark, state, cells, legend, level, pred };
})();
