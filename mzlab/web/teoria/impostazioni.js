/* Settings of Teoria and Pratica (the gear opens a side panel): theme, easy reading (fonts, spacing, backgrounds, reading ruler,
   reduced motion, numbers) and one «Ripristina predefinite» that touches only the settings. Nothing of the data view lives here. Choices live in localStorage «qqq.a11y» (JSON of codes, written through QA11Y of tema.js, which also
   puts them on <html> as data-a11y-* before the page is drawn; the rules are in teoria.css). Loaded by teoria.js; no library. */
(function () {
  "use strict";
  const $ = (q, el = document) => el.querySelector(q);
  const R = document.documentElement;
  const Q = window.QA11Y || { read: () => ({}), apply() {}, write() {} };
  const embedded = window.self !== window.top;
  let S = Q.read();
  const save = () => Q.write(Object.assign({}, S, { noside: Q.read().noside }));   // «noside» (index hidden) belongs to the toggle of comandi.js

  // ------------------------------------------------------------------ panel
  const OPT = {
    font: [["", "Predefinito"], ["atkinson", "Atkinson Hyperlegible Next"], ["dyslexic", "OpenDyslexic"]],
    bg: [["", "Come il tema"], ["cream", "Crema"], ["blue", "Azzurro tenue"], ["hc", "Alto contrasto"]],
    motion: [["auto", "Automatico"], ["always", "Sempre ridotte"]],
  };
  OPT.tema = [["auto", "Come il sistema"], ["light", "Chiaro"], ["dark", "Scuro"]];
  const tema = () => (window.QTEMA ? QTEMA.get() : "auto");
  const radios = (name, cur) => OPT[name].map(([v, t]) => `<label><input type="radio" name="a11y-${name}" value="${v}"${(cur || "") === v ? " checked" : ""}> <span>${t}</span></label>`).join("");
  const check = (key, text, hint) => `<label${hint ? ` title="${hint}"` : ""}><input type="checkbox" data-k="${key}"${S[key] ? " checked" : ""}> <span>${text}</span></label>`;

  const btn = document.createElement("button");
  btn.id = "a11y-btn"; btn.type = "button"; btn.textContent = "Aa"; btn.title = "Impostazioni";
  btn.setAttribute("aria-label", "Impostazioni"); btn.setAttribute("aria-haspopup", "dialog"); btn.setAttribute("aria-expanded", "false");
  const top = $("#top"), tbar = $("#tbar");
  if (tbar && !embedded) {   // standalone page: the gear closes the bar of comandi.js
    btn.className = "tb"; btn.innerHTML = window.TCMD.icona("impostazioni") + '<span class="l">Impostazioni</span>'; btn.title = "Impostazioni";
    btn.setAttribute("aria-label", "Impostazioni");
    tbar.appendChild(btn);
  } else if (top && !embedded) top.appendChild(btn); else { btn.classList.add("float"); document.body.appendChild(btn); }

  const panel = document.createElement("div");
  panel.id = "a11y-panel"; panel.hidden = true; panel.setAttribute("role", "dialog"); panel.setAttribute("aria-label", "Impostazioni");
  panel.innerHTML = `<button type="button" class="x" aria-label="Chiudi">&times;</button><h2 title="Le scelte restano su questo computer">Impostazioni</h2>
    <fieldset><legend>Tema</legend>${radios("tema", tema())}</fieldset>
    <fieldset><legend>Carattere</legend>${radios("font", S.font)}</fieldset>
    <fieldset><legend>Spaziatura</legend>${check("space", "Spaziatura ampia", "Più aria tra righe e lettere")}${check("nums", "Numeri leggibili", "Cifre della stessa larghezza")}</fieldset>
    <fieldset><legend>Sfondo</legend>${radios("bg", S.bg)}</fieldset>
    <fieldset><legend>Lettura</legend>${check("ruler", "Righello di lettura", "Attenua il resto della pagina")}</fieldset>
    <fieldset><legend>Animazioni</legend>${radios("motion", S.motion || "auto")}</fieldset>
    <div class="row"><button type="button" id="a11y-reset" title="Riporta testo, tema e colori ai valori iniziali">Ripristina predefinite</button></div>`;
  document.body.appendChild(panel);

  let lastFocus = null;
  function open() { lastFocus = document.activeElement; panel.hidden = false; btn.setAttribute("aria-expanded", "true"); (panel.querySelector("input:checked") || panel.querySelector("input,button")).focus(); }
  function close() { panel.hidden = true; btn.setAttribute("aria-expanded", "false"); (lastFocus && lastFocus.focus ? lastFocus : btn).focus(); }
  btn.onclick = () => (panel.hidden ? open() : close());
  $(".x", panel).onclick = close;
  panel.addEventListener("keydown", e => {
    if (e.key === "Escape") { e.stopPropagation(); close(); return; }
    if (e.key !== "Tab") return;
    const f = [...panel.querySelectorAll("input,button")].filter(x => !x.disabled), a = f[0], z = f[f.length - 1];
    if (e.shiftKey && document.activeElement === a) { e.preventDefault(); z.focus(); } else if (!e.shiftKey && document.activeElement === z) { e.preventDefault(); a.focus(); }
  });
  document.addEventListener("keydown", e => {
    if (e.key === "Escape" && !panel.hidden) close();
  });

  panel.addEventListener("change", e => {
    const t = e.target;
    if (t.type === "checkbox") S[t.dataset.k] = t.checked;
    else if (t.name === "a11y-font") S.font = t.value;
    else if (t.name === "a11y-bg") S.bg = t.value;
    else if (t.name === "a11y-motion") S.motion = t.value;
    else if (t.name === "a11y-tema") { if (window.QTEMA) QTEMA.set(t.value); return; }
    save(); sync();
    if (t.name === "a11y-bg") dispatchEvent(new Event("resize"));   // the figures draw again with the new colours
  });
  // only the settings go back to the factory values (theme included): the progress of the Pratica and the reading positions are never touched
  $("#a11y-reset", panel).onclick = () => {
    S = {}; Q.write({}); if (window.QTEMA && QTEMA.get() !== "auto") QTEMA.set("auto");
    sync(); document.dispatchEvent(new Event("tcmd-stato")); dispatchEvent(new Event("resize"));
  };
  function sync() {   // panel controls follow S (after Reset)
    panel.querySelectorAll("input[type=checkbox]").forEach(c => { c.checked = !!S[c.dataset.k]; });
    [["font", S.font || ""], ["bg", S.bg || ""], ["motion", S.motion || "auto"], ["tema", tema()]].forEach(([n, v]) => panel.querySelectorAll(`input[name=a11y-${n}]`).forEach(r => { r.checked = r.value === v; }));
  }

  // ------------------------------------------------------------------ reading ruler
  const ruler = document.createElement("div"); ruler.id = "a11y-ruler"; ruler.setAttribute("aria-hidden", "true"); document.body.appendChild(ruler);
  function lineH() { const m = $("main"); const lh = parseFloat(getComputedStyle(m || document.body).lineHeight); return isFinite(lh) ? lh : 26; }
  function band(y) { const h = lineH() * 2.5; ruler.style.setProperty("--rh", h + "px"); ruler.style.setProperty("--ry", Math.max(0, y - h / 2) + "px"); }
  addEventListener("mousemove", e => { if (S.ruler) band(e.clientY); }, { passive: true });
  document.addEventListener("focusin", e => { if (S.ruler && !panel.contains(e.target)) { const r = e.target.getBoundingClientRect(); band(r.top + Math.min(r.height, 60) / 2); } });
  band(innerHeight / 2);

  window.A11Y = { open, close, state: () => S };
  sync();
})();
