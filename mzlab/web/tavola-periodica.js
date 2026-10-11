"use strict";
// Periodic table with isotopes and abundances: ONE place for the markup, the layout and the behaviour, used by the program
// (tables.js, dialog "Tavola periodica") and by the Teoria (teoria/strumenti/tavola.js). The data of the elements is elements.js.
// Classic script (not an ES module) because the Teoria must also work from file://, where module scripts are blocked.
// TAVOLA.create({ elements, t, nome }) -> { periodic(), bindPeriodic(root), info(element) }
//   t(key, params): the texts (keys tbl.pt.*, tbl.na; the program passes I18N.t, the Teoria its Italian table); nome(element): its name.
window.TAVOLA = (() => {
  const H = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  // layout: fixed row heights, so the table never changes size; the info box scrolls if needed. Colours only from the page variables
  // (light, dark, high contrast); the focus ring is drawn inside the cell, so no scrolling parent can cut it.
  const CSS = `.pt{display:grid;grid-template-columns:repeat(18,minmax(0,1fr));grid-template-rows:repeat(7,46px) 12px repeat(2,46px);gap:3px;margin:8px 0;padding:2px;position:relative}
.pt-c{font:inherit;color:var(--ink);text-align:inherit;border:1px solid var(--line);border-radius:4px;padding:2px 3px;background:var(--panel);cursor:pointer;line-height:1.15;height:46px;min-width:24px;overflow:hidden;display:flex;flex-direction:column;align-items:center}
.pt-c span{font-size:9px;color:var(--muted);align-self:flex-start}.pt-c b{font-size:15px}.pt-c small{font-size:9px;color:var(--muted)}
.pt-c.org{background:color-mix(in srgb,var(--accent) 17%,var(--panel));border:2px solid var(--accent);color:var(--ink)}.pt-c.org span,.pt-c.org small{color:var(--ink)}.pt-c.org b{font-weight:800}.pt-c.nd b{color:var(--muted)}
.pt-c:hover,.pt-c.pin{outline:2px solid var(--accent);outline-offset:-2px;z-index:1}.pt-c:focus-visible{outline:3px solid var(--accent);outline-offset:-3px;z-index:1}
.pt-info{grid-row:1/4;grid-column:3/13;height:144px;padding:2px 10px;font-size:11px;line-height:1.3;overflow:auto}.pt-info table th,.pt-info table td{padding:0 6px}.pt-info .pt-big b{font-size:22px}
.pt-big{float:left;margin:0 10px 4px 0;border:1px solid var(--line);border-radius:6px;padding:2px 8px;text-align:center}.pt-big span{display:block;font-size:10px;color:var(--muted)}.pt-big b{font-size:26px}
.pt-gap{grid-row:8;grid-column:3/18;font-size:10px;color:var(--muted);align-self:center}.pt-bar{display:inline-block;height:8px;background:var(--accent);border-radius:2px}
@media (forced-colors:active){.pt-c.pin,.pt-c:focus-visible{outline-color:Highlight}.pt-bar{background:CanvasText}}`;
  let styled = false;
  function create({ elements, t, nome }) {
    if (!styled) { styled = true; const s = document.createElement("style"); s.id = "tavola-css"; s.textContent = CSS; document.head.appendChild(s); }
    const EL = Object.fromEntries(elements.map(e => [e.s, e]));
  const ORGANIC = new Set(["H", "C", "N", "O", "F", "P", "S", "Cl", "Br", "I", "Na", "K", "Si"]);
  function info(e) {
    const main = e.iso.length ? e.iso.reduce((a, b) => (b[2] > a[2] ? b : a)) : null;
    let h = `<div class="pt-big"><span>${e.z}</span><b>${H(e.s)}</b></div><div><b style="font-size:15px">${H(nome(e))}</b></div>
      <div>${t("tbl.pt.avgWeight", { w: e.w != null ? e.w : t("tbl.na") })}</div>`;
    if (main) {
      h += `<div>${t("tbl.pt.mono", { m: main[1].toFixed(5), a: main[0], s: H(e.s) })}</div>
        <table class="sm" style="margin-top:6px"><tr><th>${t("tbl.pt.col.iso")}</th><th class="num">${t("tbl.pt.col.mass")}</th><th class="num">${t("tbl.pt.col.abund")}</th><th class="num">${t("tbl.pt.col.delta")}</th><th></th></tr>` +
        e.iso.map(i => `<tr><td><sup>${i[0]}</sup>${H(e.s)}</td><td class="num">${i[1] != null ? i[1].toFixed(5) : ""}</td><td class="num">${i[2]}</td>` +
          `<td class="num">${i === main || i[1] == null ? "" : (i[1] - main[1] > 0 ? "+" : "") + (i[1] - main[1]).toFixed(4)}</td>` +
          `<td><i class="pt-bar" style="width:${Math.max(1, Math.round(i[2] * 0.8))}px"></i></td></tr>`).join("") + "</table>";
      if (e.iso.length > 1) {
        const sec = e.iso.filter(i => i !== main).reduce((a, b) => (b[2] > a[2] ? b : a));
        h += `<div class="muted sm" style="margin-top:4px">${t("tbl.pt.peak", { s: H(e.s), off: (sec[0] - main[0] > 0 ? "+" : "") + (sec[0] - main[0]), pct: (100 * sec[2] / main[2]).toFixed(sec[2] / main[2] < 0.1 ? 1 : 0) })}</div>`;
      }
    } else h += `<div class="muted sm" style="margin-top:6px">${t("tbl.pt.rare")}</div>`;
    return h;
  }
  function periodic() {
    const cells = elements.map(e => `<button type="button" class="pt-c${ORGANIC.has(e.s) ? " org" : ""}${e.iso.length ? "" : " nd"}" data-s="${e.s}" aria-label="${H(nome(e))}, ${e.s}, ${e.z}" style="grid-row:${e.r};grid-column:${e.c}">` +
      `<span>${e.z}</span><b>${e.s}</b><small>${e.w != null ? (e.w < 100 ? e.w.toFixed(3) : e.w.toFixed(2)) : ""}</small></button>`).join("");
    // Data sources (kept here, not shown in the page): exact masses = isotope table of OpenChemLib; abundances = representative
    // isotopic compositions, IUPAC (CIAAW); average atomic weight = Ketcher element list. Hovering an element shows masses and isotopes
    // (click pins it); the most common elements of organic molecules are highlighted.
    return `<div class="pt">${cells}<div class="pt-info" id="pt-info" tabindex="0" role="region" aria-label="${H(t("tbl.pt.info"))}" aria-live="polite">${info(EL.C)}</div><div class="pt-gap">${t("tbl.pt.gap")}</div></div>`;
  }
  function bindPeriodic(root) {
    let pinned = null; const box = root.querySelector("#pt-info");
    root.querySelectorAll(".pt-c").forEach(c => {
      c.onmouseenter = c.onfocus = () => { if (!pinned) box.innerHTML = info(EL[c.dataset.s]); };
      c.onclick = () => { root.querySelectorAll(".pt-c.pin").forEach(x => x.classList.remove("pin")); if (pinned === c.dataset.s) { pinned = null; return; }
        pinned = c.dataset.s; c.classList.add("pin"); box.innerHTML = info(EL[pinned]); };
    });
  }
    return { periodic, bindPeriodic, info };
  }
  return { create };
})();
