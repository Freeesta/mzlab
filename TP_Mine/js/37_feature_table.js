// @ts-check
// TPMINE-PRIVATE  Step «Table» of the Feature table (window.PIPET): normalisation and filters as a view of the table, counters «N of M rows», and a virtualised grid of the rows of the view
// with the areas as the engine gives them (PIPEQC session: `view` for the counts, `rows` for the sheet). Nothing is computed here: the page builds the parameters of the view, formats
// and draws. A click on a row opens the XIC and MS2 of the row in the file chosen (PIPER.detail). Texts in English (mzFinder convention).
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const ROW_H = 28, WORD = /** @type {Record<string, number>} */ ({ DETECTED: 0, TARGETED: 1, FILLED: 2 });
  const LETTER = /** @type {Record<string, string>} */ ({ TARGETED: "T", FILLED: "F", UNKNOWN: "A" });
  const css = `#pipe .ft-bar{display:flex;gap:10px;align-items:flex-end;flex-wrap:wrap;margin:6px 0}#pipe .ft-bar label{display:flex;flex-direction:column;gap:2px;font-size:12px}#pipe .ft-bar input{width:92px}#pipe .ft-bar input,#pipe .ft-bar select{min-height:28px;box-sizing:border-box}
#pipe .ft-grid{height:min(380px,48vh);overflow:auto;position:relative;border:1px solid var(--line);border-radius:6px}#pipe .ft-head{position:sticky;top:0;z-index:1;background:var(--bg)}#pipe .ft-r{display:grid;gap:6px;align-items:center;padding:0 6px;font-size:12px;white-space:nowrap}
#pipe .ft-head .ft-r{font-size:11px;color:var(--muted);border-bottom:1px solid var(--line);min-height:24px}#pipe .ft-space{position:relative}#pipe .ft-win{position:absolute;left:0;right:0;top:0}
#pipe .ft-win .ft-r{position:absolute;left:0;height:${ROW_H}px;cursor:pointer;border-bottom:1px solid var(--line);min-width:100%;box-sizing:border-box}#pipe .ft-win .ft-r.sel{background:var(--sel);box-shadow:inset 3px 0 0 var(--accent)}#pipe .ft-n{text-align:right}`;

  const num = (/** @type {HTMLInputElement} */ el) => (el.value.trim() === "" ? null : +el.value);
  /** the parameters of the view from the form (the text `normalizza`/`filtra` is the language of the engine) */
  function specOf(/** @type {(s: string) => HTMLInputElement} */ q) {
    const filtra = [], rule = (/** @type {any} */ o) => filtra.push(o);
    const det = num(q("#ft-det")), area = num(q("#ft-area")), blank = num(q("#ft-blank")), mz = [num(q("#ft-mz0")), num(q("#ft-mz1"))], rt = [num(q("#ft-rt0")), num(q("#ft-rt1"))];
    if (det) rule({ regola: "min_rilevati", valore: det });
    if (area) rule({ regola: "min_area", valore: area });
    if (blank) rule({ regola: "bianco", valore: blank });
    if (mz[0] != null || mz[1] != null) rule({ regola: "mz", da: mz[0] ?? 0, a: mz[1] ?? 1e9 });
    if (rt[0] != null || rt[1] != null) rule({ regola: "rt", da: rt[0] ?? 0, a: rt[1] ?? 1e9 });
    return JSON.stringify({ normalizza: { metodo: q("#ft-norm").value }, filtra });
  }
  const html = () => `<details open><summary>Table</summary><div class="ft-bar">
    <label>Normalisation<select id="ft-norm"><option value="nessuna">none</option><option value="somma">sum of the areas</option><option value="mediana">median of the areas</option></select></label>
    <label>Detected in at least (files)<input id="ft-det" type="number" min="0" step="1"></label><label>Area at least<input id="ft-area" type="number" min="0" step="any"></label>
    <label>Blank ratio (area over blank)<input id="ft-blank" type="number" min="0" step="any"></label><label><i>m/z</i> from<input id="ft-mz0" type="number" step="any"></label><label>to<input id="ft-mz1" type="number" step="any"></label>
    <label>RT from (min)<input id="ft-rt0" type="number" step="any"></label><label>to<input id="ft-rt1" type="number" step="any"></label></div>
    <div class="mut" id="ft-n" role="status"></div><div class="mut" id="ft-why"></div><div id="ft-g" class="ft-grid" role="grid" aria-label="Feature table" tabindex="0"></div></details>`;

  /** the grid: `head` = column titles, `fmt(row)` = cells (text); virtualised on scroll (FRAME) */
  function grid(/** @type {HTMLElement} */ el, /** @type {string[]} */ head, /** @type {(r: any[]) => string[]} */ fmt, /** @type {(r: any[], i: number) => void} */ onPick) {
    const cols = `grid-template-columns:70px 90px 70px repeat(${Math.max(0, head.length - 3)},minmax(110px,1fr))`;
    el.innerHTML = `<div class="ft-head"><div class="ft-r" style="${cols}" role="row">${head.map(h => `<span role="columnheader">${h}</span>`).join("")}</div></div><div class="ft-space"><div class="ft-win"></div></div>`;
    const space = /** @type {HTMLElement} */ (el.querySelector(".ft-space")), win = /** @type {HTMLElement} */ (el.querySelector(".ft-win"));
    /** @type {any[][]} */ let list = []; let sel = -1, raf = 0;
    const draw = () => {
      raf = 0;
      const a = Math.max(0, Math.floor(el.scrollTop / ROW_H) - 4), b = Math.min(list.length, Math.ceil((el.scrollTop + (el.clientHeight || 380)) / ROW_H) + 4);
      let s = ""; for (let i = a; i < b; i++) s += `<div class="ft-r${i === sel ? " sel" : ""}" role="row" data-i="${i}" aria-rowindex="${i + 2}" style="${cols};top:${i * ROW_H}px">${fmt(list[i]).map((c, j) => `<span role="gridcell"${j > 1 ? ' class="ft-n"' : ""}>${c}</span>`).join("")}</div>`;
      win.innerHTML = s;
    };
    const later = () => { if (!raf) raf = G.FRAME.next(draw); };
    const pick = (/** @type {number} */ i) => { if (i < 0 || i >= list.length) return; sel = i; const top = i * ROW_H; if (top < el.scrollTop) el.scrollTop = top; else if (top + ROW_H * 2 > el.scrollTop + el.clientHeight) el.scrollTop = top + ROW_H * 2 - el.clientHeight; draw(); onPick(list[i], i); };
    el.addEventListener("scroll", later);
    el.addEventListener("click", e => { const r = /** @type {HTMLElement} */ (e.target).closest("[data-i]"); if (r) pick(+(/** @type {HTMLElement} */ (r)).dataset.i); });
    el.addEventListener("keydown", e => {
      const k = /** @type {Record<string, number>} */ ({ ArrowDown: 1, ArrowUp: -1, PageDown: 10, PageUp: -10 })[e.key];
      if (k) { e.preventDefault(); pick(Math.min(list.length - 1, Math.max(0, (sel < 0 ? 0 : sel) + k))); }
    });
    return { set(/** @type {any[][]} */ l) { list = l; sel = -1; space.style.height = l.length * ROW_H + "px"; el.setAttribute("aria-rowcount", String(l.length + 1)); el.scrollTop = 0; draw(); } };
  }

  /** mounts the step in `box`. o = { session (PIPEQC.open), names, gate (PIPE.gateState), onPick(row {id, mz, rt, cells[{state, area}]}), say } */
  function mount(/** @type {HTMLElement} */ box, /** @type {any} */ o) {
    box.innerHTML = html();
    const q = (/** @type {string} */ s) => /** @type {any} */ (box.querySelector(s)), fmtv = G.PIPEQC.fmt;
    const head = ["ID", "<i>m/z</i>", "RT (min)", ...o.names.map((/** @type {string} */ n) => n.slice(0, 22))];
    const g = grid(q("#ft-g"), head, r => [String(r[0]), Number(r[1]).toFixed(4), Number(r[2]).toFixed(2), ...o.names.map((/** @type {any} */ _, /** @type {number} */ j) => (r[7 + 2 * j] === "DETECTED" ? "" : `<small>${LETTER[r[7 + 2 * j]] || ""}</small> `) + fmtv(r[6 + 2 * j]))],
      r => o.onPick({ id: r[0], mz: r[1], rt: r[2], plain: true, reasons: [], proofs: [], cells: o.names.map((/** @type {any} */ _, /** @type {number} */ j) => ({ state: r[7 + 2 * j] in WORD ? WORD[r[7 + 2 * j]] : 3, area: r[6 + 2 * j] || 0 })) }));
    let timer = 0;
    const update = () => {
      try {
        const spec = specOf(q), v = o.session.view(spec), c = v.conteggi, rej = o.session.qc(spec).scartate;
        g.set(o.session.rows(spec).slice(1));
        q("#ft-n").textContent = `${c.tenute.toLocaleString("en")} of ${c.righe.toLocaleString("en")} rows`;
        q("#ft-why").textContent = rej.length ? "Rejected by each filter, in order: " + rej.map((/** @type {any} */ r) => `${G.PIPEQC.WHY[r.regola] || r.regola} (${r.righe.toLocaleString("en")})`).join(", ") : "";
      } catch (/** @type {any} */ e) { q("#ft-n").textContent = ""; q("#ft-why").textContent = o.say ? o.say(e) : String(e.message || e); }
    };
    box.addEventListener("input", () => { clearTimeout(timer); timer = window.setTimeout(update, 250); });
    box.addEventListener("change", () => { clearTimeout(timer); timer = window.setTimeout(update, 0); });
    update();
    return { update, spec: () => specOf(q) };
  }

  G.PIPET = { mount, specOf, css };
})();
