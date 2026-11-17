// @ts-strict
// Warning before a phase uses more memory than the browser safely gives (docs/agenti/memoria-scelte.md): the phase was refused at the safe budget and its estimate fits in the high
// one. Two buttons, the safe one («Cancel») is the default (Enter, Esc, a click outside); «Use more memory» is the user's choice. feat-run.js asks it through `window.mzlabAskMore`
// (set by rust-budget.js, loaded on the first need). Colours from the variables of the page; keyboard: Tab stays inside, the focus comes back where it was.
// `confirmBox` is the same two-button question for other pages (Draw's «New»); `choiceBox` is the general one (Draw's «not saved» question, disegno-pulito.js).
const W = /** @type {any} */ (window);
const STYLE_ID = "memmore-css";
const CSS = `.mmask{position:fixed;inset:0;z-index:6300;display:flex;align-items:center;justify-content:center;padding:16px;background:color-mix(in srgb,var(--ink) 45%,transparent)}
.mmask .mm-box{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:10px;width:min(480px,100%);padding:16px 18px;display:flex;flex-direction:column;gap:12px;box-shadow:0 8px 32px color-mix(in srgb,var(--ink) 30%,transparent)}
.mmask h2{margin:0;font-size:1.0625rem}
.mmask p{margin:0;font-size:.9375rem;line-height:1.45}
.mmask #mm-err{color:var(--bad,#b3261e)}
.mmask .mm-fmt{display:flex;gap:8px;align-items:center;font-size:.9375rem}
.mmask select{min-height:32px;font:inherit;color:var(--ink);background:var(--panel);border:1px solid var(--line);border-radius:6px;padding:2px 6px}
.mmask button:disabled,.mmask select:disabled{opacity:.6;cursor:default}
.mmask .mm-act{display:flex;gap:8px;justify-content:flex-end;flex-wrap:wrap}
.mmask button{min-height:36px;padding:6px 14px;border-radius:6px;border:1px solid var(--line);background:var(--panel);color:var(--ink);font:inherit;cursor:pointer}
.mmask button:focus-visible{outline:2px solid var(--accent,currentColor);outline-offset:2px}`;

/** @param {string} k @param {any} [p] */
const t = (k, p) => (W.I18N ? W.I18N.t(k, p) : k);

/**
 * @param {{ fase?: string, stima_mb: number, budget_mb: number, libero_mb?: number, high_mb: number }} info the refusal (`error.featMemory`) and the high budget
 * @returns {Promise<boolean>} true = use the high budget for the rest of the run
 */
export function ask(info) {
  const key = `memory.phase.${info.fase}`, name = info.fase && W.I18N && t(key) !== key ? t(key) : info.fase || "";
  return confirmBox({ id: "memmore", title: t("mem.more.title"), yes: t("mem.more.yes"), no: t("mem.more.no"),
    text: t("mem.more.text", { fase: name, stima_mb: Math.round(info.stima_mb), budget_mb: Math.round(info.budget_mb), libero_mb: Math.round(info.libero_mb || 0), high_mb: Math.round(info.high_mb) }) });
}

/**
 * Question with any number of buttons; `cancel` is the safe one: default focus, Esc and a click outside choose it. Tab stays inside the box.
 * A button with `run` is async: while it runs the buttons are off, and the box closes only when it returns true (a string is shown in the box as the reason it stays open).
 * @param {{ id: string, title: string, text?: string, extra?: HTMLElement, cancel: string, buttons: { key: string, id?: string, label: string, tip?: string, run?: () => Promise<boolean | string> }[] }} o
 * @returns {Promise<string>} the `key` of the chosen button
 */
export function choiceBox(o) {
  return new Promise(resolve => {
    if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
    const before = document.activeElement, d = document.createElement("div");
    d.id = o.id; d.className = "mmask";
    d.innerHTML = `<div class="mm-box" role="alertdialog" aria-modal="true" aria-labelledby="mm-h" aria-describedby="mm-p"><h2 id="mm-h"></h2><p id="mm-p"></p><p id="mm-err" role="alert" hidden></p><div class="mm-act"></div></div>`;
    const q = (/** @type {string} */ s) => /** @type {HTMLElement} */ (d.querySelector(s));
    q("#mm-h").textContent = o.title;
    if (o.text) q("#mm-p").textContent = o.text; else { q("#mm-p").remove(); q(".mm-box").removeAttribute("aria-describedby"); }
    if (o.extra) q(".mm-box").insertBefore(o.extra, q(".mm-act"));
    const bs = new Map(), act = q(".mm-act");
    const end = (/** @type {string} */ key) => { d.remove(); if (before instanceof HTMLElement) before.focus(); resolve(key); };
    const lock = (/** @type {boolean} */ on) => d.querySelectorAll("button,select").forEach(e => { /** @type {HTMLButtonElement} */ (e).disabled = on; });
    for (const b of o.buttons) {
      const e = document.createElement("button"); e.type = "button"; if (b.id) e.id = b.id; e.textContent = b.label; if (b.tip) e.title = b.tip;
      e.onclick = async () => {
        if (!b.run) { end(b.key); return; }
        lock(true);
        /** @type {boolean | string} */ let r = false;
        try { r = await b.run(); } catch (x) { r = String(x && /** @type {any} */ (x).message || x); }
        if (r === true) { end(b.key); return; }
        lock(false); const er = q("#mm-err"); er.textContent = typeof r === "string" ? r : ""; er.hidden = !er.textContent; e.focus();
      };
      bs.set(b.key, e); act.appendChild(e);
    }
    d.addEventListener("mousedown", ev => { if (ev.target === d) end(o.cancel); });
    d.addEventListener("keydown", ev => {
      if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); end(o.cancel); return; }
      if (ev.key !== "Tab") return;
      const f = [...d.querySelectorAll("button,select")].filter(e => !(/** @type {HTMLButtonElement} */ (e)).disabled), first = f[0], last = f[f.length - 1];
      if (ev.shiftKey && document.activeElement === first) { ev.preventDefault(); /** @type {HTMLElement} */ (last).focus(); } else if (!ev.shiftKey && document.activeElement === last) { ev.preventDefault(); /** @type {HTMLElement} */ (first).focus(); }
    });
    document.body.appendChild(d);
    /** @type {HTMLElement} */ (bs.get(o.cancel)).focus();            // Enter = the safe choice
  });
}

/**
 * Two-button question; «no» is the default (Enter on the focus, Esc, a click outside).
 * @param {{ id: string, title: string, text?: string, yes: string, no: string }} o
 * @returns {Promise<boolean>} true = «yes»
 */
export function confirmBox(o) {
  return choiceBox({ id: o.id, title: o.title, text: o.text, cancel: "no", buttons: [{ key: "yes", id: "mm-yes", label: o.yes }, { key: "no", id: "mm-no", label: o.no }] }).then(k => k === "yes");
}
