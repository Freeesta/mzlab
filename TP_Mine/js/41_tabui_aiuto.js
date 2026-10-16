// @ts-check
// TPMINE-PRIVATE  TPHELP: the help of Tabui. `attach(root)` wires ONE popover (role="tooltip", id tph-pop) for every element with data-tt="key" inside root (a field and its label).
// Opens on hover (500 ms) and keyboard focus (the id joins the aria-describedby of the trigger); stays open while the pointer is over it; closes with Esc (focus stays on
// the trigger), typing, a click outside or losing the focus (WCAG 1.4.13). No `title`: the global #qtip would be a second tip.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const CSS = `
#tph-pop{position:fixed;z-index:1000;max-width:min(340px,calc(100vw - 16px));box-sizing:border-box;padding:6px 9px;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2);font-size:12px;line-height:1.4;transition:opacity .12s}
#tph-pop[hidden]{display:none}
@media(prefers-reduced-motion:reduce){#tph-pop{transition:none}}`;
  const seen = new WeakSet();                                               // an event reaching two attached roots (nested) is handled once
  const first = (/** @type {Event} */ ev) => { if (seen.has(ev)) return false; seen.add(ev); return true; };
  /** @type {HTMLElement | null} */ let pop = null;
  /** @type {HTMLElement | null} */ let cur = null;
  /** @type {any} */ let timer = 0;
  const tt = (/** @type {string} */ k) => (G.TT ? G.TT(k) : "");

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
  /** the popover id is one token of aria-describedby: a field error linked to the same attribute (TPFIELD) is kept */
  const desc = (/** @type {HTMLElement} */ el, /** @type {boolean} */ on) => {
    const t = (el.getAttribute("aria-describedby") || "").split(/\s+/).filter(x => x && x !== "tph-pop"); if (on) t.push("tph-pop");
    if (t.length) el.setAttribute("aria-describedby", t.join(" ")); else el.removeAttribute("aria-describedby");
  };
  const close = () => {
    clearTimeout(timer);
    if (cur) desc(cur, false);
    cur = null; if (pop) pop.hidden = true;
  };
  /** @param {HTMLElement} el */
  const open = el => {
    const p = ensure(), key = el.getAttribute("data-tt") || "", text = tt(key);
    if (!text) return;
    if (el.tagName === "SUMMARY" && /** @type {HTMLDetailsElement | null} */ (el.parentElement) && /** @type {HTMLDetailsElement} */ (el.parentElement).open) return;          // a menu that is open must not be covered by the help of its own button
    if (cur && cur !== el) close();
    cur = el; p.textContent = text; p.hidden = false;
    desc(el, true);
    const r = el.getBoundingClientRect(), w = p.offsetWidth, h = p.offsetHeight;
    const left = Math.max(8, Math.min(r.left, innerWidth - w - 8)), below = el.getAttribute("role") !== "combobox" && r.bottom + 4 + h <= innerHeight;          // above a combobox: its list opens below
    const side = el.closest("#tp-left"), sr = side ? side.getBoundingClientRect() : null;                 // beside the form when there is room: below or above it would cover the next field
    if (sr && sr.right + 8 + w <= innerWidth - 8) { p.style.left = sr.right + 8 + "px"; p.style.top = Math.max(8, Math.min(r.top, innerHeight - h - 8)) + "px"; return; }
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
    root.addEventListener("toggle", ev => { const d = ev.target; if (cur && d instanceof HTMLDetailsElement && d.open && d.contains(cur)) close(); }, true);          // `toggle` does not bubble: the help of the summary leaves when its menu opens
    root.addEventListener("input", ev => { if (cur && ev.target === cur) close(); });                // typing hides the tip (it would cover the suggestions)
  };
  G.TPHELP = { attach, close };
})();
