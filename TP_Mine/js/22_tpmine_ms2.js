// @ts-check
// TPMINE-PRIVATE  Tabui, low resolution: which MS2 experiment went to which row, and why (`ms2_links/1`), the correction of a match with a menu
// (`ms2_relink`, no new Dig), the corrections kept in the browser, and the pieces the detail, the table of rows and the exports take from it.
// Used by 20_tpmine.js; all the text of Tabui is English, without lang/ keys. Data: window.TPMS2 (no state outside this file but the corrections).
(() => {
  "use strict";
  const STORE = "qqq.tabui.ms2links";
  const KEY_TOL = 0.05;            // the key of a correction (as in the engine): file, precursor ±0.05, CE ±0.05, apex ±rt_tol / 2
  const WIN = 0.5;                 // the menu offers the rows within ±0.5 Da of the precursor (the Q1 window of unit resolution)
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  const STATE = {
    auto: ["auto", "Matched by m/z and by time."], ambiguous: ["ambiguous", "Another row fits as well: check the menu."],
    isotope: ["isotope", "An isotope of a stronger ion: shown, not pooled into the spectrum."], isf: ["in-source", "A fragment formed in the source from the parent."],
    user: ["user", "Chosen by you."], stale: ["stale", "A correction of yours that finds no peak any more."], none: ["no row", "No row got this experiment."] };
  const REASON = {
    no_row: "No row at this m/z", no_row_at_rt: "Rows at this m/z, but at other times", no_signal: "Rows at this m/z, but no MS2 signal at their time",
    front: "Signal only in the unretained front", parent_rt: "Fragment of the parent (same time)", polarity: "Other polarity than the series",
    stale: "A correction that finds no peak any more", no_feature: "No ion of the table there", user: "Set to \"none\" by you" };

  /** @typedef {{file:string, precursor:number, ce:number|null, apex:number, row:number|null}} Over */
  /** @type {{key:string, links:any, rows:any[], ovr:Over[], items:any[], env:any, more:boolean, det:boolean, view:string, filter:string}} */
  const cur = { key: "", links: null, rows: [], ovr: [], items: [], env: null, more: false, det: false, view: "row", filter: "" };
  const G = /** @type {any} */ (window);
  const REVIEW = new Set(["ambiguous", "isf", "stale"]);          // the states counted in "need review"
  const WORD = { no_row: "no row", no_row_at_rt: "other RT", no_signal: "no signal", front: "front", parent_rt: "parent", polarity: "polarity", stale: "stale",
    no_feature: "no ion", user: "removed", before_rt_min: "too early", no_peak: "no peak" };
  /** the tips of the card (TPHELP reads them from TABUI_T: the card adds them, with one tip per state, reason and quality of this card) */
  const HELP = {
    "ms2.n.matched": "MS2 experiments that went to a row. Click to show only them; click again to show everything.",
    "ms2.n.unmatched": "MS2 experiments that no row got, the strongest first. Click to show only them; each one can be assigned to a row.",
    "ms2.n.review": "Matches to check: another row fits as well (ambiguous), a fragment formed in the source (in-source), or a correction of yours that finds no peak any more (stale).",
    "ms2.v.row": "One line per row with MS2: how many experiments support it, the best one and its quality.",
    "ms2.v.file": "One line per MS2 experiment, file by file, with the menu that changes its row.",
    "ms2.h.count": "Number of MS2 experiments matched to this row.",
    "ms2.h.best": "The best match of the row: collision energy, apex of the trace and Δm/z from the row.",
    "ms2.h.quality": "Three marks: the match is not in doubt, Δm/z within half the tolerance, ΔRT within half the RT tolerance.",
    "ms2.h.state": "How the match was made: auto, ambiguous, isotope, in-source, user or stale. The tip of each state has the full reason.",
    "ms2.h.row": "The row this MS2 belongs to. Change it with the menu: nothing is dug again.",
    "ms2.h.reason": "Why no row got this MS2, in one word; the tip of each word has the full reason.",
    "ms2.b.spec": "Opens this row in the table, where its MS2 spectrum is drawn.",
    "ms2.b.change": "Shows the matches file by file and moves the focus to the menu of this MS2.",
    "ms2.b.reset": "Forget every correction and use the automatic matches.",
    "ms2.b.rm": "Forget the corrections that find no peak any more.",
    "ms2.b.more": "The other unmatched MS2, the weaker ones by signal-to-noise.",
    "ms2.b.details": "How an MS2 is matched to a row, and the RT drift of each file." };
  /** @type {Record<string, string>} */ let dyn = {};
  /** a tip: `data-tt` for TPHELP (focusable when `focus`), else a plain `title` @param {string} key @param {string|null} [text] @param {boolean} [focus] */
  const tip = (key, text, focus) => {
    if (text != null) dyn[key] = text;
    return G.TPHELP ? ` data-tt="${key}"${focus ? ' tabindex="0"' : ""}` : ` title="${esc(text ?? /** @type {any} */ (HELP)[key] ?? "")}"`;
  };
  const SHOWN = 6;                 // the unmatched peaks listed at once (the strongest first); a trace of a real run has many small ones

  // ---- the corrections of this series in localStorage (one object, one entry per series: the names and the scan counts of its files)
  const readAll = () => { try { const o = JSON.parse(localStorage.getItem(STORE) || "{}"); return o && typeof o === "object" && !Array.isArray(o) ? o : {}; } catch (_) { return {}; } };
  /** @param {any[]} files */
  const keyOf = files => files.filter(f => f.kind === "full" || f.kind === "ms2").map(f => `${f.name}:${f.scans ?? ""}`).sort().join("|");
  /** @param {string} key @returns {Over[]} */
  const load = key => { const v = readAll()[key]; return Array.isArray(v) ? v.filter(o => o && typeof o.file === "string" && Number.isFinite(o.precursor) && Number.isFinite(o.apex)) : []; };
  /** @param {string} key @param {Over[]} ovr */
  const save = (key, ovr) => { try { const a = readAll(); if (ovr.length) a[key] = ovr; else delete a[key]; localStorage.setItem(STORE, JSON.stringify(a)); } catch (_) { /* no storage: the corrections last as long as the page */ } };

  const num = (/** @type {any} */ v, d = 2) => v == null || !Number.isFinite(+v) ? "" : (+v).toFixed(d);
  const rowTag = (/** @type {any} */ r) => r.id >= 200000 ? "t" + (r.id - 200000) : r.id >= 100000 ? "n" + (r.id - 100000) : String(r.id);
  const rowName = (/** @type {any} */ r) => `${rowTag(r)} · ${num(r.mz)}${r.ref_rt ? " @ " + num(r.ref_rt) : ""} · ${r.name || ""}`;
  const same = (/** @type {Over} */ a, /** @type {Over} */ b) => a.file === b.file && Math.abs(a.precursor - b.precursor) <= KEY_TOL && (a.ce == null ? b.ce == null : b.ce != null && Math.abs(a.ce - b.ce) <= KEY_TOL)
    && Math.abs(a.apex - b.apex) <= (cur.links ? cur.links.rt_tol : 0.25) / 2;
  const rowOf = (/** @type {number} */ id) => cur.rows.find(r => r.id === id);

  /** One line per peak of every experiment: the matches, then the peaks and experiments no row got. */
  function itemsOf(/** @type {any} */ L) {
    /** @type {any[]} */ const out = [], seen = new Set();
    for (const k of L.links) { out.push({ exp: k.exp, peak: k.peak, row: k.row, state: k.state, dmz: k.dmz, dppm: k.dppm, drt: k.drt, tol: k.tol, why: k.why || "", alts: k.alternatives || [], reason: "" }); seen.add(k.exp); }
    for (const u of L.unmatched) {
      out.push({ exp: u.exp, peak: u.peak, row: null, state: u.reason === "stale" ? "stale" : u.reason === "user" ? "user" : "none", dmz: null, dppm: null, drt: null, why: u.why || "", alts: u.rows || [], reason: u.reason });
      if (u.exp != null) seen.add(u.exp);
    }
    L.experiments.forEach((/** @type {any} */ _, /** @type {number} */ x) => { if (!seen.has(x)) out.push({ exp: x, peak: null, row: null, state: "none", dmz: null, dppm: null, drt: null, why: "", alts: [], reason: "" }); });
    return out;
  }
  const sn = (/** @type {any} */ i) => { const p = i.exp != null && i.peak != null ? cur.links.experiments[i.exp].peaks[i.peak] : null; return p ? p.sn : -1; };
  const badge = (/** @type {string} */ s) => { const [t, h] = STATE[/** @type {keyof typeof STATE} */ (s)] || STATE.none; return `<span class="tp-st tp-st-${s}" title="${esc(h)}">${t}</span>`; };

  /** The menu of one peak: the rows inside the window (the alternatives of an ambiguous match first), the current one always, and "none". */
  function menu(/** @type {any} */ it, /** @type {any} */ x, /** @type {number} */ i) {
    const P = x.precursor, ids = new Set(it.alts);
    const fit = cur.rows.filter(r => Math.abs(r.mz - P) <= WIN || r.id === it.row || ids.has(r.id));
    fit.sort((a, b) => (ids.has(b.id) ? 1 : 0) - (ids.has(a.id) ? 1 : 0) || Math.abs(a.mz - P) - Math.abs(b.mz - P));
    const cursel = it.reason === "user" ? "none" : it.state === "none" || it.state === "stale" ? "" : String(it.row);
    const opt = (/** @type {string} */ v, /** @type {string} */ t) => `<option value="${esc(v)}"${v === cursel ? " selected" : ""}>${esc(t)}</option>`;
    const lbl = `Row for the MS2 of m/z ${num(P)} at RT ${num(x.peaks[it.peak].apex)}`;
    return `<select data-ep="${it.exp}:${it.peak}" data-i="${i}" aria-label="${esc(lbl)}" title="Choose the row this MS2 belongs to">${cursel === "" ? opt("", "not matched") : ""}`
      + fit.map(r => opt(String(r.id), rowName(r))).join("") + opt("none", "none (not this ion)") + "</select>";
  }
  /** the condition of an MS2 file (series that compares conditions), set by the page after a Dig: file name → condition, «shared» for a file with none */
  /** @type {Map<string, string> | null} */ let conditions = null;
  const condTag = (/** @type {any} */ f) => conditions && conditions.has(f) ? ` <span class="mut">· ${esc(conditions.get(f) || "shared")}</span>` : "";
  const short = (/** @type {any} */ f) => String(f).replace(/\.mzml$/i, "");
  const expCells = (/** @type {any} */ x, /** @type {boolean} */ pol) => x ? `<td title="${esc(x.file)}">${esc(short(x.file))}${x.time != null ? ` <span class="mut">t ${x.time}</span>` : ""}${condTag(x.file)}</td><td class="n">${num(x.precursor)}</td><td class="n">${x.ce == null ? "" : x.ce}</td>${pol ? `<td class="n">${x.polarity > 0 ? "+" : x.polarity < 0 ? "−" : ""}</td>` : ""}` : `<td></td><td></td><td></td>${pol ? "<td></td>" : ""}`;
  /** the help of a header cell: a focusable label when TPHELP is there */
  const th = (/** @type {string} */ t, /** @type {string} */ k) => `<th scope="col">${k ? `<span${tip(k, null, true)}>${t}</span>` : t}</th>`;
  const sdmz = (/** @type {any} */ v) => v == null ? "" : `${v >= 0 ? "+" : "−"}${Math.abs(v).toFixed(2)} Da`;

  /** One state in one word; its tip says what the state means and the full sentence of the engine. */
  function stateTag(/** @type {any} */ it) {
    const [t, h] = STATE[/** @type {keyof typeof STATE} */ (it.state)] || STATE.none;
    return `<span class="tp-st tp-st-${it.state}"${tip("ms2.s." + cur.items.indexOf(it), h + (it.why ? " " + it.why : ""), true)}>${t}</span>`;
  }
  /** The reason of an unmatched MS2 in one word; its tip has the sentence. */
  function reasonTag(/** @type {any} */ it) {
    const full = REASON[/** @type {keyof typeof REASON} */ (it.reason)] || it.why || "No row got this experiment";
    const w = WORD[/** @type {keyof typeof WORD} */ (it.reason)] || (it.reason ? String(it.reason).replace(/_/g, " ") : "no row");
    return `<span class="tp-rs"${tip("ms2.r." + cur.items.indexOf(it), full + "." + (it.why && it.why !== full ? " " + it.why : ""), true)}>${esc(w)}</span>`;
  }
  /** Three marks of a match: not in doubt, Δm/z within half the tolerance, ΔRT within half the RT tolerance; the tip says each in words. */
  function quality(/** @type {any} */ it) {
    const L = cur.links, a = it.state === "auto" || it.state === "user", b = it.dmz != null && it.tol != null && Math.abs(it.dmz) <= it.tol / 2, c = it.drt != null && Math.abs(it.drt) <= L.rt_tol / 2;
    const n = +a + +b + +c;
    const txt = `${a ? "Match not in doubt" : `Match to check (${(STATE[/** @type {keyof typeof STATE} */ (it.state)] || STATE.none)[0]})`}; Δm/z ${sdmz(it.dmz)}${it.tol != null ? ` of ±${num(it.tol)} allowed` : ""}; ΔRT ${num(it.drt)} min of ±${num(L.rt_tol)} allowed.`;
    return `<span class="tp-ms2q" role="img" aria-label="Quality ${n} of 3"${tip("ms2.q." + cur.items.indexOf(it), txt, true)}>${[a, b, c].map(v => `<i${v ? ' class="on"' : ""}></i>`).join("")}</span>`;
  }

  /** The view "By row": one line per row with MS2, the number of matches, the best one and its quality, the spectrum and the menu of the best one. */
  function byRow(/** @type {any[]} */ m) {
    /** @type {Map<number, any[]>} */ const by = new Map();
    for (const it of m) { const l = by.get(it.row); if (l) l.push(it); else by.set(it.row, [it]); }
    const rank = (/** @type {any} */ it) => REVIEW.has(it.state) ? 2 : it.state === "isotope" ? 1 : 0;
    const list = [...by].map(([id, l]) => ({ r: rowOf(id), l, b: l.slice().sort((x, y) => rank(x) - rank(y) || sn(y) - sn(x))[0] })).filter(o => o.r).sort((x, y) => x.r.mz - y.r.mz);
    const body = list.map(({ r, l, b }) => {
      const x = cur.links.experiments[b.exp], p = x.peaks[b.peak], tag = rowTag(r);
      return `<tr data-rev="${l.some(i => REVIEW.has(i.state)) ? 1 : 0}"><td><b>${esc(tag)}</b> <span class="mut">${esc(r.name || "")}</span></td><td class="n">${num(r.mz)}</td><td class="n">${l.length}</td>`
        + `<td class="n tp-ms2-best">CE ${x.ce ?? "?"} · ${num(p.apex)} min · ${sdmz(b.dmz)}</td><td>${quality(b)}</td>`
        + `<td class="tp-ms2-act"><button type="button" class="tp-ms2-sp" data-id="${r.id}" aria-label="Spectrum of row ${esc(tag)}"${tip("ms2.b.spec")}>Spectrum</button>`
        + `<button type="button" class="tp-ms2-go" data-go="${b.exp}:${b.peak}" aria-label="Change the match of row ${esc(tag)}"${tip("ms2.b.change")}>Change</button></td></tr>`;
    }).join("");
    return `<div class="tbl"><table><caption class="tp-sr">Rows with MS2 and their best match</caption><tr>${th("Row", "")}${th("<i>m/z</i>", "")}${th("MS2", "ms2.h.count")}${th("Best match", "ms2.h.best")}${th("Quality", "ms2.h.quality")}<td></td></tr>${body}</table></div>`;
  }

  /** The view "By file" (`lost` false: one line per matched MS2 with its menu) and the list of the unmatched ones (`lost` true). */
  function tableHtml(/** @type {any[]} */ list, /** @type {boolean} */ lost) {
    const L = cur.links;
    const body = list.map(it => {
      const x = L.experiments[it.exp], p = x && it.peak != null ? x.peaks[it.peak] : null;
      const sel = x && p ? menu(it, x, cur.items.indexOf(it)) : "";
      const rm = it.state === "stale" ? ` <button type="button" class="tp-ms2-rm"${tip("ms2.b.rm")}>Remove</button>` : "";
      return `<tr data-rev="${REVIEW.has(it.state) ? 1 : 0}">${expCells(x, !lost)}<td class="n">${p ? num(p.apex) : ""}</td>`
        + (lost ? `<td>${reasonTag(it)}${rm}</td><td>${sel}</td>` : `<td>${stateTag(it)}</td><td>${sel}</td><td class="n">${it.dmz == null ? "" : `${num(it.dmz)} Da · ${num(it.dppm)} ppm`}</td><td class="n">${num(it.drt)}</td>`) + "</tr>";
    }).join("");
    return `<div class="tbl"><table><caption class="tp-sr">${lost ? "MS2 experiments that no row got" : "MS2 experiments matched to a row"}</caption><tr><th scope="col">File</th><th scope="col"><i>m/z</i></th><th scope="col">CE</th>${lost ? "" : "<th scope=\"col\">Pol.</th>"}`
      + `<th scope="col">Apex (min)</th>${lost ? th("Reason", "ms2.h.reason") + th("Assign to", "ms2.h.row") : th("State", "ms2.h.state") + th("Row", "ms2.h.row") + "<th scope=\"col\"><i>Δm/z</i> (Da · ppm)</th><th scope=\"col\">ΔRT (min)</th>"}</tr>${body}</table></div>`;
  }

  /** The card: three numbers that filter, the view by row (or by file), the unmatched MS2, the button that forgets the corrections, and the details. */
  function card(/** @type {any} */ s) {
    const L = s && s.ms2_links;
    if (!L || !L.experiments || !L.experiments.length) return "";
    if (L !== cur.links) { cur.links = L; cur.rows = s.rows; }
    cur.items = itemsOf(L); dyn = {};
    const m = cur.items.filter(i => i.row != null && i.state !== "isotope"), u = cur.items.filter(i => i.row == null).sort((a, b) => (b.state === "stale" ? 1e9 : sn(b)) - (a.state === "stale" ? 1e9 : sn(a))), iso = cur.items.filter(i => i.state === "isotope");
    const rev = cur.items.filter(i => REVIEW.has(i.state)).length, nRows = new Set(m.concat(iso).map(i => i.row)).size, nx = L.experiments.length;
    const drift = [...new Set(L.experiments.map((/** @type {any} */ e) => e.file))].map(f => {
      const e = L.experiments.find((/** @type {any} */ y) => y.file === f); return `<li>${esc(short(f))} ${e.drift >= 0 ? "+" : "−"}${Math.abs(e.drift).toFixed(2)} min</li>`;
    });
    const fb = (/** @type {string} */ f, /** @type {number} */ n, /** @type {string} */ t) => `<button type="button" class="tp-ms2-f" data-f="${f}" aria-pressed="false"${n ? "" : " disabled"}${tip("ms2.n." + f)}><b>${n}</b> ${t}</button>`;
    const vb = (/** @type {string} */ v, /** @type {string} */ t) => `<button type="button" data-vw="${v}" aria-pressed="false"${tip("ms2.v." + v)}>${t}</button>`;
    const html = `<div class="card" id="tp-ms2map"><h3>MS2 matches <span class="mut">${nx} experiment${nx > 1 ? "s" : ""} · ${nRows} row${nRows === 1 ? "" : "s"} with MS2</span></h3>`
      + `<div class="tp-ms2-bar"><div class="tp-ms2-sum" role="group" aria-label="Show only">${fb("matched", m.length, "matched")}${fb("unmatched", u.length, "unmatched")}${fb("review", rev, "need review")}</div>`
      + `<div class="tp-ms2-vw" role="group" aria-label="View">${vb("row", "By row")}${vb("file", "By file")}</div></div>`
      + `<div id="tp-ms2-rows">${m.length + iso.length ? byRow(m.concat(iso)) : ""}</div><div id="tp-ms2-files" hidden>${m.length + iso.length ? tableHtml(m.concat(iso), false) : ""}</div>`
      + (u.length ? `<div id="tp-ms2-orph"><h4 class="tp-h4">Unmatched MS2 (${u.length})</h4>${tableHtml(u.slice(0, SHOWN), true)}${u.length > SHOWN ? `<details id="tp-ms2-more"${cur.more ? " open" : ""}><summary${tip("ms2.b.more")}>${u.length - SHOWN} more, weaker</summary>${tableHtml(u.slice(SHOWN), true)}</details>` : ""}</div>` : "")
      + `<div class="row"><button type="button" id="tp-ms2-reset"${tip("ms2.b.reset")}${cur.ovr.length ? "" : " disabled"}>Reset matches</button><span id="tp-ms2-msg" class="mut" role="status" aria-live="polite"></span></div>`
      + `<details id="tp-ms2-det"${cur.det ? " open" : ""}><summary${tip("ms2.b.details")}>Details</summary><div class="mut" id="tp-ms2-note">Each MS2 experiment goes to the row at its <i>m/z</i> (±${L.prec_tol} Da) and its time. A menu changes a match: nothing is dug again.</div>`
      + `<div class="mut tp-ms2-drift">RT drift per file:</div><ul class="tp-ms2-dl">${drift.join("") || "<li>none</li>"}</ul></details></div>`;
    if (G.TABUI_T && G.TABUI_T.en) Object.assign(G.TABUI_T.en, HELP, dyn);
    return html;
  }

  /** Shows the view and the filter chosen: the pressed buttons, the sections and the lines that pass. */
  function applyView(/** @type {HTMLElement} */ c) {
    const f = cur.filter, v = cur.view, rev = f === "review";
    c.querySelectorAll("[data-f]").forEach(b => b.setAttribute("aria-pressed", String(/** @type {HTMLElement} */ (b).dataset.f === f)));
    c.querySelectorAll("[data-vw]").forEach(b => b.setAttribute("aria-pressed", String(/** @type {HTMLElement} */ (b).dataset.vw === v)));
    const vw = /** @type {HTMLElement|null} */ (c.querySelector(".tp-ms2-vw")); if (vw) vw.hidden = f === "unmatched";
    c.querySelectorAll("tr[data-rev]").forEach(tr => { /** @type {HTMLElement} */ (tr).hidden = rev && /** @type {HTMLElement} */ (tr).dataset.rev !== "1"; });
    const any = (/** @type {Element} */ el) => [...el.querySelectorAll("tr[data-rev]")].some(tr => !/** @type {HTMLElement} */ (tr).hidden);
    /** @type {[string, boolean][]} */ const sec = [["#tp-ms2-rows", f !== "unmatched" && v === "row"], ["#tp-ms2-files", f !== "unmatched" && v === "file"], ["#tp-ms2-orph", f !== "matched"], ["#tp-ms2-more", true]];
    for (const [q, on] of sec) { const el = /** @type {HTMLElement|null} */ (c.querySelector(q)); if (el) el.hidden = !on || (rev && !any(el)); }
  }
  /** Moves the focus to the menu of one MS2 (`exp:peak`): the view "By file" when the menu is there, no filter, the sections open. */
  function focusMenu(/** @type {string} */ ep) {
    const c = document.getElementById("tp-ms2map"), sel = c && /** @type {HTMLElement|null} */ (c.querySelector(`select[data-ep="${ep}"]`));
    if (!c || !sel) return false;
    if (sel.closest("#tp-ms2-files")) cur.view = "file";
    cur.filter = ""; applyView(c);
    for (let d = sel.closest("details"); d; d = d.parentElement ? d.parentElement.closest("details") : null) d.open = true;
    sel.scrollIntoView({ block: "center" }); sel.focus();
    return true;
  }

  /** Recount of `rows[].ms2` from the matches, then the page redraws the card, the table and the open detail. */
  function recount(/** @type {any} */ s) { for (const r of s.rows) r.ms2 = s.ms2_links.links.filter((/** @type {any} */ k) => k.row === r.id).length; }
  async function apply(/** @type {number[]} */ touched) {
    const env = cur.env, msg = (/** @type {string} */ t, /** @type {boolean} */ e = false) => { const el = document.getElementById("tp-ms2-msg"); if (el) { el.textContent = t; el.classList.toggle("err", e); } };
    msg("Matching…");
    try {
      const L = await env.call("ms2_relink", cur.ovr), s = env.st.summary;
      s.ms2_links = L; recount(s); save(cur.key, cur.ovr);
      const ep = document.activeElement && /** @type {HTMLElement} */ (document.activeElement).dataset ? /** @type {HTMLElement} */ (document.activeElement).dataset.ep : null;
      env.st.det = {}; env.refresh(touched);
      const again = ep && /** @type {HTMLElement|null} */ (document.querySelector(`#tp-ms2map [data-ep="${ep}"]`));
      if (again) { if (again.offsetParent) again.focus(); else focusMenu(/** @type {string} */ (ep)); }            // an MS2 assigned from the unmatched list: its menu is now in the view "By file"
      msg(cur.ovr.length ? `${cur.ovr.length} correction${cur.ovr.length > 1 ? "s" : ""} kept in this browser.` : "");
    } catch (e) { msg(String(/** @type {any} */ (e).message || e), true); }
  }
  /** @param {any} it @param {string} v */
  async function pick(it, v) {
    const x = cur.links.experiments[it.exp], p = x.peaks[it.peak];
    /** @type {Over} */ const o = { file: x.file, precursor: x.precursor, ce: x.ce, apex: p.apex, row: v === "none" ? null : +v };
    cur.ovr = cur.ovr.filter(q => !same(q, o)); cur.ovr.push(o);
    await apply([it.row, o.row].filter(r => r != null));
  }
  const live = (/** @type {Over} */ o) => cur.links.experiments.some((/** @type {any} */ x) => x.file === o.file && x.peaks.some((/** @type {any} */ p) => same(o, { file: x.file, precursor: x.precursor, ce: x.ce, apex: p.apex, row: null })) && Math.abs(x.precursor - o.precursor) <= KEY_TOL);

  /** Hooks the card to its menus and buttons: `env` = { st, call, refresh(touchedRowIds), show(rowId) } of 20_tpmine.js. */
  function mount(/** @type {any} */ env) {
    cur.env = env;
    const c = document.getElementById("tp-ms2map"); if (!c) return;
    c.querySelectorAll("select[data-ep]").forEach(el => el.addEventListener("change", () => pick(cur.items[+/** @type {HTMLElement} */ (el).dataset.i], /** @type {HTMLSelectElement} */ (el).value)));
    const more = c.querySelector("#tp-ms2-more"); if (more) more.addEventListener("toggle", () => { cur.more = /** @type {HTMLDetailsElement} */ (more).open; });
    const reset = /** @type {HTMLButtonElement|null} */ (c.querySelector("#tp-ms2-reset"));
    if (reset) reset.onclick = () => { const t = cur.items.map(i => i.row).filter(r => r != null); cur.ovr = []; apply(t); };
    c.querySelectorAll(".tp-ms2-rm").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => { cur.ovr = cur.ovr.filter(live); apply([]); });
    c.querySelectorAll("[data-f]").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => { const f = /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.f); cur.filter = cur.filter === f ? "" : f; applyView(c); });
    c.querySelectorAll("[data-vw]").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => { cur.view = /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.vw); applyView(c); });
    c.querySelectorAll(".tp-ms2-sp").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => { if (env.show) env.show(+(/** @type {HTMLElement} */ (b).dataset.id || -1)); });
    c.querySelectorAll(".tp-ms2-go").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => focusMenu(/** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.go)));
    const det = c.querySelector("#tp-ms2-det"); if (det) det.addEventListener("toggle", () => { cur.det = /** @type {HTMLDetailsElement} */ (det).open; });
    if (!c.querySelector(`[data-f="${cur.filter}"]:not([disabled])`)) cur.filter = "";            // a filter left with nothing in it (after a correction) goes away
    applyView(c);
  }
  /** A new Dig: the corrections of this series go in, the summary of the Dig comes back (its `ms2_links` already uses them). */
  function start(/** @type {any[]} */ files) { cur.key = keyOf(files); cur.ovr = load(cur.key); cur.links = null; return cur.ovr; }

  // ---- the detail of a row
  const linksOf = (/** @type {any} */ d) => (d && d.ms2_links) || [];
  const best = (/** @type {any} */ d) => linksOf(d).reduce((/** @type {any} */ m, /** @type {any} */ k) => !m || k.scans > m.scans ? k : m, null);
  /** "from F at t min" for the words of a detail. */
  const from = (/** @type {any} */ d) => { const k = best(d); return k ? ` (from ${esc(String(k.file).replace(/\.mzml$/i, ""))}, peak at ${num(k.apex)} min)` : ""; };
  /** The representative scan of a detail in words: where it comes from, Δm/z in Da and ppm, ΔRT, the tolerance used and the other scans of the peak. */
  function repLine(/** @type {any} */ d) {
    const r = d && d.ms2_rep; if (!r) return "";
    const src = r.tol_window ? "half the isolation window of the file" : "table tolerance, the file has no isolation window";
    return `Representative MS2: ${String(r.file).replace(/\.mzml$/i, "")} at RT ${num(r.rt)} min (the most intense of ${r.n_in} scan${r.n_in > 1 ? "s" : ""} in the peak by ${r.intensity_src}${r.others ? `; ${r.others} other${r.others > 1 ? "s" : ""} not drawn` : ""}), precursor m/z ${num(r.precursor)}, Δm/z ${num(r.dmz)} Da (${num(r.dppm)} ppm), ΔRT ${num(r.drt)} min, tolerance ±${num(r.tol)} Da (${src}).`;
  }
  /** Why a row has no MS2 when an experiment with its precursor exists. */
  function missing(/** @type {any} */ d) {
    const L = cur.links; if (!L) return "";
    const u = L.unmatched.find((/** @type {any} */ y) => (y.rows || []).includes(d.id) || (y.exp != null && Math.abs(L.experiments[y.exp].precursor - d.mz_extracted) <= WIN));
    return u ? `${esc(REASON[/** @type {keyof typeof REASON} */ (u.reason)] || u.reason)}${u.why ? ` (${esc(u.why)})` : ""}.` : "";
  }
  const fragTable = (/** @type {any[]} */ fr) => `<table><tr><th scope="col"><i>m/z</i></th><th scope="col">%</th><th scope="col">loss</th><th scope="col">note</th></tr>${fr.map(f => `<tr><td class="n">${f.mz}</td><td class="n">${f.rel}</td><td class="n">${f.loss ?? ""}</td><td>${esc(f.note || "")}</td></tr>`).join("")}</table>`;
  /** The "Product ions (MS2)" card of a detail: one line per match with "Change", a CE menu when several CEs, then the fragments. */
  function detail(/** @type {any} */ d) {
    const ks = linksOf(d), m = d.ms2;
    const lines = ks.map((/** @type {any} */ k, /** @type {number} */ i) => `<li>${esc(String(k.file).replace(/\.mzml$/i, ""))}${condTag(k.file)} · precursor ${num(k.precursor)} · CE ${k.ce ?? "?"} · peak at ${num(k.apex)} min · ${badge(k.state)} <span class="mut">${esc(k.why || "")}</span> <button type="button" class="tp-ms2-ch" data-k="${i}" title="Go to the menu of this MS2 experiment">Change</button></li>`).join("");
    const ces = [...new Set(ks.map((/** @type {any} */ k) => k.ce))];
    const menuCe = ces.length > 1 ? `<label class="mut">Collision energy <select id="tp-ms2-ce" aria-label="Collision energy of the product ions shown">${ces.map(c => `<option value="${c ?? ""}"${c === m.collision_energy ? " selected" : ""}>${c ?? "?"}</option>`).join("")}</select></label>` : "";
    const none = m.scans ? "" : `<div class="mut">${missing(d) || "No MS2 scan with this precursor inside the peak."}</div>`;
    return `<h3>Product ions (MS2)</h3>${lines ? `<ul class="tp-ms2-l">${lines}</ul>` : ""}${m.scans ? `<div class="mut">${m.scans} scans, precursor ${m.precursor}, CE ${m.collision_energy ?? "?"}</div>${menuCe}<div id="tp-ms2-fr">${fragTable(m.fragments)}</div>` : none}`;
  }
  /** Hooks the buttons and the CE menu of the detail. */
  function bindDetail(/** @type {HTMLElement} */ root, /** @type {any} */ d) {
    const ks = linksOf(d);
    root.querySelectorAll(".tp-ms2-ch").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => {
      const k = ks[+/** @type {HTMLElement} */ (b).dataset.k], L = cur.links; if (!k || !L) return;
      const x = L.experiments.findIndex((/** @type {any} */ e) => e.file === k.file && Math.abs(e.precursor - k.precursor) <= KEY_TOL && (e.ce ?? null) === (k.ce ?? null));
      const p = x < 0 ? -1 : L.experiments[x].peaks.findIndex((/** @type {any} */ q) => Math.abs(q.apex - k.apex) <= 0.01);
      focusMenu(`${x}:${p}`);
    });
    const ce = /** @type {HTMLSelectElement|null} */ (root.querySelector("#tp-ms2-ce")), box = root.querySelector("#tp-ms2-fr");
    if (ce && box) ce.onchange = () => {
      const k = ks.filter((/** @type {any} */ y) => String(y.ce ?? "") === ce.value).reduce((/** @type {any} */ a, /** @type {any} */ y) => !a || y.scans > a.scans ? y : a, null);
      box.innerHTML = k ? fragTable(k.fragments) : "";
    };
  }

  // ---- the sheet of the Excel export and the column of the table of rows
  function sheet() {
    const L = cur.links; if (!L) return null;
    const rows = cur.items.map(it => {
      const x = L.experiments[it.exp], p = x && it.peak != null ? x.peaks[it.peak] : null, r = it.row != null ? rowOf(it.row) : null;
      return [x ? x.file : "", x && x.time != null ? x.time : "", x ? x.precursor : "", x && x.ce != null ? x.ce : "", x ? x.polarity : "", p ? p.apex : "", (STATE[/** @type {keyof typeof STATE} */ (it.state)] || STATE.none)[0],
        it.row ?? "", r ? r.name : "", it.dmz ?? "", it.dppm ?? "", it.drt ?? "", it.reason ? (REASON[/** @type {keyof typeof REASON} */ (it.reason)] || it.reason) : "", it.why];
    });
    return { name: "MS2 matches", head: ["File", "t (min)", "Precursor m/z", "CE", "Polarity", "Apex (min)", "State", "Row #", "Row", "Δm/z (Da)", "Δm/z (ppm)", "ΔRT (min)", "Reason", "Why"], rows, widths: [34, 8, 12, 6, 8, 10, 11, 7, 30, 9, 10, 9, 34, 60] };
  }
  /** The sheet «MS2 representative»: for every row with an MS2 scan inside its peak, the scan that stands for it, its Δm/z, ΔRT and tolerance, and the sentence of the fragments found. */
  function repSheet(/** @type {any[]} */ det) {
    const rows = det.filter(d => d.ms2_rep).map(d => { const r = d.ms2_rep; return [d.id, d.name, r.file, r.rt, r.precursor, r.dmz, r.dppm, r.drt, r.tol, r.tol_window ? "half isolation window" : "table tolerance", r.n_in, r.others, d.ms2_text]; });
    return rows.length ? { name: "MS2 representative", head: ["#", "Candidate", "File", "RT (min)", "Precursor m/z", "Δm/z (Da)", "Δm/z (ppm)", "ΔRT (min)", "Tolerance (Da)", "Tolerance from", "Scans in the peak", "Other scans", "Fragment ions found"], rows, widths: [6, 38, 34, 9, 12, 9, 10, 9, 12, 20, 10, 10, 120] } : null;
  }
  const cell = (/** @type {any} */ r) => r.ms2 ? `${r.ms2}<span class="tp-sr"> MS2 experiment${r.ms2 > 1 ? "s" : ""}</span>` : `<span class="tp-sr">no MS2</span>`;
  const overrides = () => cur.ovr;

  const CSS = `.tp .tp-sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}
.tp #tp-ms2map .tbl{margin:6px 0}.tp #tp-ms2map select{min-height:24px;max-width:240px;font-size:12px}.tp #tp-ms2map .tp-h4{margin:10px 0 2px;font-size:13px}
.tp #tp-ms2map td .mut{white-space:nowrap}.tp .tp-ms2-drift{margin:2px 0 6px}.tp #tp-ms2map button,.tp .tp-ms2-ch{min-height:24px;min-width:24px;font-size:12px}
.tp .tp-st{display:inline-block;padding:0 7px;border-radius:9px;font-size:11px;font-weight:600;border:1px solid var(--line);color:var(--muted);white-space:nowrap}
.tp .tp-st-auto{color:var(--ok);border-color:var(--ok)}.tp .tp-st-user{color:var(--accent);border-color:var(--accent)}.tp .tp-st-ambiguous,.tp .tp-st-isf,.tp .tp-st-stale{color:var(--warn);border-color:var(--warn)}
.tp #tp-ms2-rows td .mut{white-space:normal}.tp #tp-ms2map .tp-ms2-act,.tp #tp-ms2map .tp-ms2-best{white-space:nowrap}.tp #tp-ms2map .tp-ms2-act button+button{margin-left:4px}
.tp #tp-ms2map h3 .mut{font-weight:400}.tp .tp-ms2-bar{display:flex;flex-wrap:wrap;gap:6px 14px;align-items:center;margin:6px 0}.tp .tp-ms2-sum,.tp .tp-ms2-vw{display:flex;flex-wrap:wrap;gap:4px}
.tp .tp-ms2-vw[hidden]{display:none}.tp .tp-ms2-f b{font-size:14px;margin-right:3px}.tp .tp-ms2-f[data-f=review]:not([disabled]) b{color:var(--warn)}
.tp #tp-ms2map [aria-pressed=true]{border-color:var(--accent);color:var(--accent);box-shadow:inset 0 0 0 1px var(--accent);font-weight:600}
.tp .tp-ms2q{display:inline-flex;gap:2px;vertical-align:middle;padding:3px 2px}.tp .tp-ms2q i{display:block;width:7px;height:12px;border-radius:2px;border:1px solid var(--muted);box-sizing:border-box}
.tp .tp-ms2q i.on{background:var(--ok);border-color:var(--ok)}.tp .tp-rs{white-space:nowrap}.tp #tp-ms2-det{margin-top:4px}.tp #tp-ms2-det>summary{font-size:12px;min-height:24px}
.tp .tp-ms2-l,.tp .tp-ms2-dl{margin:4px 0 8px;padding-left:18px;font-size:12px}.tp .tp-ms2-l li{margin:3px 0}@media (pointer:coarse){.tp #tp-ms2map select,.tp #tp-ms2map button,.tp .tp-ms2-ch{min-height:40px}}`;
  const st = document.createElement("style"); st.textContent = CSS; document.head.appendChild(st);

  const setConditions = (/** @type {Map<string, string> | null} */ m) => { conditions = m; };
  /** @type {any} */ (window).TPMS2 = { start, card, mount, detail, bindDetail, from, repLine, sheet, repSheet, cell, overrides, setConditions };
})();
