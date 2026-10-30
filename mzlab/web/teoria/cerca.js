/* mzLab - Teoria: search dialog (Ctrl/⌘K or TEORIA.cerca()). Classic script, no network: it works from file:// too.
   The index (indice-ricerca.js, from tools/genera_indice_teoria.py) is read only the first time the dialog opens.
   Matching: every word typed must be the start of a word of the entry (accent and case insensitive). Results are grouped by chapter. */
"use strict";
(function () {
  const TEORIA = (window.TEORIA = window.TEORIA || {});
  const BASE = document.currentScript && document.currentScript.src ? document.currentScript.src.replace(/[^/]*$/, "") : "";
  const here = location.pathname.split("/").pop() || "index.html";
  const MAX = 40, TYPE = { capitolo: "Capitolo", pratica: "Pratica", sezione: "Sezione", glossario: "Glossario", esercizio: "Verifica", simulazione: "Figura interattiva" };
  const BONUS = { capitolo: 4, pratica: 4, glossario: 3, sezione: 2, simulazione: 1, esercizio: 1 };

  // ---------------------------------------------------------------- pure helpers (also used by the tests)
  const fold = s => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");
  const words = s => " " + fold(s).replace(/[^a-z0-9+/.]+/g, " ").trim() + " ";
  function tokens(q) { return fold(q).split(/[^a-z0-9+/.]+/).filter(Boolean); }
  /** Score of one entry [text, type, url, chapter, searchString] for the tokens; 0 = not a match. */
  function score(e, toks) {
    const title = e._t || (e._t = words(e[0])), all = e._a || (e._a = words(e[4]));
    let s = 0;
    for (const t of toks) {
      if (title.includes(" " + t + " ")) s += 4;
      else if (title.includes(" " + t)) s += 3;
      else if (all.includes(" " + t + " ")) s += 2;
      else if (all.includes(" " + t)) s += 1;
      else return 0;
    }
    return s + (BONUS[e[1]] || 0) / 10;
  }
  function search(index, q) {
    const toks = tokens(q);
    if (!toks.length || (toks.length === 1 && toks[0].length < 2)) return [];
    const hits = [];
    index.forEach((e, i) => { const s = score(e, toks); if (s) hits.push([s, i, e]); });
    hits.sort((a, b) => b[0] - a[0] || a[1] - b[1]);
    const groups = new Map();
    for (const [, , e] of hits.slice(0, MAX)) { if (!groups.has(e[3])) groups.set(e[3], []); groups.get(e[3]).push(e); }
    return [...groups].map(([cap, items]) => ({ cap, items }));
  }
  /** HTML of `text` with the typed words marked (matching ignores accents and case). */
  function mark(text, toks) {
    const map = []; let n = "";
    [...text].forEach((c, i) => { const f = fold(c); for (let k = 0; k < f.length; k++) { n += f[k]; map.push(i); } });
    const on = new Set(), chars = [...text];
    for (const t of toks) {
      for (let p = n.indexOf(t); p >= 0; p = n.indexOf(t, p + 1)) {
        if (p === 0 || !/[a-z0-9+/.]/.test(n[p - 1])) for (let k = p; k < p + t.length; k++) on.add(map[k]);
      }
    }
    let out = "", open = false;
    chars.forEach((c, i) => {
      const m = on.has(i); if (m !== open) { out += m ? "<mark>" : "</mark>"; open = m; }
      out += c === "<" ? "&lt;" : c === ">" ? "&gt;" : c === "&" ? "&amp;" : c === '"' ? "&quot;" : c;
    });
    return out + (open ? "</mark>" : "");
  }
  TEORIA._cerca = { fold, tokens, score, search, mark };

  // ---------------------------------------------------------------- index (lazy)
  let indexP = null;
  function loadIndex() {
    if (typeof INDICE_RICERCA !== "undefined") return Promise.resolve(INDICE_RICERCA);
    return indexP || (indexP = new Promise((ok, ko) => {
      const s = document.createElement("script"); s.src = BASE + "indice-ricerca.js";
      s.onload = () => ok(INDICE_RICERCA); s.onerror = () => { indexP = null; ko(new Error("indice")); };
      document.head.appendChild(s);
    }));
  }

  // ---------------------------------------------------------------- dialog
  const CSS = `
#cerca-bg{position:fixed;inset:0;z-index:300;background:rgba(0,0,0,.45);display:flex;align-items:flex-start;justify-content:center;padding:8vh 12px 12px}
#cerca-bg[hidden]{display:none}
#cerca{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:12px;width:min(640px,100%);max-height:84vh;display:flex;flex-direction:column;box-shadow:0 12px 40px rgba(0,0,0,.35)}
#cerca .ch{display:flex;gap:8px;align-items:center;padding:10px 12px;border-bottom:1px solid var(--line)}
#cerca input{flex:1;min-width:0;min-height:44px;font:inherit;color:var(--ink);background:var(--bg);border:1px solid var(--line);border-radius:8px;padding:6px 12px}
#cerca button.cx{min-height:44px;min-width:44px;font:inherit;color:var(--ink);background:var(--panel);border:1px solid var(--line);border-radius:8px;cursor:pointer;padding:0 12px}
#cerca input:focus-visible,#cerca button:focus-visible{outline:3px solid var(--accent);outline-offset:2px}
#cerca-list{overflow:auto;padding:4px 8px 10px;margin:0;list-style:none}
#cerca-list [role=group]{margin:0;padding:0}
#cerca-list .gh{font-size:.78rem;letter-spacing:.04em;text-transform:uppercase;color:var(--muted);padding:10px 8px 2px}
#cerca-list [role=option]{display:flex;flex-direction:column;justify-content:center;min-height:44px;padding:6px 10px;border-radius:8px;cursor:pointer;border:2px solid transparent}
#cerca-list [role=option] small{color:var(--muted)}
#cerca-list [role=option][aria-selected=true]{background:var(--sel);border-color:var(--accent)}
#cerca-list mark{background:transparent;color:var(--accent);font-weight:700;text-decoration:underline}
#cerca-st{padding:8px 14px;color:var(--muted);font-size:.9rem}
@media (forced-colors:active){#cerca-list [role=option][aria-selected=true]{outline:2px solid Highlight}}
`;
  let bg, box, input, list, status, opener = null, open = false, flat = [], cur = -1, seq = 0;

  function build() {
    const st = document.createElement("style"); st.textContent = CSS; document.head.appendChild(st);
    bg = document.createElement("div"); bg.id = "cerca-bg"; bg.hidden = true;
    bg.innerHTML = `<div id="cerca" role="dialog" aria-modal="true" aria-label="Cerca nella Teoria">
<div class="ch"><input id="cerca-in" type="search" role="combobox" aria-expanded="false" aria-controls="cerca-list" aria-autocomplete="list" aria-label="Cerca nella Teoria" placeholder="Cerca: frammentazione, addotto, ESI…" autocomplete="off" spellcheck="false" enterkeyhint="go"><button type="button" class="cx" aria-label="Chiudi la ricerca">Chiudi</button></div>
<div id="cerca-st" role="status" aria-live="polite"></div>
<ul id="cerca-list" role="listbox" aria-label="Risultati"></ul></div>`;
    document.body.appendChild(bg);
    box = bg.firstElementChild; input = box.querySelector("input"); list = box.querySelector("#cerca-list"); status = box.querySelector("#cerca-st");
    bg.addEventListener("mousedown", e => { if (e.target === bg) close(); });
    box.querySelector(".cx").onclick = () => close();
    input.addEventListener("input", () => run());
    list.addEventListener("click", e => { const o = e.target.closest("[role=option]"); if (o) go(flat[+o.dataset.i]); });
    list.addEventListener("mousemove", e => { const o = e.target.closest("[role=option]"); if (o && +o.dataset.i !== cur) select(+o.dataset.i, false); });
    box.addEventListener("keydown", onKey);
  }

  function onKey(e) {
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); }
    else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault(); if (!flat.length) return;
      select(e.key === "ArrowDown" ? (cur + 1) % flat.length : (cur - 1 + flat.length) % flat.length, true);
    } else if (e.key === "Enter") { e.preventDefault(); if (flat[cur]) go(flat[cur]); }
    else if (e.key === "Tab") {   // focus trap: input <-> close button
      const z = [input, box.querySelector(".cx")], i = z.indexOf(document.activeElement);
      e.preventDefault(); z[(i + (e.shiftKey ? -1 : 1) + z.length) % z.length].focus();
    }
  }
  function select(i, scroll) {
    cur = i;
    list.querySelectorAll("[role=option]").forEach(o => o.setAttribute("aria-selected", String(+o.dataset.i === i)));
    const o = list.querySelector(`[data-i="${i}"]`);
    if (o) { input.setAttribute("aria-activedescendant", o.id); if (scroll) o.scrollIntoView({ block: "nearest" }); } else input.removeAttribute("aria-activedescendant");
  }
  function go(e) {
    if (!e) return;
    close(false);
    const file = e[2].split("#")[0];
    if (file === here && e[2].includes("#")) { location.hash = e[2].split("#")[1]; const t = document.getElementById(e[2].split("#")[1]); if (t) t.scrollIntoView({ behavior: "auto", block: "start" }); }
    else location.href = e[2];
  }
  function run() {
    const q = input.value, mine = ++seq;
    loadIndex().then(idx => {
      if (mine !== seq) return;
      const toks = tokens(q), groups = search(idx, q);
      flat = []; let h = "";
      groups.forEach((g, gi) => {
        h += `<li role="presentation"><div role="group" aria-labelledby="cg${gi}"><div class="gh" id="cg${gi}">${g.cap.replace(/&/g, "&amp;")}</div>`;
        g.items.forEach(e => {
          const i = flat.push(e) - 1;
          h += `<div role="option" id="co${i}" data-i="${i}" aria-selected="false"><span>${mark(e[0], toks)}</span><small>${TYPE[e[1]] || e[1]}</small></div>`;
        });
        h += "</div></li>";
      });
      list.innerHTML = h;
      input.setAttribute("aria-expanded", String(flat.length > 0));
      status.textContent = !toks.length ? "Scrivi almeno due lettere." : flat.length ? `${flat.length} ${flat.length === 1 ? "risultato" : "risultati"}` : "Nessun risultato.";
      select(flat.length ? 0 : -1, false);
    }).catch(() => { status.textContent = "Indice di ricerca non disponibile."; });
  }

  function openDialog(q) {
    if (!bg) build();
    if (!open) { opener = document.activeElement; open = true; bg.hidden = false; document.documentElement.setAttribute("data-cerca", ""); }
    input.value = typeof q === "string" ? q : input.value;
    input.focus(); input.select(); run();
  }
  function close(back = true) {
    if (!open) return;
    open = false; bg.hidden = true; document.documentElement.removeAttribute("data-cerca"); seq++;
    if (back && opener && opener.focus && document.contains(opener)) opener.focus({ preventScroll: true });
  }
  TEORIA.cerca = openDialog;
  TEORIA.cerca.chiudi = close;
  TEORIA.cerca.aperta = () => open;

  document.addEventListener("keydown", e => {
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && (e.key || "").toLowerCase() === "k") { e.preventDefault(); open ? close() : openDialog(""); }
  });
})();
