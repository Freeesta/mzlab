// @ts-check
// The «Regulatory lists» section of the Library window (libreria.js mounts it): one row per list of the installed package (label, jurisdiction, status, version, coverage, a
// checkbox), «Download» and «Check for updates», and the disclaimer of the report. The ticked lists are kept in localStorage as the `lists` and `include` parameters of
// screen.suspects and annot.regulatory (flow-form.js copies them into a step); the mask itself is applied only by the engine (mzlab_feat::regulatory). Site: Cache Storage
// (normative-scarica.js); app: the routes `normative/*` of the desktop engine, which keep the same catalogue bytes.

import * as dl from "./normative-scarica.js";

/** localStorage key of the selection: `{lists, include}` as the steps take them. */
export const SEL_KEY = "mzlab.normative.sel";
/** The statuses screened when `include` is empty (the engine's DEFAULT_STATUS). */
export const DEFAULT_STATUS = ["in_force", "adopted_not_applicable"];

const G = /** @type {any} */ (globalThis);
const t = (/** @type {string} */ k, /** @type {Record<string, unknown>} */ p = {}) => G.I18N.t(k, p);
const desktop = () => !!G.MZLAB_DESKTOP;
const BASE = () => (/Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/");
const status = (/** @type {any} */ l) => String((l.tags && l.tags.status) || "in_force");
/** The package next to this module (`static/normative/`), whatever page loads it. */
const OPT = { base: new URL("normative/", import.meta.url).href };
const count = (/** @type {any} */ l, /** @type {string} */ k) => Number((l.coverage && l.coverage[k]) || 0);

/** The `lists` and `include` parameters for the ticked ids: `include` names each status beyond the default among them; null when nothing is ticked. */
export function paramsOf(/** @type {any[]} */ lists, /** @type {Set<string>} */ ticked) {
  const on = lists.filter(l => ticked.has(l.id));
  if (!on.length) return null;
  const extra = [...new Set(on.map(status).filter(s => !DEFAULT_STATUS.includes(s)))].sort();
  return { lists: on.map(l => l.id).join(","), include: extra.join(",") };
}

/** The stored selection, or null. */
export function selection() {
  try { const v = JSON.parse(localStorage.getItem(SEL_KEY) || "null"); return v && typeof v.lists === "string" ? v : null; } catch (_) { return null; }
}

const store = (/** @type {{lists: string, include: string} | null} */ v) => { try { if (v) localStorage.setItem(SEL_KEY, JSON.stringify(v)); else localStorage.removeItem(SEL_KEY); } catch (_) { /* no storage */ } };

/** The installed catalogue (`normative/1`), or null. */
async function installed() {
  if (!desktop()) return dl.installed(OPT);
  const r = await fetch(BASE() + "normative/catalog");
  return r.ok ? dl.parseCatalog(await r.text()) : null;
}

/** «Download» (or a newer version): the site's package into the Cache Storage, or into the app's folder of the libraries. */
async function install() {
  if (!desktop()) { await dl.ensure(OPT); return; }
  const r = await fetch(BASE() + "normative/install");
  if (!r.ok) { const j = await r.json().catch(() => ({})), key = j.error_key || "normative.dl.err.io"; throw Object.assign(new Error(key), { key, params: { detail: j.detail || "", what: j.detail || "" } }); }
}

async function check() {
  if (!desktop()) return (await dl.check(OPT)).state;
  const r = await fetch(BASE() + "normative/check");
  return r.ok ? (await r.json()).state : "offline";
}

const el = (/** @type {string} */ tag, /** @type {Record<string, any>} */ a = {}, /** @type {(Node | string)[]} */ ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(a)) if (k === "text") n.textContent = v; else if (v !== false && v != null) n.setAttribute(k, v === true ? "" : String(v));
  n.append(...kids);
  return n;
};

/** One row: checkbox, label (the citation in its title), jurisdiction, status, version date, coverage. */
function row(/** @type {any} */ l, /** @type {boolean} */ on, /** @type {() => void} */ changed) {
  const tg = l.tags || {}, label = tg.instrument || l.id, s = status(l);
  const box = el("input", { type: "checkbox", "data-reg": l.id, "aria-label": label, checked: on });
  box.addEventListener("change", changed);
  const cov = t("normative.lib.coverage", { entries: count(l, "entries"), screenable: count(l, "screenable"), groups: count(l, "groups") + count(l, "uvcb"), inorganic: count(l, "inorganic") });
  return el("tr", {}, el("td", {}, box), el("td", { title: tg.citation || "", text: label }), el("td", { text: tg.jurisdiction || "" }),
    el("td", {}, el("span", { class: `sbadge st-${DEFAULT_STATUS.includes(s) ? "ok" : "warn"}`, text: t(`normative.lib.status.${s}`) })), el("td", { text: tg.version_date || "" }), el("td", { class: "sm", text: cov }));
}

/** Fills the fieldset of the Library window; called again after a download. @param {HTMLElement} box */
export async function mount(box) {
  const say = el("div", { class: "sm", "aria-live": "polite" }), body = el("div");
  const get = /** @type {HTMLButtonElement} */ (el("button", { type: "button", id: "lib-reg-get", text: t("normative.lib.download") })), upd = el("button", { type: "button", id: "lib-reg-check", text: t("normative.lib.check") });
  box.replaceChildren(el("legend", { class: "sm", text: t("normative.lib.title") }), el("div", { class: "sm muted", text: t("normative.lib.intro") }),
    el("div", { class: "sm", style: "margin-top:6px" }, get, " ", upd), body, say, el("p", { class: "sm muted", id: "lib-reg-disclaimer", text: t("normative.lib.disclaimer") }));
  const fail = (/** @type {any} */ e) => { say.textContent = e && e.key ? t(e.key, e.params || {}) : String(e && e.message || e); };
  get.addEventListener("click", async () => { get.disabled = true; say.textContent = t("normative.lib.downloading"); try { await install(); await mount(box); } catch (e) { fail(e); get.disabled = false; } });
  upd.addEventListener("click", async () => { try { const s = await check(); say.textContent = t(`normative.dl.check.${s}`); get.hidden = s !== "update" && s !== "none"; } catch (e) { fail(e); } });
  /** @type {any} */ let cat = null;
  try { cat = await installed(); } catch (e) { fail(e); }
  get.hidden = !!cat;
  if (!cat) { body.append(el("p", { class: "sm muted", text: t("normative.dl.check.none") })); return; }
  const lists = cat.lists || [], sel = selection(), was = new Set(sel ? sel.lists.split(",") : []);
  const ticked = (/** @type {any} */ l) => (sel ? was.has(l.id) : DEFAULT_STATUS.includes(status(l)));
  const changed = () => {
    const on = new Set([...box.querySelectorAll("input[data-reg]:checked")].map(b => /** @type {HTMLElement} */ (b).dataset.reg || ""));
    const p = paramsOf(lists, on);
    store(p);
    say.textContent = p ? "" : t("normative.lib.none");
  };
  const head = el("tr", {}, el("th", {}), ...["list", "jurisdiction", "status", "version", "coverage"].map(k => el("th", { text: t(`normative.lib.col.${k}`) })));
  body.append(el("table", { "aria-label": t("normative.lib.title") }, head, ...lists.map((/** @type {any} */ l) => row(l, ticked(l), changed))));
}
