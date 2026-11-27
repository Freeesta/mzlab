// @ts-check
// TPMINE-PRIVATE  TPSLOTS: the «Slots» tab of an open row of a Tabui table. The slots and their states come from `feature-slots/1` (Rust, `table slots`); this file only lays them out
// in a fixed grid (structure, XIC, kinetics, MS2, isotopes, UV-Vis, SIM; three columns, one under 600 px) and reuses the parts of TPRIGA for the charts. A slot without evidence says which
// step or command fills it; it never shows a made-up value. State is a word and a mark, never colour alone. Texts in English; colours only from CSS variables.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /** @typedef {{ name: string, sha256: string }} SlotFile
   * @typedef {{ slot: string, state: "filled" | "empty" | "na", reason: string | null, evidence: any, source: { output: string | null, step: string | null, files: SlotFile[] } | null, fill: { step: string | null, command: string | null } | null }} Slot
   * @typedef {{ schema: string, id: number, slots: Slot[] }} Slots */

  const CSS = `
.tp .tpsl{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px;align-items:start}
@media(max-width:600px){.tp .tpsl{grid-template-columns:minmax(0,1fr)}}
.tp .tpsl-card{min-width:0;border:1px solid var(--line);border-radius:6px;padding:6px 8px;background:var(--panel)}
.tp .tpsl-card h4{display:flex;flex-wrap:wrap;gap:2px 8px;align-items:baseline;justify-content:space-between;margin:0 0 4px;font-size:12px;font-weight:600}
.tp .tpsl-st{font-size:11px;font-weight:400;color:var(--muted)}.tp .tpsl-card[data-state=filled] .tpsl-st{color:var(--ok)}
.tp .tpsl-p{margin:2px 0;font-size:12px;overflow-wrap:anywhere}.tp .tpsl-p.m{color:var(--muted)}.tp .tpsl-p code{font-size:11px}
.tp .tpsl-card details{margin:4px 0 0;font-size:11px;color:var(--muted)}.tp .tpsl-card summary{cursor:pointer;min-height:24px;display:flex;align-items:center}
.tp .tpsl-card ul{margin:2px 0;padding:0 0 0 16px}
.tp .tpsl-card .tpr-ch svg{max-height:150px}
@media(pointer:coarse){.tp .tpsl-card summary{min-height:40px}}`;

  const TITLE = /** @type {Record<string, string>} */ ({ structure: "Structure", xic: "Peak / XIC", kinetics: "Kinetics", ms2: "MS2", isotopes: "Isotopes / adducts", uv: "UV-Vis", sim: "SIM / MRM" });
  const STATE = /** @type {Record<string, string>} */ ({ filled: "✓ filled", empty: "○ empty", na: "– not applicable" });
  const WHY = /** @type {Record<string, string>} */ ({
    no_identity: "this table holds no identity", files_do_not_cover: "no file covers this feature", no_time_axis: "the files have no time axis (two or more times are needed)",
    not_a_transition_table: "this is not a transition table",
  });
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const num = (/** @type {any} */ v, /** @type {number} */ d) => (typeof v === "number" && Number.isFinite(v) ? v.toFixed(d) : "–");
  const p = (/** @type {string} */ html, /** @type {string} */ cls = "") => `<p class="tpsl-p ${cls}">${html}</p>`;

  /** the facts of a filled slot, as text (every number is the engine's, none is computed here) */
  function facts(/** @type {Slot} */ s) {
    const e = s.evidence || {};
    if (s.slot === "structure") return p(`Neutral mass ${num(e.neutral, 4)}${e.adduct ? ` · ${esc(e.adduct)}` : ""}${e.sources && e.sources.length ? ` · from ${esc(e.sources.join(", "))}` : ""}`);
    if (s.slot === "xic") return p(`${e.files} files: ${e.detected} detected, ${e.filled} filled, ${e.targeted} targeted, ${e.absent} absent, ${e.na} not acquired`) + p(`Peak edges ${num(e.rt_lo, 2)}–${num(e.rt_hi, 2)} min`, "m");
    if (s.slot === "kinetics") return p(`${e.points} points at ${esc((e.times_min || []).join(", "))} min`);
    if (s.slot === "ms2") return p(`File ${esc(e.file)}${typeof e.purity === "number" ? ` · precursor purity ${num(e.purity, 2)}` : ""}`);
    if (s.slot === "isotopes") return p(`M+1/M ${num(e.iso_m1, 3)}${e.adduct ? ` · ${esc(e.adduct)}` : ""}${e.family_size ? ` · ${e.family_size} ions in the family` : ""}`);
    if (s.slot === "sim") return p(`Q1 ${num(e.q1, 3)} → Q3 ${num(e.q3, 3)}`);
    return "";
  }

  /** where a filled slot comes from: output, step, and the input files with their SHA-256 */
  function source(/** @type {Slot} */ s) {
    const o = s.source;
    if (!o) return "";
    const fs = o.files.map(f => `<li>${esc(f.name)} <code>${esc(f.sha256)}</code></li>`).join("");
    return p(`Source: ${esc(o.output || "this table")}${o.step ? ` · step <code>${esc(o.step)}</code>` : ""}`, "m") + (fs ? `<details><summary>${o.files.length} input file${o.files.length === 1 ? "" : "s"}</summary><ul>${fs}</ul></details>` : "");
  }

  /** what an empty or n/a slot says: why, then the step and the command that fill it */
  function missing(/** @type {Slot} */ s) {
    const f = s.fill || { step: null, command: null }, why = s.state === "na" ? WHY[s.reason || ""] || s.reason || "not applicable" : "nothing in this table yet";
    const how = [f.step ? `step <code>${esc(f.step)}</code>` : "", f.command ? `<code>${esc(f.command)}</code>` : ""].filter(Boolean).join(" or ");
    return p(`${esc(why[0].toUpperCase() + why.slice(1))}.`) + (s.state === "empty" && how ? p(`Fill it with ${how}.`, "m") : "");
  }

  /** a chart of TPRIGA in its own box (`kind`: xic, spec, spec2) */
  function chart(/** @type {HTMLElement} */ body, /** @type {any} */ parts, /** @type {string} */ kind, /** @type {any} */ m) {
    const box = document.createElement("div"); box.className = "tpr-ch"; body.appendChild(box);
    if (!parts) return;
    if (kind === "xic") parts.xic(box, m); else parts.spec(box, m, kind === "spec2");
  }

  /** draws the slots of `doc` (`feature-slots/1`) into `panel`, with the model `m` of the row (TPRIGA) for the charts and the drawing
   * @param {HTMLElement} panel @param {Slots} doc @param {any} m */
  function fill(panel, doc, m) {
    panel.innerHTML = "";
    const grid = document.createElement("div"); grid.className = "tpsl"; grid.setAttribute("role", "list"); panel.appendChild(grid);
    const parts = G.TPRIGA && G.TPRIGA.parts;
    for (const s of doc.slots) {
      const card = document.createElement("section"); card.className = "tpsl-card"; card.dataset.slot = s.slot; card.dataset.state = s.state; card.setAttribute("role", "listitem");
      card.innerHTML = `<h4><span>${esc(TITLE[s.slot] || s.slot)}</span><span class="tpsl-st">${esc(STATE[s.state] || s.state)}</span></h4>`;
      const body = document.createElement("div"); card.appendChild(body);
      if (s.state === "filled") {
        body.insertAdjacentHTML("beforeend", facts(s));
        if (s.slot === "xic") chart(body, parts, "xic", m);
        else if (s.slot === "isotopes") chart(body, parts, "spec", m);
        else if (s.slot === "ms2") chart(body, parts, "spec2", m);
        else if (s.slot === "structure" && G.TPSTRUCT && m.interp) { const b = document.createElement("div"); body.appendChild(b); G.TPSTRUCT.fill(b, m.interp, m.missing && m.missing.structure); }
        body.insertAdjacentHTML("beforeend", source(s));
      } else body.insertAdjacentHTML("beforeend", missing(s));
      grid.appendChild(card);
    }
  }

  if (!document.getElementById("tpsl-css")) { const s = document.createElement("style"); s.id = "tpsl-css"; s.textContent = CSS; document.head.appendChild(s); }
  G.TPSLOTS = { fill };
})();
