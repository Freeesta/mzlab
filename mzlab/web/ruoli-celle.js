// @ts-check
"use strict";
// Cells of the loading table (role, time), the strip with the two boxes and the warnings.
Object.assign(RUOLI, (() => {
  const { t, eh, OPTS, TIMED, BOXES, BOX_ROLE, val, roleOf, label, cellLabel, refresh, texts } = RUOLI;
  // ---------------------------------------------------------------- cells of the table
  function roleCell(/** @type {Voce} */ f, /** @type {number} */ i) {
    const v = val(f);
    return `<td><select data-i="${i}" data-k="type" aria-label="${eh(cellLabel(f, t("load.table.type")))}">${OPTS.map(o => `<option value="${o === "std" ? "standard" : o}"${o === v ? " selected" : ""}>${eh(label(o))}</option>`).join("")}</select></td>`;
  }
  function timeCell(/** @type {Voce} */ f, /** @type {number} */ i, /** @type {string} */ tu, /** @type {string} */ guess) {
    const lab = `aria-label="${eh(cellLabel(f, t("load.table.time.col")))}"`, r = roleOf(f), at = `data-i="${i}" data-k="time"`;
    const g = f.gT && f.time != null ? ` class="guess" title="${eh(guess)}"` : "";
    return `<td class="num"><input type="text" inputmode="decimal" autocomplete="off" style="width:80px" ${lab} ${at} value="${eh(timeShown(f.time, tu) ?? "")}"${TIMED.has(r) ? "" : " disabled"}${g || (r === "dark" ? ` title="${eh(t("load.time.dark"))}"` : "")}></td>`;
  }

  /** the strip above the table: the two boxes, the warnings, the button «Confirm times» */
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

  /** paints the warnings (information only) without redrawing the table (the focus stays where it is) */
  async function paint(/** @type {Element} */ el, /** @type {Voce[]} */ F) {
    await refresh(F);
    const w = el.querySelector("#rl-warn"), tx = texts(F);
    if (w) { w.textContent = tx.slice(0, 2).join(" · "); w.setAttribute("title", tx.join(" · ")); }
  }

  return { roleCell, timeCell, bar, paint };
})());
