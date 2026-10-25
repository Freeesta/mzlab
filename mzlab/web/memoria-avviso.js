// @ts-strict
// Warning before a phase uses more memory than the browser safely gives (docs/studi/memoria/README.md §4): the phase was refused at the safe budget and its estimate fits in the high
// one. Two buttons, the safe one («Cancel») is the default (Enter, Esc, a click outside); «Use more memory» is the user's choice. feat-run.js asks it through `window.mzlabAskMore`
// (set by rust-budget.js, loaded on the first need). Colours from the variables of the page; keyboard: Tab stays inside, the focus comes back where it was.
const W = /** @type {any} */ (window);
const STYLE_ID = "memmore-css";
const CSS = `#memmore{position:fixed;inset:0;z-index:6300;display:flex;align-items:center;justify-content:center;padding:16px;background:color-mix(in srgb,var(--ink) 45%,transparent)}
#memmore .mm-box{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:10px;width:min(480px,100%);padding:16px 18px;display:flex;flex-direction:column;gap:12px;box-shadow:0 8px 32px color-mix(in srgb,var(--ink) 30%,transparent)}
#memmore h2{margin:0;font-size:1.0625rem}
#memmore p{margin:0;font-size:.9375rem;line-height:1.45}
#memmore .mm-act{display:flex;gap:8px;justify-content:flex-end;flex-wrap:wrap}
#memmore button{min-height:36px;padding:6px 14px;border-radius:6px;border:1px solid var(--line);background:var(--panel);color:var(--ink);font:inherit;cursor:pointer}
#memmore button:focus-visible{outline:2px solid var(--accent,currentColor);outline-offset:2px}`;

/** @param {string} k @param {any} [p] */
const t = (k, p) => (W.I18N ? W.I18N.t(k, p) : k);

/**
 * @param {{ fase?: string, stima_mb: number, budget_mb: number, libero_mb?: number, high_mb: number }} info the refusal (`error.featMemory`) and the high budget
 * @returns {Promise<boolean>} true = use the high budget for the rest of the run
 */
export function ask(info) {
  return new Promise(resolve => {
    if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
    const before = document.activeElement, d = document.createElement("div");
    const key = `memory.phase.${info.fase}`, name = info.fase && W.I18N && t(key) !== key ? t(key) : info.fase || "";
    d.id = "memmore";
    d.innerHTML = `<div class="mm-box" role="alertdialog" aria-modal="true" aria-labelledby="mm-h" aria-describedby="mm-p"><h2 id="mm-h"></h2><p id="mm-p"></p><div class="mm-act"><button type="button" id="mm-yes"></button><button type="button" id="mm-no"></button></div></div>`;
    const q = (/** @type {string} */ s) => /** @type {HTMLElement} */ (d.querySelector(s));
    q("#mm-h").textContent = t("mem.more.title");
    q("#mm-p").textContent = t("mem.more.text", { fase: name, stima_mb: Math.round(info.stima_mb), budget_mb: Math.round(info.budget_mb), libero_mb: Math.round(info.libero_mb || 0), high_mb: Math.round(info.high_mb) });
    q("#mm-yes").textContent = t("mem.more.yes"); q("#mm-no").textContent = t("mem.more.no");
    const end = (/** @type {boolean} */ yes) => { d.remove(); if (before instanceof HTMLElement) before.focus(); resolve(yes); };
    q("#mm-yes").onclick = () => end(true); q("#mm-no").onclick = () => end(false);
    d.addEventListener("mousedown", ev => { if (ev.target === d) end(false); });
    d.addEventListener("keydown", ev => {
      if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); end(false); return; }
      if (ev.key !== "Tab") return;
      const first = q("#mm-yes"), last = q("#mm-no");
      if (ev.shiftKey && document.activeElement === first) { ev.preventDefault(); last.focus(); } else if (!ev.shiftKey && document.activeElement === last) { ev.preventDefault(); first.focus(); }
    });
    document.body.appendChild(d);
    q("#mm-no").focus();            // Enter = the safe choice
  });
}
