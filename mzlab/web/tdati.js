// @ts-strict
// The data of a chart as a real HTML table (<table>, headers with scope), for readers who cannot use a canvas: one component for every chart. A chart gives
// its data with TDATI.table({ title, head, rows, note }); the panels of the Dati view and the other charts register a provider with TDATI.attach(canvas, fn).
// Also here: the short description of a chart (aria-describedby) and the live region that reads it only on request.
// The "Dati" button, the T key on a focused chart and the entry of the right-click menu open the table; D reads the description.
const T = (k, p) => I18N.t(k, p);
const EH_ = s => EH(s ?? "");
let live = null, seq = 0;
const FIRST = 200, CHUNK = 1000;      // the dialog opens with FIRST rows (laying out 1000 took ~180 ms at 4x CPU), then CHUNK at a time
// rows a table keeps (shown and copied): past it the first ROWS_MAX stay and a note names the cut and where every row is (the view's export); a Copy of millions
// of rows built a string as large as the data again, which is what kills a tab on an iPad. `TDATI.maxRows` lowers it for the tests.
let ROWS_MAX = 200000;

// ---- the description of a chart, linked to its canvas (aria-describedby); the live region announces it only when asked
function describe(cv, text) {
  if (!cv) return;
  let d = cv._desc;
  if (!d) {
    d = cv._desc = document.createElement("span"); d.id = "tdesc-" + (++seq); d.className = "sr-only";
    cv.insertAdjacentElement("afterend", d);
    cv.setAttribute("aria-describedby", d.id);
    if (!cv.hasAttribute("role")) cv.setAttribute("role", "img");
    if (!cv.hasAttribute("tabindex")) cv.tabIndex = 0;
  }
  if (d.textContent !== text) d.textContent = text;
}
function say(text) {
  if (!live) { live = document.createElement("div"); live.id = "tdati-live"; live.className = "sr-only"; live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); document.body.appendChild(live); }
  live.textContent = ""; setTimeout(() => { live.textContent = text; }, 30);
}

// ---- the table window
function table(o) {
  const old = document.getElementById("tdati"); if (old) old.remove();
  const opener = /** @type {HTMLElement | null} */ (document.activeElement);
  const d = document.createElement("dialog"); d.id = "tdati"; d.className = "tdati"; d.setAttribute("aria-labelledby", "tdati-h");
  const head = (o.head || []).map(h => `<th scope="col">${EH_(h)}</th>`).join("");
  // a long table is written FIRST rows, then CHUNK at a time, the next ones when the reader scrolls near the end (all rows at once froze the page for seconds at 100k);
  // the count and the Copy button always cover every row kept (at most ROWS_MAX)
  const given = o.rows || [], cut = given.length > ROWS_MAX, all = cut ? given.slice(0, ROWS_MAX) : given, rowHtml = r => `<tr>${r.map((v, i) => (i === 0 && o.rowHeader ? `<th scope="row">${EH_(v)}</th>` : `<td>${EH_(v)}</td>`)).join("")}</tr>`;
  const rows = all.slice(0, FIRST).map(rowHtml).join("") + (all.length > FIRST ? `<tr class="td-more" aria-hidden="true"><td colspan="${Math.max(1, (o.head || []).length)}">…</td></tr>` : "");
  d.innerHTML = `<div class="td-hd"><h3 id="tdati-h">${EH_(o.title || T("tdati.title"))}</h3><button type="button" class="bt" data-td="x" aria-label="${T("tdati.close")}" title="${T("tdati.close")}">&times;</button></div>
    ${o.note ? `<p class="sm muted">${EH_(o.note)}</p>` : ""}${cut ? `<p class="sm" id="tdati-cut">${EH_(T("tdati.capped", { shown: ROWS_MAX, n: given.length }))}</p>` : ""}
    <div class="td-sc" tabindex="0" role="region" aria-label="${EH_(T("tdati.region", { title: o.title || "" }))}"><table><caption class="sr-only">${EH_(o.title || "")}</caption><thead><tr>${head}</tr></thead><tbody>${rows}</tbody></table></div>
    <div class="td-ft"><span class="sm muted" id="tdati-n">${cut ? T("tdati.rowsOf", { shown: ROWS_MAX, n: given.length }) : T("tdati.rows", { n: all.length })}</span><button type="button" class="go" data-td="c">${T("tdati.copy")}</button><button type="button" class="go" data-td="x">${T("tdati.close")}</button></div>`;
  document.body.appendChild(d);
  const more = d.querySelector("tr.td-more");
  let io = null;
  if (more) {
    let shown = FIRST;
    io = new IntersectionObserver(es => {
      if (!es.some(e => e.isIntersecting)) return;
      more.insertAdjacentHTML("beforebegin", all.slice(shown, shown + CHUNK).map(rowHtml).join("")); shown += CHUNK;
      if (shown >= all.length) { io.disconnect(); more.remove(); }
    }, { root: d.querySelector(".td-sc"), rootMargin: "0px 0px 400px 0px" });
    io.observe(more);
  }
  const close = () => { if (io) io.disconnect(); d.close(); d.remove(); if (opener && opener.focus && document.contains(opener)) opener.focus(); };
  d.querySelectorAll('[data-td="x"]').forEach(b => { b.onclick = close; });
  d.addEventListener("cancel", e => { e.preventDefault(); close(); });
  d.querySelector('[data-td="c"]').onclick = async () => {
    const tsvP = PROV.tsv([o.head || [], ...all].map(r => r.join("\t")).join("\n"), { table: String(o.title || "chart-data") });      // not awaited: Safari takes the clipboard only while the click runs
    try { await clipboardCopy({ "text/plain": tsvP }); } catch (_) { const tsv = await tsvP, t = document.createElement("textarea"); t.value = tsv; d.appendChild(t); t.select(); try { document.execCommand("copy"); } catch (e) { /* the table itself can be selected and copied */ } t.remove(); }
    /** @type {HTMLElement} */ (document.getElementById("tdati-n")).textContent = T("tdati.copied");
  };
  d.showModal();
  d.querySelector('[data-td="x"]').focus();
  return d;
}

// ---- charts register how to get their data; T on the focused canvas opens it, D reads the description
const providers = new WeakMap();
function attach(cv, fn) {
  providers.set(cv, fn);
  if (cv._tdKeys) return;
  cv._tdKeys = true;
  cv.addEventListener("keydown", e => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "t" || e.key === "T") { e.preventDefault(); open(cv); }
    else if (e.key === "d" || e.key === "D") { e.preventDefault(); say(cv._desc ? cv._desc.textContent : ""); }
  });
}
function open(cv) { const fn = providers.get(cv); const o = fn && fn(); if (o) return table(o); say(T("tdati.none")); return null; }

// a small «Dati» button after the chart (any element: a canvas, an SVG), the chart focusable, T / D on it
// quiet: no visible button, the table stays on the T key of the focused chart
function addButton(el, fn, quiet) {
  if (!el) return;
  attach(el, fn);
  if (!el.hasAttribute("tabindex")) el.tabIndex = 0;
  if (quiet) return;
  if (el._tdBtn && el._tdBtn.isConnected) return;
  const b = document.createElement("button"); b.type = "button"; b.className = "bt tdbtn"; b.textContent = T("tdati.btn");
  b.title = T("tdati.btn.title"); b.setAttribute("aria-label", T("tdati.btn.title"));
  b.onclick = () => open(el);
  (el._desc || el).insertAdjacentElement("afterend", b); el._tdBtn = b;
}
// the lines of a small chart made of series [{ x: [], y: [], name? }]: description + table
function series(cv, o, quiet) {
  if (!cv || !o) return;
  const n = (o.series || []).length, xs = (o.series || []).flatMap(s => s.x).filter(isFinite), ys = (o.series || []).flatMap(s => s.y).filter(isFinite);
  const title = o.title || [o.yl, o.xl].filter(Boolean).join(" / ") || T("a11y.chart");
  if (!cv.hasAttribute("aria-label")) cv.setAttribute("aria-label", title);
  describe(cv, T("a11y.small", { title, n, xl: o.xl || "x", yl: o.yl || "y", a: xs.length ? num(minOf(xs), 2) : "", b: xs.length ? num(maxOf(xs), 2) : "", c: ys.length ? num(minOf(ys), 2) : "", d: ys.length ? num(maxOf(ys), 2) : "" }));
  addButton(cv, () => {
    const rows = [];
    (o.series || []).forEach((s, k) => {
      const name = s.name || T("tdati.series", { n: k + 1 }), st = Math.max(1, Math.ceil(s.x.length / 300));
      for (let i = 0; i < s.x.length; i += st) if (isFinite(s.y[i])) rows.push([name, num(s.x[i], 4), num(s.y[i], 4)]);
    });
    return { title, head: [T("tdati.col.trace"), o.xl || "x", o.yl || "y"], rows, rowHeader: true };
  }, quiet);
}
// the table that already sits next to an SVG chart (QA/QC, profile, PCA loadings): the same dialog shows it
function fromDom(tbl, title) {
  if (!tbl) return null;
  const head = [...tbl.querySelectorAll("thead th")].map(h => h.textContent.trim());
  const rows = [...tbl.querySelectorAll("tbody tr")].map(r => [...r.children].map(c => c.textContent.trim()));
  return { title, head, rows, rowHeader: false };
}
// SVG charts are drawn from strings and replaced at every refresh: one observer gives each new chart its button
const watchers = [];
function watch(selector, provider) { watchers.push([selector, provider]); scan(document); }
// the added node itself may be the chart (a section that writes its SVG straight into its box), not only one of its descendants
function scan(root) {
  if (!root.querySelectorAll) return;
  watchers.forEach(([sel, prov]) => [...(root.matches && root.matches(sel) ? [root] : []), ...root.querySelectorAll(sel)].forEach(el => {
    if (el._tdWatched) return; el._tdWatched = true;
    const t = el.getAttribute("aria-label") || el.querySelector("title")?.textContent || "";
    addButton(el, () => prov(el, t));
  }));
}
new MutationObserver(ms => { if (!watchers.length) return; for (const m of ms) m.addedNodes.forEach(n => { if (n.nodeType === 1) scan(n); }); }).observe(document.documentElement, { childList: true, subtree: true });
// the `n` most intense cells [i, j, value] of a grid (row i, column j), in decreasing order (ties in grid order, as a stable sort), kept while walking it:
// an array of every non-empty cell first (2M on a fine 3D grid) is hundreds of MB, enough to kill a tab on an iPad
function topCells(/** @type {ArrayLike<number>} */ raw, /** @type {number} */ nr, /** @type {number} */ nc, /** @type {number} */ n) {
  /** @type {number[][]} */ const out = [];
  for (let i = 0; i < nr; i++) for (let j = 0; j < nc; j++) {
    const v = raw[i * nc + j];
    if (!(v > 0) || (out.length === n && v <= out[n - 1][2])) continue;
    let k = out.length; while (k > 0 && out[k - 1][2] < v) k--;
    out.splice(k, 0, [i, j, v]); if (out.length > n) out.pop();
  }
  return out;
}
const num = (v, n = 3) => (v == null || !isFinite(v) ? "" : String(+Number(v).toFixed(n)));
window.TDATI = { topCells, maxRows: (/** @type {number} */ n) => { ROWS_MAX = n > 0 ? n : 200000; }, table, describe, say, attach, open, num, addButton, series, fromDom, watch };
