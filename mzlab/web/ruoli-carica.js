// @ts-check
"use strict";
// Roles and times in the loading table (`campioni/2`, the Rust of mzlab-core). This file packs and shows: the proposals from the file names (`proponi`), the warnings
// (`valida`) and the three boxes Blank · t0 · Infusion come from the engine through `api/campioni/<op>`; nothing is decided here.
// An entry of ST.files keeps `type` (the base type the other routes know: sample | blank | standard) and, for this table, `role` (campioni/2 `tipo`), `sub`
// (`tipo_bianco`), `prop` (the proposal was asked), `ed` (the student chose the role: a late proposal does not overwrite it). The time `f.time` is in minutes, `f.gT` is
// true while it is only proposed from the name.
const RUOLI = (() => {
  /** @typedef {{name: string, kind?: string, use: boolean, type: string, role?: string, sub?: string|null, time: number|null, gT?: boolean, conc?: number|null, prop?: number, ed?: number}} Voce */
  /** @type {(k: string, p?: any) => string} */
  const t = (k, p) => I18N.t(k, p);
  const OPTS = ["sample", "t0", "blank", "blank:solvent", "blank:procedure", "dark", "infusion", "std", "qc"];
  const BASE = /** @type {Record<string, string>} */ ({ sample: "sample", t0: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", qc: "sample" });
  const TIMED = new Set(["sample", "dark"]);        // roles with a time to write
  const BOXES = ["blank", "t0", "infusion"];
  const REQUIRED = new Set(["timeMissing", "timeUnconfirmed"]);
  const BOX_ROLE = /** @type {Record<string, string>} */ ({ blank: "blank", t0: "t0", infusion: "infusion" });
  const eh = (/** @type {any} */ s) => EH(s == null ? "" : s);
  const post = (/** @type {string} */ op, /** @type {any} */ body) => fetch("api/campioni/" + op, { method: "POST", body: JSON.stringify(body) }).then(r => r.json());

  /** the value of the role select of an entry: sample, t0, blank, blank:solvent, blank:procedure, dark, infusion, std, qc */
  const val = (/** @type {Voce} */ f) => { const r = f.role === "calibration" ? "std" : f.role || (f.type === "standard" ? "std" : f.type || "sample"); return r === "blank" && f.sub ? "blank:" + f.sub : r; };
  const roleOf = (/** @type {Voce} */ f) => val(f).split(":")[0];
  const timed = (/** @type {Voce} */ f) => TIMED.has(roleOf(f));
  const label = (/** @type {string} */ v) => t("load.role." + v.replace(":", "."));
  const cellLabel = (/** @type {Voce} */ f, /** @type {string} */ col) => t("load.cell.of", { f: f.name, col });

  /** the role `v` for an entry (the student's choice or a proposal); the time follows the role */
  function setRole(/** @type {Voce} */ f, /** @type {string} */ v) {
    const was = roleOf(f), [r, sub] = v.split(":");
    f.role = r; f.sub = sub || null; f.type = BASE[r] || "sample";
    if (r === "t0") { f.time = 0; f.gT = false; }
    else if (!TIMED.has(r)) { f.time = null; f.gT = false; }
    else if (was === "t0") { f.time = null; f.gT = false; }
  }

  /** proposals from the names for the entries not asked yet; true when something changed */
  async function propose(/** @type {() => Voce[]} */ get) {
    const todo = get().filter(f => !f.prop);   // the list is rebuilt while the answer travels: entries are looked up again by name
    if (!todo.length) return false;
    todo.forEach(f => { f.prop = 1; });
    let doc;
    try { doc = await post("proponi", { files: todo.map(f => ({ name: f.name, sha256: "", kind: f.kind || "" })) }); } catch (_) { return false; }
    let changed = false;
    for (const c of (doc && doc.campioni) || []) {
      const f = get().find(x => x.name === c.file);
      if (!f || f.ed) continue;
      let r = c.tipo === "calibration" ? "std" : c.tipo;
      if (r === "sample" && c.tempo_min === 0) r = "t0";
      setRole(f, r === "blank" && c.tipo_bianco ? "blank:" + c.tipo_bianco : r);
      if (TIMED.has(r) && c.tempo_min != null) { f.time = c.tempo_min; f.gT = true; }
      changed = true;
    }
    return changed;
  }

  // ---------------------------------------------------------------- warnings (valida)
  /** @type {{sig: string, names: string[], avvisi: {codice: string, file: number|null}[]}} */   // `file` of a finding = position among the files sent (the ones in use)
  let last = { sig: "", names: [], avvisi: [] };
  let seq = 0;
  function doc(/** @type {Voce[]} */ F) {
    return JSON.stringify({ schema: "campioni/2", campioni: F.filter(f => f.use).map((f, i) => {
      const r = roleOf(f);
      return { file: f.name, sha256: "", tipo: r, tipo_bianco: r === "blank" ? f.sub || null : null, tempo: f.time ?? null, unita_tempo: "min", tempo_min: f.time ?? null,
        tempo_confermato: f.time != null && (r === "t0" || !f.gT), serie: "", replica: 1, concentrazione: null, unita: null, ordine_iniezione: i + 1, istd: false, note: "", composto: "" };
    }) });
  }
  async function refresh(/** @type {Voce[]} */ F) {
    const sig = doc(F), n = ++seq;
    if (sig === last.sig) return last;
    let r;
    try { r = await post("valida", JSON.parse(sig)); } catch (_) { return last; }
    if (n === seq) last = { sig, names: F.filter(f => f.use).map(f => f.name), avvisi: (r && r.avvisi) || [] };
    return last;
  }
  const treated = (/** @type {Voce[]} */ F) => F.filter(f => f.use && val(f) === "sample");
  /** the files whose time is missing or unconfirmed, when there are at least two samples */
  function required(/** @type {Voce[]} */ F) {
    const n = treated(F).length;
    return n < 2 ? new Set() : new Set(last.avvisi.filter(a => REQUIRED.has(a.codice) && a.file != null).map(a => last.names[/** @type {number} */ (a.file)]));
  }
  /** warning texts (priority order) */
  function texts(/** @type {Voce[]} */ F) {
    if (treated(F).length < 2) return [];
    const has = (/** @type {string} */ c) => last.avvisi.some(a => a.codice === c);
    const out = [];
    if (last.avvisi.some(a => a.codice === "timeMissing")) out.push(t("load.warn.missing"));
    if (last.avvisi.some(a => a.codice === "timeUnconfirmed")) out.push(t("load.warn.unconfirmed"));
    if (has("noT0") || has("noControls")) out.push(t("load.warn.noT0"));
    if (has("noBlank") || has("noControls")) out.push(t("load.warn.noBlank"));
    if (has("onePoint")) out.push(t("load.warn.onePoint"));
    return out;
  }

  // ---------------------------------------------------------------- cells of the table
  function roleCell(/** @type {Voce} */ f, /** @type {number} */ i) {
    const v = val(f);
    return `<td><select data-i="${i}" data-k="type" aria-label="${eh(cellLabel(f, t("load.table.type")))}">${OPTS.map(o => `<option value="${o}"${o === v ? " selected" : ""}>${eh(label(o))}</option>`).join("")}</select></td>`;
  }
  function timeCell(/** @type {Voce} */ f, /** @type {number} */ i, /** @type {string} */ tu, /** @type {string} */ guess) {
    const lab = `aria-label="${eh(cellLabel(f, t("load.table.time.col")))}"`, r = roleOf(f), at = `data-i="${i}" data-k="time"`;
    if (r === "t0") return `<td class="num"><input type="text" disabled ${lab} ${at} style="width:80px" value="0" title="${eh(t("load.time.t0"))}"></td>`;
    const g = f.gT && f.time != null ? ` class="guess" title="${eh(guess)}"` : "";
    return `<td class="num"><input type="text" inputmode="decimal" autocomplete="off" style="width:80px" ${lab} ${at} value="${eh(timeShown(f.time, tu) ?? "")}"${TIMED.has(r) ? "" : " disabled"}${g}></td>`;
  }

  /** the strip above the table: the three boxes, the warnings, the button «Confirm times» */
  function bar(/** @type {Voce[]} */ F) {
    const used = F.filter(f => f.use);
    const boxes = BOXES.map(b => {
      const own = used.filter(f => roleOf(f) === BOX_ROLE[b]), name = own.length ? own[0].name + (own.length > 1 ? ` +${own.length - 1}` : "") : "—";
      return `<button type="button" class="sec sm rl-box" data-box="${b}" aria-haspopup="menu" aria-expanded="false" title="${eh(t("load.box." + b + ".title"))}" aria-label="${eh(t("load.box." + b))}: ${eh(name)}. ${eh(t("load.box.clear"))}"><span class="muted">${eh(t("load.box." + b))}</span> <b class="rl-name">${eh(name)}</b></button>`;
    }).join("");
    const conf = F.some(f => f.use && f.gT && f.time != null);
    return `<div class="rl-bar"><div class="rl-boxes" role="group" aria-label="${eh(t("load.box.group"))}">${boxes}</div> <span id="rl-warn" class="sm" role="status"></span> `
      + `<button type="button" class="sec sm" id="rl-conf" title="${eh(t("load.confirm.title"))}"${conf ? "" : " hidden"}>${eh(t("load.confirm"))}</button></div>`;
  }

  /** paints the warnings and the marks of the required times without redrawing the table (the focus stays where it is) */
  async function paint(/** @type {Element} */ el, /** @type {Voce[]} */ F) {
    await refresh(F);
    const w = el.querySelector("#rl-warn"), tx = texts(F), need = required(F);
    if (w) { w.textContent = tx.slice(0, 2).join(" · "); w.setAttribute("title", tx.join(" · ")); }
    el.querySelectorAll('input[data-k="time"]').forEach(x => {
      const inp = /** @type {HTMLInputElement} */ (x), f = F[+(inp.dataset.i || 0)];
      if (!f || inp.classList.contains("bad")) return;
      const on = need.has(f.name), unc = f.gT && f.time != null;
      if (on) { inp.setAttribute("aria-invalid", "true"); inp.title = t("load.time.required") + (unc ? " · " + t("load.guess.title") : ""); inp.dataset.req = "1"; }
      else if (inp.dataset.req) { inp.removeAttribute("aria-invalid"); inp.title = unc ? t("load.guess.title") : ""; delete inp.dataset.req; }
    });
  }

  /** «Open» with a missing or unconfirmed time: the focus goes to the first such cell and true is returned */
  async function gate(/** @type {Element} */ el, /** @type {Voce[]} */ F) {
    await refresh(F);
    const need = required(F), i = F.findIndex(f => f.use && need.has(f.name));
    if (i < 0) return false;
    await paint(el, F);
    const c = /** @type {HTMLInputElement|null} */ (el.querySelector(`input[data-i="${i}"][data-k="time"]`));
    if (c) { c.focus(); c.select(); }
    return true;
  }

  // ---------------------------------------------------------------- the boxes and their lists
  function closeMenu(/** @type {Element} */ el, /** @type {boolean} */ back) {
    const m = el.querySelector(".rl-menu");
    if (!m) return;
    const b = /** @type {HTMLElement|null} */ (el.querySelector(`[data-box="${m.getAttribute("data-for")}"]`));
    m.remove();
    if (b) { b.setAttribute("aria-expanded", "false"); if (back) b.focus(); }
  }
  function openMenu(/** @type {Element} */ el, /** @type {Voce[]} */ F, /** @type {HTMLElement} */ btn, /** @type {() => void} */ redraw) {
    closeMenu(el, false);
    const b = btn.dataset.box || "", r = BOX_ROLE[b], m = document.createElement("div");
    m.className = "rl-menu"; m.setAttribute("role", "menu"); m.setAttribute("data-for", b); m.setAttribute("aria-label", t("load.box.menu", { role: t("load.box." + b) }));
    const items = F.map((f, i) => f.use ? `<button type="button" role="menuitemcheckbox" class="sec sm" data-pick="${i}" aria-checked="${roleOf(f) === r}">${eh(f.name)} <span class="muted">${eh(label(val(f)))}</span></button>` : "").join("")
      + `<button type="button" role="menuitem" class="sec sm" data-add="1">${eh(t("load.box.add"))}</button>`;
    m.innerHTML = items;
    btn.setAttribute("aria-expanded", "true");
    btn.after(m);
    const all = () => /** @type {HTMLElement[]} */ ([...m.querySelectorAll("button")]);
    m.addEventListener("click", e => {
      const x = /** @type {HTMLElement} */ (e.target).closest("button"); if (!x) return;
      if (x.dataset.add) { closeMenu(el, true); const p = /** @type {HTMLElement|null} */ (document.getElementById("pick")); if (p) p.click(); return; }
      const f = F[+(x.dataset.pick || 0)];
      f.ed = 1; setRole(f, roleOf(f) === r ? "sample" : b === "blank" ? "blank" : r);
      closeMenu(el, false); redraw(); refocusBox(el, b);
    });
    m.addEventListener("keydown", e => {
      const a = all(), k = a.indexOf(/** @type {HTMLElement} */ (document.activeElement));
      if (e.key === "Escape") { e.preventDefault(); closeMenu(el, true); }
      else if (e.key === "ArrowDown") { e.preventDefault(); a[(k + 1) % a.length].focus(); }
      else if (e.key === "ArrowUp") { e.preventDefault(); a[(k - 1 + a.length) % a.length].focus(); }
    });
    const first = all()[0]; if (first) first.focus();
  }
  function refocusBox(/** @type {Element} */ el, /** @type {string} */ b) { const x = /** @type {HTMLElement|null} */ (document.querySelector(`#flist [data-box="${b}"]`)); if (x) x.focus(); }

  /** listeners of the strip; `redraw` sorts and draws the table again */
  function bind(/** @type {Element} */ el, /** @type {Voce[]} */ F, /** @type {() => void} */ redraw) {
    el.querySelectorAll("[data-box]").forEach(x => {
      const b = /** @type {HTMLElement} */ (x);
      b.onclick = () => { if (b.getAttribute("aria-expanded") === "true") closeMenu(el, true); else openMenu(el, F, b, redraw); };
      b.onkeydown = e => {
        if (e.key === "Delete" || e.key === "Backspace") {
          e.preventDefault();
          F.filter(f => roleOf(f) === BOX_ROLE[b.dataset.box || ""]).forEach(f => { f.ed = 1; setRole(f, "sample"); });
          redraw(); refocusBox(el, b.dataset.box || "");
        } else if (e.key === "ArrowDown" && b.getAttribute("aria-expanded") !== "true") { e.preventDefault(); openMenu(el, F, b, redraw); }
      };
    });
    const c = /** @type {HTMLElement|null} */ (el.querySelector("#rl-conf"));
    if (c) c.onclick = () => { F.forEach(f => { if (f.use && f.gT && f.time != null) f.gT = false; }); redraw(); refocusBox(el, "blank"); };
    paint(el, F);
  }

  return { OPTS, BASE, val, roleOf, timed, setRole, propose, doc, refresh, texts, required, roleCell, timeCell, bar, paint, gate, bind };
})();
