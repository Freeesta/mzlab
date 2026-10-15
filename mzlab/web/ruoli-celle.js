// @ts-check
"use strict";
// Cells of the loading table: role and time.
Object.assign(RUOLI, (() => {
  const { t, eh, OPTS, TIMED, val, roleOf, label, cellLabel } = RUOLI;
  function roleCell(/** @type {Voce} */ f, /** @type {number} */ i) {
    const v = val(f);
    return `<td><select data-i="${i}" data-k="type" aria-label="${eh(cellLabel(f, t("load.table.type")))}">${OPTS.map(o => `<option value="${o === "std" ? "standard" : o}"${o === v ? " selected" : ""}>${eh(label(o))}</option>`).join("")}</select></td>`;
  }
  function timeCell(/** @type {Voce} */ f, /** @type {number} */ i, /** @type {string} */ tu) {
    const lab = `aria-label="${eh(cellLabel(f, t("load.table.time.col")))}"`, r = roleOf(f);
    return `<td class="num"><input type="text" inputmode="decimal" autocomplete="off" style="width:80px" ${lab} data-i="${i}" data-k="time" value="${eh(timeShown(f.time, tu) ?? "")}"${TIMED.has(r) ? "" : " disabled"}${r === "dark" ? ` title="${eh(t("load.time.dark"))}"` : ""}></td>`;
  }
  return { roleCell, timeCell };
})());
