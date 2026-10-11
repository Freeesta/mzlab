/* «Lettura facilitata» of Teoria and Pratica: the «Aa» button opens a side panel (fonts, spacing, backgrounds, reading ruler, concentration mode,
   reduced motion, reading aloud, numbers). Choices live in localStorage «qqq.a11y» (JSON of codes, written through QA11Y of tema.js, which also
   puts them on <html> as data-a11y-* before the page is drawn; the rules are in teoria.css). Loaded by teoria.js; no library. */
(function () {
  "use strict";
  const $ = (q, el = document) => el.querySelector(q);
  const R = document.documentElement;
  const Q = window.QA11Y || { read: () => ({}), apply() {}, write() {} };
  const embedded = window.self !== window.top;
  let S = Q.read();
  const save = () => Q.write(S);
  const speech = "speechSynthesis" in window && "SpeechSynthesisUtterance" in window;

  // ------------------------------------------------------------------ panel
  const OPT = {
    font: [["", "Predefinito"], ["atkinson", "Atkinson Hyperlegible Next"], ["dyslexic", "OpenDyslexic"]],
    bg: [["", "Come il tema"], ["cream", "Crema"], ["blue", "Azzurro tenue"], ["hc", "Alto contrasto"]],
    motion: [["auto", "Automatico (come il sistema)"], ["always", "Sempre ridotte"]],
    rate: [["0.8", "0,8"], ["1", "1"], ["1.2", "1,2"]],
  };
  const radios = (name, cur) => OPT[name].map(([v, t]) => `<label><input type="radio" name="a11y-${name}" value="${v}"${(cur || "") === v ? " checked" : ""}> <span>${t}</span></label>`).join("");
  const check = (key, text, hint) => `<label><input type="checkbox" data-k="${key}"${S[key] ? " checked" : ""}> <span>${text}</span></label>${hint ? `<div class="hint">${hint}</div>` : ""}`;

  const btn = document.createElement("button");
  btn.id = "a11y-btn"; btn.type = "button"; btn.textContent = "Aa"; btn.title = "Lettura facilitata";
  btn.setAttribute("aria-label", "Lettura facilitata"); btn.setAttribute("aria-haspopup", "dialog"); btn.setAttribute("aria-expanded", "false");
  const top = $("#top");
  if (top && !embedded) top.appendChild(btn); else { btn.classList.add("float"); document.body.appendChild(btn); }

  const panel = document.createElement("div");
  panel.id = "a11y-panel"; panel.hidden = true; panel.setAttribute("role", "dialog"); panel.setAttribute("aria-label", "Lettura facilitata");
  panel.innerHTML = `<button type="button" class="x" aria-label="Chiudi">&times;</button><h2>Lettura facilitata</h2>
    <div class="hint">Per ingrandire il testo usa ⌘+ / Ctrl+ (zoom del browser). Le scelte restano su questo computer.</div>
    <fieldset><legend>Carattere</legend>${radios("font", S.font)}</fieldset>
    <fieldset><legend>Spaziatura</legend>${check("space", "Spaziatura ampia", "Righe e lettere più distanziate, testo a sinistra, righe di al massimo 65 caratteri.")}${check("nums", "Numeri più leggibili", "Cifre di larghezza uguale e più spazio nelle equazioni.")}</fieldset>
    <fieldset><legend>Sfondo</legend>${radios("bg", S.bg)}</fieldset>
    <fieldset><legend>Lettura</legend>${check("ruler", "Righello di lettura", "Una banda segue il mouse o il focus; il resto è attenuato.")}${check("focus", "Modalità concentrazione (tasto F)", "Nasconde indice e menu e mostra l'avanzamento.")}</fieldset>
    <fieldset><legend>Animazioni</legend>${radios("motion", S.motion || "auto")}</fieldset>
    ${speech ? `<fieldset id="a11y-tts"><legend>Lettura ad alta voce</legend><div class="hint">▶ accanto ai titoli legge la sezione.</div>
      <div class="row"><button type="button" id="a11y-sel">Leggi la selezione</button><button type="button" id="a11y-stop">Stop</button></div>
      <div class="hint">Velocità</div>${OPT.rate.map(([v, t]) => `<label><input type="radio" name="a11y-rate" value="${v}"${(S.rate || "1") === v ? " checked" : ""}> <span>${t}×</span></label>`).join("")}</fieldset>` : ""}
    <div class="row"><button type="button" id="a11y-reset">Ripristina</button></div>`;
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
    // F = concentration mode (not while typing, not with modifiers)
    if ((e.key === "f" || e.key === "F") && !e.ctrlKey && !e.metaKey && !e.altKey && !/^(INPUT|TEXTAREA|SELECT)$/.test((e.target.tagName || "")) && !e.target.isContentEditable) {
      S.focus = !S.focus; save(); sync();
    }
  });

  panel.addEventListener("change", e => {
    const t = e.target;
    if (t.type === "checkbox") S[t.dataset.k] = t.checked;
    else if (t.name === "a11y-font") S.font = t.value;
    else if (t.name === "a11y-bg") S.bg = t.value;
    else if (t.name === "a11y-motion") S.motion = t.value;
    else if (t.name === "a11y-rate") S.rate = t.value;
    save(); sync();
    if (t.name === "a11y-bg") dispatchEvent(new Event("resize"));   // the figures draw again with the new colours
  });
  $("#a11y-reset", panel).onclick = () => { S = {}; stop(); save(); sync(); dispatchEvent(new Event("resize")); };
  function sync() {   // panel controls follow S (after Reset or the F key)
    panel.querySelectorAll("input[type=checkbox]").forEach(c => { c.checked = !!S[c.dataset.k]; });
    [["font", S.font || ""], ["bg", S.bg || ""], ["motion", S.motion || "auto"], ["rate", S.rate || "1"]].forEach(([n, v]) => panel.querySelectorAll(`input[name=a11y-${n}]`).forEach(r => { r.checked = r.value === v; }));
    onScroll();
  }

  // ------------------------------------------------------------------ reading ruler and progress bar
  const ruler = document.createElement("div"); ruler.id = "a11y-ruler"; ruler.setAttribute("aria-hidden", "true"); document.body.appendChild(ruler);
  const prog = document.createElement("div"); prog.id = "a11y-prog"; prog.setAttribute("aria-hidden", "true"); document.body.appendChild(prog);
  function lineH() { const m = $("main"); const lh = parseFloat(getComputedStyle(m || document.body).lineHeight); return isFinite(lh) ? lh : 26; }
  function band(y) { const h = lineH() * 2.5; ruler.style.setProperty("--rh", h + "px"); ruler.style.setProperty("--ry", Math.max(0, y - h / 2) + "px"); }
  addEventListener("mousemove", e => { if (S.ruler) band(e.clientY); }, { passive: true });
  document.addEventListener("focusin", e => { if (S.ruler && !panel.contains(e.target)) { const r = e.target.getBoundingClientRect(); band(r.top + Math.min(r.height, 60) / 2); } });
  function onScroll() {
    const m = $("main"); if (!m || !S.focus) { prog.style.width = "0"; return; }
    const r = m.getBoundingClientRect(), tot = Math.max(1, r.height - innerHeight * 0.6);
    prog.style.width = Math.max(0, Math.min(100, (-r.top / tot) * 100)) + "%";
  }
  addEventListener("scroll", onScroll, { passive: true });
  band(innerHeight / 2);

  // ------------------------------------------------------------------ reading aloud (Web Speech API, voice it-IT)
  const HL = "highlights" in CSS ? "a11y-sentence" : null;
  function clearHl() { if (HL) CSS.highlights.delete(HL); document.querySelectorAll(".a11y-hlblock").forEach(x => x.classList.remove("a11y-hlblock")); }
  function stop() { if (speech) speechSynthesis.cancel(); clearHl(); queue = []; }
  let queue = [];

  /** Text of a block with a map char index -> text node. Formulas: aria-label if there is one, otherwise skipped. */
  function blockText(el) {
    let text = ""; const map = [];
    (function walk(n) {
      if (n.nodeType === 3) { map.push({ node: n, start: text.length, len: n.data.length }); text += n.data; return; }
      if (n.nodeType !== 1 || /^(SCRIPT|STYLE|SVG|CANVAS|BUTTON|SELECT|INPUT)$/i.test(n.tagName) || n.classList.contains("a11y-say") || n.getAttribute("aria-hidden") === "true") return;
      if (n.classList.contains("eq") || n.hasAttribute("aria-label")) { const l = n.getAttribute("aria-label"); if (l) text += " " + l + " "; return; }
      n.childNodes.forEach(walk);
    })(el);
    return { text, map };
  }
  function highlight(el, map, a, b) {
    if (!HL) { el.classList.add("a11y-hlblock"); return; }
    const find = i => map.find(m => i >= m.start && i <= m.start + m.len);
    const s = find(a), e = find(b); if (!s || !e) { el.classList.add("a11y-hlblock"); return; }
    const r = new Range(); r.setStart(s.node, Math.min(s.len, a - s.start)); r.setEnd(e.node, Math.min(e.len, b - e.start));
    CSS.highlights.set(HL, new Highlight(r));
  }
  function utter(text, el, map) {
    const u = new SpeechSynthesisUtterance(text.replace(/\s+/g, " ").trim()); u.lang = "it-IT"; u.rate = +(S.rate || 1);
    const v = speechSynthesis.getVoices().find(x => /^it/i.test(x.lang)); if (v) u.voice = v;
    if (el) {
      u.onstart = () => { clearHl(); if (!HL) el.classList.add("a11y-hlblock"); };
      u.onboundary = ev => {   // sentence around the word being read (the browser may not send boundaries: then the whole block stays marked)
        if (ev.name && ev.name !== "word") return;
        const t = text; let a = ev.charIndex, b = ev.charIndex;
        while (a > 0 && !/[.!?;:]\s/.test(t.slice(a - 2, a))) a--;
        while (b < t.length && !/[.!?]/.test(t[b])) b++;
        highlight(el, map, a, Math.min(t.length, b + 1));
      };
    }
    u.onend = u.onerror = () => next();
    return u;
  }
  function next() { clearHl(); const q = queue.shift(); if (q) speechSynthesis.speak(q); }
  function speakBlocks(blocks) {
    stop();
    queue = blocks.map(el => { const { text, map } = blockText(el); return text.trim() ? utter(text, el, map) : null; }).filter(Boolean);
    next();
  }
  function sectionBlocks(h) {   // from the heading to the next heading of the same or higher level
    const lvl = +h.tagName[1], out = [h]; let n = h.nextElementSibling;
    while (n && !(/^H[1-6]$/.test(n.tagName) && +n.tagName[1] <= lvl)) { if (n.matches("p,li,ul,ol,dl,blockquote,.box,.lead,.obj,.tw")) out.push(...(n.matches("ul,ol,dl,.box,.obj,.tw") ? [...n.querySelectorAll("p,li,dd,dt")] : [n])); n = n.nextElementSibling; }
    return out;
  }
  if (speech) {
    speechSynthesis.getVoices();
    document.querySelectorAll("main h2, main h3").forEach(h => {
      const b = document.createElement("button"); b.type = "button"; b.className = "a11y-say"; b.textContent = "▶";
      b.setAttribute("aria-label", "Leggi ad alta voce: " + h.textContent.trim()); b.title = "Leggi la sezione";
      b.onclick = ev => { ev.preventDefault(); ev.stopPropagation(); speakBlocks(sectionBlocks(h)); };
      h.appendChild(b);
    });
    $("#a11y-sel", panel).onclick = () => { const t = String(getSelection()).trim(); if (!t) return; stop(); speechSynthesis.speak(utter(t, null, null)); };
    $("#a11y-stop", panel).onclick = stop;
    addEventListener("pagehide", stop);
  }
  window.A11Y = { open, close, state: () => S, speakBlocks };
  sync();
})();
