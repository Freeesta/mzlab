// @ts-check
// «Stereodescriptors» card of Draw: with its toggle on, sends the molfile of the canvas to the Rust CIP rules (`cip` of the molkit .wasm, `cip/1`;
// the app asks `feat/grafo`) and lists R/S of the defined centres and E/Z of the defined double bonds as text rows (role=status) that select
// their atoms. A note says that mass spectrometry cannot tell these stereoisomers apart: the labels describe the drawing, not a sample.
import { ask, ketcher, message } from "./disegno-verifica.js";
import { molfileOf } from "./disegno-molfile.js";

const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => I18N.t(k, p);
let on = false, timer = 0, run = 0, listening = false;

/** one label: a button that selects its atoms on the canvas */
function row(/** @type {string} */ text, /** @type {number[]} */ atoms, /** @type {number[]} */ ids) {
  const li = document.createElement("li"), b = document.createElement("button");
  b.type = "button"; b.className = "cip-sel"; b.textContent = text; b.title = t("draw.cip.show.title");
  b.onclick = async () => {
    try {
      const K = await ketcher(), sel = atoms.map(i => ids[i]), bonds = /** @type {number[]} */ ([]);
      if (atoms.length === 2) K.editor.struct().bonds.forEach((/** @type {any} */ x, /** @type {number} */ id) => { if (sel.includes(x.begin) && sel.includes(x.end)) bonds.push(id); });
      K.editor.selection({ atoms: sel, bonds });
    } catch (_) { /* the canvas went away */ }
  };
  li.append(b);
  return li;
}

/** computes the labels of the whole canvas and fills the card */
export async function cipNow() {
  const status = /** @type {HTMLElement} */ (document.querySelector("#cip-status")), list = /** @type {HTMLElement} */ (document.querySelector("#cip-list"));
  const me = ++run;
  status.textContent = t("draw.cip.busy");
  try {
    const K = await ketcher(), st = K.editor.struct(), ids = [...st.atoms.keys()];
    if (me !== run || !on) return null;
    list.textContent = "";
    if (!ids.length) { status.textContent = t("draw.cip.empty"); return null; }
    const r = JSON.parse(await ask(JSON.stringify({ molfile: molfileOf(st, ids, undefined, () => true) }), "cip"));
    if (me !== run || !on) return null;
    const name = (/** @type {number} */ i) => (st.atoms.get(ids[i]).label || "") + (i + 1);
    for (const a of r.atoms) {
      const low = a.label === "r" || a.label === "s";
      list.appendChild(row(t(low ? "draw.cip.atomLow" : "draw.cip.atom", { atom: name(a.atom), label: a.label }), [a.atom], ids));
    }
    for (const b of r.bonds) list.appendChild(row(t("draw.cip.bond", { a: name(b.a), b: name(b.b), label: b.label }), [b.a, b.b], ids));
    const n = r.atoms.length + r.bonds.length;
    status.textContent = r.skip ? t(`draw.cip.skip.${r.skip}`) : n ? t("draw.cip.found", { n }) : t("draw.cip.none");
    return r;
  } catch (e) {
    if (me === run) { list.textContent = ""; status.textContent = t("draw.cip.fail", { message: message(e) }); }
    return null;
  }
}

/** the card, after the check card (or the properties card); off by default, it follows the editor while on */
export function mountCip() {
  if (document.querySelector("#cip-card")) return;
  const card = document.createElement("div");
  card.className = "card"; card.id = "cip-card";
  card.innerHTML = '<h2 data-i18n="draw.cip.heading"></h2><p class="rq-d" data-i18n="draw.cip.intro"></p>'
    + '<div style="margin:3px 0"><label><input type="checkbox" autocomplete="off" id="cip-show"> <span data-i18n="draw.cip.show"></span></label></div>'
    + '<p class="rq-d" id="cip-note" data-i18n="draw.cip.note"></p>'
    + '<div id="cip-status" class="sm" role="status"></div><ul id="cip-list" class="sm"></ul>';
  card.querySelectorAll("[data-i18n]").forEach(e => { e.textContent = t(/** @type {string} */ (e.getAttribute("data-i18n"))); });
  const css = document.createElement("style");
  css.textContent = "#cip-list{list-style:none;padding:0;margin:4px 0 0}#cip-list li{margin:2px 0}"
    + ".cip-sel{min-height:24px;min-width:24px;text-align:left;background:none;border:0;padding:2px 0;color:var(--ink);cursor:pointer;font:inherit}";
  card.appendChild(css);
  const after = document.querySelector("#chk-card") || document.querySelector("#anal-card");
  if (!after || !after.parentNode) return;
  after.parentNode.insertBefore(card, after.nextSibling);
  /** @type {HTMLInputElement} */ (card.querySelector("#cip-show")).onchange = async (e) => {
    on = /** @type {HTMLInputElement} */ (e.target).checked;
    if (!on) { run++; /** @type {HTMLElement} */ (card.querySelector("#cip-list")).textContent = ""; /** @type {HTMLElement} */ (card.querySelector("#cip-status")).textContent = ""; return; }
    cipNow();
    if (listening) return;
    listening = true;
    (await ketcher()).editor.subscribe("change", () => { if (!on) return; clearTimeout(timer); timer = window.setTimeout(cipNow, 700); });
  };
}
