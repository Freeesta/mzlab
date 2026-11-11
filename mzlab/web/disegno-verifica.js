// @ts-check
// «Drawing check» card of Draw: sends the molfile of the canvas to the Rust check (`check` of the molkit .wasm, `check/1`; the app asks `feat/grafo`) and
// lists its remarks, each with a text row (role=status, level written out, not only a colour), a click that selects its atoms on the canvas and a «Why?»
// that opens the three parts (problem, why it matters, next step). It speaks of the drawing only; auto-run after each change is off by default.
import { featManifest } from "./feat-manifesto.js";
import { molfileOf } from "./disegno-molfile.js";

const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => I18N.t(k, p);
/** @type {any} */ let last = null;          // the last answer, with the Ketcher ids of its atoms
let auto = false, timer = 0, run = 0;

/** @type {Promise<any> | null} */ let modulo = null;

/** hex SHA-256 of some bytes */
async function sha256(/** @type {ArrayBuffer} */ b) {
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
}

/** the molkit module, loaded on the first check; the .wasm is checked against the SHA-256 of wasm-feat/manifest.json and the bridge version */
export function carica() {
  if (modulo) return modulo;
  modulo = (async () => {
    const base = new URL("./wasm-feat/", import.meta.url), e = (await featManifest(base)).molkit;
    if (!e) throw new Error("molkit");
    const bytes = await (await fetch(new URL(e.wasm, base))).arrayBuffer();
    if ((await sha256(bytes)) !== e.sha256) throw new Error("sha");
    const mod = await import(new URL(e.js, base).href);
    await mod.default({ module_or_path: bytes });
    if (mod.version() !== "molkit/1") throw new Error("molkit version");
    return mod;
  })();
  modulo.catch(() => { modulo = null; });
  return modulo;
}

/** Ketcher of the frame, once Draw has started it */
async function ketcher() {
  await /** @type {any} */ (window).TPDraw.when();
  return /** @type {any} */ (document.querySelector("#kframe")).contentWindow.ketcher;
}

/** `check/1` text of a request: the native engine in the app, the molkit .wasm on the site */
async function ask(/** @type {string} */ request) {
  if (/** @type {any} */ (window).MZLAB_DESKTOP) {
    const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
    const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", kind: "check", request }) });
    const x = await r.text();
    if (r.status !== 200) throw x;
    return x;
  }
  return (await carica()).check(request);
}

/** the message of an engine error (`{error_key, params}` text) or of anything else */
function message(/** @type {any} */ e) {
  try { const j = JSON.parse(String(e)); if (j.error_key) return t(j.error_key, j.params || {}); } catch (_) { /* not JSON */ }
  return e && e.message ? e.message : String(e);
}

/** the parameters of the texts of a remark */
function params(/** @type {any} */ w) {
  const sign = (/** @type {number} */ q) => q > 0 ? "+" + q : q < 0 ? "−" + -q : "0";
  return { el: w.el || "", valence: w.valence, allowed: (w.allowed || []).join("/"), q: sign(w.q || 0), fix: sign(w.fix || 0), iso: w.iso, n: w.n };
}

/** selects the atoms and bonds of a remark on the canvas */
async function select(/** @type {any} */ w) {
  const K = await ketcher(), ids = last ? last.ids : [], atoms = w.atoms.map((/** @type {number} */ i) => ids[i]).filter((/** @type {any} */ x) => x !== undefined);
  const pairs = w.bonds.map((/** @type {number[]} */ p) => [ids[p[0]], ids[p[1]]]), bonds = /** @type {number[]} */ ([]);
  K.editor.struct().bonds.forEach((/** @type {any} */ b, /** @type {number} */ id) => { if (pairs.some((/** @type {number[]} */ p) => (b.begin === p[0] && b.end === p[1]) || (b.begin === p[1] && b.end === p[0]))) bonds.push(id); });
  K.editor.selection(atoms.length || bonds.length ? { atoms, bonds } : null);
}

/** «label: text» as nodes, the label in bold (text only, no HTML) */
function labelled(/** @type {string} */ label, /** @type {string} */ text) {
  const f = document.createDocumentFragment(), b = document.createElement("b");
  b.textContent = label + ":";
  f.append(b, " " + text);
  return f;
}

/** one remark: the row (select) and «Why?» with its three parts */
function row(/** @type {any} */ w, /** @type {number} */ k) {
  const li = document.createElement("li"), p = params(w);
  li.className = "chk-" + w.level; li.dataset.code = w.code;
  const atomsText = w.atoms.length ? " (" + t("draw.check.atoms", { list: w.atoms.map((/** @type {number} */ i) => (last.els[i] || "") + (i + 1)).join(", ") }) + ")" : "";
  const sel = document.createElement("button");
  sel.type = "button"; sel.className = "chk-sel"; sel.title = t("draw.check.show.title");
  sel.append(labelled(t(`draw.check.level.${w.level}`), t(`draw.check.${w.code}.what`, p) + atomsText));
  sel.onclick = () => { select(w).catch(() => { /* the canvas went away */ }); };
  const why = document.createElement("button");
  why.type = "button"; why.className = "sm chk-why"; why.textContent = t("draw.check.why"); why.title = t("draw.check.why.title");
  why.setAttribute("aria-expanded", "false"); why.setAttribute("aria-controls", "chk-x" + k);
  const box = document.createElement("div");
  box.id = "chk-x" + k; box.className = "sm chk-x"; box.hidden = true;
  for (const part of ["what", "why", "next"]) {
    const d = document.createElement("div");
    d.append(labelled(t("draw.check.part." + part), t(`draw.check.${w.code}.${part}`, p)));
    box.append(d);
  }
  why.onclick = () => { box.hidden = !box.hidden; why.setAttribute("aria-expanded", String(!box.hidden)); };
  li.append(sel, " ", why, box);
  return li;
}

/** runs the check on the whole canvas and fills the card */
export async function checkNow() {
  const status = /** @type {HTMLElement} */ (document.querySelector("#chk-status")), list = /** @type {HTMLElement} */ (document.querySelector("#chk-list"));
  const me = ++run;
  status.textContent = t("draw.check.busy");
  try {
    const K = await ketcher(), st = K.editor.struct(), ids = [...st.atoms.keys()];
    if (me !== run) return null;
    list.textContent = "";
    if (!ids.length) { last = null; status.textContent = t("draw.check.empty"); return null; }
    const mf = molfileOf(st, ids, undefined, () => true);
    const r = JSON.parse(await ask(JSON.stringify({ molfile: mf })));
    if (me !== run) return null;
    last = { ids, els: ids.map(i => st.atoms.get(i).label), r };
    r.warnings.forEach((/** @type {any} */ w, /** @type {number} */ k) => list.appendChild(row(w, k)));
    status.textContent = r.warnings.length ? t("draw.check.found", { n: r.warnings.length }) : t("draw.check.ok");
    return r;
  } catch (e) {
    if (me === run) { list.textContent = ""; status.textContent = t("draw.check.fail", { message: message(e) }); }
    return null;
  }
}

/** the card, after the properties card; the auto-run listens to the editor once it is on */
export function mountCheck() {
  if (document.querySelector("#chk-card")) return;
  const card = document.createElement("div");
  card.className = "card"; card.id = "chk-card";
  card.innerHTML = '<h2 data-i18n="draw.check.heading"></h2><p class="muted sm" style="margin:0 0 4px" data-i18n="draw.check.intro"></p>'
    + '<div class="bar"><button type="button" id="chk-run" data-i18n="draw.check.run" data-i18n-title="draw.check.run.title"></button></div>'
    + '<div style="margin:3px 0"><label><input type="checkbox" autocomplete="off" id="chk-auto"> <span data-i18n="draw.check.auto"></span></label></div>'
    + '<div id="chk-status" class="sm" role="status"></div><ul id="chk-list" class="sm"></ul>';
  card.querySelectorAll("[data-i18n]").forEach(e => { e.textContent = t(/** @type {string} */ (e.getAttribute("data-i18n"))); });
  card.querySelectorAll("[data-i18n-title]").forEach(e => { e.setAttribute("title", t(/** @type {string} */ (e.getAttribute("data-i18n-title")))); });
  const css = document.createElement("style");
  css.textContent = "#chk-list{list-style:none;padding:0;margin:4px 0 0}#chk-list li{margin:4px 0;padding-left:6px;border-left:3px solid var(--muted)}#chk-list li.chk-warn{border-left-color:var(--bad)}"
    + "#chk-list button{min-height:24px;min-width:24px}.chk-sel{text-align:left;background:none;border:0;padding:2px 0;color:var(--ink);cursor:pointer;font:inherit}.chk-x{margin:2px 0 4px}";
  card.appendChild(css);
  const after = document.querySelector("#prop-card");
  if (!after || !after.parentNode) return;
  after.parentNode.insertBefore(card, after.nextSibling);
  /** @type {HTMLElement} */ (card.querySelector("#chk-run")).onclick = () => { checkNow(); };
  let listening = false;
  /** @type {HTMLInputElement} */ (card.querySelector("#chk-auto")).onchange = async (e) => {
    auto = /** @type {HTMLInputElement} */ (e.target).checked;
    if (!auto) return;
    checkNow();
    if (listening) return;
    listening = true;
    (await ketcher()).editor.subscribe("change", () => { if (!auto) return; clearTimeout(timer); timer = window.setTimeout(checkNow, 700); });
  };
  const menu = /** @type {any} */ (window).TPDraw && /** @type {any} */ (window).TPDraw.menu;
  if (menu) menu.add({ id: "check-why", order: 70, rows: whyRows });
}

/** «Why?» rows of the canvas menu: the remarks of the last check on the atom or bond under the pointer, or on the selection */
async function whyRows(/** @type {any} */ c) {
  if (!last || !last.r.warnings.length) return [];
  const K = c.K, st = K.editor.struct(), on = new Set(c.atoms);
  if (c.whole && c.ev) {
    const hit = K.editor.findItem(c.ev, ["atoms", "bonds"]), b = hit && hit.map === "bonds" ? st.bonds.get(hit.id) : null;
    if (b) on.add(b.begin).add(b.end); else if (hit) on.add(hit.id);
  }
  /** @type {{text: string, title: string, run: () => void}[]} */ const rows = [];
  last.r.warnings.forEach((/** @type {any} */ w, /** @type {number} */ k) => {
    const ids = [...w.atoms, ...w.bonds.flat()].map((/** @type {number} */ i) => last.ids[i]);
    if (!ids.some((/** @type {number} */ id) => on.has(id))) return;
    rows.push({ text: t("draw.check.ctx", { what: t(`draw.check.${w.code}.what`, params(w)) }), title: t("draw.check.why.title"), run: () => openWhy(k) });
  });
  return rows;
}

/** opens the three parts of remark k in the card and moves the focus to its «Why?» */
function openWhy(/** @type {number} */ k) {
  const box = document.getElementById("chk-x" + k), why = document.querySelector(`[aria-controls="chk-x${k}"]`);
  if (!box || !(why instanceof HTMLElement)) return;
  box.hidden = false; why.setAttribute("aria-expanded", "true");
  why.scrollIntoView({ block: "nearest" }); why.focus();
}
