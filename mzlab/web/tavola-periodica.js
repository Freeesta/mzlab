// @ts-check
// @ts-strict
// @ts-noany
"use strict";
// Periodic table for mass spectrometry: ONE place for the markup, the layout and the behaviour, used by the program (tables.js, dialog
// "Tavola periodica"). The data is the Rust `elements/1` (route api/elements = `mzlab-cli ion elements --json`): names, classes, masses, isotopes.
// Classic script (not an ES module).
// TAVOLA.create({ data, t }) -> { periodic(), bindPeriodic(root), info(element) }
//   data: the parsed elements/1 document; t(key, params): the texts (keys tbl.pt.*, tbl.na; the program passes I18N.t).
/** @typedef {{ a: number, mass: number, abund: number }} TavIso */
/** @typedef {{ z: number, symbol: string, name: string, class: string, mono: number | null, avg: number | null, mono_only: boolean, m2_rel: number | null, isotopes: TavIso[] }} TavEl */
/** @typedef {{ key: string, code: string, name: string }} TavClass */
window.TAVOLA = (() => {
  // Class colours: Okabe-Ito hues mixed into the panel colour (so the ink keeps its contrast in light, dark and high contrast), a 4 px
  // bar of the pure hue, the letter code of the class and a thin pattern: colour is never the only signal. Fixed row heights, so the
  // table never changes size; the focus ring is drawn inside the cell, so no scrolling parent can cut it.
  const HUES = { alkali: "#d55e00", alkaline: "#e69f00", transition: "#56b4e9", post_transition: "#009e73", metalloid: "#f0e442", nonmetal: "#cc79a7", noble: "#0072b2", lanthanide: "#999999", actinide: "#6b6b6b" };
  const LINE = "color-mix(in srgb,var(--ink) 18%,transparent)";
  const PATTERNS = {
    alkali: `repeating-linear-gradient(45deg,${LINE} 0 1px,transparent 1px 5px)`,
    alkaline: `repeating-linear-gradient(135deg,${LINE} 0 1px,transparent 1px 5px)`,
    transition: "none",
    post_transition: `repeating-linear-gradient(0deg,${LINE} 0 1px,transparent 1px 5px)`,
    metalloid: `repeating-linear-gradient(90deg,${LINE} 0 1px,transparent 1px 5px)`,
    nonmetal: `radial-gradient(${LINE} 1px,transparent 1.5px) 0 0/6px 6px`,
    noble: `repeating-linear-gradient(45deg,${LINE} 0 1px,transparent 1px 6px),repeating-linear-gradient(135deg,${LINE} 0 1px,transparent 1px 6px)`,
    lanthanide: `repeating-linear-gradient(0deg,${LINE} 0 2px,transparent 2px 7px)`,
    actinide: `repeating-linear-gradient(90deg,${LINE} 0 2px,transparent 2px 7px)`,
  };
  const classCss = Object.keys(HUES).map(k => {
    const h = /** @type {Record<string, string>} */ (HUES)[k], p = /** @type {Record<string, string>} */ (PATTERNS)[k];
    return `.pt-c[data-k="${k}"],.pt-key[data-k="${k}"]{background:${p},color-mix(in srgb,${h} 26%,var(--panel));border-left:4px solid ${h}}
.pt[data-f="${k}"] .pt-c:not([data-k="${k}"]){opacity:.3}`;
  }).join("\n");
  const CSS = `.pt{display:grid;grid-template-columns:repeat(18,minmax(0,1fr));grid-template-rows:repeat(7,46px) 12px repeat(2,46px);gap:3px;margin:8px 0;padding:2px;position:relative}
.pt-c{font:inherit;color:var(--ink);text-align:inherit;border:1px solid var(--line);border-radius:4px;padding:2px 3px;background:var(--panel);cursor:pointer;line-height:1.1;height:46px;min-width:24px;overflow:hidden;position:relative;display:flex;flex-direction:column;align-items:center}
.pt-t{display:flex;justify-content:space-between;width:100%;font-size:8px;color:var(--ink)}.pt-c b{font-size:15px;line-height:1.05}.pt-c small{font-size:9px;color:var(--ink)}
.pt-m{position:absolute;right:3px;top:12px;font-size:8px;font-weight:600;color:var(--ink)}
.pt-c.org{box-shadow:inset 0 0 0 2px var(--accent)}.pt-c.org b{font-weight:800}
.pt-c:hover,.pt-c.pin{outline:2px solid var(--accent);outline-offset:-2px;z-index:1}.pt-c:focus-visible{outline:3px solid var(--accent);outline-offset:-3px;z-index:1}
${classCss}
.pt-info{grid-row:1/4;grid-column:3/13;height:144px;padding:2px 10px;font-size:11px;line-height:1.3;overflow:auto;scrollbar-gutter:stable}.pt-info table th,.pt-info table td{padding:0 6px}
.pt-big{float:left;margin:0 10px 4px 0;border:1px solid var(--line);border-radius:6px;padding:2px 8px;text-align:center}.pt-big span{display:block;font-size:10px;color:var(--muted)}.pt-big b{font-size:26px}
.pt-bar{display:inline-block;height:8px;background:var(--accent);border-radius:2px}
.pt-leg{display:flex;flex-wrap:wrap;gap:4px 6px;align-items:center;margin:2px 0 6px;font-size:10px;color:var(--ink)}
.pt-key{font:inherit;font-size:10px;color:var(--ink);border:1px solid var(--line);border-radius:4px;padding:2px 6px;min-height:24px;cursor:pointer}.pt-key[aria-pressed="true"]{outline:2px solid var(--accent);outline-offset:-2px;font-weight:700}
.pt-key:focus-visible{outline:3px solid var(--accent);outline-offset:-3px}
@media (forced-colors:active){.pt-c,.pt-key{border-color:CanvasText}.pt-c.pin,.pt-c:focus-visible,.pt-key[aria-pressed="true"]{outline-color:Highlight}.pt-bar{background:CanvasText}.pt-c.org{box-shadow:inset 0 0 0 2px Highlight}}`;
  let styled = false;
  /** Grid cell of an element: the 18 columns of the main table, the f-block (La-Yb, Ac-No) on rows 9 and 10, Lu and Lr under Y. @param {number} z */
  function cell(z) {
    if (z >= 57 && z <= 70) return [9, z - 54];
    if (z >= 89 && z <= 102) return [10, z - 86];
    if (z === 1) return [1, 1];
    if (z === 2) return [1, 18];
    if (z <= 10) return [2, z <= 4 ? z - 2 : z + 8];
    if (z <= 18) return [3, z <= 12 ? z - 10 : z];
    if (z <= 36) return [4, z - 18];
    if (z <= 54) return [5, z - 36];
    if (z <= 56) return [6, z - 54];
    if (z <= 86) return [6, z - 68];
    if (z <= 88) return [7, z - 86];
    return [7, z - 100];
  }
  /** @param {{ data: { classes: TavClass[], elements: TavEl[] }, t: (key: string, params?: Record<string, unknown>) => string }} arg */
  function create({ data, t }) {
    if (!styled) { styled = true; const s = document.createElement("style"); s.id = "tavola-css"; s.textContent = CSS; document.head.appendChild(s); }
    const elements = data.elements, classes = data.classes;
    const EL = Object.fromEntries(elements.map(e => [e.symbol, e])), CL = Object.fromEntries(classes.map(c => [c.key, c]));
    const ORGANIC = new Set(["H", "C", "N", "O", "F", "P", "S", "Cl", "Br", "I", "Na", "K", "Si", "Li"]);
    const mono = (/** @type {TavEl} */ e) => (e.mono == null ? "" : e.mono < 100 ? e.mono.toFixed(4) : e.mono.toFixed(3));
    const pct = (/** @type {number} */ a) => String(+a.toFixed(4));
    function info(/** @type {TavEl} */ e) {
      const main = e.isotopes.length ? e.isotopes.reduce((a, b) => (b.abund > a.abund ? b : a)) : null;
      let h = `<div class="pt-big"><span>${e.z}</span><b>${EH(e.symbol)}</b></div><div><b style="font-size:15px">${EH(e.name)}</b></div>`;
      if (e.mono == null) return h + `<div>${t("tbl.na")}</div>`;
      if (!main || e.avg == null) return h + `<div>${t("tbl.pt.monoOnly", { m: e.mono.toFixed(5) })}</div>`;
      h += `<div>${t("tbl.pt.card", { w: e.avg.toFixed(4), m: e.mono.toFixed(5) })}</div>
        <table class="sm" style="margin-top:6px"><tr><th>${t("tbl.pt.col.iso")}</th><th class="num">${t("tbl.pt.col.mass")}</th><th class="num" title="${EH(t("tbl.pt.tip.abund"))}">${t("tbl.pt.col.abund")}</th><th class="num" title="${EH(t("tbl.pt.tip.delta"))}">${t("tbl.pt.col.delta")}</th><th></th></tr>` +
        e.isotopes.map(i => `<tr><td><sup>${i.a}</sup>${EH(e.symbol)}</td><td class="num">${i.mass.toFixed(5)}</td><td class="num">${pct(i.abund)}</td>` +
          `<td class="num">${i === main ? "" : (i.mass - main.mass > 0 ? "+" : "") + (i.mass - main.mass).toFixed(4)}</td>` +
          `<td><i class="pt-bar" style="width:${Math.max(1, Math.round(i.abund * 0.8))}px"></i></td></tr>`).join("") + "</table>";
      return h;
    }
    function periodic() {
      const cells = elements.map(e => {
        const [r, c] = cell(e.z), cls = CL[e.class];
        const label = e.mono == null ? t("tbl.pt.ariaNone", { name: e.name, s: e.symbol, z: e.z, cls: cls.name }) : e.avg == null ? t("tbl.pt.ariaMono", { name: e.name, s: e.symbol, z: e.z, cls: cls.name, m: e.mono.toFixed(5) }) : t("tbl.pt.aria", { name: e.name, s: e.symbol, z: e.z, cls: cls.name, m: e.mono.toFixed(5), w: e.avg.toFixed(4) });
        const mark = e.m2_rel != null ? `<sup class="pt-m" title="${EH(t("tbl.pt.tip.m2"))}">+2</sup>` : e.mono_only ? `<sup class="pt-m" title="${EH(t("tbl.pt.tip.one"))}">1</sup>` : "";
        return `<button type="button" class="pt-c${ORGANIC.has(e.symbol) ? " org" : ""}${e.mono == null ? " nd" : ""}" data-s="${e.symbol}" data-k="${e.class}" data-r="${r}" data-c="${c}" tabindex="${e.z === 1 ? 0 : -1}" aria-label="${EH(label)}" style="grid-row:${r};grid-column:${c}">` +
          `<span class="pt-t"><span>${e.z}</span><span>${cls.code}</span></span><b>${e.symbol}</b>${mark}<small>${mono(e)}</small></button>`;
      }).join("");
      const keys = classes.map(c => `<button type="button" class="pt-key" data-k="${c.key}" aria-pressed="false" title="${EH(t("tbl.pt.tip.class"))}">${c.code} ${EH(c.name)}</button>`).join("");
      const legend = `<div class="pt-leg" role="group" aria-label="${EH(t("tbl.pt.legend"))}">${keys}<span>${t("tbl.pt.key.use")}</span><span title="${EH(t("tbl.pt.tip.one"))}">1 ${t("tbl.pt.key.one")}</span><span title="${EH(t("tbl.pt.tip.m2"))}">+2 ${t("tbl.pt.key.m2")}</span></div>`;
      // Data sources (kept here, not shown in the page): see crates/mzlab-core/src/periodic.rs. Hovering or focusing an element shows its card
      // (click or Enter pins it); the elements of the formulas are boxed with the accent colour.
      return `${legend}<div class="pt" role="group" aria-label="${EH(t("icon.pt"))}">${cells}<div class="pt-info" id="pt-info" tabindex="0" role="region" aria-label="${EH(t("tbl.pt.info"))}" aria-live="polite">${info(EL.C)}</div></div>`;
    }
    function bindPeriodic(/** @type {HTMLElement} */ root) {
      /** @type {string | null | undefined} */ let pinned = null; const box = /** @type {HTMLElement} */ (root.querySelector("#pt-info")), grid = /** @type {HTMLElement} */ (root.querySelector(".pt"));
      /** @type {NodeListOf<HTMLElement>} */
      const tiles = root.querySelectorAll(".pt-c"), at = new Map([...tiles].map(c => [`${c.dataset.r},${c.dataset.c}`, c]));
      tiles.forEach(c => {
        c.onmouseenter = c.onfocus = () => { if (!pinned) box.innerHTML = info(EL[/** @type {string} */ (c.dataset.s)]); };
        c.onclick = () => { root.querySelectorAll(".pt-c.pin").forEach(x => x.classList.remove("pin")); if (pinned === c.dataset.s) { pinned = null; return; }
          pinned = c.dataset.s; c.classList.add("pin"); box.innerHTML = info(EL[/** @type {string} */ (pinned)]); };
        c.onkeydown = ev => {          // arrows move over the cells that exist; Home/End to the ends of the row
          const r = +(c.dataset.r || 0), k = +(c.dataset.c || 0), dr = ev.key === "ArrowDown" ? 1 : ev.key === "ArrowUp" ? -1 : 0, dc = ev.key === "ArrowRight" ? 1 : ev.key === "ArrowLeft" ? -1 : 0;
          /** @type {HTMLElement | undefined} */ let to;
          if (dc) for (let x = k + dc; x >= 1 && x <= 18 && !to; x += dc) to = at.get(`${r},${x}`);
          else if (dr) { let rr = r + dr; if (rr === 8) rr += dr; if (rr >= 1 && rr <= 10) for (let d = 0; d < 18 && !to; d++) to = at.get(`${rr},${k + d}`) || at.get(`${rr},${k - d}`); }
          else if (ev.key === "Home" || ev.key === "End") for (let x = ev.key === "Home" ? 1 : 18; x >= 1 && x <= 18 && !to; x += ev.key === "Home" ? 1 : -1) to = at.get(`${r},${x}`);
          if (!to) return;
          ev.preventDefault(); tiles.forEach(x => { x.tabIndex = -1; }); to.tabIndex = 0; to.focus();
        };
      });
      root.querySelectorAll(".pt-key").forEach(b => {          // the legend dims the other classes; the same key again clears it
        /** @type {HTMLElement} */ (b).onclick = () => { const on = b.getAttribute("aria-pressed") !== "true"; root.querySelectorAll(".pt-key").forEach(x => x.setAttribute("aria-pressed", "false"));
          b.setAttribute("aria-pressed", String(on)); if (on) grid.dataset.f = /** @type {HTMLElement} */ (b).dataset.k; else delete grid.dataset.f; };
      });
    }
    return { periodic, bindPeriodic, info };
  }
  return { create };
})();
