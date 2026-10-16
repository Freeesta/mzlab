// @ts-check
// TPMINE-PRIVATE  Table of levels of Attributio (admin mode): `ATTRIBUTIO_TABLE.mount(box, data)` draws the rows of the engine (`{columns, rows}`), the count per level on top, the filters of level
// and polarity and the sort by column. Pure DOM, no engine: the colours of the levels are the `.lvb.lv-*` classes of the page (CSS variables), every control has a label and works from the keyboard.
// Texts in English, without keys of lang/ (the exception of mzFinder).
(() => {
  "use strict";
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  const HEAD = { pol: "pol", id: "id", mz: "m/z", rt: "RT", level: "level", sub: "sub", basis: "basis", name: "name", identity: "identity", reasons: "reasons" };
  const LEVELS = ["1", "2a", "2b", "3a", "3b", "4", "0"];

  /** the level of a row as the key of the profile («2a», «3b», «4», «0») @param {any[]} r */
  const key = r => String(r[4]) + (r[5] || "");
  /** @param {any[][]} rows @returns {Record<string, number>} */
  function counts(rows) {
    /** @type {Record<string, number>} */
    const c = {};
    for (const r of rows) c[key(r)] = (c[key(r)] || 0) + 1;
    return c;
  }
  /** the line of text above the table: «15 features · L1 1 · L2a 3 …» @param {any[][]} rows */
  function summary(rows) {
    const c = counts(rows);
    return `${rows.length} features` + LEVELS.filter(l => c[l]).map(l => ` · L${l} ${c[l]}`).join("");
  }
  /** rows kept by the filters and ordered by `sort` ({col, dir}); numbers by value, text by locale @param {any[][]} rows @param {{level: string, pol: string, col: number, dir: number}} f */
  function view(rows, f) {
    const out = rows.filter(r => (!f.level || key(r) === f.level) && (!f.pol || r[0] === f.pol));
    if (f.col >= 0) out.sort((a, b) => {
      const x = a[f.col], y = b[f.col];
      if (x == null || x === "") return y == null || y === "" ? 0 : 1;
      if (y == null || y === "") return -1;
      return f.dir * (typeof x === "number" && typeof y === "number" ? x - y : String(x).localeCompare(String(y)));
    });
    return out;
  }
  /** @param {any[]} r */
  const row = r => `<tr><td>${esc(r[0])}</td><td>${esc(r[1])}</td><td>${r[2] == null ? "" : Number(r[2]).toFixed(4)}</td><td>${r[3] == null ? "" : Number(r[3]).toFixed(2)}</td>`
    + `<td><span class="lvb lv-${esc(key(r))}">L${esc(key(r))}</span></td><td>${esc(r[5])}</td><td>${esc(r[6])}</td><td>${esc(r[7])}</td><td>${esc(r[8])}</td><td class="at-why">${esc(r[9])}</td>`
    + `<td><button type="button" class="at-rev" data-pol="${esc(r[0])}" data-id="${esc(r[1])}" aria-label="Review ${esc(r[0])} ${esc(r[1])}">Review</button></td></tr>`;

  /** @param {HTMLElement} box @param {{columns: string[], rows: any[][]}} data @param {{ onReview?: (list: {pol: string, id: number}[], index: number) => void }} [hooks] the Banco 2b opens from the button of a row */
  function mount(box, data, hooks = {}) {
    const f = { level: "", pol: "", col: -1, dir: 1 };
    const opts = (/** @type {string[]} */ l) => l.map(v => `<option value="${esc(v)}">${v ? "L" + esc(v) : "all"}</option>`).join("");
    box.innerHTML = `<p id="at-counts" role="status"></p>
      <div class="row"><label>Level <select id="at-flv" aria-label="Filter by level">${opts(["", ...LEVELS])}</select></label>
      <label>Polarity <select id="at-fpol" aria-label="Filter by polarity"><option value="">all</option><option>POS</option><option>NEG</option></select></label></div>
      <div class="at-scroll"><table id="at-table"><caption class="mut">Features with their level of identification</caption><thead><tr>${data.columns.map((c, i) =>
        `<th scope="col" aria-sort="none"><button type="button" data-col="${i}" aria-label="Sort by ${esc(HEAD[/** @type {keyof typeof HEAD} */ (c)] || c)}">${esc(HEAD[/** @type {keyof typeof HEAD} */ (c)] || c)}</button></th>`).join("")}<th scope="col">review</th></tr></thead><tbody></tbody></table></div>`;
    const q = (/** @type {string} */ s) => /** @type {HTMLElement} */ (box.querySelector(s));
    /** @type {any[][]} */
    let shown = [];
    const draw = () => {
      const rows = shown = view(data.rows, f);
      q("#at-counts").textContent = summary(data.rows) + (rows.length === data.rows.length ? "" : ` · ${rows.length} shown`);
      q("tbody").innerHTML = rows.map(row).join("");
      box.querySelectorAll("th:has(button)").forEach((th, i) => th.setAttribute("aria-sort", i === f.col ? (f.dir > 0 ? "ascending" : "descending") : "none"));
    };
    q("#at-flv").onchange = e => { f.level = /** @type {HTMLSelectElement} */ (e.target).value; draw(); };
    q("#at-fpol").onchange = e => { f.pol = /** @type {HTMLSelectElement} */ (e.target).value; draw(); };
    box.querySelectorAll("th button").forEach(b => b.addEventListener("click", () => {
      const i = Number(/** @type {HTMLElement} */ (b).dataset.col);
      f.dir = f.col === i ? -f.dir : 1; f.col = i; draw();
    }));
    q("tbody").addEventListener("click", e => {
      const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest(".at-rev"));
      if (!b || !hooks.onReview) return;
      const list = shown.map(r => ({ pol: String(r[0]), id: Number(r[1]) })), i = list.findIndex(x => x.pol === b.dataset.pol && String(x.id) === b.dataset.id);
      hooks.onReview(list, Math.max(0, i));
    });
    draw();
  }
  /** @type {any} */ (window).ATTRIBUTIO_TABLE = { mount, summary, counts, view, key };
})();
