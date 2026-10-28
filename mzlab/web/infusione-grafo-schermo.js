// @ts-strict
// Full screen of the «Fragmentation graph», the same way as the panels of the data view (pannelli.js `_setMax`): same button, icons and texts, Esc to leave (ESC registry),
// everything else on the page inert while it lasts. The stage keeps its place in the DOM, so the zoom, the scroll and the selected ion stay as they are.

/** @type {HTMLElement | null} */
let cur = null;

/** Makes everything around the stage inert (or not): the siblings of the stage and of each of its ancestors. */
function inertAround(/** @type {HTMLElement} */ stage, /** @type {boolean} */ on) {
  for (let e = /** @type {HTMLElement | null} */ (stage); e && e !== document.body; e = e.parentElement) {
    for (const s of Array.from(e.parentElement ? e.parentElement.children : [])) if (s !== e) { if (on) s.setAttribute("inert", ""); else s.removeAttribute("inert"); }
  }
}

function paint(/** @type {HTMLElement} */ stage, /** @type {HTMLElement} */ btn, /** @type {boolean} */ on) {
  const k = on ? "panel.max.exit" : "panel.max.title";
  btn.innerHTML = on ? IC_FSX : IC_FS; btn.title = I18N.t(k); btn.setAttribute("aria-label", I18N.t(k)); btn.setAttribute("aria-pressed", String(on));
  stage.classList.toggle("ig-max", on);
}

/** Leaves the full screen of the stage in force, if any. */
export function leaveFull() {
  const stage = cur; if (!stage) return;
  cur = null; inertAround(stage, false);
  const btn = /** @type {HTMLElement | null} */ (stage.querySelector("#ig-fs"));
  if (btn) { paint(stage, btn, false); if (stage.isConnected) btn.focus(); } else stage.classList.remove("ig-max");
}

/** The full screen button: written into `bar` (the first toolbar) and wired to `stage`. */
export function fullButton(/** @type {HTMLElement} */ stage, /** @type {HTMLElement} */ bar) {
  const btn = document.createElement("button");
  btn.type = "button"; btn.id = "ig-fs"; btn.className = "bt fsb";
  paint(stage, btn, false);
  btn.onclick = () => {
    if (cur === stage) { leaveFull(); return; }
    leaveFull(); cur = stage; inertAround(stage, true); paint(stage, btn, true);
  };
  bar.appendChild(btn);
}

ESC.registra({
  id: "grafo-schermo-intero", prio: 50,
  aperto: () => { if (cur && !cur.isConnected) cur = null; return !!cur; },
  chiudi: () => leaveFull(),
});
