/* mzLab - Teoria: didactic tools in a side panel (TEORIA.strumenti.massa(), .tavola(), .frammenta()). Classic script, no network:
   it works from file:// too. The panel is loaded with every page, each tool (strumenti/*.js and its data) only when it is opened.
   The tools only compute: they never say what a signal is. */
"use strict";
(function () {
  const TEORIA = (window.TEORIA = window.TEORIA || {});
  const DIR = document.currentScript && document.currentScript.src ? document.currentScript.src.replace(/[^/]*$/, "") : "";
  const defs = {}, loaded = {};                       // defs[name] = { titolo, monta(body, arg) } registered by the tool script
  const CSS = `
#strum{position:fixed;top:0;right:0;bottom:0;z-index:250;width:min(560px,100%);display:flex;flex-direction:column;background:var(--panel);color:var(--ink);border-left:1px solid var(--line);box-shadow:-8px 0 28px rgba(0,0,0,.25)}
#strum[hidden]{display:none}
#strum .sh{display:flex;gap:8px;align-items:center;padding:10px 12px;border-bottom:1px solid var(--line)}
#strum .sh h2{flex:1;margin:0;font-size:1.05rem;border:0;padding:0}
#strum .sx{min-height:44px;min-width:44px;font:inherit;color:var(--ink);background:var(--panel);border:1px solid var(--line);border-radius:8px;cursor:pointer;padding:0 12px}
#strum .sb{overflow:auto;padding:10px 14px 24px;flex:1}
#strum .sx:focus-visible,#strum input:focus-visible,#strum .sb button:not(.pt-c):focus-visible{outline:3px solid var(--accent);outline-offset:2px}
#strum .sb input[type=text]{min-height:44px}
#strum .sb .sim{margin:12px 0}
#strum .sb table{border-collapse:collapse;width:100%;font-size:14px}#strum .sb th,#strum .sb td{border-bottom:1px solid var(--line);padding:4px 6px;text-align:left}
#strum .sb td.n,#strum .sb th.n{text-align:right;font-variant-numeric:tabular-nums}
@media (max-width:700px){#strum{width:100%;border-left:0}}`;
  let el, body, title, opener = null, open = false, cur = "", seq = 0;

  function build() {
    const st = document.createElement("style"); st.textContent = CSS; document.head.appendChild(st);
    el = document.createElement("aside"); el.id = "strum"; el.hidden = true; el.setAttribute("role", "dialog"); el.setAttribute("aria-modal", "false");
    el.innerHTML = `<div class="sh"><h2 id="strum-t"></h2><button type="button" class="sx" aria-label="Chiudi lo strumento">Chiudi</button></div><div class="sb" tabindex="-1"></div>`;
    el.setAttribute("aria-labelledby", "strum-t");
    document.body.appendChild(el);
    title = el.querySelector("h2"); body = el.querySelector(".sb");
    el.querySelector(".sx").onclick = () => chiudi();
    el.addEventListener("keydown", e => {
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); chiudi(); }
      else if (e.key === "Tab") {   // focus stays in the panel while it is open on a narrow screen (it covers the page)
        if (!matchMedia("(max-width:700px)").matches) return;
        const z = [...el.querySelectorAll("button,input,select,textarea,[tabindex]:not([tabindex='-1'])")].filter(x => !x.disabled && x.offsetParent);
        if (!z.length) return;
        if (e.shiftKey && document.activeElement === z[0]) { e.preventDefault(); z[z.length - 1].focus(); }
        else if (!e.shiftKey && document.activeElement === z[z.length - 1]) { e.preventDefault(); z[0].focus(); }
      }
    });
  }
  function script(src) {
    return new Promise((ok, ko) => {
      const s = document.createElement("script"); s.src = src; s.onload = ok; s.onerror = () => ko(new Error(src)); document.head.appendChild(s);
    });
  }
  /** Loads what a tool needs (once) and opens it. needs: [[test, src]] scripts in order; tool: file in strumenti/. */
  async function apri(nome, arg) {
    if (!el) build();
    const mine = ++seq;
    if (!open) { opener = document.activeElement; open = true; }
    el.hidden = false;
    if (cur !== nome) { title.textContent = ""; body.innerHTML = '<p class="note">Caricamento…</p>'; }
    try {
      if (!loaded[nome]) {
        loaded[nome] = (async () => {
          await script(DIR + nome + ".js");
          for (const [test, src] of defs[nome].needs || []) if (!test()) await script(DIR + src);
        })();
      }
      await loaded[nome];
    } catch (_) { loaded[nome] = null; if (mine === seq) body.innerHTML = '<p class="note">Strumento non disponibile.</p>'; return; }
    if (mine !== seq) return;
    cur = nome; title.textContent = defs[nome].titolo; body.innerHTML = "";
    defs[nome].monta(body, arg);
    body.focus({ preventScroll: true });
    const first = body.querySelector("input,button,select"); if (first) first.focus({ preventScroll: true });
  }
  function chiudi(back = true) {
    if (!open) return;
    open = false; seq++; cur = ""; el.hidden = true; body.innerHTML = "";
    if (back && opener && opener.focus && document.contains(opener)) opener.focus({ preventScroll: true });
  }
  TEORIA.strumenti = {
    massa: formula => apri("massa", formula),
    tavola: simbolo => apri("tavola", simbolo),
    frammenta: esempio => apri("frammenta", esempio),
    chiudi, aperto: () => open ? cur : "",
    registra(nome, def) { defs[nome] = def; },        // called by the tool scripts
  };
})();
