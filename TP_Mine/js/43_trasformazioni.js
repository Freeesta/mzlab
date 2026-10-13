// @ts-check
// TPMINE-PRIVATE  The list of transformations of mzFinder as a table (window.TPTR): one row per transformation, grouped by class, with an on/off box, the change
// as a formula and its Δm. The text of #tp-tr stays the only source: the table reads it (rows and Δm come from the .wasm, `transf_rows`) and a click rewrites a line of it
// (a row that is off is a commented line, «# name;change»). «Edit as text» shows the text itself. Also `TPTR.change`: the readable form of the names of the column «Change».
// Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const CLASSES = [["oxidation", "Oxidations (oxygen added)"], ["gain", "Additions"], ["loss", "Losses"], ["halogen", "Halogens"]];

  /** A formula or a change with subscripts: «+H2O3» -> «+H₂O₃» as HTML. @param {string} s */
  const formula = s => esc(s).replace(/([A-Za-z)])(\d+)/g, "$1<sub>$2</sub>").replace(/-/g, "−");
  /** The name of a rule from its identifier («loss_of_c2h2» -> «loss of C2H2»). @param {string} t */
  const rule = t => {
    if (t === "rottura") return "cleavage";
    const s = t.replace(/_/g, " ");
    const m = /^loss of ([a-z0-9]+)$/.exec(s);
    return m && /^(?:[a-z]\d*)+$/.test(m[1]) && (m[1].length <= 3 || /\d/.test(m[1])) ? "loss of " + m[1].toUpperCase() : s;
  };
  /** Readable text of a label of «Change»: «rottura+hydration» -> «cleavage + hydration»; the same step twice -> «2 × loss of C2H2»; a change of formula stays one. @param {string} label */
  const changeText = label => {
    if (/^[+-]/.test(label)) return label;
    const n = new Map();
    for (const t of String(label).split("+").filter(Boolean)) { const r = rule(t); n.set(r, (n.get(r) || 0) + 1); }
    return [...n].map(([r, k]) => (k > 1 ? k + " × " : "") + r).join(" + ");
  };
  /** The same as HTML, with the subscripts of the formulas. @param {string} label */
  const change = label => /^[+-]/.test(label) ? formula(label) : esc(changeText(label)).replace(/\b([A-Z][A-Z0-9]*\d[A-Z0-9]*|[A-Z]{1,3})\b/g, f => formula(f));

  const css = `.tptr table{border-collapse:collapse;width:100%;font-size:12px}.tptr th,.tptr td{padding:3px 6px;text-align:left;border-bottom:1px solid var(--line)}
.tptr .cls th{background:var(--panel);color:var(--muted);font-weight:600}.tptr td.num{text-align:right;font-variant-numeric:tabular-nums}.tptr tr.off td{color:var(--muted)}
.tptr input[type=checkbox]{width:24px;height:24px;margin:0;cursor:pointer}.tptr .tr-add{display:flex;gap:6px;flex-wrap:wrap;margin:8px 0 4px}.tptr .tr-add input[type=text]{min-height:28px;min-width:90px;flex:1}
.tptr button{min-height:28px;min-width:28px}.tptr .tr-bar{display:flex;gap:6px;align-items:center;margin:6px 0}.tptr .tr-msg{min-height:1.2em;font-size:12px}.tptr .tr-msg.err{color:var(--bad)}`;

  /** @param {HTMLTextAreaElement} ta the text of the list @param {(fn: string, ...a: any[]) => Promise<any>} call the worker */
  function mount(ta, call) {
    const det = /** @type {HTMLDetailsElement} */ (ta.closest("details"));
    if (!det || det.querySelector(".tptr")) { det && det.querySelector(".tptr") && /** @type {any} */ (det).tptr(); return; }
    if (!document.getElementById("tptr-css")) { const s = document.createElement("style"); s.id = "tptr-css"; s.textContent = css; document.head.appendChild(s); }
    const sum = det.querySelector("summary"); if (sum) sum.textContent = "Transformations";
    const box = document.createElement("div"); box.className = "tptr";
    box.innerHTML = `<div class="tr-bar"><button type="button" class="tr-mode" aria-pressed="false" title="Show the list as plain text: name;change;note, # = off">Edit as text</button><span class="mut">Rows that are off are not searched.</span></div>
      <div class="tr-tab"></div>
      <div class="tr-add" role="group" aria-label="Add a transformation"><input type="text" class="tr-n" placeholder="Name" aria-label="Name of the transformation" spellcheck="false"><input type="text" class="tr-c" placeholder="Change, e.g. +O-H2" aria-label="Change of formula, e.g. +O-H2" spellcheck="false"><input type="text" class="tr-o" placeholder="Note (optional)" aria-label="Note" spellcheck="false"><button type="button" class="tr-go">Add</button></div>
      <div class="tr-msg" role="status"></div>`;
    det.insertBefore(box, ta);
    const $ = (/** @type {string} */ q) => /** @type {HTMLElement} */ (box.querySelector(q));
    const tab = $(".tr-tab"), msg = $(".tr-msg"), mode = $(".tr-mode");
    const say = (/** @type {string} */ t, err = false) => { msg.textContent = t; msg.classList.toggle("err", err); };
    let text = false, seq = 0;
    ta.hidden = true;
    /** @param {string} t */ const rows = async t => JSON.parse(await call("transf_rows", t)).rows;
    const lines = () => ta.value.replace(/\r/g, "").split("\n");
    /** @param {string[]} l */ const put = l => { ta.value = l.join("\n"); ta.dispatchEvent(new Event("change", { bubbles: true })); render(); };

    async function render() {
      if (text) return;
      const me = ++seq;
      let r; try { r = await rows(ta.value.replace(/\r/g, "")); } catch (e) { tab.textContent = ""; ta.hidden = false; say("The table is not available here; edit the text.", true); return; }
      if (me !== seq) return;
      const bad = r.filter((/** @type {any} */ x) => x.error);
      let h = `<table><thead><tr><th scope="col">On</th><th scope="col">Name</th><th scope="col">Change</th><th scope="col">Δm (Da)</th><th scope="col">Note</th></tr></thead>`;
      for (const [k, lab] of CLASSES) {
        const g = r.filter((/** @type {any} */ x) => !x.error && x.class === k); if (!g.length) continue;
        h += `<tbody><tr class="cls"><th colspan="5" scope="colgroup">${lab}</th></tr>` + g.map((/** @type {any} */ x) =>
          `<tr class="${x.on ? "" : "off"}"><td><input type="checkbox" data-l="${x.line}"${x.on ? " checked" : ""} aria-label="Search ${esc(x.name)}"></td><td>${esc(x.name)}${x.lit ? ' <span class="mut" title="Literature: level 2">lit</span>' : ""}</td><td>${formula(x.change)}</td><td class="num">${x.dm > 0 ? "+" : x.dm < 0 ? "−" : ""}${Math.abs(x.dm).toFixed(4)}</td><td>${esc(x.note)}</td></tr>`).join("") + "</tbody>";
      }
      tab.innerHTML = h + "</table>";
      tab.querySelectorAll("input[data-l]").forEach(el => el.addEventListener("change", () => {
        const l = lines(), i = +/** @type {HTMLInputElement} */ (el).dataset.l; l[i] = /** @type {HTMLInputElement} */ (el).checked ? l[i].replace(/^\s*#+\s*/, "") : "# " + l[i].trim(); put(l);
      }));
      say(bad.length ? "Line " + (bad[0].line + 1) + ": " + bad[0].error + (bad.length > 1 ? " (+" + (bad.length - 1) + " more)" : "") : "", !!bad.length);
    }
    mode.addEventListener("click", () => {
      text = !text; ta.hidden = !text; tab.hidden = text; mode.setAttribute("aria-pressed", String(text)); mode.textContent = text ? "Show as table" : "Edit as text";
      if (text) ta.focus(); else render();
    });
    ta.addEventListener("input", render);
    det.addEventListener("toggle", render);
    /** the line typed in the three boxes, or "" @returns {string} */
    const typed = () => { const v = (/** @type {string} */ q) => /** @type {HTMLInputElement} */ ($(q)).value.trim().replace(/;/g, ","); return v(".tr-n") + ";" + v(".tr-c") + (v(".tr-o") ? ";" + v(".tr-o") : ""); };
    let prev = 0;
    $(".tr-add").addEventListener("input", async () => {            // Δm of what is typed, before adding it
      const me = ++prev; const n = /** @type {HTMLInputElement} */ ($(".tr-c")).value.trim(); if (!n) { say(""); return; }
      try { const r = (await rows("x;" + n))[0]; if (me === prev) say(r && !r.error ? "Δm " + (r.dm > 0 ? "+" : "") + r.dm.toFixed(4) + " Da" : "", false); } catch (_) { /* no preview */ }
    });
    $(".tr-go").addEventListener("click", async () => {
      const n = /** @type {HTMLInputElement} */ ($(".tr-n")).value.trim(), c = /** @type {HTMLInputElement} */ ($(".tr-c")).value.trim();
      if (!n || !c) { say("Give a name and a change.", true); return; }
      let r; try { r = (await rows(typed()))[0]; } catch (e) { say(String(/** @type {any} */ (e).message || e), true); return; }
      if (!r || r.error) { say(r ? r.error : "Not a valid change.", true); return; }
      const l = lines(); while (l.length && !l[l.length - 1].trim()) l.pop(); l.push(typed()); put(l);
      box.querySelectorAll(".tr-add input").forEach(i => { /** @type {HTMLInputElement} */ (i).value = ""; });
      say("Added: " + r.name + " (Δm " + (r.dm > 0 ? "+" : "") + r.dm.toFixed(4) + " Da)");
    });
    /** @type {any} */ (det).tptr = render;
    render();
  }

  G.TPTR = { mount, change, changeText, formula };
})();
