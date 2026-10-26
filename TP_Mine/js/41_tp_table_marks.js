// @ts-check
// TPMINE-PRIVATE  Marks of the TPTAB table of Tabui: state symbols, cell squares on the green area scale, condition dots, legend, level and prediction badges, area format (used by 42_tp_table.js).
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const STATES = /** @type {Record<string, [string, string]>} */ ({ detected: ["●", "detected"], filled: ["◐", "filled"], targeted: ["◌", "targeted"], absent: ["○", "absent"], na: ["–", "n/a"] });
  const tt = (/** @type {string | undefined} */ k) => k && G.TPHELP ? ` data-tt="${k}"` : "";
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
  const AREA_NOTE = "white = none, darker green = larger area";
  const sci = (/** @type {number} */ v) => !Number.isFinite(v) ? "–" : v === 0 ? "0" : Math.abs(v) >= 1e4 || Math.abs(v) < 0.01 ? v.toExponential(1) : String(+v.toPrecision(3));
  /** a point u in 0..1 of the green scale of five CSS variables `--tpr-g0..4` (light → dark, one hue): a mix of the two stops around it */
  function ramp(/** @type {number} */ u) {
    const x = Math.max(0, Math.min(1, u)) * 4, i = Math.min(3, Math.floor(x)), f = Math.round((x - i) * 100);
    return `color-mix(in srgb,var(--tpr-g${i + 1}) ${f}%,var(--tpr-g${i}))`;
  }
  /** the position on the green scale of the areas of one row: log10 of the area against the maximum of the row over three decades (the maximum is 1; no area → null, drawn white).
   * @param {(number | null | undefined)[]} areas @returns {(a: number | null | undefined) => number | null} */
  function areaScale(areas) {
    const mx = Math.max(0, ...areas.filter((/** @type {any} */ a) => typeof a === "number" && Number.isFinite(a)).map(Number));
    return a => !(typeof a === "number" && a > 0 && mx > 0) ? null : Math.max(0.08, Math.min(1, 1 + Math.log10(a / mx) / 3));
  }
  /** the class and the style of a square for the position `u` of the scale: white when there is no area, the symbol dark on the light greens and white on the dark ones
   * @param {number | null} u */
  const paint = u => ({ cls: u == null ? "" : u < 0.6 ? " on-k" : " on-w", style: u == null ? "" : ` style="background:${ramp(u)}"` });
  /** group of a sample in the order of the row: 0 blank or dark, 1 t0, 2 timed, 3 other @param {{ type?: string, time?: number | null }} x */
  const grp = x => /^(blank|dark|bianco|buio)/i.test(x.type || "") ? 0 : (/^t0/i.test(x.type || "") || x.time === 0) ? 1 : typeof x.time === "number" && Number.isFinite(x.time) ? 2 : 3;
  /** a row of symbols, at most `max` and then «+N»; the label sums them up («5 detected, 1 filled»).
   * With `info` (one `{file, type, time, area}` per state) the samples go in the order of the matrix (blank / dark, t0, growing times), every symbol is a focusable stop with its help
   * (sample, time, area, state) and its background says the area on the log scale of the matrix (darker = more); the shape still says the state.
   * @param {string[]} states @param {number} [max] @param {{ file?: string, type?: string, time?: number | null, area?: number }[]} [info] */
  function cells(states, max = 12, info) {
    const n = /** @type {Record<string, number>} */ ({}); for (const s of states) n[s] = (n[s] || 0) + 1;
    const lab = Object.keys(STATES).filter(k => n[k]).map(k => `${n[k]} ${STATES[k][1]}`).join(", ") || "no samples";
    const sym = (/** @type {string} */ s) => `<span class="tpt-st tpt-st-${STATES[s] ? s : "na"}" aria-hidden="true">${(STATES[s] || STATES.na)[0]}</span>`;
    const more = (/** @type {number} */ k) => k > max ? `<span class="tpt-more">+${k - max}</span>` : "";
    if (!info || info.length !== states.length) return `<span class="tpt-cells" role="img" aria-label="${lab}">${states.slice(0, max).map(sym).join("")}${more(states.length)}</span>`;
    const ix = states.map((_, i) => i).sort((a, b) => grp(info[a]) - grp(info[b]) || (+(info[a].time ?? 0)) - (+(info[b].time ?? 0))), sc = areaScale(info.map(x => x.area));
    const GN = ["blank / dark", "t0", "timed", "other"], esq = (/** @type {string} */ t) => t.replace(/"/g, "&quot;");
    const one = (/** @type {number} */ i, /** @type {number} */ k) => {
      const x = info[i], a = x.area, g = grp(x), [, word] = STATES[states[i]] || STATES.na, pr = paint(sc(a));
      const tip = `${x.file ? x.file + " · " : ""}${x.type || GN[g]}${typeof x.time === "number" ? ", time " + x.time : ""} · area ${typeof a === "number" ? sci(a) : "none"} · peak ${word}`;
      return `<span class="tpt-cell${pr.cls}${k && grp(info[ix[k - 1]]) !== g ? " tpt-gs" : ""}" role="img" aria-label="${esq(tip)}"${G.TPHELP ? ` tabindex="0" data-tt="ft.riga.cell" data-tt-text="${esq(tip)}"` : ""}${pr.style}>${sym(states[i])}</span>`;
    };
    return `<span class="tpt-cells tpt-cells-g" role="group" aria-label="${lab}">${ix.slice(0, max).map(one).join("")}${more(states.length)}</span>`;
  }
  /** one dot per condition: ● present, ○ absent (the shape says it, not the colour); the label names the conditions («present in O2, Air; absent in N2»)
   * @param {{ id: string, present: boolean }[]} items */
  function dots(items) {
    const yes = items.filter(c => c.present).map(c => c.id), no = items.filter(c => !c.present).map(c => c.id);
    const esc = (/** @type {string} */ t) => t.replace(/[&<>"]/g, ch => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);
    const lab = [yes.length ? "present in " + yes.join(", ") : "", no.length ? "absent in " + no.join(", ") : ""].filter(Boolean).join("; ");
    return `<span class="tpt-cells tpt-dots" role="img" aria-label="${esc(lab)}" title="${esc(lab)}">${items.map(c => `<span aria-hidden="true">${c.present ? "●" : "○"}</span>`).join("")}</span>`;
  }
  /** the legend of the symbols under the table; with `area` it also says what the green of the squares means @param {string[]} keys @param {boolean} [area] */
  function legend(keys, area) {
    return `<div class="tpt-leg">${area ? `<span>${AREA_NOTE}</span>` : ""}${keys.map(k => `<span${G.TPHELP ? ` tabindex="0"` : ""}${tt("ft.state." + k)}>${state(k)} ${(STATES[k] || STATES.na)[1]}</span>`).join("")}</div>`;
  }
  /** the badge of the Schymanski level («L3», «L2a»); a level decided by the chemist has the double border and the proposed one in the label (none when
   * the programme proposed no level, as at unit resolution) @param {string | number} lv @param {{ decided?: string | number | null }} [opt] */
  function level(lv, opt) {
    const L = G.LIVELLI, tag = (/** @type {any} */ x) => L ? L.tag(x) : "L" + x, cls = (/** @type {any} */ x) => L ? L.cls(x) : "lv-" + x;
    const d = opt && opt.decided, prop = lv != null && lv !== "" ? `; proposed ${tag(lv)}` : "; no level proposed";
    if (d === "nottp") return `<span class="lvb tpt-nottp" role="img" aria-label="Not a TP, decided by the chemist${prop}"${tt("ft.col.level")}>${prop.startsWith("; proposed") ? `<s>${tag(lv)}</s> ` : ""}not a TP</span>`;
    if (d != null && d !== "") return `<span class="lvb ${cls(d)} tpt-dec" role="img" aria-label="${tag(d)}, decided by the chemist${prop}"${tt("ft.col.level")}>${tag(d)}</span>`;
    return `<span class="lvb ${cls(lv)}" role="img" aria-label="${tag(lv)}, proposed"${tt("ft.col.level")}>${tag(lv)}</span>`;
  }
  /** the badge of the prediction scale («Prev. 2»): only the outline, so it is not taken for a level @param {number} n */
  function pred(n) {
    return `<span class="lvb pd-${n}" role="img" aria-label="Prev. ${n}, prediction scale"${tt("ft.col.pred")}>Prev. ${n}</span>`;
  }

  /** an area in engineering notation, never with the suffixes k / M: 821430000 -> «821.43e+6»; a value that rounds up to 1000.00 moves to the next power @param {number} v */
  const area = v => {
    if (!Number.isFinite(v) || v === 0) return "0";
    let e = Math.floor(Math.log10(Math.abs(v)) / 3) * 3, m = (v / 10 ** e).toFixed(2);
    if (Math.abs(+m) >= 1000) { e += 3; m = (v / 10 ** e).toFixed(2); }
    return m + "e" + (e < 0 ? "-" : "+") + Math.abs(e);
  };
  G.TPTAB_MARKS = { spark, state, cells, dots, legend, areaScale, paint, AREA_NOTE, level, pred, area };
})();
