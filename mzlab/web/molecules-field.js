// @ts-strict
// @ts-noany
// Search box for a molecule (ARIA combobox with a listbox): types a name, a formula or a mass and shows the molecules of the database (the one search of the engine).
// The caller gives every text (`texts`), so a tool shows its own language; an optional «Manage» button opens the window «Molecules». Used by the tools that need a
// molecule (Tabui, Infusion MSn, ...) and by tests/e2e_molecules_ui.py.
import { search } from "./molecules-archive.js";

/** @typedef {import("./molecules-archive.js").Found} Found */
/**
 * @typedef {{ label: string, placeholder?: string, none: string, count: (n: number) => string, manage?: string }} FieldTexts
 * @typedef {{ texts: FieldTexts, onPick: (m: Found) => void, limit?: number, ppm?: number, wide?: boolean, prepare?: (t: string) => Promise<string>, manage?: (() => void) | null, value?: string, display?: (name: string) => string, input?: HTMLInputElement, skip?: (t: string) => boolean }} FieldOptions
 */

const STYLE_ID = "molc-style";
const CSS = `.molc{position:relative;display:flex;flex-direction:column;gap:4px;max-width:420px}
.molc label{font-size:.875rem;font-weight:600;color:var(--ink)}
.molc input{font:inherit;color:var(--ink);background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:6px 8px;min-height:32px;box-sizing:border-box;width:100%}
.molc input:focus-visible,.molc .molc-man:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.molc ul{position:absolute;top:100%;left:0;right:0;z-index:6100;margin:2px 0 0;padding:0;list-style:none;max-height:260px;overflow:auto;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2)}
.molc ul[hidden]{display:none}
.molc li{display:flex;gap:10px;justify-content:space-between;align-items:baseline;padding:6px 10px;min-height:24px;cursor:pointer}
.molc li[aria-selected=true]{background:var(--sel);outline:2px solid var(--accent);outline-offset:-2px}
.molc .molc-f,.molc .molc-m{color:var(--muted);font-size:.8125rem;white-space:nowrap}
.molc .molc-man{align-self:flex-start;background:none;border:0;padding:4px 0;min-height:24px;font:inherit;font-size:.8125rem;color:var(--accent);text-decoration:underline;cursor:pointer}
.molc .molc-sr{position:absolute;width:1px;height:1px;margin:-1px;padding:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}
.molc.molc-existing{flex:1;min-width:0;max-width:none}
@media (pointer:coarse){.molc li{min-height:40px}.molc input{min-height:40px}.molc .molc-man{min-height:40px}}`;

let seq = 0;

/** The elements of the box: label, input, listbox and the status line that the screen readers hear; `-i`, `-b`, `-l` ids tie them with ARIA. */
function build(/** @type {string} */ id, /** @type {FieldTexts} */ texts, /** @type {HTMLInputElement | undefined} */ existing) {
  const el = (/** @type {string} */ tag, /** @type {Record<string, string>} */ attr) => {
    const e = document.createElement(tag);
    for (const k in attr) e.setAttribute(k, attr[k]);
    return e;
  };
  const root = el("div", { class: "molc" });
  const aria = { role: "combobox", "aria-autocomplete": "list", "aria-expanded": "false", "aria-controls": id + "-b", autocomplete: "off", spellcheck: "false" };
  const list = el("ul", { id: id + "-b", role: "listbox", ...(existing ? { "aria-label": texts.label } : { "aria-labelledby": id + "-l" }) });
  list.hidden = true;
  const status = el("span", { class: "molc-sr", role: "status", "aria-live": "polite" });
  if (existing) {                       // an input of the page (already labelled there) becomes the combobox; it keeps its id and its place
    for (const k in aria) existing.setAttribute(k, aria[/** @type {keyof typeof aria} */ (k)]);
    root.classList.add("molc-existing");
    existing.replaceWith(root);
    root.append(existing, list, status);
    return { root, input: existing, list, status };
  }
  const label = el("label", { id: id + "-l", for: id + "-i" });
  label.textContent = texts.label;
  const input = /** @type {HTMLInputElement} */ (el("input", { id: id + "-i", type: "text", ...aria }));
  if (texts.placeholder) input.placeholder = texts.placeholder;
  root.append(label, input, list, status);
  return { root, input, list, status };
}

/**
 * Builds the search box inside `host`. Returns `input` (for the caller's focus), `set` (writes the text without searching) and `destroy`.
 * @param {HTMLElement} host
 * @param {FieldOptions} opts
 */
export function field(host, opts) {
  if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
  const { texts } = opts, id = "molc" + ++seq, display = opts.display || ((/** @type {string} */ n) => n);
  const { root, input, list, status } = build(id, texts, opts.input);
  if (opts.value) input.value = opts.value;
  if (opts.manage !== null && texts.manage) {
    const b = document.createElement("button");
    b.type = "button"; b.className = "molc-man"; b.textContent = texts.manage;
    b.onclick = opts.manage || (() => /** @type {any} */ (window).carica("m:molecules-ui.js").then(() => /** @type {any} */ (window).MOLDB.open()));
    root.appendChild(b);
  }
  if (!opts.input) host.appendChild(root);   // an input of the page is replaced in place by `build`

  /** @type {Found[]} */ let found = [];
  let active = -1, token = 0;
  /** @type {ReturnType<typeof setTimeout> | undefined} */ let wait;

  const setOpen = (/** @type {boolean} */ si) => {
    list.hidden = !si;
    input.setAttribute("aria-expanded", String(si));
    if (!si) { active = -1; input.removeAttribute("aria-activedescendant"); }
  };
  const highlight = (/** @type {number} */ i) => {
    active = i;
    [...list.children].forEach((li, k) => li.setAttribute("aria-selected", String(k === i)));
    if (i >= 0) { input.setAttribute("aria-activedescendant", `${id}-o${i}`); list.children[i].scrollIntoView({ block: "nearest" }); } else input.removeAttribute("aria-activedescendant");
  };
  const pick = (/** @type {number} */ i) => {
    const m = found[i];
    if (!m) return;
    input.value = display(m.name);
    setOpen(false);
    status.textContent = "";
    opts.onPick(m);
  };
  const draw = () => {
    list.replaceChildren(...found.map((m, i) => {
      const li = document.createElement("li");
      li.id = `${id}-o${i}`; li.setAttribute("role", "option"); li.setAttribute("aria-selected", "false");
      const n = document.createElement("span"), f = document.createElement("span"), z = document.createElement("span");
      n.textContent = display(m.name); f.className = "molc-f"; f.textContent = m.formula; z.className = "molc-m"; z.textContent = m.mass.toFixed(4);
      li.append(n, f, z);
      li.onmousedown = ev => ev.preventDefault();
      li.onclick = () => pick(i);
      return li;
    }));
    setOpen(found.length > 0);
    status.textContent = found.length ? texts.count(found.length) : texts.none;
  };
  const searchNow = async () => {
    const run = ++token, t = input.value.trim();
    if (t.length < 2 || (opts.skip && opts.skip(t))) { found = []; list.replaceChildren(); setOpen(false); status.textContent = ""; return; }   // the old rows go too: they must not outlive the search that made them
    /** @type {Found[]} */ let r;
    try { r = await search(opts.prepare ? await opts.prepare(t) : t, opts.limit || 8, opts.ppm || 10, !!opts.wide); } catch (_) { r = []; }
    if (run !== token) return;
    found = r;
    draw();
  };

  input.addEventListener("input", () => { clearTimeout(wait); const t = input.value.trim(); if (t.length < 2 || (opts.skip && opts.skip(t))) searchNow(); else wait = setTimeout(searchNow, 120); });   // nothing to search: the old rows and status go at once
  input.addEventListener("keydown", ev => {
    const n = found.length;
    if (ev.key === "ArrowDown" || ev.key === "ArrowUp") {
      ev.preventDefault();
      if (list.hidden) { if (n) setOpen(true); else { clearTimeout(wait); searchNow(); } return; }
      const step = ev.key === "ArrowDown" ? 1 : -1;
      highlight(active < 0 ? (step > 0 ? 0 : n - 1) : (active + step + n) % n);
    } else if (ev.key === "Enter" && !list.hidden && active >= 0) { ev.preventDefault(); pick(active); }
    else if (ev.key === "Escape" && !list.hidden) { ev.preventDefault(); ev.stopPropagation(); setOpen(false); }
    else if (ev.key === "Tab") setOpen(false);
  });
  input.addEventListener("blur", () => setOpen(false));

  return {
    input,
    set: (/** @type {string} */ v) => { input.value = v; },
    destroy: () => { clearTimeout(wait); token++; root.remove(); },
  };
}
