// @ts-check
// TPMINE-PRIVATE  TPHELP: the help of Tabui. `button(key, label)` gives the HTML of a «?» button; `attach(root)` wires ONE popover (role="tooltip", id tph-pop) for every
// «?» button and every element with data-tt="key" inside root. Opens on hover (500 ms), keyboard focus, touch and Enter/Space; stays open while the pointer is over it;
// closes with Esc (focus stays on the trigger), a click outside or losing the focus (WCAG 1.4.13). No `title`: the global #qtip would be a second tip.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const CSS = `
.tph-b{box-sizing:border-box;display:inline-grid;place-items:center;min-width:24px;min-height:24px;padding:0;border:1px solid var(--muted);border-radius:50%;background:transparent;color:var(--muted);font:600 12px/1 inherit;cursor:help}
.tph-b:hover,.tph-b[aria-expanded="true"]{color:var(--ink);border-color:var(--ink)}.tph-b:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
@media(pointer:coarse){.tph-b{min-width:40px;min-height:40px}}
#tph-pop{position:fixed;z-index:1000;max-width:min(340px,calc(100vw - 16px));box-sizing:border-box;padding:6px 9px;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2);font-size:12px;line-height:1.4;transition:opacity .12s}
#tph-pop[hidden]{display:none}
@media(prefers-reduced-motion:reduce){#tph-pop{transition:none}}`;
  const seen = new WeakSet();                                               // an event reaching two attached roots (nested) is handled once
  const first = (/** @type {Event} */ ev) => { if (seen.has(ev)) return false; seen.add(ev); return true; };
  /** @type {HTMLElement | null} */ let pop = null;
  /** @type {HTMLElement | null} */ let cur = null;
  /** @type {any} */ let timer = 0;
  const esc = (/** @type {string} */ s) => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  const tt = (/** @type {string} */ k) => (G.TT ? G.TT(k) : "");

  /** @param {string} key @param {string} [label] @param {string} [id] */
  const button = (key, label, id) => `<button type="button" class="tph-b"${id ? ` id="${esc(id)}"` : ""} data-tt="${esc(key)}" aria-label="${esc((label ? label + " " : "") + "help")}" aria-expanded="false" aria-controls="tph-pop">?</button>`;

  const ensure = () => {
    if (pop) return pop;
    if (!document.getElementById("tph-css")) { const st = document.createElement("style"); st.id = "tph-css"; st.textContent = CSS; document.head.appendChild(st); }
    pop = document.createElement("div"); pop.id = "tph-pop"; pop.setAttribute("role", "tooltip"); pop.hidden = true; document.body.appendChild(pop);
    pop.addEventListener("mouseenter", () => clearTimeout(timer));
    pop.addEventListener("mouseleave", () => close());
    document.addEventListener("pointerdown", ev => { const t = /** @type {Node} */ (ev.target); if (cur && !cur.contains(t) && !(pop && pop.contains(t))) close(); }, true);
    document.addEventListener("keydown", ev => { if (ev.key === "Escape" && cur) { const c = cur; close(); c.focus(); } }, true);
    return pop;
  };
  const close = () => {
    clearTimeout(timer);
    if (cur) { cur.removeAttribute("aria-describedby"); if (cur.classList.contains("tph-b")) cur.setAttribute("aria-expanded", "false"); }
    cur = null; if (pop) pop.hidden = true;
  };
  /** @param {HTMLElement} el */
  const open = el => {
    const p = ensure(), key = el.getAttribute("data-tt") || "", text = tt(key);
    if (!text) return;
    if (cur && cur !== el) close();
    cur = el; p.textContent = text; p.hidden = false;
    el.setAttribute("aria-describedby", "tph-pop"); if (el.classList.contains("tph-b")) el.setAttribute("aria-expanded", "true");
    const r = el.getBoundingClientRect(), w = p.offsetWidth, h = p.offsetHeight;
    const left = Math.max(8, Math.min(r.left, innerWidth - w - 8)), below = r.bottom + 4 + h <= innerHeight;
    p.style.left = left + "px"; p.style.top = Math.max(8, below ? r.bottom + 4 : r.top - h - 4) + "px";
  };
  /** @param {HTMLElement} root */
  const attach = root => {
    ensure();
    const trig = (/** @type {EventTarget | null} */ t) => (t instanceof Element ? /** @type {HTMLElement | null} */ (t.closest("[data-tt]")) : null);
    root.addEventListener("mouseover", ev => { if (!first(ev)) return; const el = trig(ev.target); if (!el || el === cur) return; clearTimeout(timer); timer = setTimeout(() => open(el), 500); });
    root.addEventListener("mouseout", ev => { if (!first(ev)) return;
      const el = trig(ev.target), to = /** @type {Node | null} */ (ev.relatedTarget);
      if (!el || (to && el.contains(to))) return;
      clearTimeout(timer); if (el === cur) timer = setTimeout(() => { if (!(pop && pop.matches(":hover"))) close(); }, 150);
    });
    root.addEventListener("focusin", ev => { if (!first(ev)) return; const el = trig(ev.target); if (el) { clearTimeout(timer); open(el); } });
    root.addEventListener("focusout", ev => { if (!first(ev)) return; const el = trig(ev.target); if (el && el === cur) close(); });
    /** @type {{ el: HTMLElement, wasOpen: boolean } | null} */ let down = null;
    root.addEventListener("pointerdown", ev => { if (!first(ev)) return; const el = trig(ev.target); down = el ? { el, wasOpen: cur === el } : null; });
    root.addEventListener("click", ev => {                                  // touch, Enter, Space (a button clicks itself): a pointer press on an open one closes it, anything else opens it
      if (!first(ev)) return;
      const el = trig(ev.target); if (!el || !el.classList.contains("tph-b")) return;
      const was = !!down && down.el === el && down.wasOpen; down = null;
      clearTimeout(timer); if (was) close(); else open(el);
    });
  };
  G.TPHELP = { button, attach, close };
})();
