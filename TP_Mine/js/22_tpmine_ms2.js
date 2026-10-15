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
  /** @type {{key:string, links:any, rows:any[], ovr:Over[], items:any[], env:any, more:boolean}} */
  const cur = { key: "", links: null, rows: [], ovr: [], items: [], env: null, more: false };
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
    for (const k of L.links) { out.push({ exp: k.exp, peak: k.peak, row: k.row, state: k.state, dmz: k.dmz, drt: k.drt, why: k.why || "", alts: k.alternatives || [], reason: "" }); seen.add(k.exp); }
    for (const u of L.unmatched) {
      out.push({ exp: u.exp, peak: u.peak, row: null, state: u.reason === "stale" ? "stale" : u.reason === "user" ? "user" : "none", dmz: null, drt: null, why: u.why || "", alts: u.rows || [], reason: u.reason });
      if (u.exp != null) seen.add(u.exp);
    }
    L.experiments.forEach((/** @type {any} */ _, /** @type {number} */ x) => { if (!seen.has(x)) out.push({ exp: x, peak: null, row: null, state: "none", dmz: null, drt: null, why: "", alts: [], reason: "" }); });
    return out;
  }
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
  const expCells = (/** @type {any} */ x) => x ? `<td title="${esc(x.file)}">${esc(String(x.file).replace(/\.mzml$/i, ""))}${x.time != null ? ` <span class="mut">t ${x.time}</span>` : ""}</td><td class="n">${num(x.precursor)}</td><td class="n">${x.ce == null ? "" : x.ce}</td><td class="n">${x.polarity > 0 ? "+" : x.polarity < 0 ? "−" : ""}</td>` : `<td></td><td></td><td></td><td></td>`;

  function tableHtml(/** @type {any[]} */ list, /** @type {boolean} */ lost) {
    const L = cur.links, head = lost ? "<th scope=\"col\">Reason</th>" : "<th scope=\"col\">Row</th><th scope=\"col\"><i>Δm/z</i></th><th scope=\"col\">ΔRT</th>";
    const body = list.map(it => {
      const x = L.experiments[it.exp], p = x && it.peak != null ? x.peaks[it.peak] : null, why = lost ? REASON[/** @type {keyof typeof REASON} */ (it.reason)] || it.why : it.why;
      const sel = x && p ? menu(it, x, cur.items.indexOf(it)) : "";
      const rm = it.state === "stale" ? ` <button type="button" class="tp-ms2-rm" title="Forget the corrections that find no peak any more">Remove</button>` : "";
      return `<tr>${expCells(x)}<td class="n">${p ? num(p.apex) : ""}</td><td>${badge(it.state)}</td>`
        + (lost ? `<td title="${esc(it.why)}">${esc(why)}${rm}</td><td>${sel}</td>` : `<td>${sel}</td><td class="n">${num(it.dmz)}</td><td class="n">${num(it.drt)}</td><td class="mut">${esc(why)}</td>`) + "</tr>";
    }).join("");
    return `<div class="tbl"><table><caption class="tp-sr">${lost ? "MS2 experiments that no row got" : "MS2 experiments matched to a row"}</caption><tr><th scope="col">File</th><th scope="col"><i>m/z</i></th><th scope="col">CE</th><th scope="col">Pol.</th>`
      + `<th scope="col">Apex (min)</th><th scope="col">State</th>${lost ? head + "<th scope=\"col\">Assign to</th>" : head + "<th scope=\"col\">Why</th>"}</tr>${body}</table></div>`;
  }

  /** The card: the matches, then the experiments no row got, the drift per file and the button that forgets the corrections. */
  function card(/** @type {any} */ s) {
    const L = s && s.ms2_links;
    if (!L || !L.experiments || !L.experiments.length) return "";
    if (L !== cur.links) { cur.links = L; cur.rows = s.rows; }
    cur.items = itemsOf(L);
    const sn = (/** @type {any} */ i) => { const p = i.exp != null && i.peak != null ? L.experiments[i.exp].peaks[i.peak] : null; return p ? p.sn : -1; };
    const m = cur.items.filter(i => i.row != null && i.state !== "isotope"), u = cur.items.filter(i => i.row == null).sort((a, b) => (b.state === "stale" ? 1e9 : sn(b)) - (a.state === "stale" ? 1e9 : sn(a))), iso = cur.items.filter(i => i.state === "isotope");
    const drift = [...new Set(L.experiments.map((/** @type {any} */ e) => e.file))].map(f => {
      const e = L.experiments.find((/** @type {any} */ y) => y.file === f); return `${esc(String(f).replace(/\.mzml$/i, ""))} ${e.drift >= 0 ? "+" : "−"}${Math.abs(e.drift).toFixed(2)} min`;
    });
    return `<div class="card" id="tp-ms2map"><h3>MS2 matches <span class="mut">${L.experiments.length} experiment${L.experiments.length > 1 ? "s" : ""} · ${m.length} matched${u.length ? ` · ${u.length} unmatched` : ""}</span></h3>`
      + (`<div class="mut" id="tp-ms2-note">Each MS2 experiment goes to the row at its m/z (±${L.prec_tol} Da) and its time. Change a match with its menu: nothing is dug again.</div>`
        + (m.length + iso.length ? tableHtml(m.concat(iso), false) : "")
        + (u.length ? `<h4 class="tp-h4">Unmatched MS2</h4>${tableHtml(u.slice(0, SHOWN), true)}${u.length > SHOWN ? `<details id="tp-ms2-more"${cur.more ? " open" : ""}><summary>${u.length - SHOWN} more, weaker</summary>${tableHtml(u.slice(SHOWN), true)}</details>` : ""}` : "")
        + `<div class="mut tp-ms2-drift">Drift per file: ${drift.join(" · ") || "none"}</div>`
        + `<div class="row"><button type="button" id="tp-ms2-reset" title="Forget every correction and use the automatic matches"${cur.ovr.length ? "" : " disabled"}>Reset matches</button><span id="tp-ms2-msg" class="mut" role="status" aria-live="polite"></span></div>`) + "</div>";
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
      const again = ep && document.querySelector(`#tp-ms2map [data-ep="${ep}"]`); if (again) /** @type {HTMLElement} */ (again).focus();
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

  /** Hooks the card to its menus: `env` = { st, call, refresh(touchedRowIds) } of 20_tpmine.js. */
  function mount(/** @type {any} */ env) {
    cur.env = env;
    const c = document.getElementById("tp-ms2map"); if (!c) return;
    c.querySelectorAll("select[data-ep]").forEach(el => el.addEventListener("change", () => pick(cur.items[+/** @type {HTMLElement} */ (el).dataset.i], /** @type {HTMLSelectElement} */ (el).value)));
    const more = c.querySelector("#tp-ms2-more"); if (more) more.addEventListener("toggle", () => { cur.more = /** @type {HTMLDetailsElement} */ (more).open; });
    const reset = /** @type {HTMLButtonElement|null} */ (c.querySelector("#tp-ms2-reset"));
    if (reset) reset.onclick = () => { const t = cur.items.map(i => i.row).filter(r => r != null); cur.ovr = []; apply(t); };
    c.querySelectorAll(".tp-ms2-rm").forEach(b => /** @type {HTMLElement} */ (b).onclick = () => { cur.ovr = cur.ovr.filter(live); apply([]); });
  }
  /** A new Dig: the corrections of this series go in, the summary of the Dig comes back (its `ms2_links` already uses them). */
  function start(/** @type {any[]} */ files) { cur.key = keyOf(files); cur.ovr = load(cur.key); cur.links = null; return cur.ovr; }

  // ---- the detail of a row
  const linksOf = (/** @type {any} */ d) => (d && d.ms2_links) || [];
  const best = (/** @type {any} */ d) => linksOf(d).reduce((/** @type {any} */ m, /** @type {any} */ k) => !m || k.scans > m.scans ? k : m, null);
  /** "from F at t min" for the words of a detail. */
  const from = (/** @type {any} */ d) => { const k = best(d); return k ? ` (from ${esc(String(k.file).replace(/\.mzml$/i, ""))}, peak at ${num(k.apex)} min)` : ""; };
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
    const lines = ks.map((/** @type {any} */ k, /** @type {number} */ i) => `<li>${esc(String(k.file).replace(/\.mzml$/i, ""))} · precursor ${num(k.precursor)} · CE ${k.ce ?? "?"} · peak at ${num(k.apex)} min · ${badge(k.state)} <span class="mut">${esc(k.why || "")}</span> <button type="button" class="tp-ms2-ch" data-k="${i}" title="Go to the menu of this MS2 experiment">Change</button></li>`).join("");
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
      const sel = /** @type {HTMLElement|null} */ (document.querySelector(`#tp-ms2map [data-ep="${x}:${p}"]`)); if (sel) { sel.scrollIntoView({ block: "center" }); sel.focus(); }
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
        it.row ?? "", r ? r.name : "", it.dmz ?? "", it.drt ?? "", it.reason ? (REASON[/** @type {keyof typeof REASON} */ (it.reason)] || it.reason) : "", it.why];
    });
    return { name: "MS2 matches", head: ["File", "t (min)", "Precursor m/z", "CE", "Polarity", "Apex (min)", "State", "Row #", "Row", "Δm/z", "ΔRT (min)", "Reason", "Why"], rows, widths: [34, 8, 12, 6, 8, 10, 11, 7, 30, 8, 9, 34, 60] };
  }
  const cell = (/** @type {any} */ r) => r.ms2 ? `${r.ms2}<span class="tp-sr"> MS2 experiment${r.ms2 > 1 ? "s" : ""}</span>` : `<span class="tp-sr">no MS2</span>`;
  const overrides = () => cur.ovr;

  const CSS = `.tp .tp-sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}
.tp #tp-ms2map .tbl{margin:6px 0}.tp #tp-ms2map select{min-height:24px;max-width:240px;font-size:12px}.tp #tp-ms2map .tp-h4{margin:10px 0 2px;font-size:13px}
.tp #tp-ms2map td .mut{white-space:nowrap}.tp .tp-ms2-drift{margin:2px 0 6px}.tp #tp-ms2map button,.tp .tp-ms2-ch{min-height:24px;min-width:24px;font-size:12px}
.tp .tp-st{display:inline-block;padding:0 7px;border-radius:9px;font-size:11px;font-weight:600;border:1px solid var(--line);color:var(--muted);white-space:nowrap}
.tp .tp-st-auto{color:var(--ok);border-color:var(--ok)}.tp .tp-st-user{color:var(--accent);border-color:var(--accent)}.tp .tp-st-ambiguous,.tp .tp-st-isf,.tp .tp-st-stale{color:var(--warn);border-color:var(--warn)}
.tp .tp-ms2-l{margin:4px 0 8px;padding-left:18px;font-size:12px}.tp .tp-ms2-l li{margin:3px 0}@media (pointer:coarse){.tp #tp-ms2map select,.tp #tp-ms2map button,.tp .tp-ms2-ch{min-height:40px}}`;
  const st = document.createElement("style"); st.textContent = CSS; document.head.appendChild(st);

  /** @type {any} */ (window).TPMS2 = { start, card, mount, detail, bindDetail, from, sheet, cell, overrides };
})();
