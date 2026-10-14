// @ts-check
"use strict";
// Cells of the loading table (role, time), the strip with the three boxes and the warnings, the gate of «Open».
Object.assign(RUOLI, (() => {
  const { t, eh, OPTS, TIMED, BOXES, BOX_ROLE, val, roleOf, label, cellLabel, refresh, texts, required } = RUOLI;
  // ---------------------------------------------------------------- cells of the table
  function roleCell(/** @type {Voce} */ f, /** @type {number} */ i) {
    const v = val(f);
    return `<td><select data-i="${i}" data-k="type" aria-label="${eh(cellLabel(f, t("load.table.type")))}">${OPTS.map(o => `<option value="${o === "std" ? "standard" : o}"${o === v ? " selected" : ""}>${eh(label(o))}</option>`).join("")}</select></td>`;
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
      return `<button type="button" class="sec sm rl-box" data-box="${b}" aria-haspopup="menu" aria-expanded="false" title="${eh(t(`load.box.${b}.title`))}" aria-label="${eh(t(`load.box.${b}`))}: ${eh(name)}. ${eh(t("load.box.clear"))}"><span class="muted">${eh(t(`load.box.${b}`))}</span> <b class="rl-name">${eh(name)}</b></button>`;
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

  return { roleCell, timeCell, bar, paint, gate };
})());
