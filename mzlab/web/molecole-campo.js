// @ts-strict
// @ts-noany
// Search box for a molecule (ARIA combobox with a listbox): types a name, a formula or a mass and shows the molecules of the database (the one search of the engine).
// The caller gives every text (`testi`), so a tool shows its own language; an optional «Manage» button opens the window «Molecules». Used by the tools that need a
// molecule (Tabui, Infusion MSn, ...) and by tests/e2e_molecole_ui.py.
import { cerca } from "./molecole-archivio.js";

/** @typedef {import("./molecole-archivio.js").Trovata} Trovata */
/**
 * @typedef {{ label: string, placeholder?: string, none: string, count: (n: number) => string, manage?: string }} TestiCampo
 * @typedef {{ testi: TestiCampo, onScegli: (m: Trovata) => void, limite?: number, ppm?: number, ampia?: boolean, interroga?: (t: string) => Promise<string>, gestisci?: (() => void) | null, valore?: string, etichetta?: (nome: string) => string }} OpzioniCampo
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
@media (pointer:coarse){.molc li{min-height:40px}.molc input{min-height:40px}.molc .molc-man{min-height:40px}}`;

let seq = 0;

/** The elements of the box: label, input, listbox and the status line that the screen readers hear; `-i`, `-b`, `-l` ids tie them with ARIA. */
function costruisci(/** @type {string} */ id, /** @type {TestiCampo} */ testi) {
  const el = (/** @type {string} */ tag, /** @type {Record<string, string>} */ attr) => {
    const e = document.createElement(tag);
    for (const k in attr) e.setAttribute(k, attr[k]);
    return e;
  };
  const root = el("div", { class: "molc" });
  const label = el("label", { id: id + "-l", for: id + "-i" });
  label.textContent = testi.label;
  const input = /** @type {HTMLInputElement} */ (el("input", { id: id + "-i", type: "text", role: "combobox", "aria-autocomplete": "list", "aria-expanded": "false", "aria-controls": id + "-b", autocomplete: "off", spellcheck: "false" }));
  if (testi.placeholder) input.placeholder = testi.placeholder;
  const lista = el("ul", { id: id + "-b", role: "listbox", "aria-labelledby": id + "-l" });
  lista.hidden = true;
  const stato = el("span", { class: "molc-sr", role: "status", "aria-live": "polite" });
  root.append(label, input, lista, stato);
  return { root, input, lista, stato };
}

/**
 * Builds the search box inside `host`. Returns `input` (for the caller's focus), `set` (writes the text without searching) and `destroy`.
 * @param {HTMLElement} host
 * @param {OpzioniCampo} opz
 */
export function campo(host, opz) {
  if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
  const { testi } = opz, id = "molc" + ++seq, etichetta = opz.etichetta || ((/** @type {string} */ n) => n);
  const { root, input, lista, stato } = costruisci(id, testi);
  if (opz.valore) input.value = opz.valore;
  if (opz.gestisci !== null && testi.manage) {
    const b = document.createElement("button");
    b.type = "button"; b.className = "molc-man"; b.textContent = testi.manage;
    b.onclick = opz.gestisci || (() => /** @type {any} */ (window).carica("m:molecole-ui.js").then(() => /** @type {any} */ (window).MOLDB.apri()));
    root.appendChild(b);
  }
  host.appendChild(root);

  /** @type {Trovata[]} */ let trovate = [];
  let attiva = -1, token = 0;
  /** @type {ReturnType<typeof setTimeout> | undefined} */ let attesa;

  const apre = (/** @type {boolean} */ si) => {
    lista.hidden = !si;
    input.setAttribute("aria-expanded", String(si));
    if (!si) { attiva = -1; input.removeAttribute("aria-activedescendant"); }
  };
  const evidenzia = (/** @type {number} */ i) => {
    attiva = i;
    [...lista.children].forEach((li, k) => li.setAttribute("aria-selected", String(k === i)));
    if (i >= 0) { input.setAttribute("aria-activedescendant", `${id}-o${i}`); lista.children[i].scrollIntoView({ block: "nearest" }); } else input.removeAttribute("aria-activedescendant");
  };
  const sceglie = (/** @type {number} */ i) => {
    const m = trovate[i];
    if (!m) return;
    input.value = etichetta(m.nome);
    apre(false);
    stato.textContent = "";
    opz.onScegli(m);
  };
  const disegna = () => {
    lista.replaceChildren(...trovate.map((m, i) => {
      const li = document.createElement("li");
      li.id = `${id}-o${i}`; li.setAttribute("role", "option"); li.setAttribute("aria-selected", "false");
      const n = document.createElement("span"), f = document.createElement("span"), z = document.createElement("span");
      n.textContent = etichetta(m.nome); f.className = "molc-f"; f.textContent = m.formula; z.className = "molc-m"; z.textContent = m.massa.toFixed(4);
      li.append(n, f, z);
      li.onmousedown = ev => ev.preventDefault();
      li.onclick = () => sceglie(i);
      return li;
    }));
    apre(trovate.length > 0);
    stato.textContent = trovate.length ? testi.count(trovate.length) : testi.none;
  };
  const cercaOra = async () => {
    const mio = ++token, t = input.value.trim();
    if (t.length < 2) { trovate = []; apre(false); stato.textContent = ""; return; }
    /** @type {Trovata[]} */ let r;
    try { r = await cerca(opz.interroga ? await opz.interroga(t) : t, opz.limite || 8, opz.ppm || 10, !!opz.ampia); } catch (_) { r = []; }
    if (mio !== token) return;
    trovate = r;
    disegna();
  };

  input.addEventListener("input", () => { clearTimeout(attesa); attesa = setTimeout(cercaOra, 120); });
  input.addEventListener("keydown", ev => {
    const n = trovate.length;
    if (ev.key === "ArrowDown" || ev.key === "ArrowUp") {
      ev.preventDefault();
      if (lista.hidden) { if (n) apre(true); else { clearTimeout(attesa); cercaOra(); } return; }
      const passo = ev.key === "ArrowDown" ? 1 : -1;
      evidenzia(attiva < 0 ? (passo > 0 ? 0 : n - 1) : (attiva + passo + n) % n);
    } else if (ev.key === "Enter" && !lista.hidden && attiva >= 0) { ev.preventDefault(); sceglie(attiva); }
    else if (ev.key === "Escape" && !lista.hidden) { ev.preventDefault(); ev.stopPropagation(); apre(false); }
    else if (ev.key === "Tab") apre(false);
  });
  input.addEventListener("blur", () => apre(false));

  return {
    input,
    set: (/** @type {string} */ v) => { input.value = v; },
    destroy: () => { clearTimeout(attesa); token++; root.remove(); },
  };
}
