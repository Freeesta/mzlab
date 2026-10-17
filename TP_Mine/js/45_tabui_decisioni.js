// @ts-check
// TPMINE-PRIVATE  TPDEC: the level that the CHEMIST decides for a row of a Tabui table, and a note. The programme proposes and shows the evidence; the decided level stays empty until the chemist
// chooses, and 2b is written only here, never by the programme. The rules are those of `LIVELLI.decide` (mzlab/web/livelli.js): this file writes none. The decisions stay in the browser,
// per series (`localStorage` `qqq.tabui.decisions`, at most 50 series), and enter the exports of the tables. Texts in English; the helps are keys of the catalogue of Tabui.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const STORE = typeof NOMI !== "undefined" ? NOMI.KEY("tabui.decisions") : "qqq.tabui.decisions", MAX_SERIES = 50, NOT_TP = "nottp", EVENT = "tpdec";

  /** @typedef {{ files?: (string | { name?: string, size?: number })[], formula?: string }} Series the files of the series (names, sizes when known) and the formula of the parent */
  /** @typedef {{ l: string, n: string }} Dec one decision: level ("" for none, "nottp", or a level of LIVELLI) and note */
  /** @typedef {{ t: number, rows: Record<string, Dec> }} Entry */

  /** @returns {Record<string, Entry>} */
  function read() {
    try { const o = JSON.parse(localStorage.getItem(STORE) || "{}"); return o && typeof o === "object" && !Array.isArray(o) ? o : {}; } catch (_) { return {}; }
  }
  /** @param {Record<string, Entry>} o */
  function write(o) {
    const ks = Object.keys(o);
    if (ks.length > MAX_SERIES) for (const k of ks.sort((a, b) => (o[a].t || 0) - (o[b].t || 0)).slice(0, ks.length - MAX_SERIES)) delete o[k];   // the oldest series leave
    try { localStorage.setItem(STORE, JSON.stringify(o)); } catch (_) { /* private mode: the decision is not kept */ }
  }

  /** the print of a series: names and sizes of the files, sorted, and the formula of the parent (the same files give the same key on every run)
   * @param {Series} series */
  function key(series) {
    const fs = (series && series.files || []).map(f => typeof f === "string" ? f : `${f && f.name || ""}:${f && f.size != null ? f.size : ""}`).sort();
    const s = fs.join("|") + "#" + (series && series.formula || "");
    let h = 0x811c9dc5; for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0; }
    return h.toString(16).padStart(8, "0") + "-" + fs.length;
  }
  /** the key of a row that stays the same between two equal runs: m/z at 0.01 (HR: 0.0001) and RT at 0.05 min, not the id
   * @param {number} mz @param {number} rt @param {boolean} [hr] */
  function rowKey(mz, rt, hr) { return `${(+mz).toFixed(hr ? 4 : 2)}@${(Math.round(+rt / 0.05) * 0.05).toFixed(2)}`; }

  /** @param {Series} series @param {string} rk @returns {{ level: string, note: string } | null} */
  function get(series, rk) {
    const e = read()[key(series)], d = e && e.rows && e.rows[rk];
    return d && (d.l || d.n) ? { level: d.l || "", note: d.n || "" } : null;
  }
  /** all the decisions of a series: row key → { level, note } @param {Series} series */
  function all(series) {
    const e = read()[key(series)], out = /** @type {Record<string, { level: string, note: string }>} */ ({});
    if (e && e.rows) for (const [k, d] of Object.entries(e.rows)) if (d && (d.l || d.n)) out[k] = { level: d.l || "", note: d.n || "" };
    return out;
  }
  /** the levels that can be decided: those of LIVELLI, and «not a TP» @param {string} lv */
  const known = lv => lv === "" || lv === NOT_TP || (!!G.LIVELLI && G.LIVELLI.ORDER.includes(lv));
  /** stores a decision (level "" and no note remove it); false if the level is not one of the choices @param {Series} series @param {string} rk @param {string} level @param {string} [note] */
  function set(series, rk, level, note) {
    if (!known(level)) return false;
    const o = read(), k = key(series), e = o[k] || { t: 0, rows: {} };
    e.t = Date.now(); e.rows = e.rows || {};
    if (!level && !(note || "").trim()) delete e.rows[rk]; else e.rows[rk] = { l: level, n: (note || "").trim().slice(0, 200) };
    o[k] = e; write(o);
    window.dispatchEvent(new CustomEvent(EVENT, { detail: { fp: k, rowKey: rk } }));
    return true;
  }

  /** the proposed level of a table (numbers 0..5 for the ranking) in the scale of LIVELLI: a bare 3 is read as 3a @param {string | number} p */
  const scaleOf = p => { const s = String(p); return s === "3" ? "3a" : s; };
  /** whether the chemist may decide `lv` on a row whose proposed level is `proposed`: the rule of `LIVELLI.decide`, nothing else @param {string | number} proposed @param {string} lv @returns {{ ok: boolean, msg?: string }} */
  function check(proposed, lv) {
    if (lv === "" || lv === NOT_TP) return { ok: true };
    if (!G.LIVELLI) return { ok: false, msg: "The level rules are not loaded." };
    return G.LIVELLI.decide({ proposed: scaleOf(proposed), decided: null, std: null }, lv);
  }
  /** the words of a decided level for an export or a label @param {string} lv */
  const text = lv => !lv ? "" : lv === NOT_TP ? "not a TP" : G.LIVELLI ? G.LIVELLI.tag(lv) : "L" + lv;

  /**
   * The control of a row: a `<select>` of the decided level and a one-line note; the message of `LIVELLI.decide` goes into a `role=status` when the choice is not allowed.
   * @param {Series} series @param {{ key: string, level?: string | number | null, name?: string }} row `key` = `rowKey(...)`, `level` = the proposed level @param {(level: string, note: string) => void} [onChange]
   */
  function control(series, row, onChange) {
    const L = G.LIVELLI, w = document.createElement("span"); w.className = "tpd";
    const nm = row.name || row.key, tt = (/** @type {string} */ k) => G.TPHELP ? ` data-tt="${k}"` : "";
    const opts = [`<option value="">(no decision)</option>`, ...(L ? L.ORDER.map((/** @type {string} */ l) => `<option value="${l}">${L.tag(l)} · ${L.name(l)}</option>`) : []), `<option value="${NOT_TP}">Not a TP</option>`].join("");
    w.innerHTML = `<select class="tpd-sel" aria-label="Decided level for ${nm.replace(/[&<>"]/g, "")}"${tt("ft.decided")}>${opts}</select>`
      + `<input type="text" class="tpd-note" maxlength="200" placeholder="Note" aria-label="Note for ${nm.replace(/[&<>"]/g, "")}"${tt("ft.decided.note")}><span class="tpd-msg" role="status"></span>`;
    const sel = /** @type {HTMLSelectElement} */ (w.querySelector("select")), note = /** @type {HTMLInputElement} */ (w.querySelector("input")), msg = /** @type {HTMLElement} */ (w.querySelector(".tpd-msg"));
    const show = () => { const d = get(series, row.key); sel.value = d ? d.level : ""; if (document.activeElement !== note) note.value = d ? d.note : ""; };
    show();
    sel.addEventListener("change", () => {
      const r = check(row.level == null ? "" : row.level, sel.value);
      if (!r.ok) { msg.textContent = r.msg || "Not allowed."; show(); return; }
      msg.textContent = ""; set(series, row.key, sel.value, note.value); if (onChange) onChange(sel.value, note.value);
    });
    note.addEventListener("change", () => { set(series, row.key, sel.value, note.value); if (onChange) onChange(sel.value, note.value); });
    const sync = (/** @type {Event} */ e) => { if (!w.isConnected) { window.removeEventListener(EVENT, sync); return; } const d = /** @type {CustomEvent} */ (e).detail; if (d && d.fp === key(series) && d.rowKey === row.key) { msg.textContent = ""; show(); } };
    window.addEventListener(EVENT, sync);
    return w;
  }

  const CSS = `
.tp .tpd{display:inline-flex;flex-wrap:wrap;gap:4px 6px;align-items:center}
.tp .tpd select,.tp .tpd input{box-sizing:border-box;min-height:24px;font:inherit;font-size:12px;padding:0 6px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
.tp .tpd input{width:14em}.tp .tpd select:focus-visible,.tp .tpd input:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.tp .tpd-msg{font-size:12px;color:var(--bad)}
.tp .lvb.tpt-nottp{border-style:double;border-width:4px;padding:0 4px;background:var(--panel);color:var(--ink)}.tp .lvb.tpt-nottp s{opacity:.8}`;
  if (!document.getElementById("tpd-css")) { const s = document.createElement("style"); s.id = "tpd-css"; s.textContent = CSS; document.head.appendChild(s); }

  G.TPDEC = { key, rowKey, get, set, all, control, check, text, NOT_TP, EVENT };
})();
